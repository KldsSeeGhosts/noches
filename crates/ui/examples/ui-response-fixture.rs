//! Headed, offline production Shell/List replay. All profiles are temporary.
//! Timings measure CPU draw/input work, not GPU presentation or physical input.
use gpui::{AppContext, AsyncApp, Bounds, WindowBounds, WindowOptions, point, px, size};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use zeron_ui::*;

async fn pause(cx: &mut AsyncApp, ms: u64) {
    cx.background_executor()
        .timer(Duration::from_millis(ms))
        .await;
}

fn wheel(
    window: gpui::WindowHandle<shell::Shell>,
    cx: &mut AsyncApp,
    dy: f32,
) -> anyhow::Result<()> {
    window.update(cx, |_, w, cx| {
        w.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position: point(px(740.), px(400.)),
                delta: gpui::ScrollDelta::Pixels(point(px(0.), px(dy))),
                modifiers: gpui::Modifiers::default(),
                touch_phase: gpui::TouchPhase::Moved,
            }),
            cx,
        );
    })?;
    Ok(())
}

fn capture(
    window: gpui::WindowHandle<shell::Shell>,
    cx: &mut AsyncApp,
    output: &std::path::Path,
    name: &str,
) -> anyhow::Result<()> {
    window.update(cx, |_, w, _| {
        w.render_to_image()?
            .save(output.join(format!("{name}.png")))?;
        anyhow::Ok(())
    })?
}

fn history(turns: usize) -> Vec<zeron_doc::SessionMessageEntry> {
    let mut entries = Vec::new();
    for i in 0..turns {
        entries.push(serde_json::from_value(serde_json::json!({
            "id": format!("user-{i}"), "role": "user",
            "parts": [{"id": "text", "kind": "text", "text": format!("Review iteration {i}: verify scrolling, focus and the workspace layout.")}],
            "createdAt": 1788900000000_i64 + i as i64 * 2000, "deviceId": "local"
        })).unwrap());
        let code = (0..18)
            .map(|line| {
                format!("    let value_{line} = input.iter().map(|x| x + {line}).sum::<usize>();\n")
            })
            .collect::<String>();
        let text = format!(
            "## Iteration {i}\n\nThe transcript should respond immediately in **both directions**, including after a direction reversal. This is representative settled history with prose, code and tools.\n\n- Preserve the user's viewport.\n- Keep the control-plane design.\n- Measure the UI, not just the engine.\n\n```rust\nfn iteration_{i}(input: &[usize]) {{\n{code}}}\n```\n\n| Path | Result |\n| --- | --- |\n| Scroll | Checked |\n| Panels | Checked |\n\nA final paragraph with [a local link](./src/main.rs) and `inline code`."
        );
        entries.push(serde_json::from_value(serde_json::json!({
            "id": format!("assistant-{i}"), "role": "assistant", "status": "complete",
            "parts": [
                {"id":"text","kind":"text","text":text},
                {"id":"tool","kind":"tool","call":{"kind":"exec","command":"cargo test"},"output":"Tests passed","resolved":true}
            ],
            "createdAt": 1788900001000_i64 + i as i64 * 2000, "deviceId":"local"
        })).unwrap());
    }
    entries
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("warn").init();
    let output = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&output)?;
    let temp = tempfile::tempdir()?;
    let data = temp.path().to_path_buf();
    let turns = std::env::var("NOCHES_RESPONSE_TURNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);
    let entries = history(turns);
    let interactive = std::env::var_os("NOCHES_RESPONSE_INTERACTIVE").is_some();
    let light = std::env::var_os("NOCHES_RESPONSE_LIGHT").is_some();
    let failure = std::sync::Arc::new(std::sync::Mutex::new(None));
    let result = failure.clone();
    gpui_platform::application().with_assets(icons::Assets).run(move |cx| {
        gpui_tokio::init(cx);
        gpui_base::init(cx);
        let mut settings = settings::UiSettings::default();
        if let Ok(ms) = std::env::var("NOCHES_RESPONSE_PANEL_MS") {
            settings.panel_animation_ms = ms.parse().unwrap();
        }
        settings::init(settings.clone(), data.clone(), cx);
        motion::set_panel_animation_ms(settings.panel_animation_ms);
        motion::set_reduced_motion(cx, std::env::var_os("NOCHES_RESPONSE_REDUCED").is_some());
        let fonts = typography::register_fonts(cx);
        typography::init(settings.ui_font_family.clone(), settings.ui_font_size, settings.terminal_font_family.clone(), settings.terminal_font_size, settings.code_font_family.clone(), settings.code_font_size, fonts, cx);
        theme_library::init(data.clone(), cx);
        appearance::init(if light { appearance::AppearanceMode::Light } else { appearance::AppearanceMode::Dark }, settings.theme_selection, settings.accent, settings.surface, cx);
        history::init(settings.git_history_columns, settings.git_history_column_widths, settings.git_history_column_order, settings.git_history_author_display, cx);
        composer::init(cx, settings.composer_send_behavior);
        terminal::panel::init(cx);
        app_menus::init(cx);
        gpui::set_frame_trace_enabled(true);
        let state = cx.new(|_| {
            let mut s = state::AppState::new();
            s.connection = zeron_proto::view::ConnectionStatus::Ready;
            s.workspace_scope = Some(zeron_proto::WorkspaceScope::Local);
            s.local_device_id = Some("local".into());
            s.selected_chat = Some("response".into());
            s.selected_space = Some("project".into());
            s.auto_selected = true; s.chats_synced = true; s.spaces_synced = true;
            s.devices = vec![serde_json::from_value(serde_json::json!({"id":"local","name":"Fixture device","platform":std::env::consts::OS,"lastSeenAt":null})).unwrap()];
            s.spaces = vec![serde_json::from_value(serde_json::json!({"id":"project","deviceId":"local","path":"/tmp/noches-response","createdAt":"2026-09-08T00:00:00Z"})).unwrap()];
            s.chats = (0..40).map(|i| serde_json::from_value(serde_json::json!({"id":if i == 0 {"response".to_string()} else {format!("background-{i}")},"deviceId":"local","spaceId":"project","title":format!("Responsiveness check {i}"),"archived":false,"createdAt":"2026-09-08T00:00:00Z","config":{"harness":"claude-code","model":"claude-sonnet-4-6","reasoning":null,"sandbox":"workspace-write"}})).unwrap()).collect();
            s
        });
        let boot = EngineBootConfig { remote: None, data_dir: data, ipc_port: 0, edge_url: String::new(), edge_token: None, org_id: None, workos_client_id: None, default_harness: HarnessId::ClaudeCode };
        let window = cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(20.), px(40.)), size(px(1320.), px(880.))))),
            titlebar: Some(gpui::TitlebarOptions {title:Some("Noches UI response fixture".into()), appears_transparent:true, traffic_light_position:Some(point(px(14.),px(14.)))}),
            app_owns_titlebar_drag: true,
            ..Default::default()
        }, |_, cx| cx.new(|cx| shell::Shell::new(state.clone(), boot, cx))).unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let run: anyhow::Result<()> = async {
                state.update(cx, |s, cx| {
                    s.receive_transcript_frame(zeron_doc::TranscriptFrame::Reset { reset: entries }, cx).unwrap();
                });
                pause(cx, 2000).await;
                let transcript = window.read_with(cx, |shell, _| shell.fixture_response_transcript())?;
                transcript.update(cx, |t, cx| t.fixture_response_seek(0.5, cx));
                pause(cx, 500).await;
                if interactive {
                    eprintln!("fixture ready pid={} profile={:?}", std::process::id(), temp.path());
                    // No timer/input work while the user or Sky exercises the fixture.
                    std::future::pending::<()>().await;
                }
                let mut report = Vec::new();
                for phase in ["cold-up", "warm-down", "reversals", "left-close", "left-open", "right-open", "right-close"] {
                    let before = window.read_with(cx, |s, _| s.fixture_response_counters())?;
                    let mut frames = gpui::FrameTimingCollector::new();
                    frames.collect_unseen();
                    let mut samples = Vec::new();
                    let panel = phase.starts_with("left") || phase.starts_with("right");
                    if panel {
                        window.update(cx, |s, _, cx| s.fixture_response_toggle(phase.starts_with("right"), cx))?;
                    }
                    for i in 0..if panel { 32 } else { 180 } {
                        let started = Instant::now();
                        if !panel {
                            let dy = if phase == "cold-up" || (phase == "reversals" && (i / 15) % 2 == 0) { 36. } else { -36. };
                            wheel(window, cx, dy)?;
                        }
                        let input_us = started.elapsed().as_micros() as u64;
                        pause(cx, 16).await;
                        let position = transcript.read_with(cx, |t, _| t.fixture_response_position());
                        let panels = window.read_with(cx, |s, cx| s.fixture_response_panels(cx))?;
                        samples.push(serde_json::json!({"input_us":input_us,"position":position,"panels":panels}));
                    }
                    let timings = frames.collect_unseen().into_iter().map(|f| serde_json::json!({
                        "draw_us": f.draw_duration().as_micros() as u64,
                        "dirty_to_draw_us": f.dirty_to_draw_duration().map(|d| d.as_micros() as u64),
                        "invalidations":f.invalidations
                    })).collect::<Vec<_>>();
                    let after = window.read_with(cx, |s, _| s.fixture_response_counters())?;
                    eprintln!("phase={phase} frames={}", timings.len());
                    report.push(serde_json::json!({"phase":phase,"before":before,"after":after,"frames":timings,"samples":samples}));
                }
                std::fs::write(output.join("response.json"), serde_json::to_vec_pretty(&serde_json::json!({
                    "turns":turns, "panel_ms":motion::panel_animation_ms(), "phases":report,
                    "timing_scope":"Headed native CPU draw/dirty-to-draw; not physical-input or GPU presentation latency"
                }))?)?;
                // Readbacks and extra scenarios are outside the measured
                // phases. These exercise the production native event/layout
                // loop, not OS input or GPU-presentation timing.
                if std::env::var_os("NOCHES_RESPONSE_VERIFY").is_some() {
                    transcript.update(cx, |t, cx| t.scroll_to_row(0, cx));
                    pause(cx, 40).await;
                    wheel(window, cx, 36.)?;
                    anyhow::ensure!(!transcript.read_with(cx, |t, _| t.fixture_response_position())["navigation"].as_bool().unwrap(), "wheel did not cancel rail navigation");
                    pause(cx, 550).await;
                    let mut live = history(20);
                    live.last_mut().unwrap().status = Some(zeron_doc::MessageStatus::Streaming);
                    state.update(cx, |s, cx| {
                        s.receive_transcript_frame(zeron_doc::TranscriptFrame::reset(&live), cx).unwrap();
                    });
                    pause(cx, 300).await;
                    transcript.update(cx, |t, cx| t.fixture_response_seek(0.5, cx));
                    pause(cx, 100).await;
                    for i in 0..30 {
                        let mut next = live.clone();
                        if let zeron_doc::MessagePart::Text { text, .. } = &mut next.last_mut().unwrap().parts[0] {
                            text.push_str(" More live output.");
                        }
                        let frame = zeron_doc::diff_transcript(&live, &next);
                        state.update(cx, |s, cx| { s.receive_transcript_frame(frame, cx).unwrap(); });
                        live = next;
                        wheel(window, cx, if (i / 5) % 2 == 0 {36.} else {-36.})?;
                        pause(cx, 20).await;
                        anyhow::ensure!(!transcript.read_with(cx, |t, _| t.fixture_response_position())["pinned"].as_bool().unwrap(), "background streaming stole scroll ownership");
                    }
                    for mode in [appearance::AppearanceMode::Dark, appearance::AppearanceMode::Light] {
                        let label = if mode == appearance::AppearanceMode::Dark {"dark"} else {"light"};
                        cx.update(|cx| appearance::set_mode(mode, cx));
                        pause(cx, 300).await;
                        capture(window, cx, &output, &format!("settled-{label}"))?;
                        for right in [false, true] {
                            let side = if right {"right"} else {"left"};
                            window.update(cx, |s, _, cx| s.fixture_response_toggle(right, cx))?;
                            pause(cx, 40).await;
                            capture(window, cx, &output, &format!("{side}-mid-{label}"))?;
                            let (before, from) = window.update(cx, |s, _, cx| s.fixture_response_reverse(right, cx))?;
                            anyhow::ensure!((before - from).abs() < 1., "{side} panel reversal jumped: {before} -> {from}");
                            pause(cx, 400).await;
                            capture(window, cx, &output, &format!("{side}-restored-{label}"))?;
                        }
                    }
                    for (ms, reduced) in [(200, true), (0, false)] {
                        cx.update(|cx| {
                            motion::set_panel_animation_ms(ms);
                            motion::set_reduced_motion(cx, reduced);
                        });
                        for right in [false, true] {
                            window.update(cx, |s, _, cx| s.fixture_response_toggle(right, cx))?;
                            pause(cx, 32).await;
                            let p = window.read_with(cx, |s, cx| s.fixture_response_panels(cx))?;
                            let side = if right {"right"} else {"left"};
                            anyhow::ensure!(p[side] == p[format!("{side}_target")], "reduced/instant motion did not snap");
                            window.update(cx, |s, _, cx| s.fixture_response_toggle(right, cx))?;
                            pause(cx, 32).await;
                        }
                    }
                    eprintln!("native verification: wheel cancels rail; streaming preserves ownership; interrupted panels; dark/light captures; reduced/instant snaps: PASS");
                }
                drop(temp);
                Ok(())
            }.await;
            if let Err(error) = run {
                eprintln!("native fixture FAILED: {error:#}");
                *result.lock().unwrap() = Some(format!("{error:#}"));
                // AppKit termination can exit directly instead of returning
                // from Application::run; don't silently turn failure into 0.
                std::process::exit(1);
            }
            cx.update(|cx| cx.quit());
        }).detach();
    });
    if let Some(error) = failure.lock().unwrap().take() {
        anyhow::bail!("{error}");
    }
    Ok(())
}
