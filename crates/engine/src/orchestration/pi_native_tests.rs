//! Native Pi through the production orchestration host: real `PiHarness`,
//! fake `pi --mode rpc` (`crates/harness/tests/fixtures/fake-pi.py`), real
//! runner, store and transfer delivery. The fixture's record shapes were
//! observed on a live Pi 1.0.4.
#![cfg(unix)]

use crate::{EngineCore, HarnessRegistry};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use zeron_harness::PiHarness;
use zeron_proto::{
    HarnessId,
    orchestration::*,
    provider_settings::WriteProviderInstance,
    orchestration_mcp::{T3ThreadSendInputMode, T3ThreadSendResult},
    transfer::{ForkThreadParams, ThreadSourcePoint},
};
use zeron_rpc::{RpcClient, memory_client, methods};

use super::thread_service::{ThreadSendRequest, ThreadService as _};

struct Fixture {
    core: EngineCore,
    client: RpcClient,
    log: PathBuf,
    sessions: PathBuf,
}

fn fake_pi() -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../harness/tests/fixtures/fake-pi.py");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

async fn setup(root: &Path) -> Fixture {
    let log = root.join("pi.log");
    let sessions = root.join("pi-sessions");
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(PiHarness::new()));
    let core = EngineCore::assemble(&root.join("data"), registry, HarnessId::Pi, None).unwrap();
    let client = memory_client(core.rpc_service());
    // A real provider instance, exactly as settings create one: the fake is the
    // instance's `binaryPath`, the fixture knobs are its environment.
    let env: Vec<Value> = [
        ("FAKE_PI_LOG", log.display().to_string()),
        ("FAKE_PI_SESSIONS", sessions.display().to_string()),
        ("PI_CODING_AGENT_DIR", root.join("agent").display().to_string()),
        ("PI_CODING_AGENT_SESSION_DIR", sessions.display().to_string()),
    ]
    .into_iter()
    .map(|(name, value)| json!({"name": name, "value": value}))
    .collect();
    let input: WriteProviderInstance = serde_json::from_value(json!({
        "instanceId":"pi-instance",
        "instance":{"driver":"pi","displayName":"Pi (fake)","enabled":true,
            "config":{"binaryPath":fake_pi()},"environment":env}
    }))
    .unwrap();
    client.create_provider_instance(&input, None).await.unwrap();
    core.workspace
        .create_space("project", &core.device_id, root.to_str().unwrap(), None, false)
        .unwrap();
    client
        .call(
            methods::MUTATE,
            json!({
                "op":"createChat","chatId":"source","spaceId":"project","cwd":root,
                "config":{"instanceId":"pi-instance","harness":"pi","model":"cpa/gemini-3.8-flash",
                    "reasoning":null,"modelOptions":{},"sandbox":"workspace-write",
                    "runtimeMode":"full-access","interactionMode":"default"}
            }),
        )
        .await
        .unwrap();
    // The catalog is discovered lazily (one short `pi --mode rpc` probe); a
    // run is only admitted once the provider reads as authenticated.
    let models = client
        .call(methods::LIST_MODELS, json!({"instanceId":"pi-instance"}))
        .await
        .unwrap();
    assert!(models.as_array().unwrap().iter().any(|m| m["id"] == "cpa/gemini-3.8-flash"));
    Fixture {
        core,
        client,
        log,
        sessions,
    }
}

impl Fixture {
    async fn send_with(
        &self,
        chat: &str,
        key: &str,
        mode: T3ThreadSendInputMode,
    ) -> T3ThreadSendResult {
        self.core
            .orchestration_host
            .as_ref()
            .unwrap()
            .threads
            .send_to_thread(ThreadSendRequest {
                project_id: "project".into(),
                thread_id: chat.into(),
                command_id: key.into(),
                message_id: format!("message:{key}").into(),
                scheduled_task_id: None,
                sender_thread_id: None,
                text: key.into(),
                attachments: vec![],
                model_selection: None,
                mode,
                created_by: OrchestrationV2Actor::User,
                creation_source: OrchestrationV2CreationSource::Web,
            })
            .await
            .unwrap()
    }

    async fn completed(&self, chat: &str, run_id: &RunId) -> OrchestrationV2Run {
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if let Some(run) = self
                    .core
                    .orchestration
                    .store
                    .thread(&chat.into())
                    .unwrap()
                    .unwrap()
                    .runs
                    .into_iter()
                    .find(|r| &r.id == run_id && r.status == OrchestrationV2RunStatus::Completed)
                {
                    break run;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the Pi run completes")
    }

    async fn send(&self, chat: &str, key: &str) -> OrchestrationV2Run {
        let sent = self.send_with(chat, key, T3ThreadSendInputMode::Auto).await;
        self.completed(chat, &sent.run_id).await
    }

    fn log(&self) -> Vec<Value> {
        std::fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    fn prompts(&self) -> Vec<String> {
        self.log()
            .into_iter()
            .filter(|r| r["kind"] == "command" && r["type"] == "prompt")
            .map(|r| r["message"].as_str().unwrap().to_owned())
            .collect()
    }

    fn provider_turns(&self, chat: &str) -> Vec<Value> {
        self.core
            .orchestration
            .store
            .thread(&chat.into())
            .unwrap()
            .unwrap()
            .records
            .get("provider-turn")
            .cloned()
            .unwrap_or_default()
    }
}

fn session_entries(path: &str) -> Vec<(String, String)> {
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

fn native_thread(core: &EngineCore, chat: &str, run: &OrchestrationV2Run) -> String {
    core.orchestration
        .store
        .thread(&chat.into())
        .unwrap()
        .unwrap()
        .attempts
        .iter()
        .find(|a| Some(&a.id) == run.active_attempt_id.as_ref())
        .and_then(|a| match &a.native_thread_id {
            Optional::Present(id) => Some(id.clone()),
            Optional::Absent => None,
        })
        .expect("the attempt recorded its native thread")
}

#[tokio::test]
async fn pi_turns_record_the_session_file_and_user_entry_refs_and_resume_it() {
    let root = tempfile::tempdir().unwrap();
    let fixture = setup(root.path()).await;
    let first = fixture.send("source", "first goal").await;
    let session = native_thread(&fixture.core, "source", &first);
    assert!(Path::new(&session).is_file(), "the native id is the session file: {session}");
    assert!(session.starts_with(fixture.sessions.to_str().unwrap()));

    // The turn's native ref is its first user entry, which `fork` re-roots at.
    let entries = session_entries(&session);
    let user = entries.iter().find(|(_, role)| role == "user").unwrap().0.clone();
    let turns = fixture.provider_turns("source");
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0]["nativeTurnRef"]["nativeId"], user.as_str());
    assert_eq!(turns[0]["nativeTurnRef"]["strength"], "strong");

    // The next run resumes by that file; no new session appears.
    fixture.send("source", "second goal").await;
    let launches: Vec<_> = fixture
        .log()
        .into_iter()
        .filter(|r| r["kind"] == "start" && r["argv"][0] == "--mode")
        // (the sessionless catalog probe also launches `--mode rpc`)
        .filter(|r| r["argv"].as_array().unwrap().iter().all(|a| a != "--no-session"))
        .collect();
    assert_eq!(launches.len(), 2);
    assert!(launches[0]["argv"].as_array().unwrap().iter().all(|a| a != "--session"));
    assert_eq!(launches[1]["argv"][2], "--session");
    assert_eq!(launches[1]["argv"][3], session.as_str());
    assert_eq!(std::fs::read_dir(&fixture.sessions).unwrap().count(), 1);
    fixture.core.shutdown().await;
}

#[tokio::test]
async fn a_native_pi_fork_carries_only_the_turns_up_to_the_selected_one() {
    let root = tempfile::tempdir().unwrap();
    let fixture = setup(root.path()).await;
    let first = fixture.send("source", "original goal").await;
    fixture.send("source", "later parent secret").await;
    let source_session = native_thread(&fixture.core, "source", &first);

    let reply = fixture
        .client
        .fork_thread(
            ForkThreadParams {
                chat_id: "source".into(),
                command_id: "fork-one".into(),
                target_chat_id: "forked".into(),
                source_point: ThreadSourcePoint::Run {
                    run_id: first.id.0.clone(),
                },
                title: Some("Pinned fork".into()),
            },
            &fixture.core.device_id,
        )
        .await
        .unwrap();
    assert!(reply.refusal.is_none(), "{reply:?}");
    // Forking is an idle bookkeeping step: nothing prompts Pi yet.
    assert_eq!(fixture.prompts(), ["original goal", "later parent secret"]);

    let child = fixture.send("forked", "child improvement").await;
    let child_session = native_thread(&fixture.core, "forked", &child);
    assert_ne!(child_session, source_session);
    // Native delivery: the child resumed a real Pi fork, so its prompt is the
    // user's text alone (no portable-history preamble), and Pi's own history
    // holds turn one but not the later parent turn.
    assert_eq!(fixture.prompts().last().unwrap(), "child improvement");
    let history = std::fs::read_to_string(&child_session).unwrap();
    assert!(history.contains("original goal"), "{history}");
    assert!(!history.contains("later parent secret"), "fork is pinned");
    assert!(history.contains("child improvement"));
    // The fork was cut by Pi itself, before the next turn's user entry.
    let cuts: Vec<_> = fixture
        .log()
        .into_iter()
        .filter(|r| r["kind"] == "command" && r["type"] == "fork")
        .collect();
    assert_eq!(cuts.len(), 1);
    let second_user = session_entries(&source_session)
        .into_iter()
        .filter(|(_, role)| role == "user")
        .nth(1)
        .unwrap()
        .0;
    assert_eq!(cuts[0]["entryId"], second_user.as_str());
    fixture.core.shutdown().await;
}

#[tokio::test]
async fn a_steer_is_confirmed_by_pis_queued_ack_not_by_the_local_boundary() {
    let root = tempfile::tempdir().unwrap();
    let fixture = setup(root.path()).await;
    let started = fixture
        .send_with("source", "slow", T3ThreadSendInputMode::Auto)
        .await;
    // Let the run reach its long tool before steering into it.
    tokio::time::timeout(Duration::from_secs(10), async {
        while !fixture.prompts().contains(&"slow".to_owned()) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let steer = fixture
        .send_with("source", "also say hi", T3ThreadSendInputMode::Steer)
        .await;
    fixture.completed("source", &started.run_id).await;
    assert_eq!(steer.run_id, started.run_id, "the steer joined the running turn");

    // Pi answered the steer's prompt `queued`; that receipt, keyed by the
    // message id, is what the durable queue recorded as acceptance.
    let prompts = fixture
        .log()
        .into_iter()
        .filter(|r| r["kind"] == "command" && r["type"] == "prompt")
        .collect::<Vec<_>>();
    assert_eq!(prompts[1]["message"], "also say hi");
    assert_eq!(prompts[1]["streamingBehavior"], "steer");
    let accepted: i64 = fixture
        .core
        .orchestration
        .store
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM orchestration_steering_acceptances WHERE message_id=?1",
                ["message:also say hi"],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(accepted, 1, "exactly one native receipt for the steered message");
    fixture.core.shutdown().await;
}

#[test]
fn pi_model_and_effort_changes_ride_the_resumed_session_while_other_acp_agents_still_hand_off() {
    use super::task::{SelectionTransition, selection_transition};
    use zeron_proto::provider_instance::ModelSelection;
    let selection = |model: &str| -> ModelSelection {
        serde_json::from_value(json!({"instanceId":"pi-instance","model":model})).unwrap()
    };
    // Pi's RPC `set_model` switches the session in place before the turn.
    assert_eq!(
        selection_transition("pi", &selection("cpa/a"), &selection("cpa/b")),
        SelectionTransition::ApplyOnNextTurn
    );
    // A generic ACP agent has no negotiated in-session switch.
    assert_eq!(
        selection_transition("hermes", &selection("a"), &selection("b")),
        SelectionTransition::CreateWithHandoff
    );
    assert_eq!(
        selection_transition("hermes", &selection("a"), &selection("a")),
        SelectionTransition::ApplyOnNextTurn
    );
}
