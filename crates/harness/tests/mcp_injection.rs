#![cfg(unix)]

use futures::StreamExt;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use zeron_harness::mcp::{McpServerEntry, SessionMcpContext};
use zeron_harness::{
    AcpHarness, CancellationToken, ClaudeHarness, CodexHarness, CursorHarness, Harness, RunControls,
};
use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, InteractionMode, RunRequest, RuntimeMode, SandboxLevel,
};

struct LogCapture(Arc<Mutex<String>>);

impl tracing::Subscriber for LogCapture {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        struct Fields<'a>(&'a mut String);
        impl tracing::field::Visit for Fields<'_> {
            fn record_debug(&mut self, _: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                let _ = writeln!(self.0, "{value:?}");
            }
        }
        event.record(&mut Fields(&mut self.0.lock().unwrap()));
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

fn captured_logs() -> &'static Arc<Mutex<String>> {
    static LOGS: OnceLock<Arc<Mutex<String>>> = OnceLock::new();
    LOGS.get_or_init(|| {
        let logs = Arc::new(Mutex::new(String::new()));
        tracing::subscriber::set_global_default(LogCapture(logs.clone())).unwrap();
        logs
    })
}

fn fixture() -> (tempfile::TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp-peer");
    std::fs::write(&path, include_str!("fixtures/mcp-peer.py")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    (dir, path)
}

fn context(executable: &std::path::Path) -> SessionMcpContext {
    let stub = McpServerEntry::stdio("stub", executable, vec!["--mcp".into()]);
    let mut scope = McpServerEntry::http(
        "scope",
        "http://127.0.0.1:1/mcp",
        [(
            "Authorization".into(),
            "Bearer opaque-session-secret".into(),
        )]
        .into(),
    );
    scope.allowed_tools = vec!["mcp__scope__echo".into()];
    SessionMcpContext::new(vec![stub, scope], "SESSION_INSTRUCTIONS".into(), Vec::new())
        .unwrap()
        .with_executable(PathBuf::from(env!("CARGO_BIN_EXE_noches-mcp")))
}

fn request(resume: bool) -> RunRequest {
    RunRequest {
        instance_id: None,
        prompt: "MCP fixture".into(),
        harness: None,
        model: None,
        reasoning: None,
        model_options: Default::default(),
        cwd: String::new(),
        sandbox: SandboxLevel::ReadOnly,
        auto_approve: false,
        runtime_mode: Default::default(),
        interaction_mode: Default::default(),
        attachments: Vec::new(),
        worktree: None,
        resume: resume.then(|| "previous".into()),
    }
}

async fn fixture_adapter(harness: &dyn Harness, context: SessionMcpContext, resume: bool) {
    fixture_adapter_mode(harness, context, resume, RuntimeMode::FullAccess).await;
}

async fn fixture_adapter_mode(
    harness: &dyn Harness,
    context: SessionMcpContext,
    resume: bool,
    mode: RuntimeMode,
) {
    let logs = captured_logs();
    let revokes = Arc::new(AtomicUsize::new(0));
    let count = revokes.clone();
    context.on_revoke(move || {
        count.fetch_add(1, Ordering::SeqCst);
    });
    let (_steer, steering) = mpsc::channel(1);
    let controls = RunControls {
        mcp: context.clone(),
        browser: None,
        request_permission: zeron_harness::refuse_permissions(),
        computer_use_socket: None,
        request_input: Box::new(|_| {
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(Vec::new());
            rx
        }),
        steering,
        interrupt: CancellationToken::new(),
    };
    let mut request = request(resume);
    request.runtime_mode = mode;
    let mut stream = harness.run(request, controls).await.unwrap();
    let events = tokio::time::timeout(Duration::from_secs(10), async {
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            let event = event.unwrap();
            let done = matches!(event, AgentEvent::Done { .. });
            events.push(event);
            if done {
                break;
            }
        }
        events
    })
    .await
    .unwrap();
    assert!(
        events.iter().any(|e| matches!(
            e,
            AgentEvent::Done {
                status: DoneStatus::Completed,
                ..
            }
        )),
        "{events:?}"
    );
    let state = serde_json::to_string(&events).unwrap();
    assert!(state.contains("MCP_CONFIG_OK"), "{state}");
    assert!(!state.contains("opaque-session-secret"), "{state}");
    assert!(state.contains("[redacted]"), "{state}");
    let policy_text = events
        .iter()
        .find_map(|event| {
            let text = match event {
                AgentEvent::TextDelta { text } => text,
                AgentEvent::Done {
                    result: Some(text), ..
                } => text,
                _ => return None,
            };
            text.split_once("\nPOLICY_CONFIG:")
                .map(|(_, policy)| policy)
        })
        .expect("MCP fixture must also report applied runtime policy");
    let applied: serde_json::Value = serde_json::from_str(policy_text).unwrap();
    let compiled =
        zeron_harness::policy::compile(harness.id(), mode, InteractionMode::Default).unwrap();
    match harness.id() {
        HarnessId::ClaudeCode => {
            assert_eq!(applied["permissionMode"], compiled.claude_permission_mode)
        }
        HarnessId::Codex => {
            assert_eq!(applied["approvalPolicy"], compiled.codex_approval);
            assert_eq!(
                applied["sandbox"],
                serde_json::to_value(compiled.codex_sandbox).unwrap()
            );
            assert_eq!(applied["approvalsReviewer"], compiled.codex_reviewer);
        }
        HarnessId::Cursor => {
            assert_eq!(applied["runtimeMode"], serde_json::to_value(mode).unwrap());
            assert_eq!(applied["autoReview"], compiled.cursor_auto_review);
            assert_eq!(applied["sandboxEnabled"], compiled.cursor_sandbox);
        }
        HarnessId::Grok => assert_eq!(
            applied["args"],
            serde_json::json!(zeron_harness::policy::grok_args(mode))
        ),
        HarnessId::Antigravity => assert_eq!(
            applied["mode"],
            match mode {
                RuntimeMode::FullAccess => "yolo",
                RuntimeMode::AutoAcceptEdits => "auto_edit",
                _ => "default",
            }
        ),
        HarnessId::Devin | HarnessId::Hermes => assert_eq!(
            applied["mode"],
            match mode {
                RuntimeMode::FullAccess => "bypassPermissions",
                RuntimeMode::AutoAcceptEdits => "acceptEdits",
                _ => "default",
            }
        ),
        _ => panic!("unexpected fixture harness"),
    }
    drop(stream);
    tokio::time::timeout(Duration::from_secs(3), async {
        while !context.is_revoked() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(revokes.load(Ordering::SeqCst), 1);
    context.revoke();
    assert_eq!(revokes.load(Ordering::SeqCst), 1);
    let logs = logs.lock().unwrap();
    assert!(!logs.contains("opaque-session-secret"), "{logs}");
    assert!(
        logs.contains("[redacted]"),
        "fixture stderr was not captured: {logs}"
    );
}

#[tokio::test]
async fn claude_injects_servers_allowlist_and_system_instructions() {
    let (_dir, path) = fixture();
    fixture_adapter(
        &ClaudeHarness::new().with_executable(&path),
        context(&path),
        false,
    )
    .await;
}

#[tokio::test]
async fn codex_start_and_resume_use_session_config() {
    let (_dir, path) = fixture();
    let harness = CodexHarness::new().with_executable(&path);
    fixture_adapter(&harness, context(&path), false).await;
    fixture_adapter(&harness, context(&path), true).await;
    for mode in RuntimeMode::ALL {
        let context = context(&path);
        let id = harness
            .fork_session_with_mcp(
                "source",
                "",
                context.clone(),
                mode,
                InteractionMode::Default,
            )
            .await
            .unwrap();
        assert_eq!(id, "mcp-codex");
        assert!(context.is_revoked());
    }
}

#[tokio::test]
async fn runtime_policy_and_mcp_injection_compose_on_start_and_resume() {
    let (_dir, path) = fixture();
    let adapters: Vec<Box<dyn Harness>> = vec![
        Box::new(ClaudeHarness::new().with_executable(&path)),
        Box::new(CodexHarness::new().with_executable(&path)),
        Box::new(CursorHarness::new().with_executable(&path)),
        Box::new(AcpHarness::grok().with_executable(&path)),
        Box::new(AcpHarness::antigravity().with_executable(&path)),
        Box::new(AcpHarness::hermes().with_executable(&path)),
        Box::new(AcpHarness::devin().with_executable(&path)),
    ];
    for adapter in adapters {
        for mode in RuntimeMode::ALL {
            if zeron_harness::policy::compile(adapter.id(), mode, InteractionMode::Default).is_err()
            {
                continue;
            }
            for resume in [false, true] {
                fixture_adapter_mode(adapter.as_ref(), context(&path), resume, mode).await;
            }
        }
    }
}

#[tokio::test]
async fn cursor_passes_sdk_servers_and_instruction_text_to_shim() {
    let (_dir, path) = fixture();
    fixture_adapter(
        &CursorHarness::new().with_executable(&path),
        context(&path),
        false,
    )
    .await;
}

#[tokio::test]
async fn acp_flavors_receive_stdio_baseline_and_private_fallback() {
    let (_dir, path) = fixture();
    for harness in [
        AcpHarness::grok(),
        AcpHarness::antigravity(),
        AcpHarness::hermes(),
    ] {
        fixture_adapter(&harness.with_executable(&path), context(&path), false).await;
    }
    fixture_adapter(
        &AcpHarness::grok().with_executable(&path),
        context(&path),
        true,
    )
    .await;
}

#[tokio::test]
async fn terminal_fallback_calls_a_real_stdio_mcp_server() {
    let (dir, path) = fixture();
    let result = tokio::process::Command::new(env!("CARGO_BIN_EXE_noches-mcp"))
        .args(["acp-mcp-call", "echo", r#"{"value":"fallback"}"#])
        .env(
            "NOCHES_SESSION_MCP_ENTRIES",
            serde_json::json!([{"name":"stub","config":{
            "command":path,"args":["--mcp"],"env":{}}}])
            .to_string(),
        )
        .current_dir(dir.path())
        .output()
        .await
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["content"][0]["text"], "MCP_RESULT:fallback");
    assert_eq!(value["structuredContent"]["ok"], true);
}

#[tokio::test]
async fn startup_failure_revokes_each_adapter_scope() {
    let (dir, stub) = fixture();
    let missing = dir.path().join("missing-provider");
    let adapters: Vec<Box<dyn Harness>> = vec![
        Box::new(ClaudeHarness::new().with_executable(&missing)),
        Box::new(CodexHarness::new().with_executable(&missing)),
        Box::new(CursorHarness::new().with_executable(&missing)),
        Box::new(AcpHarness::grok().with_executable(&missing)),
        Box::new(zeron_harness::OpencodeHarness::new().with_executable(&missing)),
    ];
    for adapter in adapters {
        let context = context(&stub);
        let (_steer, steering) = mpsc::channel(1);
        let controls = RunControls {
            mcp: context.clone(),
            browser: None,
            request_permission: zeron_harness::refuse_permissions(),
            computer_use_socket: None,
            request_input: Box::new(|_| {
                let (tx, rx) = oneshot::channel();
                let _ = tx.send(Vec::new());
                rx
            }),
            steering,
            interrupt: CancellationToken::new(),
        };
        assert!(adapter.run(request(false), controls).await.is_err());
        assert!(context.is_revoked(), "{}", adapter.display_name());
    }
}

#[tokio::test]
async fn unqualified_fallback_discovers_unique_tools_among_multiple_servers() {
    let (_dir, path) = fixture();
    let entries = serde_json::json!([
        {"name":"other","config":{"command":path,"args":["--mcp","--empty"],"env":{}}},
        {"name":"stub","config":{"command":path,"args":["--mcp","--paginated"],"env":{}}}
    ]);
    for tool in ["echo", "mcp__stub__echo"] {
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            tokio::process::Command::new(env!("CARGO_BIN_EXE_noches-mcp"))
                .args(["acp-mcp-call", tool, r#"{"value":"unique"}"#])
                .env("NOCHES_SESSION_MCP_ENTRIES", entries.to_string())
                .kill_on_drop(true)
                .output(),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(value["content"][0]["text"], "MCP_RESULT:unique");
    }
}

/// Installed Codex app-server + an actual stdio MCP process. Calling a tool
/// needs provider credentials/model access; this is deliberately opt-in.
#[tokio::test]
#[ignore = "requires NOCHES_MCP_TEST_CODEX executable and authenticated Codex model access"]
async fn live_codex_calls_injected_stdio_mcp() {
    let executable = std::env::var_os("NOCHES_MCP_TEST_CODEX").expect("set NOCHES_MCP_TEST_CODEX");
    let (dir, stub) = fixture();
    let record = dir.path().join("mcp-methods");
    let entry = McpServerEntry::stdio(
        "noches_stub",
        stub,
        vec![
            "--mcp".into(),
            "--record".into(),
            record.display().to_string(),
        ],
    );
    let context = SessionMcpContext::new(vec![entry], "Use the noches_stub MCP echo tool to return the requested marker. Do not execute shell commands or modify any files.".into(), Vec::new()).unwrap();
    let (_steer, steering) = mpsc::channel(1);
    let interrupt = CancellationToken::new();
    let controls = RunControls {
        mcp: context.clone(),
        browser: None,
        request_permission: zeron_harness::refuse_permissions(),
        computer_use_socket: None,
        request_input: Box::new(|_| {
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(Vec::new());
            rx
        }),
        steering,
        interrupt: interrupt.clone(),
    };
    let mut req = request(false);
    req.cwd = dir.path().display().to_string();
    req.model = std::env::var("NOCHES_MCP_TEST_MODEL").ok();
    req.prompt = "Call the noches_stub MCP echo tool with {\"value\":\"live-noches-mcp\"}. Reply with the returned text. Do not use any other tool.".into();
    let harness = CodexHarness::new().with_executable(executable);
    let mut stream = harness.run(req, controls).await.unwrap();
    let events = tokio::time::timeout(Duration::from_secs(120), async {
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            let event = event.unwrap();
            let done = matches!(event, AgentEvent::Done { .. });
            events.push(event);
            if done {
                break;
            }
        }
        events
    })
    .await;
    interrupt.cancel();
    drop(stream);
    let events = events.expect("live MCP call timed out");
    assert!(
        events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolCall {
                call: zeron_proto::ToolCall::Mcp { server, tool, .. },
                ..
            } if server == "noches_stub" && tool == "echo"
        )),
        "native Codex transport must report the injected MCP tool call: {events:?}"
    );
    let reply = events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::TextDelta { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert!(reply.contains("MCP_RESULT:live-noches-mcp"), "{events:?}");
    let methods = std::fs::read_to_string(record).unwrap();
    assert!(
        methods.contains("tools/list") && methods.contains("tools/call"),
        "{methods}"
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        while !context.is_revoked() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("live MCP scope was not revoked after session teardown");
}
