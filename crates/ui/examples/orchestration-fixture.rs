//! Real transfer RPCs and the production Shell/Transcript/Details renderer.
//! Uses one immediate mock provider and temporary local data, never real agents.
use std::{path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use gpui::{AppContext, AsyncApp, Bounds, WindowBounds, WindowOptions, px, size};
use serde_json::json;
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_harness::{Harness, HarnessError, RunControls};
use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, Model, ReasoningLevel, RunRequest, orchestration::*,
    orchestration_mcp::T3ThreadSendInputMode,
};
use zeron_rpc::{memory_client, methods};
use zeron_ui::*;

struct FixtureHarness;

#[async_trait]
impl Harness for FixtureHarness {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Fixture"
    }
    fn supports_steering(&self) -> bool {
        false
    }
    fn steering_mode(&self) -> zeron_proto::SteeringMode {
        zeron_proto::SteeringMode::TurnBoundary
    }
    fn deterministic_turn_end(&self) -> bool {
        true
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[]
    }
    async fn models(&self) -> Result<Vec<Model>, HarnessError> {
        Ok(vec![Model {
            id: "mock-1".into(),
            label: "Fixture".into(),
            description: None,
            reasoning_levels: vec![],
            options: vec![],
        }])
    }
    async fn run(
        &self,
        request: RunRequest,
        _: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        let native = uuid::Uuid::new_v4().to_string();
        let text = if request
            .prompt
            .ends_with("Check the retry boundary in this fork.")
        {
            "Keep the original source run pinned, and retain the same request identity when a response is lost. Merge-back transfers conversation context to the parent’s next message; it does not merge working files."
        } else {
            "Use separate identities for the conversation, run, provider attempt, and native provider session. A conversation fork stays idle until its first message, and each harness resumes only its own accepted native history."
        };
        Ok(Box::pin(stream::iter([
            Ok(AgentEvent::SessionStarted {
                instance_id: None,
                session_id: native.clone(),
                harness: HarnessId::Mock,
                model: "mock-1".into(),
                cwd: request.cwd,
                tools: vec![],
                assistant_message_id: uuid::Uuid::new_v4().to_string(),
            }),
            Ok(AgentEvent::TextDelta { text: text.into() }),
            Ok(AgentEvent::Done {
                status: DoneStatus::Completed,
                result: None,
                error: None,
                session_id: Some(native),
            }),
        ])))
    }
}

async fn send(core: &EngineCore, chat: &str, key: &str, text: &str) -> anyhow::Result<()> {
    let result = core
        .orchestration_host
        .as_ref()
        .unwrap()
        .threads
        .send_to_thread(
            zeron_engine::orchestration::thread_service::ThreadSendRequest {
                project_id: "fixture-project".into(),
                thread_id: chat.into(),
                command_id: key.into(),
                message_id: format!("message:{key}").into(),
                scheduled_task_id: None,
                sender_thread_id: None,
                text: text.into(),
                attachments: vec![],
                model_selection: None,
                mode: T3ThreadSendInputMode::Auto,
                created_by: OrchestrationV2Actor::User,
                creation_source: OrchestrationV2CreationSource::Web,
            },
        )
        .await
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if core
                .orchestration
                .store
                .thread(&chat.into())
                .unwrap()
                .unwrap()
                .runs
                .iter()
                .any(|run| {
                    run.id == result.run_id && run.status == OrchestrationV2RunStatus::Completed
                })
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await?;
    Ok(())
}

async fn pause(cx: &mut AsyncApp, ms: u64) {
    cx.background_executor()
        .timer(Duration::from_millis(ms))
        .await;
}

fn capture(
    window: gpui::AnyWindowHandle,
    cx: &mut AsyncApp,
    directory: &std::path::Path,
    name: &str,
) -> anyhow::Result<()> {
    window.update(cx, |_, window, cx| {
        window.draw(cx).clear();
        window
            .render_to_image()?
            .save(directory.join(format!("{name}.png")))?;
        Ok(())
    })?
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_file(true)
        .with_line_number(true)
        .init();
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().expect("output directory"));
    let mode = args.next().unwrap_or_else(|| "dark".into());
    let appearance = match mode.as_str() {
        "dark" => appearance::AppearanceMode::Dark,
        "light" => appearance::AppearanceMode::Light,
        _ => anyhow::bail!("Expected appearance dark or light, got {mode}"),
    };
    std::fs::create_dir_all(&output)?;
    let scratch = tempfile::tempdir()?;
    let checkout = scratch.path().join("checkout");
    std::fs::create_dir(&checkout)?;
    std::fs::write(checkout.join("README.md"), "# Fixture checkout\n")?;
    for args in [
        vec!["init", "-q", "--initial-branch=main"],
        vec!["add", "README.md"],
        vec![
            "-c",
            "user.name=Noches Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "Fixture",
        ],
    ] {
        anyhow::ensure!(
            std::process::Command::new("git")
                .args(args)
                .current_dir(&checkout)
                .status()?
                .success(),
            "Could not prepare the isolated fixture checkout"
        );
    }
    let runtime = tokio::runtime::Runtime::new()?;
    let core = runtime.block_on(async {
        let registry = Arc::new(HarnessRegistry::new());
        registry.register(Arc::new(FixtureHarness));
        EngineCore::assemble(
            &scratch.path().join("engine"),
            registry,
            HarnessId::Mock,
            None,
        )
    })?;
    let core = Arc::new(core);
    core.workspace.create_space(
        "fixture-project",
        &core.device_id,
        checkout.to_str().unwrap(),
        None,
        false,
    )?;
    let client = runtime.block_on(async { memory_client(core.rpc_service()) });
    runtime.block_on(async {
        client.call(methods::MUTATE, json!({
            "op":"createChat", "chatId":"fixture-parent", "spaceId":"fixture-project", "cwd":checkout,
            "config":{"harness":"mock", "model":"mock-1", "reasoning":null, "sandbox":"workspace-write",
                "runtimeMode":"full-access", "interactionMode":"default"}
        })).await?;
        core.workspace.rename_chat("fixture-parent", "Review orchestration boundaries")?;
        core.workspace.set_chat_branch("fixture-parent", "main")?;
        send(&core, "fixture-parent", "initial", "Review how conversation forks and agent handoffs preserve context.").await
    })?;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    let _ipc = runtime.block_on(zeron_engine::serve_ipc(port, core.rpc_service()))?;
    let data = scratch.path().join("ui");
    std::fs::create_dir(&data)?;
    let boot = EngineBootConfig {
        remote: None,
        data_dir: data.clone(),
        ipc_port: port,
        edge_url: String::new(),
        edge_token: None,
        org_id: None,
        workos_client_id: None,
        default_harness: HarnessId::Mock,
    };
    let handle = runtime.block_on(state::EngineHandle::bootstrap(boot.clone()))?;
    let chats = core.workspace.read_chats()?;
    let spaces = core.workspace.read_spaces()?;
    let device = core.device_id.clone();
    let errors = Arc::new(std::sync::Mutex::new(None));
    let failed = errors.clone();
    let renderer_core = core.clone();
    gpui_platform::application().with_assets(icons::Assets).run(move |cx| {
        gpui_tokio::init(cx);
        gpui_base::init(cx);
        let settings = settings::UiSettings::default();
        settings::init(settings.clone(), data.clone(), cx);
        let fonts = typography::register_fonts(cx);
        typography::init(settings.ui_font_family.clone(), settings.ui_font_size,
            settings.terminal_font_family.clone(), settings.terminal_font_size,
            settings.code_font_family.clone(), settings.code_font_size, fonts, cx);
        theme_library::init(data.clone(), cx);
        appearance::init(appearance, settings.theme_selection,
            settings.accent, settings.surface, cx);
        history::init(settings.git_history_columns, settings.git_history_column_widths,
            settings.git_history_column_order, settings.git_history_author_display, cx);
        composer::init(cx, settings.composer_send_behavior);
        terminal::panel::init(cx);
        app_menus::init(cx);
        let state = cx.new(|_| {
            let mut state = state::AppState::new();
            state.fixture_attachment_engine(handle);
            state.connection = zeron_proto::view::ConnectionStatus::Ready;
            state.workspace_scope = Some(zeron_proto::WorkspaceScope::Development);
            state.local_device_id = Some(device.clone());
            state.devices = vec![serde_json::from_value(json!({
                "id":device, "name":"This device", "platform":std::env::consts::OS, "lastSeenAt":null,
            })).unwrap()];
            state.chats = chats;
            state.spaces = spaces;
            state.auto_selected = true;
            state.chats_synced = true;
            state.spaces_synced = true;
            state
        });
        let window = cx.open_window(WindowOptions {
            window_background: theme::Theme::of(cx).window_background_appearance(),
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                gpui::point(px(20.), px(40.)), size(px(1320.), px(900.)),
            ))), ..Default::default()
        }, |_, cx| cx.new(|cx| shell::Shell::new(state.clone(), boot, cx))).unwrap();
        window.update(cx, |shell, _, cx| shell.fixture_orchestration_open("fixture-parent".into(), cx)).unwrap();
        cx.spawn(async move |cx| {
            let run: anyhow::Result<()> = async {
                pause(cx, 1800).await;
                let parent = state.read_with(cx, |state, _| details::DetailsModel::for_chat(state, "fixture-parent"));
                anyhow::ensure!(parent.transfers_supported && parent.fork_run_id.is_some(), "Source fork control did not load");
                capture(window.into(), cx, &output, &format!("parent-{mode}"))?;
                // Exercise the actual UI action/RPC/navigation path.
                window.update(cx, |shell, _, cx| shell.fixture_orchestration_fork("fixture-parent".into(), cx))?;
                let mut child = None;
                for _ in 0..100 {
                    pause(cx, 100).await;
                    child = state.read_with(cx, |state, _| state.selected_chat.clone().filter(|id| id != "fixture-parent"));
                    if child.is_some() { break; }
                }
                let child = child.ok_or_else(|| anyhow::anyhow!("UI fork did not navigate to its child"))?;
                window.update(cx, |shell, _, cx| shell.fixture_orchestration_open(child.clone(), cx))?;
                pause(cx, 1000).await;
                let projection = renderer_core.orchestration.store.thread(&child.clone().into())?.unwrap();
                anyhow::ensure!(projection.runs.is_empty(), "Fork eagerly started a provider");
                let model = state.read_with(cx, |state, _| details::DetailsModel::for_chat(state, &child));
                anyhow::ensure!(model.merge_target.as_deref() == Some("fixture-parent"), "Fork parent lineage missing");
                window.update(cx, |shell, _, cx| shell.fixture_orchestration_transcript_start(cx))?;
                pause(cx, 300).await;
                capture(window.into(), cx, &output, &format!("idle-fork-{mode}"))?;
                let send_core = renderer_core.clone();
                let send_chat = child.clone();
                gpui_tokio::Tokio::spawn(cx, async move {
                    send(&send_core, &send_chat, "child-review", "Check the retry boundary in this fork.").await
                }).await??;
                state.update(cx, |state, cx| state.refresh_details(&child, true, cx));
                pause(cx, 1200).await;
                let model = state.read_with(cx, |state, _| details::DetailsModel::for_chat(state, &child));
                anyhow::ensure!(model.merge_run_id.is_some(), "Merge-back run did not load");
                capture(window.into(), cx, &output, &format!("continued-fork-{mode}"))?;
                window.update(cx, |shell, _, cx| shell.fixture_orchestration_merge(child.clone(), cx))?;
                for _ in 0..100 {
                    pause(cx, 100).await;
                    if state.read_with(cx, |state, _| state.selected_chat.as_deref() == Some("fixture-parent")) { break; }
                }
                anyhow::ensure!(state.read_with(cx, |state, _| state.selected_chat.as_deref() == Some("fixture-parent")),
                    "UI merge-back did not navigate to the parent");
                pause(cx, 800).await;
                let parent = state.read_with(cx, |state, _| details::DetailsModel::for_chat(state, "fixture-parent"));
                anyhow::ensure!(parent.transfers.iter().any(|row| row.title == "Conversation fork" && row.status == "Delivered"),
                    "Parent Details did not show the child's native acceptance receipt");
                anyhow::ensure!(parent.transfers.iter().any(|row| row.title == "Merge-back context" && row.status == "Pending"),
                    "Merge-back was not pending for the parent's next message");
                anyhow::ensure!(renderer_core.orchestration.store.thread(&"fixture-parent".into())?.unwrap().runs.len() == 1,
                    "Preparing merge-back eagerly started a parent provider");
                capture(window.into(), cx, &output, &format!("merge-back-{mode}"))?;
                window.update(cx, |_, window, _| window.resize(size(px(960.), px(720.))))?;
                pause(cx, 400).await;
                capture(window.into(), cx, &output, &format!("merge-back-{mode}-960"))?;
                anyhow::ensure!(std::fs::read_to_string(checkout.join("README.md"))? == "# Fixture checkout\n",
                    "A conversation transfer changed working files");
                std::fs::write(output.join(format!("result-{mode}.txt")),
                    format!("PASS ({mode}): production UI fork/merge handlers and RPCs; idle fork; \
                        inherited lineage/text; continued fork; context-only merge; native GPUI \
                        regular/narrow render. Mock provider only; no live-provider or physical-device claims.\n"))?;
                Ok(())
            }.await;
            if let Err(error) = run {
                eprintln!("Orchestration fixture failed: {error:#}");
                *failed.lock().unwrap() = Some(format!("{error:#}"));
            }
            let _ = window.update(cx, |_, window, _| window.remove_window());
            cx.update(|cx| cx.quit());
        }).detach();
    });
    runtime.block_on(core.shutdown());
    if let Some(error) = errors.lock().unwrap().take() {
        anyhow::bail!(error);
    }
    Ok(())
}
