//! Opt-in LIVE probe of the native Pi driver against the installed `pi` and a
//! real model. Never runs in CI or by default:
//!
//! ```text
//! NOCHES_PI_LIVE=1 cargo test -p zeron-harness --test pi_live -- --ignored --test-threads=1
//! ```
//!
//! `NOCHES_PI_LIVE_MODEL` (default `cpa/gemini-3.8-flash`) must be a cheap
//! `provider/id` Pi can reach. Sessions go to a temp session dir and a scratch
//! cwd, so the user's own Pi history is untouched; Pi's agent dir (auth,
//! models, extensions) is the user's. Total model calls stay under ~15.
#![cfg(unix)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use futures::stream::BoxStream;
use tokio::sync::{mpsc, oneshot};
use zeron_harness::instance::InstanceLaunch;
use zeron_harness::session_lifecycle::{NativeForkRequest, SessionLifecycle};
use zeron_harness::{CancellationToken, Harness, HarnessError, PiHarness, RunControls, SteerMessage};
use zeron_proto::{
    AgentEvent, DoneStatus, PermissionDecision, ReasoningLevel, RunRequest, RuntimeMode,
    SandboxLevel,
};

fn live() -> Option<(PiHarness, tempfile::TempDir, String)> {
    if std::env::var_os("NOCHES_PI_LIVE").is_none() {
        eprintln!("NOCHES_PI_LIVE not set; skipping");
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let harness = PiHarness::new().with_instance_launch(InstanceLaunch::new(
        [(
            "PI_CODING_AGENT_SESSION_DIR".to_owned(),
            sessions.display().to_string(),
        )]
        .into(),
        Vec::new(),
    ));
    let model = std::env::var("NOCHES_PI_LIVE_MODEL").unwrap_or_else(|_| "cpa/gemini-3.8-flash".into());
    Some((harness, dir, model))
}

struct Run {
    stream: BoxStream<'static, Result<AgentEvent, HarnessError>>,
    steer: mpsc::Sender<SteerMessage>,
    interrupt: CancellationToken,
    permissions: Arc<Mutex<Vec<String>>>,
}

async fn begin(
    harness: &PiHarness,
    cwd: &std::path::Path,
    model: &str,
    prompt: &str,
    resume: Option<String>,
    mode: RuntimeMode,
    decision: PermissionDecision,
) -> Run {
    let (steer, steering) = mpsc::channel(8);
    let interrupt = CancellationToken::new();
    let permissions = Arc::new(Mutex::new(Vec::new()));
    let seen = permissions.clone();
    let controls = RunControls {
        mcp: Default::default(),
        browser: None,
        computer_use_socket: None,
        request_permission: Box::new(move |request| {
            seen.lock().unwrap().push(format!("{}: {}", request.tool, request.description));
            let (tx, rx) = oneshot::channel();
            let option = request
                .options
                .into_iter()
                .find(|o| o.decision == decision)
                .unwrap_or_default();
            let _ = tx.send(option);
            zeron_harness::PermissionReceiver::new(rx, || {})
        }),
        request_input: Box::new(|_| {
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(Vec::new());
            rx
        }),
        steering,
        interrupt: interrupt.clone(),
    };
    let request = RunRequest {
        instance_id: None,
        prompt: prompt.into(),
        harness: None,
        model: Some(model.into()),
        reasoning: Some(ReasoningLevel::Low),
        model_options: Default::default(),
        cwd: cwd.display().to_string(),
        sandbox: SandboxLevel::WorkspaceWrite,
        runtime_mode: mode,
        interaction_mode: Default::default(),
        auto_approve: true,
        attachments: Vec::new(),
        worktree: None,
        resume,
    };
    Run {
        stream: harness.run(request, controls).await.expect("run starts"),
        steer,
        interrupt,
        permissions,
    }
}

async fn until_done(run: &mut Run) -> Vec<AgentEvent> {
    let mut events = Vec::new();
    tokio::time::timeout(Duration::from_secs(120), async {
        while let Some(event) = run.stream.next().await {
            let event = event.unwrap();
            let end = matches!(event, AgentEvent::Done { .. });
            events.push(event);
            if end {
                break;
            }
        }
    })
    .await
    .expect("live turn settles");
    events
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

fn done(events: &[AgentEvent]) -> (DoneStatus, Option<String>, Option<String>) {
    match events.last() {
        Some(AgentEvent::Done { status, error, session_id, .. }) => {
            (*status, error.clone(), session_id.clone())
        }
        other => panic!("expected Done, got {other:?}"),
    }
}

fn scratch(dir: &tempfile::TempDir, name: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[tokio::test]
#[ignore = "live: needs the installed pi and a reachable model"]
async fn live_turn_resume_models_and_compact() {
    let Some((harness, dir, model)) = live() else { return };
    let cwd = scratch(&dir, "work");
    let (free, ..) = (
        harness.models().await.expect("discovery"),
    );
    assert!(free.iter().any(|m| m.id == model), "{model} is listed: {:?}", free.iter().map(|m| &m.id).collect::<Vec<_>>());
    println!("models: {}", free.len());

    let mut run = begin(
        &harness, &cwd, &model, "Remember the code word ZEBRA. Reply with the single word OK.",
        None, RuntimeMode::FullAccess, PermissionDecision::Accept,
    ).await;
    let first = until_done(&mut run).await;
    let (status, error, session) = done(&first);
    assert_eq!(status, DoneStatus::Completed, "{error:?}");
    let session = session.expect("session file");
    assert!(PathBuf::from(&session).is_file(), "{session}");
    assert!(text(&first).to_uppercase().contains("OK"), "{}", text(&first));
    assert!(first.iter().any(|e| matches!(e, AgentEvent::InputAccepted)));
    assert!(first.iter().any(|e| matches!(
        e, AgentEvent::NativeReference { turn_id: Some(_), .. })));
    assert!(first.iter().any(|e| matches!(
        e, AgentEvent::ContextUsageSnapshot { usage } if usage.window.is_some() && usage.tokens.is_some())));
    drop(run);

    // Resume the same session by path and prove Pi really has the history.
    let mut run = begin(
        &harness, &cwd, &model, "What was the code word? Reply with only the word.",
        Some(session.clone()), RuntimeMode::FullAccess, PermissionDecision::Accept,
    ).await;
    let second = until_done(&mut run).await;
    let (status, error, resumed) = done(&second);
    assert_eq!(status, DoneStatus::Completed, "{error:?}");
    assert_eq!(resumed.as_deref(), Some(session.as_str()));
    assert!(text(&second).to_uppercase().contains("ZEBRA"), "{}", text(&second));

    // A small session cannot be compacted: Pi's own error is the outcome.
    let mut run = begin(
        &harness, &cwd, &model, "/compact", Some(session), RuntimeMode::FullAccess,
        PermissionDecision::Accept,
    ).await;
    let compact = until_done(&mut run).await;
    let (status, error, _) = done(&compact);
    println!("compact: {status:?} {error:?}");
    assert!(
        status == DoneStatus::Errored && error.as_deref().is_some_and(|e| e.contains("Nothing to compact"))
            || status == DoneStatus::Completed,
        "{status:?} {error:?}"
    );
}

#[tokio::test]
#[ignore = "live: needs the installed pi and a reachable model"]
async fn live_steering_receipts_and_interrupt() {
    let Some((harness, dir, model)) = live() else { return };
    let cwd = scratch(&dir, "work");
    let mut run = begin(
        &harness, &cwd, &model,
        "Use the bash tool to run `sleep 8`, then reply with the single word FIRST.",
        None, RuntimeMode::FullAccess, PermissionDecision::Accept,
    ).await;
    let mut events = Vec::new();
    while let Some(event) = run.stream.next().await {
        let event = event.unwrap();
        let tool = matches!(event, AgentEvent::ToolCall { .. });
        events.push(event);
        if tool { break; }
    }
    let (tx, rx) = oneshot::channel();
    run.steer.send(SteerMessage {
        prompt: "Also append the word STEERED to your final reply.".into(),
        message_id: Some("live-steer".into()),
        attachments: Vec::new(),
        notification_acceptance: Some(tx),
    }).await.unwrap();
    events.extend(until_done(&mut run).await);
    assert!(rx.await.unwrap(), "Pi accepted the steer");
    println!("steer events: {:?}", events.iter().filter(|e| matches!(e,
        AgentEvent::InputAcceptedFor{..} | AgentEvent::Steered{..} | AgentEvent::Done{..})).collect::<Vec<_>>());
    let accepted = events.iter().position(|e| matches!(e,
        AgentEvent::InputAcceptedFor { message_id } if message_id == "live-steer")).expect("receipt");
    let steered = events.iter().position(|e| matches!(e, AgentEvent::Steered { .. })).expect("boundary");
    assert!(accepted < steered, "queued ack precedes delivery");
    assert!(text(&events).to_uppercase().contains("STEERED"), "{}", text(&events));
    assert_eq!(done(&events).0, DoneStatus::Completed);

    // Interrupt a running tool.
    let mut run = begin(
        &harness, &cwd, &model,
        "Use the bash tool to run `sleep 60`, then reply DONE.",
        None, RuntimeMode::FullAccess, PermissionDecision::Accept,
    ).await;
    while let Some(event) = run.stream.next().await {
        if matches!(event.unwrap(), AgentEvent::ToolCall { .. }) { break; }
    }
    run.interrupt.cancel();
    let events = until_done(&mut run).await;
    assert_eq!(done(&events).0, DoneStatus::Interrupted, "{:?}", done(&events));
}

#[tokio::test]
#[ignore = "live: needs the installed pi and a reachable model"]
async fn live_native_fork_keeps_only_the_earlier_turns() {
    let Some((harness, dir, model)) = live() else { return };
    let cwd = scratch(&dir, "work");
    let mut run = begin(
        &harness, &cwd, &model, "Remember the code word ZEBRA. Reply with the single word OK.",
        None, RuntimeMode::FullAccess, PermissionDecision::Accept,
    ).await;
    let first = until_done(&mut run).await;
    let (_, _, session) = done(&first);
    let session = session.unwrap();
    let first_turn = first.iter().find_map(|e| match e {
        AgentEvent::NativeReference { turn_id: Some(id), .. } => Some(id.clone()),
        _ => None,
    }).expect("turn ref");
    run.steer.send(SteerMessage {
        prompt: "Now remember the code word LLAMA instead. Reply with the single word OK.".into(),
        message_id: Some("m2".into()), attachments: Vec::new(), notification_acceptance: None,
    }).await.unwrap();
    let second = until_done(&mut run).await;
    drop(run);
    assert_eq!(done(&second).0, DoneStatus::Completed);
    // The second turn's ref comes from the stream the engine would record.
    let users: Vec<String> = std::fs::read_to_string(&session).unwrap().lines().filter_map(|line| {
        let entry: serde_json::Value = serde_json::from_str(line).ok()?;
        (entry["type"] == "message" && entry["message"]["role"] == "user")
            .then(|| entry["id"].as_str().unwrap().to_owned())
    }).collect();
    println!("user entries: {users:?}, first turn ref: {first_turn}");
    assert_eq!(users.len(), 2);
    assert_eq!(users[0], first_turn, "the turn ref is the first user entry");

    let fork_cwd = scratch(&dir, "fork-work");
    let request = NativeForkRequest {
        source_thread_id: session.clone(),
        source_turn_id: Some(first_turn),
        source_next_turn_id: Some(users[1].clone()),
        rollback_turns: None,
        cwd: fork_cwd.display().to_string(),
        model: model.clone(),
        runtime_mode: Default::default(),
        interaction_mode: Default::default(),
        mcp: Default::default(),
    };
    assert!(harness.can_fork_now(&request).await.unwrap());
    let forked = harness.fork_thread(request).await.expect("native fork");
    assert_ne!(forked, session);
    let header: serde_json::Value = serde_json::from_str(
        std::fs::read_to_string(&forked).unwrap().lines().next().unwrap()).unwrap();
    println!("fork header: {header}");
    assert!(header["cwd"].as_str().unwrap().ends_with("fork-work"), "re-homed: {header}");

    let mut run = begin(
        &harness, &fork_cwd, &model, "What was the code word? Reply with only the word.",
        Some(forked), RuntimeMode::FullAccess, PermissionDecision::Accept,
    ).await;
    let answer = until_done(&mut run).await;
    assert_eq!(done(&answer).0, DoneStatus::Completed);
    let reply = text(&answer).to_uppercase();
    assert!(reply.contains("ZEBRA") && !reply.contains("LLAMA"), "{reply}");
}

#[tokio::test]
#[ignore = "live: needs the installed pi and a reachable model"]
async fn live_supervised_mode_blocks_tools_until_the_gate_allows_them() {
    let Some((harness, dir, model)) = live() else { return };
    let cwd = scratch(&dir, "work");
    let mut run = begin(
        &harness, &cwd, &model,
        "Use the bash tool to run `echo gate-probe > probe.txt`, then reply DONE.",
        None, RuntimeMode::ApprovalRequired, PermissionDecision::Decline,
    ).await;
    let events = until_done(&mut run).await;
    let asked = run.permissions.lock().unwrap().clone();
    println!("declined: asked={asked:?}");
    assert!(asked.iter().any(|a| a.starts_with("bash:")), "{asked:?}");
    assert!(!cwd.join("probe.txt").exists(), "a declined tool must not run");
    assert!(events.iter().any(|e| matches!(e, AgentEvent::ToolResult { is_error: true, .. })));
    drop(run);

    let mut run = begin(
        &harness, &cwd, &model,
        "Use the bash tool to run `echo gate-probe > probe.txt`, then reply DONE.",
        None, RuntimeMode::ApprovalRequired, PermissionDecision::Accept,
    ).await;
    until_done(&mut run).await;
    assert!(cwd.join("probe.txt").exists(), "an allowed tool runs");
    assert!(!run.permissions.lock().unwrap().is_empty());
}
