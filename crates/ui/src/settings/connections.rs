//! Saved native connections. Each computer opens in a separate window so local
//! files, pending edits and running sessions keep their original engine owner.
//!
//! A local window also hosts: the "Pair a phone" section below mints a code for
//! the iOS companion, renders it as a QR, and runs the tailnet gateway in this
//! process (`crate::remote_access`).
use super::widgets;
use crate::{
    popover, remote_access,
    state::{AppState, EngineBootConfig},
    theme::Theme,
    typography,
};
use gpui::{
    AnyElement, Context, Entity, Image, ImageFormat, SharedString, Subscription, Window, div,
    prelude::*, px,
};
use std::sync::Arc;
use zeron_rpc::remote::{ConnectionProfile, ConnectionState, Connections, GATEWAY_PORT};

pub struct ConnectionsPage {
    state: Entity<AppState>,
    boot: EngineBootConfig,
    hosts: Vec<ConnectionProfile>,
    error: Option<String>,
    notice: Option<String>,
    /// Devices paired with this computer through the in-app gateway.
    paired: Vec<ConnectionProfile>,
    /// The live pairing code. Deliberately page state only: never logged and
    /// never written to disk. `Done` clears it.
    code: Option<String>,
    qr: Option<Arc<Image>>,
    pairing: bool,
    _observe: Subscription,
}
impl ConnectionsPage {
    pub fn new(state: Entity<AppState>, boot: EngineBootConfig, cx: &mut Context<Self>) -> Self {
        let result = Connections::load(&boot.data_dir);
        let (hosts, error) = match result {
            Ok(c) => (c.hosts, None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        };
        let paired = remote_access::clients(&boot.data_dir).unwrap_or_default();
        let observe = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            boot,
            hosts,
            error,
            notice: None,
            paired,
            code: None,
            qr: None,
            pairing: false,
            _observe: observe,
        }
    }
    fn paste(&mut self, cx: &mut Context<Self>) {
        let result = (|| -> anyhow::Result<()> {
            let code = cx
                .read_from_clipboard()
                .and_then(|item| item.text())
                .ok_or_else(|| anyhow::anyhow!("Copy a Noches connection code first."))?;
            let host = ConnectionProfile::from_code(&code)?;
            let name = host.name.clone();
            let mut connections = Connections::load(&self.boot.data_dir)?;
            connections.add(host);
            connections.save(&self.boot.data_dir)?;
            self.hosts = connections.hosts;
            self.notice = Some(format!(
                "{name} added. Open it to see its projects and sessions."
            ));
            Ok(())
        })();
        self.error = result.err().map(|e| e.to_string());
        cx.notify();
    }
    fn forget(&mut self, id: &str, cx: &mut Context<Self>) {
        let result = (|| -> anyhow::Result<()> {
            let mut connections = Connections::load(&self.boot.data_dir)?;
            connections.hosts.retain(|h| h.id != id);
            connections.save(&self.boot.data_dir)?;
            self.hosts = connections.hosts;
            self.notice =
                Some("Connection removed from this computer. Existing windows remain open.".into());
            Ok(())
        })();
        self.error = result.err().map(|e| e.to_string());
        cx.notify();
    }

    /// Mint a pairing code for a phone, start the gateway, and hold the code for
    /// the QR. The whole flow is async because it dials the local engine.
    fn pair_phone(&mut self, cx: &mut Context<Self>) {
        if self.pairing {
            return;
        }
        let Some(tailnet) = zeron_rpc::remote::tailnet_ipv4() else {
            self.error = Some(
                "Tailscale isn't connected on this computer. Connect it, then try again.".into(),
            );
            cx.notify();
            return;
        };
        self.pairing = true;
        self.error = None;
        self.notice = None;
        let name = remote_access::computer_name();
        let endpoint = format!("ws://{tailnet}:{GATEWAY_PORT}");
        let upstream = format!("ws://127.0.0.1:{}", self.boot.ipc_port);
        let credentials = remote_access::credentials_path(&self.boot.data_dir);
        // The task must be `'static`, so it owns its inputs.
        let pair_upstream = upstream.clone();
        let pair_credentials = credentials.clone();
        let pair_name = name.clone();
        let pair_endpoint = endpoint.clone();
        // `pair_client` dials a WebSocket, so it has to run on the tokio runtime
        // (`Tokio::spawn`), not gpui's background executor.
        let pair = gpui_tokio::Tokio::spawn(cx, async move {
            remote_access::pair_client(
                &pair_upstream,
                &pair_credentials,
                &pair_name,
                &pair_endpoint,
            )
            .await
        });
        cx.spawn(async move |this, cx| {
            let result = match pair.await {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!("{error}")),
            };
            // `update` returns a Result here (AsyncApp); a closed window is
            // nothing to report.
            let _ = this.update(cx, |page, cx| {
                page.pairing = false;
                match result {
                    Ok(profile) => {
                        let code = profile.code().ok();
                        page.qr = code.as_deref().and_then(|code| {
                            remote_access::qr_png(code)
                                .ok()
                                .map(|bytes| Arc::new(Image::from_bytes(ImageFormat::Png, bytes)))
                        });
                        if page.qr.is_none() {
                            page.error = Some("Could not render the pairing code.".into());
                        }
                        page.code = code;
                        // Registering the client and serving are one action from
                        // the user's side, so a bind failure is reported now.
                        let outcome =
                            remote_access::ensure_running(cx, upstream, tailnet, credentials);
                        match outcome {
                            remote_access::GatewayOutcome::Started { endpoint }
                            | remote_access::GatewayOutcome::AlreadyRunning { endpoint } => {
                                page.notice = Some(format!("Remote access on at {endpoint}"));
                            }
                            remote_access::GatewayOutcome::Failed { message } => {
                                page.error = Some(message);
                            }
                        }
                        page.paired =
                            remote_access::clients(&page.boot.data_dir).unwrap_or_default();
                    }
                    Err(error) => page.error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Clear the code from memory. The client stays paired and the gateway keeps
    /// running; revoking is a separate, explicit action.
    fn finish_pairing(&mut self, cx: &mut Context<Self>) {
        if let Some(image) = self.qr.take() {
            let image = image.clone();
            cx.defer(move |cx| gpui::ImageSource::Image(image).evict(None, cx));
        }
        self.code = None;
        self.notice = Some("Code cleared. The phone keeps access until you revoke it.".into());
        cx.notify();
    }

    fn copy_code(&mut self, cx: &mut Context<Self>) {
        match self.code.clone() {
            Some(code) => {
                remote_access::copy_to_clipboard(cx, code);
                self.notice = Some("Pairing code copied.".into());
            }
            None => self.error = Some("Create a pairing code first.".into()),
        }
        cx.notify();
    }

    fn revoke(&mut self, id: &str, cx: &mut Context<Self>) {
        let result = remote_access::revoke(&self.boot.data_dir, id);
        match result {
            Ok(_) => {
                self.paired = remote_access::clients(&self.boot.data_dir).unwrap_or_default();
                self.notice =
                    Some("Device revoked. Its sessions close within five seconds.".into());
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn remote_status(&self) -> Option<String> {
        remote_access::status().label()
    }

    /// The "Pair a phone" section. Rendered only for a local window: a remote
    /// window is a client of another host and has no local engine to share.
    fn render_pairing(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let monospace = |text: String| {
            div()
                .font_family(theme.font_mono.clone())
                .text_size(px(11.0))
                .text_color(theme.text_muted)
                .child(SharedString::from(text))
        };
        let mut section = div()
            .mt(px(28.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(widgets::row_title(theme, "Pair a phone"))
            .child(widgets::page_subtitle(
                theme,
                "Run Noches remote access on this computer and pair the iOS app over Tailscale.",
            ));

        if let Some(code) = self.code.clone() {
            let mut code_panel = div()
                .mt(px(4.0))
                .p(px(14.0))
                .rounded(px(10.0))
                .border_1()
                .border_color(theme.border)
                .bg(theme.surface)
                .flex()
                .flex_row()
                .items_start()
                .gap(px(16.0));
            if let Some(qr) = &self.qr {
                let side = remote_access::qr_side(&code);
                code_panel = code_panel.child(
                    div()
                        .flex_none()
                        .w(px(side as f32))
                        .h(px(side as f32))
                        .rounded(px(6.0))
                        .overflow_hidden()
                        .bg(gpui::white())
                        .child(gpui::img(qr.clone()).size(px(side as f32))),
                );
            }
            code_panel = code_panel.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(widgets::page_subtitle(
                        theme,
                        "On your phone, open Noches, tap Pair a computer, and scan this code. The phone must be on the same tailnet.",
                    ))
                    .child(monospace(code.clone()))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(8.0))
                            .child(
                                popover::btn_ghost(theme, "Copy code", "copy-pair-code")
                                    .id("copy-pair-code")
                                    .on_click(cx.listener(|this, _, _, cx| this.copy_code(cx))),
                            )
                            .child(
                                popover::btn_primary(theme, "Done")
                                    .id("finish-pairing")
                                    .on_click(cx.listener(|this, _, _, cx| this.finish_pairing(cx))),
                            ),
                    ),
            );
            section = section.child(code_panel);
        } else {
            section = section.child(
                div().mt(px(4.0)).child(
                    popover::btn_primary(
                        theme,
                        if self.pairing {
                            "Creating code…"
                        } else {
                            "Pair a phone"
                        },
                    )
                    .id("pair-a-phone")
                    .on_click(cx.listener(|this, _, _, cx| this.pair_phone(cx))),
                ),
            );
        }

        if let Some(status) = self.remote_status() {
            section = section.child(monospace(status));
        }

        section = section.child(
            div()
                .mt(px(8.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(widgets::field_label(theme, "Paired devices"))
                .children(self.paired.clone().into_iter().map(|device| {
                    let revoke_id = device.id.clone();
                    div()
                        .p(px(12.0))
                        .rounded(px(10.0))
                        .bg(theme.surface)
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(widgets::row_tile(theme, crate::icons::SMARTPHONE))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .child(widgets::row_title(theme, &device.name))
                                .child(monospace(format!(
                                    "{} · {}",
                                    short_id(&device.id),
                                    device.endpoint
                                ))),
                        )
                        .child(
                            popover::btn_ghost(theme, "Revoke", format!("revoke-{}", device.id))
                                .id(SharedString::from(format!("revoke-{}", device.id)))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.revoke(&revoke_id, cx)),
                                ),
                        )
                })),
        );
        section.into_any_element()
    }
}

/// First eight characters of a client id — enough to tell two devices apart
/// without printing the whole uuid.
fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

pub fn status_label(status: Option<&ConnectionState>) -> &'static str {
    match status {
        Some(ConnectionState::Connected) => "Connected",
        Some(ConnectionState::Connecting) => "Connecting",
        Some(ConnectionState::Reconnecting) => "Reconnecting",
        Some(ConnectionState::Unauthorized) => "Access revoked",
        None => "Saved computer",
    }
}

impl Render for ConnectionsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let active = self.boot.remote.as_ref().map(|p| p.id.as_str());
        let is_local = self.boot.remote.is_none();
        let state = self.state.read(cx);
        let active_status = state.remote_connection.clone();
        let rows: Vec<_> = self
            .hosts
            .clone()
            .into_iter()
            .map(|host| {
                let is_active = active == Some(host.id.as_str());
                let open_host = host.clone();
                let remove_id = host.id.clone();
                let caption = if is_active {
                    status_label(active_status.as_ref())
                } else {
                    "Saved computer"
                };
                div()
                    .p(px(14.0))
                    .rounded(px(10.0))
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(widgets::row_tile(&theme, crate::icons::MONITOR))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(widgets::row_title(&theme, &host.name))
                            .child(
                                div()
                                    .text_size(typography::ui_rems(12.0))
                                    .text_color(theme.text_muted)
                                    .child(SharedString::from(format!(
                                        "{caption} · {}",
                                        host.endpoint
                                    ))),
                            ),
                    )
                    .child(
                        popover::btn_ghost(&theme, "Remove", format!("forget-{}", host.id))
                            .id(SharedString::from(format!("forget-{}", host.id)))
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.forget(&remove_id, cx)),
                            ),
                    )
                    .child(
                        popover::btn_primary(&theme, "Open computer")
                            .id(SharedString::from(format!("open-{}", host.id)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                crate::open_connection_window(
                                    this.boot.clone(),
                                    Some(open_host.clone()),
                                    cx,
                                );
                            })),
                    )
            })
            .collect();
        let pairing = is_local.then(|| self.render_pairing(&theme, cx));
        div().id("connections-page").size_full().overflow_y_scroll()
            .child(widgets::page_column()
                .child(widgets::page_header(&theme, "Connections", Some(self.hosts.len())))
                .child(widgets::page_subtitle(&theme, "Open another computer's projects and sessions. Work keeps running there when this connection drops."))
                .child(div().mt(px(20.0)).flex().items_center().justify_between()
                    .child(widgets::row_title(&theme, "This computer"))
                    .child(popover::btn_ghost(&theme, "Open local window", "open-local-window")
                        .id("open-local-window")
                        .on_click(cx.listener(|this, _, _, cx| crate::open_connection_window(this.boot.clone(), None, cx)))))
                .child(div().mt(px(20.0)).flex().flex_col().gap(px(10.0)).children(rows))
                .child(div().mt(px(20.0))
                    .child(popover::btn_primary(&theme, "Paste connection code")
                        .id("paste-connection-code")
                        .on_click(cx.listener(|this, _, _, cx| this.paste(cx)))))
                .child(widgets::page_subtitle(&theme, "Use a connection code from a computer with Noches remote access enabled. Both computers need access to the same tailnet."))
                .when_some(pairing, |el, pairing| el.child(pairing))
                .when_some(self.notice.clone(), |el, notice| el.child(div().mt(px(12.0)).text_color(theme.text_muted).child(notice)))
                .when_some(self.error.clone(), |el, error| el.child(div().mt(px(12.0)).text_color(theme.danger).child(error))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;

    fn boot(dir: &std::path::Path, remote: Option<ConnectionProfile>) -> EngineBootConfig {
        EngineBootConfig {
            remote,
            data_dir: dir.to_path_buf(),
            ipc_port: 27654,
            edge_url: "http://127.0.0.1:1".into(),
            edge_token: None,
            org_id: None,
            workos_client_id: None,
            default_harness: crate::HarnessId::Mock,
        }
    }

    /// A page in a real window, so the render path (including the QR element) is
    /// exercised rather than just the state.
    fn render_page(cx: &mut gpui::TestAppContext, boot: EngineBootConfig) -> bool {
        let window = cx.add_window(|_, cx| {
            let state = cx.new(|_| AppState::new());
            ConnectionsPage::new(state, boot, cx)
        });
        cx.update_window(window.into(), |_, w, cx| {
            w.draw(cx).clear();
        })
        .is_ok()
    }

    #[gpui::test]
    fn pairing_section_renders_only_for_a_local_window(cx: &mut gpui::TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        cx.update(|cx| {
            gpui_base::init(cx);
            cx.set_global(Theme::default());
        });
        assert!(
            render_page(cx, boot(dir.path(), None)),
            "local window renders"
        );
        // A remote window is a client of another host: no local engine to share,
        // so the section is absent. Rendering must still succeed.
        let remote = ConnectionProfile {
            id: "host".into(),
            name: "Other".into(),
            endpoint: "ws://100.64.0.2:27657".into(),
            token: "a".repeat(64),
            device_id: "device".into(),
        };
        assert!(
            render_page(cx, boot(dir.path(), Some(remote))),
            "remote window renders without the pairing section"
        );
    }

    #[gpui::test]
    fn a_held_code_renders_its_qr_and_clears_on_done(cx: &mut gpui::TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        cx.update(|cx| {
            gpui_base::init(cx);
            cx.set_global(Theme::default());
        });
        let window = cx.add_window(|_, cx| {
            let state = cx.new(|_| AppState::new());
            ConnectionsPage::new(state, boot(dir.path(), None), cx)
        });
        window
            .update(cx, |page, _, _| {
                let profile = ConnectionProfile {
                    id: "client-12345678".into(),
                    name: "Studio Mac".into(),
                    endpoint: "ws://100.64.0.1:27657".into(),
                    token: "b".repeat(64),
                    device_id: "device".into(),
                };
                let code = profile.code().unwrap();
                let side = remote_access::qr_side(&code);
                page.qr = Some(Arc::new(Image::from_bytes(
                    ImageFormat::Png,
                    remote_access::qr_png(&code).unwrap(),
                )));
                page.code = Some(code);
                assert_eq!(short_id(&profile.id), "client-1");
                assert!(side > 0 && side <= remote_access::QR_TARGET_SIDE);
            })
            .unwrap();
        cx.update_window(window.into(), |_, w, cx| {
            w.draw(cx).clear();
        })
        .unwrap();
        window
            .update(cx, |page, _, cx| {
                page.finish_pairing(cx);
                assert!(page.code.is_none(), "Done clears the code from memory");
                assert!(page.qr.is_none(), "and releases the QR image");
            })
            .unwrap();
        cx.update_window(window.into(), |_, w, cx| {
            w.draw(cx).clear();
        })
        .unwrap();
    }
}
