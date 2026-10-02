//! Shared, literal host settings used by Noches, standalone Pi and the CLI.
//! Settings select the installed binary and app compatibility, never authority.

use super::linux::{CuaResult, canonical_executable};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const HOST_KEYS: &[&str] = &["CUA_DRIVER_PATH", "CUA_HYPRLAND_LOCAL_PACKAGES"];
const DESKTOP_KEYS: &[&str] = &[
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "HYPRLAND_INSTANCE_SIGNATURE",
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
    "XAUTHORITY",
];

pub(super) fn parse_environment(text: &str, keys: &[&str]) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            if !keys.contains(&key) || value.is_empty() {
                return None;
            }
            // Only literal values. Never evaluate shell substitutions or escapes.
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

pub(super) fn settings() -> CuaResult<BTreeMap<String, String>> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    let Some(config) = config else {
        return Ok(BTreeMap::new());
    };
    let path = config.join("cua-driver/host.env");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(err) => return Err(format!("Cannot read {}: {err}", path.display())),
    };
    if text.len() > 16 * 1024 {
        return Err("CUA host settings exceed 16 KiB".into());
    }
    Ok(parse_environment(&text, HOST_KEYS))
}

/// A headless engine can precede the graphical login, or outlive a relogin.
/// Take only desktop connection variables from the current user manager.
pub(super) async fn desktop_environment() -> BTreeMap<String, String> {
    let mut command = tokio::process::Command::new("systemctl");
    command
        .args(["--user", "show-environment"])
        .kill_on_drop(true);
    match tokio::time::timeout(std::time::Duration::from_secs(2), command.output()).await {
        Ok(Ok(output)) if output.status.success() => {
            parse_environment(&String::from_utf8_lossy(&output.stdout), DESKTOP_KEYS)
        }
        _ => BTreeMap::new(),
    }
}

pub(super) fn native_executable(path: &Path) -> CuaResult<PathBuf> {
    let path = canonical_executable(path)?;
    let mut magic = [0; 4];
    File::open(&path)
        .and_then(|mut file| file.read_exact(&mut magic))
        .map_err(|err| format!("Cannot read driver {}: {err}", path.display()))?;
    if magic != *b"\x7fELF" {
        return Err(format!(
            "{} is a launcher, not a native Linux driver. Set CUA_DRIVER_PATH to the actual binary in ~/.config/cua-driver/host.env.",
            path.display()
        ));
    }
    Ok(path)
}

/// Advisory ownership shared with the CLI's flock wrapper and standalone Pi.
/// The file is never unlinked, so all clients lock the same inode. The caller
/// retains it until its driver has been killed and reaped.
pub(super) fn desktop_lease() -> CuaResult<File> {
    let uid = unsafe { libc::geteuid() };
    acquire_lease(&PathBuf::from(format!("/run/user/{uid}/cua-driver")))
}

pub(super) fn acquire_lease(root: &Path) -> CuaResult<File> {
    use std::os::unix::fs::DirBuilderExt;
    match std::fs::DirBuilder::new().mode(0o700).create(root) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(err) => return Err(format!("Cannot create CUA ownership directory: {err}")),
    }
    let meta = std::fs::symlink_metadata(root).map_err(|e| e.to_string())?;
    let uid = unsafe { libc::geteuid() };
    if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o077 != 0 {
        return Err("CUA ownership directory must be private and owned by the desktop user".into());
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root.join("control.lock"))
        .map_err(|e| e.to_string())?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.uid() != uid || meta.mode() & 0o077 != 0 {
        return Err("CUA ownership file must be private and owned by the desktop user".into());
    }
    file.try_lock().map_err(|err| format!(
        "Computer use is busy in another Noches or Pi session, or ownership is unavailable: {err}"
    ))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_settings_cannot_import_authority_or_evaluate_shell() {
        let parsed = parse_environment(
            "CUA_DRIVER_PATH=/a path/driver\nCUA_DRIVER_PERMISSION_MODE=unrestricted\nCUA_HYPRLAND_OPEN_INPUT=1\nCUA_HYPRLAND_LOCAL_PACKAGES=zen=1\n",
            HOST_KEYS,
        );
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed["CUA_DRIVER_PATH"], "/a path/driver");
    }

    #[test]
    fn ownership_is_exclusive_and_released_without_deleting_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ownership");
        let first = acquire_lease(&root).unwrap();
        assert!(acquire_lease(&root).is_err());
        let status = std::process::Command::new("flock")
            .args(["--nonblock", "--conflict-exit-code", "75"])
            .arg(root.join("control.lock"))
            .arg("true")
            .status()
            .unwrap();
        assert_eq!(
            status.code(),
            Some(75),
            "Pi/CLI flock must see the same lock"
        );
        drop(first);
        assert!(acquire_lease(&root).is_ok());
        assert!(root.join("control.lock").exists());
    }

    #[test]
    fn native_identity_rejects_wrappers_before_starting_them() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("cua-driver");
        std::fs::write(&script, "#!/bin/sh\nexec some-other-binary\n").unwrap();
        assert!(native_executable(&script).unwrap_err().contains("host.env"));
        assert!(native_executable(&std::env::current_exe().unwrap()).is_ok());
    }
}
