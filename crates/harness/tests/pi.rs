//! PiHarness integration tests against the fake `pi --mode rpc` in
//! `tests/fixtures/fake-pi.py` (no real `pi` binary involved). Record shapes in
//! the fixture were observed on a live Pi 1.0.4; the opt-in live probe is in
//! `pi_live.rs`.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use futures::stream::BoxStream;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use zeron_harness::instance::InstanceLaunch;
use zeron_harness::session_lifecycle::{NativeForkRequest, SessionLifecycle};
use zeron_harness::{
    CancellationToken, Harness, HarnessError, PiHarness, RunControls, SteerMessage,
};
use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, InteractionMode, PermissionDecision, ReasoningLevel,
    RunRequest, RuntimeMode, SandboxLevel, ToolCall, UserInputAnswer,
};

fn fixture_path() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fake-pi.py");
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
    path
}

/// A sandbox for one test: sessions, command log, agent dir, cwd.
struct Env {
    dir: tempfile::TempDir,
    extra: BTreeMap<String, String>,
    args: Vec<String>,
}

impl Env {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            extra: BTreeMap::new(),
            args: Vec::new(),
        }
    }
    fn with(mut self, key: &str, value: &str) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
    fn cwd(&self) -> String {
        let cwd = self.path("work");
        std::fs::create_dir_all(&cwd).unwrap();
        cwd.display().to_string()
    }
    fn sessions(&self) -> PathBuf {
        self.path("sessions")
    }
    fn harness(&self) -> PiHarness {
        let mut environment = BTreeMap::from([
            ("FAKE_PI_LOG".to_owned(), self.path("log.jsonl").display().to_string()),
            ("FAKE_PI_SESSIONS".to_owned(), self.sessions().display().to_string()),
            ("PI_CODING_AGENT_DIR".to_owned(), self.path("agent").display().to_string()),
            ("PI_CODING_AGENT_SESSION_DIR".to_owned(), self.sessions().display().to_string()),
        ]);
        environment.extend(self.extra.clone());
        PiHarness::new()
            .with_executable(fixture_path())
            .with_graces(Duration::from_millis(200), Duration::from_millis(500))
            .with_instance_launch(InstanceLaunch::new(environment, self.args.clone()))
    }
    /// Every record the fake logged, in order.
    fn log(&self) -> Vec<Value> {
        std::fs::read_to_string(self.path("log.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
    fn commands(&self, kind: &str) -> Vec<Value> {
        self.log()
            .into_iter()
            .filter(|r| r["kind"] == "command" && r["type"] == kind)
            .collect()
    }
    fn starts(&self) -> Vec<Value> {
        self.log().into_iter().filter(|r| r["kind"] == "start").collect()
    }
}

fn request(env: &Env, prompt: &str) -> RunRequest {
    RunRequest {
        instance_id: None,
        prompt: prompt.into(),
        harness: None,
        model: None,
        reasoning: None,
        model_options: serde_json::Map::new(),
        cwd: env.cwd(),
        sandbox: SandboxLevel::WorkspaceWrite,
        runtime_mode: Default::default(),
        interaction_mode: Default::default(),
        auto_approve: true,
        attachments: Vec::new(),
        worktree: None,
        resume: None,
    }
}

struct Handles {
    steer: mpsc::Sender<SteerMessage>,
    interrupt: CancellationToken,
    permissions: Arc<Mutex<Vec<String>>>,
}

/// `permission`: the decision every permission request gets; `answer`: the
/// label every content question gets.
fn make_controls(permission: PermissionDecision, answer: &'static str) -> (RunControls, Handles) {
    let (steer_tx, steer_rx) = mpsc::channel(8);
    let interrupt = CancellationToken::new();
    let permissions = Arc::new(Mutex::new(Vec::new()));
    let seen = permissions.clone();
    let controls = RunControls {
        mcp: Default::default(),
        browser: None,
        computer_use_socket: None,
        request_permission: Box::new(move |request| {
            seen.lock().unwrap().push(format!("{}|{}", request.tool, request.description));
            let (tx, rx) = oneshot::channel();
            let option = request
                .options
                .into_iter()
                .find(|o| o.decision == permission)
                .unwrap_or_default();
            let _ = tx.send(option);
            zeron_harness::PermissionReceiver::new(rx, || {})
        }),
        request_input: Box::new(move |questions| {
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(
                questions
                    .into_iter()
                    .map(|q| UserInputAnswer {
                        question_id: q.id,
                        labels: if answer.is_empty() { vec![] } else { vec![answer.into()] },
                    })
                    .collect(),
            );
            rx
        }),
        steering: steer_rx,
        interrupt: interrupt.clone(),
    };
    (
        controls,
        Handles {
            steer: steer_tx,
            interrupt,
            permissions,
        },
    )
}

fn default_controls() -> (RunControls, Handles) {
    make_controls(PermissionDecision::Accept, "alpha")
}

async fn start(
    harness: &PiHarness,
    request: RunRequest,
    controls: RunControls,
) -> BoxStream<'static, Result<AgentEvent, HarnessError>> {
    harness.run(request, controls).await.unwrap()
}

/// Collect events until `done` Done events have arrived or the stream ends.
async fn until_done(
    stream: &mut BoxStream<'static, Result<AgentEvent, HarnessError>>,
    done: usize,
) -> Vec<AgentEvent> {
    let mut events = Vec::new();
    let mut seen = 0;
    tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(event) = stream.next().await {
            let event = event.unwrap();
            if matches!(event, AgentEvent::Done { .. }) {
                seen += 1;
            }
            events.push(event);
            if seen >= done {
                break;
            }
        }
    })
    .await
    .expect("run did not settle");
    events
}

async fn collect(env: &Env, prompt: &str) -> Vec<AgentEvent> {
    let (controls, _handles) = default_controls();
    let mut stream = start(&env.harness(), request(env, prompt), controls).await;
    until_done(&mut stream, 1).await
}

fn text(events: &[AgentEvent]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::TextDelta { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn done(events: &[AgentEvent]) -> (&DoneStatus, Option<&str>, Option<&str>) {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            AgentEvent::Done { status, error, session_id, .. } => {
                Some((status, error.as_deref(), session_id.as_deref()))
            }
            _ => None,
        })
        .expect("a Done event")
}

fn session_files(env: &Env) -> Vec<PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(env.sessions())
        .map(|dir| dir.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    files.sort();
    files
}

fn entry_ids(path: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .skip(1)
        .map(|line| {
            let entry: Value = serde_json::from_str(line).unwrap();
            let role = entry
                .pointer("/message/role")
                .and_then(Value::as_str)
                .unwrap_or_else(|| entry["type"].as_str().unwrap())
                .to_owned();
            (entry["id"].as_str().unwrap().to_owned(), role)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Turns
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_turn_streams_text_and_settles_with_the_session_file_as_native_id() {
    let env = Env::new();
    let events = collect(&env, "hello").await;

    let started = events
        .iter()
        .find_map(|e| match e {
            AgentEvent::SessionStarted { harness, model, session_id, cwd, .. } => {
                Some((harness, model, session_id, cwd))
            }
            _ => None,
        })
        .expect("SessionStarted");
    assert_eq!(*started.0, HarnessId::Pi);
    assert_eq!(started.1, "cpa/gemini-3.8-flash");
    assert_eq!(started.3, &env.cwd());
    // The session file path is the durable native thread id.
    let files = session_files(&env);
    assert_eq!(files.len(), 1);
    assert_eq!(started.2, &files[0].display().to_string());

    assert!(events.iter().any(|e| matches!(e, AgentEvent::InputAccepted)));
    assert_eq!(text(&events), "reply:hello\n\n");
    let (status, error, session) = done(&events);
    assert_eq!(*status, DoneStatus::Completed);
    assert_eq!(error, None);
    assert_eq!(session, Some(started.2.as_str()));

    // Order: the session exists before input is accepted, and Done is last.
    let position = |pred: &dyn Fn(&AgentEvent) -> bool| events.iter().position(pred).unwrap();
    assert!(
        position(&|e| matches!(e, AgentEvent::SessionStarted { .. }))
            < position(&|e| matches!(e, AgentEvent::InputAccepted))
    );
    assert!(matches!(events.last(), Some(AgentEvent::Done { .. })));

    // The turn's native ref is its first user entry in the session tree.
    let user_entry = entry_ids(&files[0])
        .into_iter()
        .find(|(_, role)| role == "user")
        .unwrap()
        .0;
    assert!(events.iter().any(|e| matches!(
        e,
        AgentEvent::NativeReference { thread_id, turn_id: Some(turn) }
            if thread_id == started.2 && *turn == user_entry
    )));
    // Authoritative context snapshot at settle: replaces, with the compaction
    // threshold from Pi's own default reserve and the session token totals.
    assert!(events.iter().any(|e| matches!(
        e,
        AgentEvent::ContextUsageSnapshot { usage }
            if usage.tokens == Some(11260) && usage.window == Some(1_048_576)
                && usage.compact_at == Some(1_048_576 - 16_384)
                && usage.session.is_some_and(|s| s.input == 11259 && s.cache_read == 5)
    )));
}

#[tokio::test]
async fn tool_calls_map_to_typed_chips_with_results_and_inline_diffs() {
    let env = Env::new();
    let events = collect(&env, "tool").await;
    let calls: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::ToolCall { id, call } => Some((id.as_str(), call.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(calls[0], ("call|1", ToolCall::Exec { command: "echo hi".into() }));
    assert!(matches!(&calls[1].1, ToolCall::WriteFile { path, content }
        if path == "a.txt" && content.as_deref() == Some("A")));
    assert!(matches!(&calls[2].1, ToolCall::EditFile { old_string, new_string, .. }
        if old_string.as_deref() == Some("A") && new_string.as_deref() == Some("B")));
    assert!(matches!(&calls[3].1, ToolCall::Mcp { server, tool, .. }
        if server == "t3-code" && tool == "task_status"));
    let results: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::ToolResult { id, is_error, output, diff } => {
                Some((id.as_str(), *is_error, output.clone(), diff.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].2.as_deref(), Some("hi\n"));
    assert!(results[1].3.as_ref().is_some_and(|d| d.old_text.is_none() && d.new_text == "A"));
    assert!(results[2].3.as_ref().is_some_and(|d| d.old_text.as_deref() == Some("A") && d.new_text == "B"));
    assert!(results.iter().all(|r| !r.1));
}

#[tokio::test]
async fn consecutive_assistant_messages_are_separate_paragraphs() {
    let env = Env::new();
    let events = collect(&env, "two-messages").await;
    assert_eq!(text(&events), "first\n\nsecond\n\n");
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, AgentEvent::AssistantMessageCompleted { .. }))
            .count(),
        2
    );
}

#[tokio::test]
async fn model_errors_and_exhausted_retries_end_the_turn_as_errored() {
    let env = Env::new();
    let errored = collect(&env, "error").await;
    let (status, error, _) = done(&errored);
    assert_eq!((*status, error), (DoneStatus::Errored, Some("model exploded")));

    let exhausted = collect(&env, "retry-fail").await;
    let (status, error, _) = done(&exhausted);
    assert_eq!((*status, error), (DoneStatus::Errored, Some("Connection error.")));
    // Intermediate retry failures are not terminal: exactly one Done.
    assert_eq!(
        exhausted.iter().filter(|e| matches!(e, AgentEvent::Done { .. })).count(),
        1
    );
}

#[tokio::test]
async fn a_recovered_retry_does_not_settle_as_failed() {
    let env = Env::new();
    let events = collect(&env, "retry-ok").await;
    let (status, error, _) = done(&events);
    assert_eq!((*status, error), (DoneStatus::Completed, None));
    assert_eq!(text(&events), "recovered\n\n");
}

#[tokio::test]
async fn a_rejected_prompt_is_reported_and_a_crash_carries_stderr() {
    let env = Env::new();
    let rejected = collect(&env, "reject").await;
    let (status, error, _) = done(&rejected);
    assert_eq!(*status, DoneStatus::Errored);
    assert!(error.unwrap().contains("Fixture rejected this prompt"), "{error:?}");

    let crashed = collect(&env, "crash").await;
    let (status, error, _) = done(&crashed);
    assert_eq!(*status, DoneStatus::Errored);
    let error = error.unwrap();
    assert!(error.contains("exit code 23"), "{error}");
    assert!(error.contains("pi fatal: crashed mid-turn"), "{error}");
}

#[tokio::test]
async fn a_pi_that_dies_at_startup_names_its_stderr() {
    let env = Env::new().with("FAKE_PI_STARTUP_EXIT", "9");
    let events = collect(&env, "hello").await;
    let (status, error, _) = done(&events);
    assert_eq!(*status, DoneStatus::Errored);
    let error = error.unwrap();
    assert!(error.contains("exit code 9") && error.contains("startup failed"), "{error}");
}

#[tokio::test]
async fn an_extension_command_is_handled_without_starting_a_run() {
    let env = Env::new();
    let events = collect(&env, "/handled now").await;
    assert!(events.iter().any(|e| matches!(e, AgentEvent::InputAccepted)));
    assert_eq!(text(&events), "");
    let (status, _, _) = done(&events);
    assert_eq!(*status, DoneStatus::Completed);
}

// ---------------------------------------------------------------------------
// Launch: arguments, extensions, environment, versions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_process_launches_with_rpc_mode_private_extensions_and_a_clean_environment() {
    let mut env = Env::new().with("FAKE_PI_SENTINEL", "kept");
    // A stale parent binding must never reach this run's Pi.
    env.extra.insert("NOCHES_SESSION_MCP_ENTRIES".into(), "[{\"stale\":true}]".into());
    env.extra.insert("NOCHES_CUA_SOCKET".into(), "/stale.sock".into());
    env.args = vec!["--provider".into(), "cpa".into(), "--model".into(), "gemini-3.8-flash".into()];
    collect(&env, "hello").await;
    let starts = env.starts();
    let launch = starts.iter().find(|s| s["argv"][0] == "--mode").unwrap();
    let argv: Vec<_> = launch["argv"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    assert_eq!(&argv[..2], ["--mode", "rpc"]);
    // User arguments pass through; Noches' own extensions follow as `-e`.
    assert_eq!(&argv[2..6], ["--provider", "cpa", "--model", "gemini-3.8-flash"]);
    assert!(!argv.contains(&"--session") && !argv.contains(&"--no-extensions"));
    assert_eq!(argv.iter().filter(|a| **a == "-e").count(), 2, "{argv:?}");
    assert_eq!(launch["extensions_exist"], json!([true, true]));
    let env_seen = &launch["env"];
    assert_eq!(env_seen["FAKE_PI_SENTINEL"], "kept");
    assert_eq!(env_seen["NOCHES_PI_RUNTIME_MODE"], "full-access");
    assert!(env_seen["NOCHES_SESSION_MCP_ENTRIES"].is_null(), "{env_seen}");
    assert!(env_seen["NOCHES_CUA_SOCKET"].is_null());
    assert_eq!(launch["cwd"].as_str().map(|p| p.ends_with("work")), Some(true));
}

#[tokio::test]
async fn unsupported_launch_args_and_old_versions_fail_before_a_prompt() {
    let mut env = Env::new();
    env.args = vec!["--session".into(), "x".into()];
    let (controls, _h) = default_controls();
    let error = env.harness().run(request(&env, "hello"), controls).await.err().unwrap();
    assert!(error.to_string().contains("'--session' is controlled by Noches"), "{error}");

    let old = Env::new().with("FAKE_PI_VERSION", "0.70.1");
    let (controls, _h) = default_controls();
    let error = old.harness().run(request(&old, "hello"), controls).await.err().unwrap();
    assert!(error.to_string().contains("too old") && error.to_string().contains("0.80.5"), "{error}");
    assert!(old.commands("prompt").is_empty());
}

#[tokio::test]
async fn runtime_modes_reach_the_policy_extension_and_unenforceable_ones_are_refused() {
    for (mode, expected) in [
        (RuntimeMode::ApprovalRequired, "approval-required"),
        (RuntimeMode::AutoAcceptEdits, "auto-accept-edits"),
        (RuntimeMode::FullAccess, "full-access"),
    ] {
        let env = Env::new();
        let mut req = request(&env, "hello");
        req.runtime_mode = mode;
        let (controls, _h) = default_controls();
        let mut stream = start(&env.harness(), req, controls).await;
        until_done(&mut stream, 1).await;
        let starts = env.starts();
        let launch = starts.iter().find(|s| s["argv"][0] == "--mode").unwrap();
        assert_eq!(launch["env"]["NOCHES_PI_RUNTIME_MODE"], expected);
    }
    for (mode, interaction) in [
        (RuntimeMode::Auto, InteractionMode::Default),
        (RuntimeMode::FullAccess, InteractionMode::Plan),
    ] {
        let env = Env::new();
        let mut req = request(&env, "hello");
        req.runtime_mode = mode;
        req.interaction_mode = interaction;
        let (controls, _h) = default_controls();
        let error = env.harness().run(req, controls).await.err().unwrap();
        assert!(error.to_string().contains("cannot enforce"), "{error}");
        assert!(env.starts().is_empty(), "nothing may spawn for a refused policy");
    }
}

#[tokio::test]
async fn the_requested_model_and_effort_are_selected_exactly_over_rpc() {
    let env = Env::new();
    let mut req = request(&env, "hello");
    req.model = Some("cpa/devin/swe-2".into());
    let (controls, _h) = default_controls();
    let mut stream = start(&env.harness(), req, controls).await;
    let events = until_done(&mut stream, 1).await;
    // Slugs split on the first separator only; the model's own name rides on.
    assert_eq!(
        env.commands("set_model")[0]["modelId"],
        "devin/swe-2",
        "{:?}",
        env.commands("set_model")
    );
    assert_eq!(env.commands("set_model")[0]["provider"], "cpa");
    assert!(events.iter().any(|e| matches!(
        e, AgentEvent::SessionStarted { model, .. } if model == "cpa/devin/swe-2")));

    // Effort clamps to the model's own ladder: xhigh -> high on a low..high model.
    let env = Env::new();
    let mut req = request(&env, "hello");
    req.model = Some("cpa/gemini-3.8-flash".into());
    req.reasoning = Some(ReasoningLevel::XHigh);
    let (controls, _h) = default_controls();
    let mut stream = start(&env.harness(), req, controls).await;
    until_done(&mut stream, 1).await;
    assert!(env.commands("set_model").is_empty(), "already on that model");
    assert_eq!(env.commands("set_thinking_level")[0]["level"], "high");
}

#[tokio::test]
async fn an_unknown_model_fails_loudly_instead_of_running_another() {
    let env = Env::new();
    let mut req = request(&env, "hello");
    req.model = Some("cpa/does-not-exist".into());
    let (controls, _h) = default_controls();
    let mut stream = start(&env.harness(), req, controls).await;
    let events = until_done(&mut stream, 1).await;
    let (status, error, _) = done(&events);
    assert_eq!(*status, DoneStatus::Errored);
    assert!(error.unwrap().contains("Model not found: cpa/does-not-exist"), "{error:?}");
    assert!(env.commands("prompt").is_empty());

    let env = Env::new();
    let mut req = request(&env, "hello");
    req.model = Some("noslash".into());
    let (controls, _h) = default_controls();
    let mut stream = start(&env.harness(), req, controls).await;
    let events = until_done(&mut stream, 1).await;
    assert!(done(&events).1.unwrap().contains("provider/model format"));
}

#[tokio::test]
async fn the_default_model_restores_the_configured_one_on_a_resumed_session() {
    let env = Env::new();
    std::fs::create_dir_all(env.path("agent")).unwrap();
    std::fs::write(
        env.path("agent").join("settings.json"),
        r#"{"defaultProvider":"cpa","defaultModel":"gemini-3.8-flash"}"#,
    )
    .unwrap();
    // First run pins the other model; the session remembers it.
    let mut req = request(&env, "hello");
    req.model = Some("cpa/devin/swe-2".into());
    let (controls, _h) = default_controls();
    let mut stream = start(&env.harness(), req, controls).await;
    let events = until_done(&mut stream, 1).await;
    let session = done(&events).2.unwrap().to_owned();
    // The fake does not persist model changes on resume, so the model Pi is on
    // equals the configured one and no set_model is needed...
    let mut req = request(&env, "again");
    req.resume = Some(session);
    let (controls, _h) = default_controls();
    let before = env.commands("set_model").len();
    let mut stream = start(&env.harness(), req, controls).await;
    until_done(&mut stream, 1).await;
    assert_eq!(env.commands("set_model").len(), before);
}

// ---------------------------------------------------------------------------
// Resume
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resume_continues_the_stored_session_and_a_vanished_one_starts_fresh() {
    let env = Env::new();
    let first = collect(&env, "hello").await;
    let session = done(&first).2.unwrap().to_owned();

    let mut req = request(&env, "again");
    req.resume = Some(session.clone());
    let (controls, _h) = default_controls();
    let mut stream = start(&env.harness(), req, controls).await;
    let resumed = until_done(&mut stream, 1).await;
    let launches = env.starts();
    let launch = launches.iter().rev().find(|s| s["argv"][0] == "--mode").unwrap();
    assert_eq!(launch["argv"][2], "--session");
    assert_eq!(launch["argv"][3], session.as_str());
    assert_eq!(done(&resumed).2, Some(session.as_str()));
    assert_eq!(session_files(&env).len(), 1, "no new session file");

    // A legacy ACP id (the session UUID) resolves through the session store.
    let id = Path::new(&session)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .rsplit('_')
        .next()
        .unwrap()
        .to_owned();
    let mut req = request(&env, "legacy");
    req.resume = Some(id);
    let (controls, _h) = default_controls();
    let mut stream = start(&env.harness(), req, controls).await;
    let legacy = until_done(&mut stream, 1).await;
    assert_eq!(done(&legacy).2, Some(session.as_str()));

    // A missing file is never handed to `--session` (Pi would create a NEW
    // session at that path): the run starts a fresh session.
    let gone = env.path("gone.jsonl");
    let mut req = request(&env, "fresh");
    req.resume = Some(gone.display().to_string());
    let (controls, _h) = default_controls();
    let mut stream = start(&env.harness(), req, controls).await;
    let fresh = until_done(&mut stream, 1).await;
    let launches = env.starts();
    let launch = launches.iter().rev().find(|s| s["argv"][0] == "--mode").unwrap();
    assert!(!launch["argv"].as_array().unwrap().iter().any(|a| a == "--session"));
    assert!(!gone.exists());
    assert_ne!(done(&fresh).2, Some(session.as_str()));
}

/// A multi-megabyte session in Pi's file format, ending on an assistant entry.
fn seed_large_session(env: &Env) -> (String, String) {
    let file = env.sessions().join("2026-01-01T00-00-00-000Z_seeded.jsonl");
    std::fs::create_dir_all(env.sessions()).unwrap();
    let filler = "x".repeat(128 * 1024);
    let mut text = format!(
        "{}\n",
        json!({"type": "session", "version": 3, "id": "seeded", "cwd": env.cwd()})
    );
    let mut parent = Value::Null;
    let mut last = String::new();
    for n in 1..=40 {
        let id = format!("e{n:05}");
        let role = if n % 2 == 1 { "user" } else { "assistant" };
        text.push_str(&format!(
            "{}\n",
            json!({"type": "message", "id": id, "parentId": parent,
                "message": {"role": role, "content": [{"type": "text", "text": filler}]}})
        ));
        parent = json!(id);
        last = id;
    }
    std::fs::write(&file, text).unwrap();
    (file.display().to_string(), last)
}

#[tokio::test]
async fn a_huge_session_never_needs_a_full_entry_listing_and_stays_forkable() {
    // This fake never answers `get_entries` without a cursor, like a Pi whose
    // reply to a huge session exceeds the framer's cap.
    let env = Env::new().with("FAKE_PI_NO_FULL_ENTRIES", "1");
    let (session, last) = seed_large_session(&env);
    let mut req = request(&env, "again");
    req.resume = Some(session.clone());
    let (controls, _h) = default_controls();
    let harness = env.harness();
    let started = std::time::Instant::now();
    let mut stream = start(&harness, req, controls).await;
    let events = until_done(&mut stream, 1).await;
    assert!(started.elapsed() < Duration::from_secs(4), "run start/finish waited on a listing");
    assert_eq!(done(&events).0, &DoneStatus::Completed);
    assert_eq!(done(&events).2, Some(session.as_str()));

    // Every listing used the locally derived cursor.
    let listings = env.commands("get_entries");
    assert!(!listings.is_empty());
    assert!(listings.iter().all(|c| c["since"] == last.as_str()), "{listings:?}");

    // The turn still gets its native ref (its own user entry), so native fork
    // keeps working on the big chat.
    let entries = entry_ids(Path::new(&session));
    let user = &entries[entries.len() - 2];
    assert_eq!(user.1, "user");
    assert!(events.iter().any(|e| matches!(
        e,
        AgentEvent::NativeReference { turn_id: Some(turn), .. } if *turn == user.0
    )));
    std::fs::create_dir_all(env.path("fork-cwd")).unwrap();
    assert!(
        harness
            .can_fork_now(&fork_request(&env, &session, Some(&user.0), None))
            .await
            .unwrap()
    );
}

// ---------------------------------------------------------------------------
// Steering receipts
// ---------------------------------------------------------------------------

async fn steer(
    handles: &Handles,
    prompt: &str,
    message_id: &str,
    receipt: bool,
) -> Option<oneshot::Receiver<bool>> {
    let (tx, rx) = oneshot::channel();
    handles
        .steer
        .send(SteerMessage {
            prompt: prompt.into(),
            message_id: Some(message_id.into()),
            attachments: Vec::new(),
            notification_acceptance: receipt.then_some(tx),
        })
        .await
        .unwrap();
    receipt.then_some(rx)
}

#[tokio::test]
async fn a_steer_during_a_run_is_receipted_from_pis_queued_ack_and_lands_at_the_step_boundary() {
    let env = Env::new();
    let (controls, handles) = default_controls();
    let mut stream = start(&env.harness(), request(&env, "slow"), controls).await;
    // Wait for the tool to be running, then steer.
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        let tool = matches!(event, AgentEvent::ToolCall { .. });
        events.push(event);
        if tool {
            break;
        }
    }
    let receipt = steer(&handles, "also say hi", "m-steer-1", true).await.unwrap();
    events.extend(until_done(&mut stream, 1).await);

    assert!(receipt.await.unwrap(), "Pi accepted the steer natively");
    let accepted = events
        .iter()
        .position(|e| matches!(e, AgentEvent::InputAcceptedFor { message_id } if message_id == "m-steer-1"))
        .expect("exact receipt for the steer's own id");
    let steered = events
        .iter()
        .position(|e| matches!(e, AgentEvent::Steered { .. }))
        .expect("transcript boundary where Pi injected the message");
    // The receipt is Pi's queued ack; the boundary is the delivery, later.
    assert!(accepted < steered);
    assert!(text(&events).contains("steered:also say hi"));
    // One turn, one Done: the steer was part of the same run.
    assert_eq!(events.iter().filter(|e| matches!(e, AgentEvent::Done { .. })).count(), 1);
    assert_eq!(done(&events).0, &DoneStatus::Completed);
    let prompts = env.commands("prompt");
    assert_eq!(prompts[1]["streamingBehavior"], "steer");
    // The fire-and-forget shape T3 used is not what we send: ids correlate.
    assert!(prompts.iter().all(|p| p["id"].as_str().is_some()));
}

#[tokio::test]
async fn an_idle_steer_starts_a_new_run_atomically_and_is_still_receipted() {
    let env = Env::new();
    let (controls, handles) = default_controls();
    let mut stream = start(&env.harness(), request(&env, "hello"), controls).await;
    let first = until_done(&mut stream, 1).await;
    assert_eq!(done(&first).0, &DoneStatus::Completed);

    let receipt = steer(&handles, "next thing", "m-idle", true).await.unwrap();
    let second = until_done(&mut stream, 1).await;
    assert!(receipt.await.unwrap());
    assert!(second.iter().any(
        |e| matches!(e, AgentEvent::InputAcceptedFor { message_id } if message_id == "m-idle")
    ));
    assert!(second.iter().any(|e| matches!(e, AgentEvent::Steered { .. })));
    assert_eq!(text(&second), "reply:next thing\n\n");
    // `streamingBehavior: steer` makes the race atomic: Pi answered `started`.
    assert_eq!(env.commands("prompt")[1]["streamingBehavior"], "steer");
    assert!(env.commands("steer").is_empty(), "a bare `steer` would queue forever when idle");
}

#[tokio::test]
async fn a_refused_steer_gets_no_receipt_and_does_not_end_the_turn() {
    let env = Env::new();
    let (controls, handles) = default_controls();
    let mut stream = start(&env.harness(), request(&env, "hello"), controls).await;
    until_done(&mut stream, 1).await;

    // The fixture rejects the text "reject" with Pi's own error.
    let receipt = steer(&handles, "reject", "m-bad", true).await.unwrap();
    let mut seen = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(event) = stream.next().await {
            let event = event.unwrap();
            let failed = matches!(&event, AgentEvent::Error { message } if message.contains("Steering failed"));
            seen.push(event);
            if failed {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert!(!receipt.await.unwrap(), "rejected, so the durable queue keeps ownership");
    assert!(!seen.iter().any(|e| matches!(e, AgentEvent::InputAcceptedFor { .. })));
    assert!(!seen.iter().any(|e| matches!(e, AgentEvent::Done { .. })));
}

#[tokio::test]
async fn a_steer_without_a_mailbox_receipt_still_acknowledges_its_message_id() {
    let env = Env::new();
    let (controls, handles) = default_controls();
    let mut stream = start(&env.harness(), request(&env, "hello"), controls).await;
    until_done(&mut stream, 1).await;
    steer(&handles, "plain", "m-plain", false).await;
    let second = until_done(&mut stream, 1).await;
    assert!(second.iter().any(
        |e| matches!(e, AgentEvent::InputAcceptedFor { message_id } if message_id == "m-plain")
    ));
}

#[tokio::test]
async fn closing_the_mailbox_while_idle_ends_the_run_after_its_done() {
    let env = Env::new();
    let (controls, handles) = default_controls();
    let mut stream = start(&env.harness(), request(&env, "hello"), controls).await;
    until_done(&mut stream, 1).await;
    drop(handles);
    let rest: Vec<_> = tokio::time::timeout(Duration::from_secs(10), stream.collect::<Vec<_>>())
        .await
        .unwrap();
    assert!(rest.iter().all(|e| !matches!(e, Ok(AgentEvent::Done { .. }))), "no second Done");
}

// ---------------------------------------------------------------------------
// Interrupt
// ---------------------------------------------------------------------------

#[tokio::test]
async fn interrupt_clears_the_queue_then_aborts_and_reports_interrupted() {
    let env = Env::new();
    let (controls, handles) = default_controls();
    let mut stream = start(&env.harness(), request(&env, "slow"), controls).await;
    while let Some(event) = stream.next().await {
        if matches!(event.unwrap(), AgentEvent::ToolCall { .. }) {
            break;
        }
    }
    handles.interrupt.cancel();
    let events = until_done(&mut stream, 1).await;
    let (status, error, _) = done(&events);
    assert_eq!((*status, error), (DoneStatus::Interrupted, None));
    // Pi's own errored "operation was aborted" message is the requested
    // outcome, not a failure; the tool ends as an error result.
    assert!(events.iter().any(|e| matches!(e, AgentEvent::ToolResult { is_error: true, .. })));
    let kinds: Vec<_> = env
        .log()
        .into_iter()
        .filter(|r| r["kind"] == "command" && matches!(r["type"].as_str(), Some("clear_queue" | "abort")))
        .map(|r| r["type"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(kinds, ["clear_queue", "abort"]);
}

#[tokio::test]
async fn interrupting_an_idle_run_ends_it_with_a_single_interrupted_done() {
    let env = Env::new();
    let (controls, handles) = default_controls();
    let mut stream = start(&env.harness(), request(&env, "hello"), controls).await;
    until_done(&mut stream, 1).await;
    handles.interrupt.cancel();
    let rest: Vec<_> = tokio::time::timeout(Duration::from_secs(10), stream.collect::<Vec<_>>())
        .await
        .unwrap();
    let dones: Vec<_> = rest
        .iter()
        .filter_map(|e| match e {
            Ok(AgentEvent::Done { status, .. }) => Some(*status),
            _ => None,
        })
        .collect();
    assert_eq!(dones, [DoneStatus::Interrupted]);
}

// ---------------------------------------------------------------------------
// Compaction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn compact_maps_to_the_rpc_command_and_reports_its_outcome() {
    let env = Env::new();
    let refused = collect(&env, "/compact keep the marker").await;
    let (status, error, _) = done(&refused);
    assert_eq!(*status, DoneStatus::Errored);
    assert!(error.unwrap().contains("Nothing to compact"), "{error:?}");
    let sent = &env.commands("compact")[0];
    assert_eq!(sent["customInstructions"], "keep the marker");
    assert!(env.commands("prompt").is_empty(), "/compact is never a prompt");

    let env = Env::new().with("FAKE_PI_ALWAYS_COMPACT", "1");
    let compacted = collect(&env, "/compact").await;
    assert!(compacted.iter().any(|e| matches!(e, AgentEvent::InputAccepted)));
    assert_eq!(text(&compacted), "Compacted the conversation: 150K → ~32K tokens.");
    assert_eq!(done(&compacted).0, &DoneStatus::Completed);
    assert!(env.commands("compact")[0].get("customInstructions").is_none());
}

#[tokio::test]
async fn compact_cannot_interrupt_a_running_turn_and_runs_as_an_idle_steer() {
    let env = Env::new().with("FAKE_PI_ALWAYS_COMPACT", "1");
    let (controls, handles) = default_controls();
    let mut stream = start(&env.harness(), request(&env, "slow"), controls).await;
    while let Some(event) = stream.next().await {
        if matches!(event.unwrap(), AgentEvent::ToolCall { .. }) {
            break;
        }
    }
    let receipt = steer(&handles, "/compact", "m-compact", true).await.unwrap();
    assert!(!receipt.await.unwrap(), "compact would abort the running agent");
    assert!(env.commands("compact").is_empty());
    handles.interrupt.cancel();
    until_done(&mut stream, 1).await;

    // Idle: it becomes its own turn with an exact receipt.
    let env = Env::new().with("FAKE_PI_ALWAYS_COMPACT", "1");
    let (controls, handles) = default_controls();
    let mut stream = start(&env.harness(), request(&env, "hello"), controls).await;
    until_done(&mut stream, 1).await;
    let receipt = steer(&handles, "/compact", "m-compact", true).await.unwrap();
    let second = until_done(&mut stream, 1).await;
    assert!(receipt.await.unwrap());
    assert!(second.iter().any(
        |e| matches!(e, AgentEvent::InputAcceptedFor { message_id } if message_id == "m-compact")
    ));
    assert!(second.iter().any(|e| matches!(e, AgentEvent::Steered { .. })));
    assert!(text(&second).starts_with("Compacted the conversation"));
}

// ---------------------------------------------------------------------------
// Dialogs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn policy_confirmations_go_through_the_permission_gate() {
    let env = Env::new();
    let (controls, handles) = make_controls(PermissionDecision::Accept, "alpha");
    let mut stream = start(&env.harness(), request(&env, "confirm"), controls).await;
    let events = until_done(&mut stream, 1).await;
    assert!(text(&events).contains(r#"confirm:{"confirmed": true}"#), "{}", text(&events));
    let asked = handles.permissions.lock().unwrap().clone();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].starts_with("bash|bash: "), "{asked:?}");
    assert!(events.iter().any(|e| matches!(e, AgentEvent::ToolResult { is_error: false, .. })));

    let (controls, _handles) = make_controls(PermissionDecision::Decline, "alpha");
    let mut stream = start(&env.harness(), request(&env, "confirm"), controls).await;
    let events = until_done(&mut stream, 1).await;
    assert!(text(&events).contains(r#"confirm:{"confirmed": false}"#));
    assert!(events.iter().any(|e| matches!(e, AgentEvent::ToolResult { is_error: true, .. })));
}

#[tokio::test]
async fn extension_dialogs_become_content_questions_answered_back_to_pi() {
    let env = Env::new();
    let (controls, _h) = make_controls(PermissionDecision::Accept, "beta");
    let mut stream = start(&env.harness(), request(&env, "select"), controls).await;
    let events = until_done(&mut stream, 1).await;
    assert!(text(&events).contains(r#"dialog:{"value": "beta"}"#), "{}", text(&events));

    // A dialog the user dismisses is cancelled, not answered with garbage.
    let (controls, _h) = make_controls(PermissionDecision::Accept, "");
    let mut stream = start(&env.harness(), request(&env, "select"), controls).await;
    let events = until_done(&mut stream, 1).await;
    assert!(text(&events).contains(r#"dialog:{"cancelled": true}"#), "{}", text(&events));
}

// ---------------------------------------------------------------------------
// Prompt payloads
// ---------------------------------------------------------------------------

#[tokio::test]
async fn images_are_inlined_and_skill_references_use_pis_native_syntax() {
    let env = Env::new();
    let image = env.path("shot.png");
    std::fs::write(&image, [0x89, b'P', b'N', b'G', 0, 0, 0, 0]).unwrap();
    let mut req = request(&env, "$review check this");
    req.attachments = vec![image.display().to_string(), env.path("missing.png").display().to_string()];
    let (controls, _h) = default_controls();
    let mut stream = start(&env.harness(), req, controls).await;
    until_done(&mut stream, 1).await;
    let prompt = &env.commands("prompt")[0];
    assert_eq!(prompt["message"], "/skill:review check this");
    assert_eq!(prompt["has_images"], true);
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

#[tokio::test]
async fn discovery_lists_exact_models_commands_and_readiness() {
    let env = Env::new();
    let harness = env.harness();
    let models = harness.models().await.unwrap();
    let ids: Vec<_> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["default", "cpa/gemini-3.8-flash", "cpa/devin/swe-2"]);
    assert_eq!(
        models[1].reasoning_levels,
        [ReasoningLevel::Low, ReasoningLevel::Medium, ReasoningLevel::High]
    );
    assert!(models[2].reasoning_levels.is_empty());
    assert_eq!(models[1].label, "Gemini 3.8 Flash");
    assert_eq!(harness.authenticated().await.unwrap(), Some(true));
    let commands = harness.commands().await.unwrap();
    assert_eq!(commands[0].name, "compact");
    assert!(commands.iter().any(|c| c.name == "skill:review"));
    // Declared windows are learned for handoff bounding.
    assert_eq!(
        harness.model_context_window("cpa/gemini-3.8-flash", &Default::default()),
        Some(1_048_576)
    );
    assert_eq!(harness.model_context_window("default", &Default::default()), Some(1_048_576));
    // The probe is sessionless and shares one process for all three calls.
    assert!(env.starts().iter().any(|s| s["argv"].as_array().unwrap().iter().any(|a| a == "--no-session")));
}

#[tokio::test]
async fn a_pi_without_models_is_not_authenticated() {
    let env = Env::new().with("FAKE_PI_MODELS", "[]");
    let harness = env.harness();
    assert_eq!(harness.models().await.unwrap().len(), 1);
    assert_eq!(harness.authenticated().await.unwrap(), Some(false));
}

#[tokio::test]
async fn a_missing_binary_is_not_installed() {
    let harness = PiHarness::new().with_executable("/nonexistent/never-a-pi");
    assert!(!harness.installed());
    assert!(matches!(harness.models().await, Err(HarnessError::NotInstalled(_)) | Err(HarnessError::Io(_)) | Err(HarnessError::Protocol(_))));
}

// ---------------------------------------------------------------------------
// Native fork
// ---------------------------------------------------------------------------

/// Two settled turns in one session; returns (session path, [user entry ids]).
async fn two_turn_session(env: &Env) -> (String, Vec<String>) {
    let (controls, handles) = default_controls();
    let mut stream = start(&env.harness(), request(env, "first"), controls).await;
    let first = until_done(&mut stream, 1).await;
    steer(&handles, "second", "m2", false).await;
    until_done(&mut stream, 1).await;
    let session = done(&first).2.unwrap().to_owned();
    let users = entry_ids(Path::new(&session))
        .into_iter()
        .filter(|(_, role)| role == "user")
        .map(|(id, _)| id)
        .collect();
    (session, users)
}

fn fork_request(env: &Env, source: &str, turn: Option<&str>, next: Option<&str>) -> NativeForkRequest {
    NativeForkRequest {
        source_thread_id: source.into(),
        source_turn_id: turn.map(str::to_owned),
        source_next_turn_id: next.map(str::to_owned),
        rollback_turns: None,
        cwd: env.path("fork-cwd").display().to_string(),
        model: "default".into(),
        runtime_mode: Default::default(),
        interaction_mode: Default::default(),
        mcp: Default::default(),
    }
}

#[tokio::test]
async fn a_native_fork_cuts_before_the_next_turn_in_the_destination_cwd() {
    let env = Env::new();
    std::fs::create_dir_all(env.path("fork-cwd")).unwrap();
    let (session, users) = two_turn_session(&env).await;
    assert_eq!(users.len(), 2);
    let harness = env.harness();
    let request = fork_request(&env, &session, Some(&users[0]), Some(&users[1]));
    assert!(harness.can_fork_now(&request).await.unwrap());

    let before = session_files(&env);
    let forked = harness.fork_thread(request).await.unwrap();
    assert_ne!(forked, session);
    // Exactly the first turn survives; the second user message is cut off.
    let kept: Vec<_> = entry_ids(Path::new(&forked)).into_iter().map(|(_, role)| role).collect();
    assert!(kept.contains(&"user".to_owned()));
    assert_eq!(kept.iter().filter(|r| *r == "user").count(), 1);
    let source_entries = entry_ids(Path::new(&session));
    let cut = source_entries.iter().position(|(id, _)| *id == users[1]).unwrap();
    assert_eq!(entry_ids(Path::new(&forked)), source_entries[..cut].to_vec());
    // The source is untouched. `--fork` wrote a full copy first and Pi names
    // it the child's parent, so both new files exist (nothing dangles).
    assert!(source_entries.len() > cut);
    let after = session_files(&env);
    assert_eq!(after.len(), before.len() + 2, "{after:?}");
    // The short-lived process re-homes the session and never runs user code.
    let fork_launch = env
        .starts()
        .into_iter()
        .find(|s| s["argv"].as_array().unwrap().iter().any(|a| a == "--fork"))
        .unwrap();
    let argv: Vec<_> = fork_launch["argv"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    assert!(argv.contains(&"--no-extensions") && argv.contains(&"--no-tools"), "{argv:?}");
    assert!(!argv.contains(&"-e"));
    assert!(fork_launch["cwd"].as_str().unwrap().ends_with("fork-cwd"));
    let header: Value = serde_json::from_str(
        std::fs::read_to_string(&forked).unwrap().lines().next().unwrap(),
    )
    .unwrap();
    let copy = header["parentSession"].as_str().unwrap();
    assert!(Path::new(copy).is_file(), "the child's parent exists");
    assert_ne!(copy, session);
    assert_eq!(
        entry_ids(Path::new(copy)),
        source_entries,
        "the parent is the CLI's full copy of the source"
    );
    assert_eq!(argv[argv.iter().position(|a| *a == "--fork").unwrap() + 1], session);
}

#[tokio::test]
async fn a_head_fork_copies_the_whole_session() {
    let env = Env::new();
    std::fs::create_dir_all(env.path("fork-cwd")).unwrap();
    let (session, users) = two_turn_session(&env).await;
    let harness = env.harness();
    let forked = harness
        .fork_thread(fork_request(&env, &session, Some(&users[1]), None))
        .await
        .unwrap();
    assert_ne!(forked, session);
    assert_eq!(entry_ids(Path::new(&forked)).len(), entry_ids(Path::new(&session)).len());
    assert!(env.commands("fork").is_empty(), "no cut: the CLI copy is the fork");
}

#[tokio::test]
async fn a_fork_that_cannot_be_proven_is_refused_up_front() {
    let env = Env::new();
    let (session, users) = two_turn_session(&env).await;
    let harness = env.harness();
    // An assistant entry is not a cut point (Pi: "Invalid entry ID for forking").
    let assistant = entry_ids(Path::new(&session))
        .into_iter()
        .find(|(_, role)| role == "assistant")
        .unwrap()
        .0;
    for bad in [
        fork_request(&env, &session, Some(&users[0]), Some(&assistant)),
        fork_request(&env, &session, Some(&users[0]), Some("nope")),
        fork_request(&env, &session, None, None),
        fork_request(&env, "01a11698-d036-75cf-ad2b-434fcca3e719", Some(&users[0]), None),
        fork_request(&env, &env.path("gone.jsonl").display().to_string(), Some(&users[0]), None),
    ] {
        assert!(!harness.can_fork_now(&bad).await.unwrap(), "{bad:?}", bad = bad.source_thread_id);
        assert!(harness.fork_thread(bad).await.is_err());
    }
    assert!(harness.can_fork_from_turn());
    assert!(env.commands("fork").is_empty());
}
