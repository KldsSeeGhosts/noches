//! In-app remote access: the pairing code, the QR rendering, and the process-wide
//! gateway that the desktop runs for a paired phone.
//!
//! One process can show several windows, so the gateway is a process-wide
//! singleton rather than per-window state: the first window that needs it binds
//! `{tailnet}:{GATEWAY_PORT}` and every other window observes the same status
//! through [`status`]. The pairing code itself is never stored here — it lives
//! only in the page that minted it.

use crate::state::EngineBootConfig;
use gpui::{App, ClipboardItem};
use std::{
    net::Ipv4Addr,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock, PoisonError},
};
use zeron_rpc::remote::{self, Credentials, GATEWAY_PORT, ServerOptions};

pub use zeron_rpc::remote::pair_client;

/// The credentials file the in-app gateway reads, under the app data directory.
/// Distinct from the CLI's `access.json`, so the two never fight over a file.
pub fn credentials_path(data_dir: &Path) -> PathBuf {
    data_dir.join("remote-access.json")
}

/// Whether remote access is serving, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayStatus {
    /// No gateway has been started in this process yet.
    NotRunning,
    /// Bound and serving.
    Running { endpoint: String },
    /// The last start attempt failed; the string is user-facing copy.
    Error { message: String },
}

impl GatewayStatus {
    /// One-line status text for the Connections page: the plain error, or the
    /// endpoint the phone dials.
    pub fn label(&self) -> Option<String> {
        match self {
            Self::NotRunning => None,
            Self::Running { endpoint } => Some(format!("Remote access on at {endpoint}")),
            Self::Error { message } => Some(message.clone()),
        }
    }
}

/// What a start attempt did, so the caller can report a real failure instead of
/// assuming success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayOutcome {
    Started { endpoint: String },
    AlreadyRunning { endpoint: String },
    Failed { message: String },
}

/// Handle to a running gateway: dropping it aborts `serve`, which drops the
/// listener and closes the port.
pub struct GatewayHandle {
    endpoint: String,
    /// The address the listener actually bound. Compared on every start so a
    /// Tailscale address change rebinds instead of silently serving the old one.
    bind: String,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl Drop for GatewayHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Default)]
struct GatewaySlot {
    handle: Option<GatewayHandle>,
    last_error: Option<String>,
}

fn slot() -> &'static Mutex<GatewaySlot> {
    static SLOT: OnceLock<Mutex<GatewaySlot>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(GatewaySlot::default()))
}

/// Current status without starting anything. A handle whose task panicked or
/// finished on its own counts as not running.
pub fn status() -> GatewayStatus {
    let mut slot = slot().lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(handle) = &slot.handle {
        if handle.task.is_finished() {
            slot.handle = None;
        } else {
            return GatewayStatus::Running {
                endpoint: handle.endpoint.clone(),
            };
        }
    }
    match &slot.last_error {
        Some(message) => GatewayStatus::Error {
            message: message.clone(),
        },
        None => GatewayStatus::NotRunning,
    }
}

/// Number of clients in the gateway's credentials file. `None` when the file
/// cannot be read.
pub fn client_count(data_dir: &Path) -> Option<usize> {
    Credentials::load(&credentials_path(data_dir))
        .ok()
        .map(|store| store.clients.len())
}

/// Every paired client, or the load error.
pub fn clients(data_dir: &Path) -> anyhow::Result<Vec<remote::ConnectionProfile>> {
    Ok(Credentials::load(&credentials_path(data_dir))?.clients)
}

/// Revoke one paired client and stop the gateway when it was the last one.
///
/// A live `serve` loop re-reads the credentials file every five seconds, so the
/// revoked key's sessions close on their own; stopping the listener here keeps
/// an empty gateway from holding the port with nothing to authorize.
pub fn revoke(data_dir: &Path, id: &str) -> anyhow::Result<bool> {
    let path = credentials_path(data_dir);
    remote::revoke_client(&path, id)?;
    let remaining = client_count(data_dir).unwrap_or(0);
    if remaining == 0 {
        stop();
    }
    Ok(remaining == 0)
}

/// Bind and serve, idempotently.
///
/// `upstream` must be this host's loopback engine WebSocket and `tailnet` the
/// address the phone dials; `remote::serve` enforces the first and the bind
/// failure is reported as-is for the second. Safe to call from any window.
pub fn ensure_running(
    cx: &App,
    upstream: String,
    tailnet: Ipv4Addr,
    credentials: PathBuf,
) -> GatewayOutcome {
    let mut slot = slot().lock().unwrap_or_else(PoisonError::into_inner);
    let endpoint = format!("ws://{tailnet}:{GATEWAY_PORT}");
    let bind = format!("{tailnet}:{GATEWAY_PORT}");
    if let Some(handle) = &slot.handle {
        if !handle.task.is_finished() {
            // Already serving exactly what was asked for.
            if handle.bind == bind {
                return GatewayOutcome::AlreadyRunning {
                    endpoint: handle.endpoint.clone(),
                };
            }
            // The computer's tailnet address changed: release the old listener
            // before rebinding, or the bind below reports `AddrInUse` for our
            // own gateway.
            slot.handle = None;
        } else {
            slot.handle = None;
        }
    }
    if !Credentials::load(&credentials).is_ok_and(|store| !store.clients.is_empty()) {
        let message = "Pair a phone before enabling remote access.".to_string();
        slot.last_error = Some(message.clone());
        return GatewayOutcome::Failed { message };
    }
    let options = ServerOptions::new(upstream, credentials);
    // The listener is bound synchronously so `AddrInUse` is a return value, not
    // a background log line the page can never show.
    let outcome = gpui_tokio::Tokio::handle(cx).block_on({
        let bind = bind.clone();
        async move {
            let listener = tokio::net::TcpListener::bind(&bind).await?;
            let task = tokio::spawn(remote::serve(listener, options));
            anyhow::Ok(task)
        }
    });
    match outcome {
        Ok(task) => {
            slot.last_error = None;
            slot.handle = Some(GatewayHandle {
                endpoint: endpoint.clone(),
                bind,
                task,
            });
            GatewayOutcome::Started { endpoint }
        }
        Err(error) => {
            let message = bind_failure_message(&bind, &error);
            slot.last_error = Some(message.clone());
            GatewayOutcome::Failed { message }
        }
    }
}

/// Plain copy for the two ways a start realistically fails: another gateway
/// already owns the port, or the address is not on this machine any more.
fn bind_failure_message(bind: &str, error: &anyhow::Error) -> String {
    let text = error.to_string();
    if let Some(source) = error.downcast_ref::<std::io::Error>() {
        if source.kind() == std::io::ErrorKind::AddrInUse {
            return format!(
                "Port {GATEWAY_PORT} is already in use on {bind}. Quit the other `noches-connect serve` on this computer, then try again."
            );
        }
        if source.kind() == std::io::ErrorKind::AddrNotAvailable {
            return format!(
                "Tailscale is not listening on {bind} any more. Reconnect Tailscale, then try again."
            );
        }
    }
    format!("Could not start remote access on {bind}: {text}")
}

/// Stop the gateway and release the port.
pub fn stop() {
    let mut slot = slot().lock().unwrap_or_else(PoisonError::into_inner);
    slot.handle = None;
    slot.last_error = None;
}

/// Start on app launch when a phone is already paired, so a restart does not
/// silently drop remote access. No-op for remote windows (they are clients, not
/// hosts) and when nothing is paired.
pub fn autostart(boot: &EngineBootConfig, cx: &App) {
    if boot.remote.is_some() {
        return;
    }
    let credentials = credentials_path(&boot.data_dir);
    if !Credentials::load(&credentials).is_ok_and(|store| !store.clients.is_empty()) {
        return;
    }
    let Some(tailnet) = remote::tailnet_ipv4() else {
        // Paired, but off the tailnet right now. Not an error the user needs on
        // launch: pairing again from the page reports it.
        return;
    };
    let outcomes = ensure_running(
        cx,
        format!("ws://127.0.0.1:{}", boot.ipc_port),
        tailnet,
        credentials,
    );
    tracing::info!(outcome = ?outcomes, "remote access autostart");
}

/// The computer's name for the pairing code's label. macOS prefers the user-set
/// ComputerName; elsewhere the hostname without the `.local` tail.
pub fn computer_name() -> String {
    #[cfg(target_os = "macos")]
    if let Some(name) = macos_computer_name() {
        return name;
    }
    hostname_without_local()
}

#[cfg(target_os = "macos")]
fn macos_computer_name() -> Option<String> {
    let output = std::process::Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8(output.stdout).ok()?;
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn hostname_without_local() -> String {
    // No hostname crate in this tree: the engine reads the same sources, from
    // `HOSTNAME` to the platform's own file.
    let host = std::env::var("HOSTNAME")
        .ok()
        .filter(|host| !host.trim().is_empty())
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
        .unwrap_or_else(|| "This computer".into());
    let host = host.trim().trim_end_matches('.').to_string();
    host.strip_suffix(".local")
        .map(str::to_string)
        .unwrap_or(host)
}

/// Copy text to the clipboard. Only the user's explicit Copy action reaches
/// this — the pairing key is never placed anywhere the user did not ask for.
pub fn copy_to_clipboard(cx: &App, text: String) {
    cx.write_to_clipboard(ClipboardItem::new_string(text));
}

/// The QR module matrix for a pairing code: `true` is a dark module, no quiet
/// zone (the renderer adds it). Error correction is M — the level the iOS
/// scanner and the CLI's `scripts/pairing-qr.swift` both use — which tolerates a
/// printed or photographed code without inflating the version.
pub fn qr_matrix(code: &str) -> anyhow::Result<(usize, Vec<bool>)> {
    let qr = qrcode::QrCode::with_error_correction_level(code.as_bytes(), qrcode::EcLevel::M)?;
    let width = qr.width();
    let bits = qr
        .to_colors()
        .into_iter()
        .map(|color| color == qrcode::Color::Dark)
        .collect::<Vec<_>>();
    anyhow::ensure!(bits.len() == width * width, "QR matrix is not square");
    Ok((width, bits))
}

/// Side length, in pixels, of the QR PNG for a pairing code.
///
/// Modules scale by a whole number of pixels (never a fraction, which would
/// blur the grid) at the largest step that lands at or under
/// [`QR_TARGET_SIDE`]. This is the only size the page draws the image at, so the
/// code is never resampled. A code that cannot be encoded falls back to the
/// target, where the renderer reports the real error.
pub const QR_TARGET_SIDE: u32 = 240;

pub fn qr_side(code: &str) -> u32 {
    let Ok((width, _)) = qr_matrix(code) else {
        return QR_TARGET_SIDE;
    };
    let total_modules = width as u32 + 8; // 4-module quiet zone on each side
    let mut scale = 1;
    while (scale + 1) * total_modules <= QR_TARGET_SIDE {
        scale += 1;
    }
    scale * total_modules
}

/// Render a pairing code as a PNG for the page: black modules on white with a
/// 4-module quiet zone (the QR spec's minimum), at [`qr_side`] pixels square.
pub fn qr_png(code: &str) -> anyhow::Result<Vec<u8>> {
    const QUIET: usize = 4;
    let (width, bits) = qr_matrix(code)?;
    let total_modules = width + QUIET * 2;
    let scale = qr_side(code) / total_modules as u32;
    let side = total_modules as u32 * scale;
    let mut png = std::io::Cursor::new(Vec::new());
    let mut canvas = image::GrayImage::from_pixel(side, side, image::Luma([255u8]));
    for row in 0..width {
        for column in 0..width {
            if !bits[row * width + column] {
                continue;
            }
            let x = (column + QUIET) as u32 * scale;
            let y = (row + QUIET) as u32 * scale;
            for dy in 0..scale {
                for dx in 0..scale {
                    canvas.put_pixel(x + dx, y + dy, image::Luma([0u8]));
                }
            }
        }
    }
    image::DynamicImage::ImageLuma8(canvas).write_to(&mut png, image::ImageFormat::Png)?;
    Ok(png.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_encodes_the_code_and_decodes_to_a_square_png() {
        let profile = remote::ConnectionProfile {
            id: "id".into(),
            name: "Studio Mac".into(),
            endpoint: "ws://100.114.177.75:27657".into(),
            token: "a".repeat(64),
            device_id: "device".into(),
        };
        let code = profile.code().unwrap();
        let bytes = qr_png(&code).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        // Square, and a whole number of pixels per module.
        let (width, _) = qr_matrix(&code).unwrap();
        assert_eq!(decoded.width(), decoded.height());
        assert_eq!(decoded.width() % (width as u32 + 8), 0);
        let scale = decoded.width() / (width as u32 + 8);
        // The 4-module quiet zone is white; the first finder pattern's outer
        // ring starts right after it, dark at its corner.
        let luma = decoded.to_luma8();
        let white = image::Luma([255u8]);
        for i in 0..4 * scale {
            for j in 0..4 * scale {
                assert_eq!(luma.get_pixel(i, j), &white, "quiet zone");
            }
        }
        assert_eq!(
            luma.get_pixel(4 * scale, 4 * scale),
            &image::Luma([0u8]),
            "finder pattern corner"
        );
        // The rendered side is exactly `qr_side`, and the page draws it 1:1.
        assert_eq!(decoded.width(), qr_side(&code));
        assert!(decoded.width() <= QR_TARGET_SIDE);
    }

    #[test]
    fn qr_matrix_marks_the_three_finder_patterns() {
        let (width, bits) = qr_matrix("noches-connect:test").unwrap();
        for (corner_x, corner_y) in [(0, 0), (width - 7, 0), (0, width - 7)] {
            // Each finder pattern's outer ring is dark on all four edges.
            for i in 0..7 {
                assert!(bits[corner_y * width + corner_x + i], "top edge");
                assert!(bits[(corner_y + 6) * width + corner_x + i], "bottom");
                assert!(bits[(corner_y + i) * width + corner_x], "left edge");
                assert!(bits[(corner_y + i) * width + corner_x + 6], "right edge");
            }
            // The 1-module white ring separates it from the dark centre.
            assert!(!bits[(corner_y + 1) * width + corner_x + 1], "white ring");
            assert!(bits[(corner_y + 3) * width + corner_x + 3], "dark centre");
        }
    }

    #[test]
    fn credentials_path_and_status_helpers_are_stable() {
        assert_eq!(
            credentials_path(Path::new("/data")),
            PathBuf::from("/data/remote-access.json")
        );
        assert_eq!(GatewayStatus::NotRunning.label(), None);
        assert_eq!(
            GatewayStatus::Running {
                endpoint: "ws://100.64.0.1:27657".into()
            }
            .label()
            .unwrap(),
            "Remote access on at ws://100.64.0.1:27657"
        );
        assert_eq!(
            GatewayStatus::Error {
                message: "nope".into()
            }
            .label()
            .unwrap(),
            "nope"
        );
    }

    #[test]
    fn bind_failures_name_the_real_cause() {
        let in_use = anyhow::Error::new(std::io::Error::from(std::io::ErrorKind::AddrInUse));
        let message = bind_failure_message("100.64.0.1:27657", &in_use);
        assert!(message.contains("already in use"), "{message}");
        assert!(message.contains("noches-connect serve"), "{message}");
        let unavailable =
            anyhow::Error::new(std::io::Error::from(std::io::ErrorKind::AddrNotAvailable));
        let message = bind_failure_message("100.64.0.1:27657", &unavailable);
        assert!(message.contains("Tailscale"), "{message}");
    }

    #[test]
    fn computer_name_is_a_usable_label() {
        let name = computer_name();
        assert!(!name.trim().is_empty());
        assert!(!name.ends_with(".local"), "{name}");
        assert!(name.len() <= 128, "{name}");
    }

    #[tokio::test]
    async fn client_count_and_revoke_round_trip_through_the_credentials_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = credentials_path(dir.path());
        let profile = remote::ConnectionProfile {
            id: "one".into(),
            name: "Phone".into(),
            endpoint: "ws://100.64.0.1:27657".into(),
            token: "b".repeat(64),
            device_id: "device".into(),
        };
        let mut store = Credentials::default();
        store.clients.push(profile);
        store.save(&path).unwrap();

        assert_eq!(client_count(dir.path()), Some(1));
        assert_eq!(clients(dir.path()).unwrap().len(), 1);
        // Revoking the last client also stops the gateway (a no-op here: none is
        // running in this test process).
        assert!(revoke(dir.path(), "one").unwrap());
        assert_eq!(client_count(dir.path()), Some(0));
        // Revoking an unknown id is an error and changes nothing.
        assert!(revoke(dir.path(), "one").is_err());
        assert_eq!(status(), GatewayStatus::NotRunning);
    }
}
