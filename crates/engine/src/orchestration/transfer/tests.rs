use super::context::*;
use super::delivery::*;
use super::*;
use crate::orchestration::{ReceiptStatus, WriteBoundary};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use zeron_proto::orchestration::*;
use zeron_sync::DocsStore;

pub(crate) fn sample(name: &str) -> Value {
    let cases: std::collections::BTreeMap<String, Vec<Value>> = serde_json::from_str(include_str!(
        "../../../../proto/tests/t3_oracle/fixtures/serde-cases.json"
    ))
    .unwrap();
    cases[name][0].clone()
}

#[test]
fn premerge_slice_database_versions_reconcile_without_losing_metadata() {
    for wave3 in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let docs = Arc::new(DocsStore::open(dir.path()).unwrap());
        docs.with_connection(|conn| -> rusqlite::Result<()> {
            conn.execute_batch(include_str!("../schema.sql"))?;
            conn.execute_batch(
                "CREATE TABLE orchestration_schema_migrations
                 (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL) STRICT;
                 INSERT INTO orchestration_schema_migrations VALUES(1,1),(2,2);",
            )?;
            if wave3 {
                conn.execute_batch(include_str!("../schema_scheduler.sql"))?;
                conn.execute_batch(include_str!("../schema_git_actions.sql"))?;
                conn.execute_batch(
                    "INSERT INTO orchestration_schema_migrations VALUES(3,3);
                     INSERT INTO git_actions_kv VALUES('test','retained','original');",
                )?;
            } else {
                conn.execute_batch(include_str!("../schema_transfer.sql"))?;
                conn.execute_batch(
                    "INSERT INTO orchestration_transfer_delivery
                     VALUES('retained','run:original',NULL,'inline_staged','{}');",
                )?;
            }
            Ok(())
        })
        .unwrap();
        for _ in 0..2 {
            let kernel = Kernel::open(docs.clone(), "host").unwrap();
            kernel.store.read(|conn| {
                for table in [
                    "orchestration_file_checkpoints", "orchestration_transfer_delivery",
                    "orchestration_scheduled_tasks", "orchestration_scheduled_runs", "git_actions_kv",
                ] {
                    let exists: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                        [table], |row| row.get(0),
                    )?;
                    assert_eq!(exists, 1, "missing {table}, wave3={wave3}");
                }
                let retained: String = if wave3 {
                    conn.query_row("SELECT value FROM git_actions_kv WHERE key='retained'", [], |row| row.get(0))?
                } else {
                    conn.query_row("SELECT target_run_id FROM orchestration_transfer_delivery WHERE transfer_id='retained'", [], |row| row.get(0))?
                };
                assert_eq!(retained, if wave3 { "original" } else { "run:original" });
                Ok(())
            }).unwrap();
        }
    }
}

pub(crate) fn fixture(cwd: &std::path::Path) -> (tempfile::TempDir, Kernel, OrchestrationV2Run) {
    let dir = tempfile::tempdir().unwrap();
    let kernel = Kernel::open(Arc::new(DocsStore::open(dir.path()).unwrap()), "host").unwrap();
    let mut create = sample("OrchestrationV2Command");
    create["type"] = json!("thread.create");
    create["commandId"] = json!("create:source");
    create["threadId"] = json!("source");
    create["projectId"] = json!("project");
    create["title"] = json!("Source");
    create["worktreePath"] = json!(cwd);
    let receipt = kernel
        .store
        .dispatch(
            &Command::wire(serde_json::from_value(create).unwrap()).unwrap(),
            1_800_000_000_000,
        )
        .unwrap();
    assert_eq!(receipt.status, ReceiptStatus::Accepted);
    let mut run = sample("OrchestrationV2Run");
    run["id"] = json!("run:source");
    run["threadId"] = json!("source");
    run["ordinal"] = json!(1);
    run["status"] = json!("completed");
    run["checkpointId"] = json!("checkpoint:source");
    run["providerThreadId"] = json!("provider:source");
    run["activeAttemptId"] = json!("attempt:source");
    let mut provider = sample("OrchestrationV2ProviderThread");
    provider["id"] = json!("provider:source");
    provider["appThreadId"] = json!("source");
    provider["nativeThreadRef"] =
        json!({"driver":"codex","strength":"strong","nativeId":"native:source"});
    kernel.store.write(|tx| {
        tx.execute("INSERT INTO orchestration_projection_runs (id,thread_id,ordinal,status,provider_instance_id,payload_json,last_sequence) VALUES('run:source','source',1,'completed',?1,?2,1)",
            rusqlite::params![run["providerInstanceId"].as_str().unwrap(),run.to_string()])?;
        tx.execute("INSERT INTO orchestration_projection_records (thread_id,kind,id,payload_json,last_sequence) VALUES('source','provider-thread','provider:source',?1,1)",[provider.to_string()])?;
        Ok(())
    }).unwrap();
    (dir, kernel, serde_json::from_value(run).unwrap())
}

fn message(id: &str, role: &str, text: &str) -> Value {
    json!({"itemId":id,"role":role,"text":text,"threadId":"thread:handoff","runId":"run:source",
        "providerThreadId":"provider-thread:source","status":"interrupted","kind":if role=="user" {"user_message"} else {"assistant_message"}})
}
fn messages() -> Vec<Value> {
    vec![
        message(
            "item:one",
            "user",
            "Preserve every line.\n\n  And this indentation.\n",
        ),
        message(
            "item:two",
            "assistant",
            &format!("Partial work: 日本語 🧪 مرحبا\n{}", "x".repeat(600)),
        ),
    ]
}

async fn start(kernel: &Kernel, thread: &str, message_id: &str) -> OrchestrationV2Run {
    start_text(
        kernel,
        thread,
        message_id,
        "Current request stays intact.\n🧪",
    )
    .await
}

async fn start_text(
    kernel: &Kernel,
    thread: &str,
    message_id: &str,
    text: &str,
) -> OrchestrationV2Run {
    kernel
        .task_command(
            &thread.into(),
            format!("start:{message_id}").into(),
            crate::orchestration::task::TaskOperation::ExternalMessage {
                prompt: text.into(),
                driver: zeron_proto::provider_instance::ProviderDriverKind("mock".into()),
                message_id: message_id.into(),
            },
        )
        .await
        .unwrap();
    kernel
        .store
        .thread(&thread.into())
        .unwrap()
        .unwrap()
        .runs
        .last()
        .unwrap()
        .clone()
}

#[tokio::test]
async fn unaccepted_input_is_recovered_once_in_native_continuation_across_restart() {
    for restart in [false, true] {
        let cwd = tempfile::tempdir().unwrap();
        let (db, kernel, _) = fixture(cwd.path());
        let first = start_text(
            &kernel,
            "source",
            "known-input",
            "Already in native history.",
        )
        .await;
        accept(&kernel, &first, cwd.path(), "retained-native").await;
        observe(
            &kernel,
            &first,
            zeron_proto::AgentEvent::TextDelta {
                text: "Do not duplicate the accepted decision.".into(),
            },
        )
        .await;
        observe(&kernel, &first, done(zeron_proto::DoneStatus::Completed)).await;
        let missed_text = "Input never accepted by the provider.\n  Preserve 日本語 🧪.";
        let missed = start_text(&kernel, "source", "unaccepted-input", missed_text).await;
        observe(
            &kernel,
            &missed,
            zeron_proto::AgentEvent::Done {
                status: zeron_proto::DoneStatus::Errored,
                result: None,
                error: Some("Provider unavailable before accepting input.".into()),
                session_id: None,
            },
        )
        .await;
        let p = kernel.store.thread(&missed.thread_id).unwrap().unwrap();
        assert!(
            p.attempts
                .iter()
                .find(|a| Some(&a.id) == missed.active_attempt_id.as_ref())
                .unwrap()
                .native_thread_id
                .as_ref()
                .is_none()
        );
        assert!(
            !crate::orchestration::task::records(&p, "provider-turn")
                .iter()
                .any(|turn| Some(turn["runAttemptId"].as_str().unwrap_or(""))
                    == missed.active_attempt_id.as_ref().map(|id| id.0.as_str()))
        );
        let kernel = if restart {
            let reopened =
                Kernel::open(Arc::new(DocsStore::open(db.path()).unwrap()), "host").unwrap();
            reopened.recover(crate::now_ms()).await.unwrap();
            reopened
        } else {
            kernel
        };
        let next = start(&kernel, "source", "continue-after-outage").await;
        assert_eq!(next.provider_thread_id, first.provider_thread_id);
        let mut input = request(cwd.path());
        prepare(&kernel, &next, &mut input).await.unwrap();
        assert_eq!(input.resume.as_deref(), Some("retained-native"));
        assert!(
            input.prompt.contains(missed_text),
            "restart={restart}: missing unaccepted input"
        );
        assert!(
            input
                .prompt
                .contains("Provider unavailable before accepting input.")
        );
        assert!(
            !input
                .prompt
                .contains("Do not duplicate the accepted decision.")
        );
        assert!(input.prompt.ends_with(&request(cwd.path()).prompt));
        accept(&kernel, &next, cwd.path(), "retained-native").await;
        let p = kernel.store.thread(&next.thread_id).unwrap().unwrap();
        let retry = crate::orchestration::task::records(&p, "context-handoff")
            .iter()
            .find(|h| h["targetRunId"] == next.id.0)
            .unwrap();
        assert_eq!(retry["delivery"]["status"], "inline");
        assert_eq!(
            retry["coveredRunOrdinals"],
            json!({"from":missed.ordinal,"to":missed.ordinal})
        );
        let mut accepted = request(cwd.path());
        prepare(&kernel, &next, &mut accepted).await.unwrap();
        assert_eq!(accepted.prompt, request(cwd.path()).prompt);
        observe(&kernel, &next, done(zeron_proto::DoneStatus::Completed)).await;
        let later = start(&kernel, "source", "normal-next-input").await;
        let mut later_input = request(cwd.path());
        prepare(&kernel, &later, &mut later_input).await.unwrap();
        assert_eq!(later_input.prompt, request(cwd.path()).prompt);
        assert_eq!(
            kernel
                .store
                .thread_transfers(&later.thread_id)
                .unwrap()
                .len(),
            1
        );
    }
}

#[tokio::test]
async fn initialized_but_rejected_input_is_recovered_after_projection_rebuild() {
    for (restart, status) in [
        (false, zeron_proto::DoneStatus::Errored),
        (true, zeron_proto::DoneStatus::Errored),
        (false, zeron_proto::DoneStatus::Interrupted),
        (true, zeron_proto::DoneStatus::Interrupted),
    ] {
        let cwd = tempfile::tempdir().unwrap();
        let (db, kernel, _) = fixture(cwd.path());
        let known = start_text(&kernel, "source", "known", "Native already knows this.").await;
        accept(&kernel, &known, cwd.path(), "retained-native").await;
        observe(&kernel, &known, done(zeron_proto::DoneStatus::Completed)).await;
        let missed = start_text(
            &kernel,
            "source",
            "rejected",
            "Missing after turn/start rejection.",
        )
        .await;
        let mut original = request(cwd.path());
        prepare(&kernel, &missed, &mut original).await.unwrap();
        observe(
            &kernel,
            &missed,
            session_started(cwd.path(), "retained-native"),
        )
        .await;
        observe(
            &kernel,
            &missed,
            zeron_proto::AgentEvent::Steered {
                assistant_message_id: None,
                next_assistant_message_id: None,
            },
        )
        .await;
        observe(
            &kernel,
            &missed,
            zeron_proto::AgentEvent::NativeReference {
                thread_id: "retained-native".into(),
                turn_id: None,
            },
        )
        .await;
        observe(
            &kernel,
            &missed,
            zeron_proto::AgentEvent::ContextUsage {
                tokens: Some(10),
                window: Some(128_000),
            },
        )
        .await;
        let p = kernel.store.thread(&missed.thread_id).unwrap().unwrap();
        let attempt = p
            .attempts
            .iter()
            .find(|a| Some(&a.id) == missed.active_attempt_id.as_ref())
            .unwrap();
        assert_eq!(
            attempt.native_thread_id.as_ref().map(String::as_str),
            Some("retained-native")
        );
        assert!(
            attempt.provider_turn_id.as_ref().is_none(),
            "thread initialization is not input acceptance"
        );
        assert!(
            !crate::orchestration::task::records(&p, "provider-turn")
                .iter()
                .any(|t| t["runAttemptId"] == attempt.id.0),
            "usage/thread telemetry must not manufacture accepted turns"
        );
        observe(
            &kernel,
            &missed,
            zeron_proto::AgentEvent::Done {
                status,
                result: None,
                error: Some("turn/start rejected".into()),
                session_id: Some("retained-native".into()),
            },
        )
        .await;
        let kernel = if restart {
            let reopened =
                Kernel::open(Arc::new(DocsStore::open(db.path()).unwrap()), "host").unwrap();
            reopened.store.rebuild().unwrap();
            reopened.recover(crate::now_ms()).await.unwrap();
            reopened
        } else {
            kernel
        };
        let next = start(&kernel, "source", "next").await;
        let mut input = request(cwd.path());
        input.attachments = vec!["/tmp/current-attachment.png".into()];
        prepare(&kernel, &next, &mut input).await.unwrap();
        assert_eq!(input.resume.as_deref(), Some("retained-native"));
        assert!(input.prompt.contains("Missing after turn/start rejection."));
        assert!(input.prompt.contains("turn/start rejected"));
        assert!(!input.prompt.contains("Native already knows this."));
        assert_eq!(input.attachments, vec!["/tmp/current-attachment.png"]);
        observe(&kernel, &missed, zeron_proto::AgentEvent::InputAccepted).await;
        let p = kernel.store.thread(&next.thread_id).unwrap().unwrap();
        assert_eq!(
            p.records["context-handoff"].last().unwrap()["delivery"]["status"],
            "pending",
            "stale failed-attempt acceptance cannot settle the current retry"
        );
        accept(&kernel, &next, cwd.path(), "retained-native").await;
        observe(&kernel, &next, done(zeron_proto::DoneStatus::Completed)).await;
        let later = start(&kernel, "source", "later").await;
        let mut input = request(cwd.path());
        prepare(&kernel, &later, &mut input).await.unwrap();
        assert_eq!(input.prompt, request(cwd.path()).prompt);
        assert_eq!(
            kernel
                .store
                .thread_transfers(&later.thread_id)
                .unwrap()
                .len(),
            1
        );
    }
}

#[tokio::test]
async fn warm_assignment_acceptance_attaches_and_settles_without_another_session_init() {
    let cwd = tempfile::tempdir().unwrap();
    let (_, kernel, _) = fixture(cwd.path());
    let known = start(&kernel, "source", "known").await;
    accept(&kernel, &known, cwd.path(), "warm-native").await;
    observe(&kernel, &known, done(zeron_proto::DoneStatus::Completed)).await;
    let missed = start_text(&kernel, "source", "missed", "Undelivered warm input.").await;
    observe(&kernel, &missed, done(zeron_proto::DoneStatus::Errored)).await;
    let next = start(&kernel, "source", "warm-retry").await;
    let mut input = request(cwd.path());
    prepare(&kernel, &next, &mut input).await.unwrap();
    assert!(input.prompt.contains("Undelivered warm input."));
    observe(
        &kernel,
        &next,
        zeron_proto::AgentEvent::Steered {
            assistant_message_id: None,
            next_assistant_message_id: None,
        },
    )
    .await;
    let p = kernel.store.thread(&next.thread_id).unwrap().unwrap();
    assert!(
        p.attempts
            .iter()
            .find(|a| Some(&a.id) == next.active_attempt_id.as_ref())
            .unwrap()
            .provider_turn_id
            .as_ref()
            .is_none(),
        "a local steering boundary is not provider acceptance"
    );
    observe(
        &kernel,
        &next,
        zeron_proto::AgentEvent::InputAcceptedFor {
            message_id: known.user_message_id.0.clone(),
        },
    )
    .await;
    let p = kernel.store.thread(&next.thread_id).unwrap().unwrap();
    assert!(
        p.attempts
            .iter()
            .find(|a| Some(&a.id) == next.active_attempt_id.as_ref())
            .unwrap()
            .provider_turn_id
            .as_ref()
            .is_none(),
        "an acknowledgement of an older message cannot accept the next root input"
    );
    assert!(
        p.records["context-handoff"][0]["delivery"].is_null()
            || p.records["context-handoff"][0]["delivery"]["status"] == "pending"
    );
    observe(
        &kernel,
        &next,
        zeron_proto::AgentEvent::InputAcceptedFor {
            message_id: next.user_message_id.0.clone(),
        },
    )
    .await;
    let p = kernel.store.thread(&next.thread_id).unwrap().unwrap();
    let attempt = p
        .attempts
        .iter()
        .find(|a| Some(&a.id) == next.active_attempt_id.as_ref())
        .unwrap();
    assert_eq!(
        attempt.native_thread_id.as_ref().map(String::as_str),
        Some("warm-native")
    );
    assert!(attempt.provider_turn_id.as_ref().is_some());
    assert_eq!(
        p.records["context-handoff"][0]["delivery"]["status"],
        "inline"
    );
    assert_eq!(
        p.records["turn-item"]
            .iter()
            .filter(|i| i["type"] == "user_message" && i["messageId"] == next.user_message_id.0)
            .count(),
        1
    );
}

#[tokio::test]
async fn full_model_switch_handoff_already_covers_unaccepted_input() {
    let cwd = tempfile::tempdir().unwrap();
    let (_, kernel, _) = fixture(cwd.path());
    let known = start(&kernel, "source", "known").await;
    accept(&kernel, &known, cwd.path(), "old-model-native").await;
    observe(&kernel, &known, done(zeron_proto::DoneStatus::Completed)).await;
    let missed = start_text(
        &kernel,
        "source",
        "missed",
        "Untold input before model switch.",
    )
    .await;
    observe(&kernel, &missed, done(zeron_proto::DoneStatus::Errored)).await;
    select_instance(
        &kernel,
        &known.provider_instance_id.0,
        Some("different-model"),
    )
    .await;
    let next = start(&kernel, "source", "switch").await;
    let mut input = request(cwd.path());
    prepare(&kernel, &next, &mut input).await.unwrap();
    assert_eq!(
        input
            .prompt
            .matches("Untold input before model switch.")
            .count(),
        1
    );
    assert!(input.resume.is_none());
    assert_eq!(
        kernel
            .store
            .thread_transfers(&next.thread_id)
            .unwrap()
            .len(),
        1,
        "a full switch must not allocate a duplicate retry handoff"
    );
}

#[tokio::test]
async fn real_acceptance_signals_bind_root_turns_but_not_session_metadata() {
    use zeron_proto::AgentEvent;
    for event in [
        AgentEvent::InputAccepted,
        AgentEvent::NativeReference {
            thread_id: "native-accepted".into(),
            turn_id: Some("actual-turn".into()),
        },
        AgentEvent::TextDelta {
            text: "Response.".into(),
        },
        AgentEvent::ReasoningDelta {
            text: "Thinking.".into(),
        },
        AgentEvent::UserMessage {
            text: "Confirmed assignment.".into(),
        },
        AgentEvent::ToolCall {
            id: "tool".into(),
            call: zeron_proto::ToolCall::Exec {
                command: "pwd".into(),
            },
        },
        done(zeron_proto::DoneStatus::Completed),
    ] {
        let cwd = tempfile::tempdir().unwrap();
        let (_, kernel, _) = fixture(cwd.path());
        let run = start(&kernel, "source", "signal").await;
        observe(
            &kernel,
            &run,
            session_started(cwd.path(), "native-accepted"),
        )
        .await;
        observe(
            &kernel,
            &run,
            AgentEvent::ContextUsageSnapshot {
                usage: zeron_proto::ContextUsage {
                    tokens: Some(100),
                    window: Some(128_000),
                    ..Default::default()
                },
            },
        )
        .await;
        let p = kernel.store.thread(&run.thread_id).unwrap().unwrap();
        assert!(
            !crate::orchestration::task::records(&p, "provider-turn")
                .iter()
                .any(|t| Some(t["runAttemptId"].as_str().unwrap_or(""))
                    == run.active_attempt_id.as_ref().map(|id| id.0.as_str()))
        );
        observe(&kernel, &run, event.clone()).await;
        let p = kernel.store.thread(&run.thread_id).unwrap().unwrap();
        let attempt = p
            .attempts
            .iter()
            .find(|a| Some(&a.id) == run.active_attempt_id.as_ref())
            .unwrap();
        let turn = attempt
            .provider_turn_id
            .as_ref()
            .unwrap_or_else(|| panic!("unbound acceptance: {event:?}"));
        assert!(
            p.records["provider-turn"]
                .iter()
                .any(|t| t["id"] == turn.0 && t["nodeId"] == attempt.root_node_id.0)
        );
        assert!(
            p.records["turn-item"]
                .iter()
                .any(|i| i["type"] == "user_message"
                    && i["messageId"] == run.user_message_id.0
                    && i["providerTurnId"] == turn.0)
        );
        let user_ordinal = p.records["turn-item"]
            .iter()
            .find(|i| i["type"] == "user_message" && i["messageId"] == run.user_message_id.0)
            .unwrap()["ordinal"]
            .as_i64()
            .unwrap();
        for assistant in p.records["turn-item"].iter().filter(|i| {
            i["type"] == "assistant_message"
                && i["runId"] == run.id.0
                && i["providerTurnId"] == turn.0
        }) {
            assert!(
                user_ordinal < assistant["ordinal"].as_i64().unwrap(),
                "acceptance and first output in one command must preserve user-before-assistant ordering: {event:?}"
            );
        }
    }
}

#[tokio::test]
async fn accepted_failed_or_interrupted_turns_are_not_replayed() {
    for status in [
        zeron_proto::DoneStatus::Errored,
        zeron_proto::DoneStatus::Interrupted,
    ] {
        let cwd = tempfile::tempdir().unwrap();
        let (_, kernel, _) = fixture(cwd.path());
        let run = start_text(
            &kernel,
            "source",
            "accepted-failure",
            "Already delivered, not retry context.",
        )
        .await;
        accept(&kernel, &run, cwd.path(), "native-accepted").await;
        observe(&kernel, &run, done(status)).await;
        let next = start(&kernel, "source", "next").await;
        let mut input = request(cwd.path());
        prepare(&kernel, &next, &mut input).await.unwrap();
        assert_eq!(input.resume.as_deref(), Some("native-accepted"));
        assert_eq!(input.prompt, request(cwd.path()).prompt);
        assert!(
            kernel
                .store
                .thread_transfers(&next.thread_id)
                .unwrap()
                .is_empty()
        );
    }
}

#[tokio::test]
async fn retry_budget_omissions_are_durable_and_not_reinjected() {
    let cwd = tempfile::tempdir().unwrap();
    let (db, kernel, _) = fixture(cwd.path());
    let known = start(&kernel, "source", "known").await;
    accept(&kernel, &known, cwd.path(), "native-budget").await;
    observe(&kernel, &known, done(zeron_proto::DoneStatus::Completed)).await;
    let missed = start_text(&kernel, "source", "oversized-failure", &"🧪".repeat(40_000)).await;
    observe(&kernel, &missed, done(zeron_proto::DoneStatus::Errored)).await;
    let next = start(&kernel, "source", "retry").await;
    let mut input = request(cwd.path());
    prepare(&kernel, &next, &mut input).await.unwrap();
    assert!(!input.prompt.contains("🧪🧪"));
    assert!(input.prompt.ends_with(&request(cwd.path()).prompt));
    accept(&kernel, &next, cwd.path(), "native-budget").await;
    observe(&kernel, &next, done(zeron_proto::DoneStatus::Completed)).await;
    let reopened = Kernel::open(Arc::new(DocsStore::open(db.path()).unwrap()), "host").unwrap();
    reopened.store.rebuild().unwrap();
    let p = reopened.store.thread(&next.thread_id).unwrap().unwrap();
    let receipt = &p.records["context-handoff"][0]["delivery"];
    assert_eq!(receipt["status"], "inline");
    assert!(!receipt["omittedItemIds"].as_array().unwrap().is_empty());
    let later = start(&reopened, "source", "later").await;
    let mut input = request(cwd.path());
    prepare(&reopened, &later, &mut input).await.unwrap();
    assert_eq!(input.prompt, request(cwd.path()).prompt);
    assert_eq!(
        reopened
            .store
            .thread_transfers(&later.thread_id)
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn session_ready_does_not_settle_ambiguous_inline_delivery() {
    let cwd = tempfile::tempdir().unwrap();
    let (_, kernel, _) = fixture(cwd.path());
    let known = start(&kernel, "source", "known").await;
    accept(&kernel, &known, cwd.path(), "native-uncertain").await;
    observe(&kernel, &known, done(zeron_proto::DoneStatus::Completed)).await;
    let missed = start(&kernel, "source", "missed").await;
    observe(&kernel, &missed, done(zeron_proto::DoneStatus::Errored)).await;
    let next = start(&kernel, "source", "uncertain-retry").await;
    let mut input = request(cwd.path());
    prepare(&kernel, &next, &mut input).await.unwrap();
    observe(
        &kernel,
        &next,
        session_started(cwd.path(), "native-uncertain"),
    )
    .await;
    let p = kernel.store.thread(&next.thread_id).unwrap().unwrap();
    assert_eq!(
        p.records["context-handoff"][0]["delivery"]["status"],
        "pending"
    );
    assert!(
        p.attempts
            .iter()
            .find(|a| Some(&a.id) == next.active_attempt_id.as_ref())
            .unwrap()
            .provider_turn_id
            .as_ref()
            .is_none()
    );
    observe(&kernel, &next, done(zeron_proto::DoneStatus::Errored)).await;
    let later = start(&kernel, "source", "later").await;
    let mut input = request(cwd.path());
    assert!(
        prepare(&kernel, &later, &mut input)
            .await
            .unwrap_err()
            .to_string()
            .contains(UNCERTAIN_ERROR)
    );
    let p = kernel.store.thread(&later.thread_id).unwrap().unwrap();
    assert!(
        p.records["context-handoff"]
            .iter()
            .all(|h| h["delivery"]["status"] != "inline")
    );
}

async fn observe(kernel: &Kernel, run: &OrchestrationV2Run, event: zeron_proto::AgentEvent) {
    let caps = crate::orchestration::assembly::capabilities(&zeron_harness::mock::MockHarness {
        script: vec![],
    });
    kernel
        .task_command(
            &run.thread_id,
            format!("event:{}", uuid::Uuid::new_v4()).into(),
            crate::orchestration::task::TaskOperation::RunnerEvent {
                run_id: run.id.clone(),
                attempt_id: run.active_attempt_id.clone().unwrap(),
                event,
                capabilities: Some(Box::new(caps)),
            },
        )
        .await
        .unwrap();
}

fn request(cwd: &std::path::Path) -> zeron_proto::RunRequest {
    serde_json::from_value(json!({"prompt":"Current request stays intact.\n🧪",
        "cwd":cwd,"sandbox":"workspace-write","autoApprove":false}))
    .unwrap()
}

#[tokio::test]
async fn steering_restart_resume_requires_the_exact_accepted_immediate_predecessor() {
    let cwd = tempfile::tempdir().unwrap();
    let other_cwd = tempfile::tempdir().unwrap();
    let (_, kernel, _) = fixture(cwd.path());
    let original = start(&kernel, "source", "original").await;
    accept(&kernel, &original, cwd.path(), "accepted-restart-native").await;
    kernel.store.rebuild().unwrap();
    let accepted = kernel.store.thread(&original.thread_id).unwrap().unwrap();
    for fence in [
        "exact",
        "no-acceptance",
        "turn-id",
        "turn-root",
        "turn-provider",
        "native-id",
        "not-superseded",
        "not-immediate",
        "not-restart",
        "current-run",
        "current-root",
        "current-provider",
        "current-instance",
        "previous-instance",
        "model",
        "options",
        "cwd",
    ] {
        let mut p = accepted.clone();
        let previous_index = p
            .attempts
            .iter()
            .position(|attempt| Some(&attempt.id) == original.active_attempt_id.as_ref())
            .unwrap();
        let mut current = p.attempts[previous_index].clone();
        p.attempts[previous_index].status = OrchestrationV2RunAttemptStatus::Superseded;
        current.id = "attempt:replacement".into();
        current.root_node_id = "node:replacement".into();
        current.attempt_ordinal += 1;
        current.reason = OrchestrationV2RunAttemptReason::SteeringRestart;
        current.status = OrchestrationV2RunAttemptStatus::Pending;
        current.native_thread_id = Optional::Absent;
        current.provider_turn_id = None;
        let mut run = p
            .runs
            .iter()
            .find(|run| run.id == original.id)
            .unwrap()
            .clone();
        run.status = OrchestrationV2RunStatus::Starting;
        run.active_attempt_id = Some(current.id.clone());
        run.root_node_id = Some(current.root_node_id.clone());
        match fence {
            "no-acceptance" => p.records.get_mut("provider-turn").unwrap().clear(),
            "turn-id" | "turn-root" | "turn-provider" => {
                let key = match fence {
                    "turn-id" => "id",
                    "turn-root" => "nodeId",
                    _ => "providerThreadId",
                };
                let turn = p
                    .records
                    .get_mut("provider-turn")
                    .unwrap()
                    .iter_mut()
                    .find(|turn| turn["runAttemptId"] == p.attempts[previous_index].id.0)
                    .unwrap();
                turn[key] = json!("unrelated");
            }
            "native-id" => {
                p.attempts[previous_index].native_thread_id = Optional::Present("unrelated".into())
            }
            "not-superseded" => {
                p.attempts[previous_index].status = OrchestrationV2RunAttemptStatus::Completed
            }
            "not-immediate" => current.attempt_ordinal += 1,
            "not-restart" => current.reason = OrchestrationV2RunAttemptReason::Initial,
            "current-run" => current.run_id = "unrelated".into(),
            "current-root" => current.root_node_id = "unrelated".into(),
            "current-provider" => current.provider_thread_id = "unrelated".into(),
            "current-instance" => current.provider_instance_id = "unrelated".into(),
            "previous-instance" => {
                p.attempts[previous_index].provider_instance_id = "unrelated".into()
            }
            "model" => run.model_selection.model = "different-model".into(),
            "options" => {
                run.model_selection.options =
                    serde_json::from_value(json!([{"id":"reasoningEffort","value":"high"}]))
                        .unwrap()
            }
            _ => {}
        }
        p.attempts.push(current);
        // Keep the original accepted run's saved selection as the comparison
        // point; a caller cannot authorize resume by changing only its request.
        let saved = p.runs.iter_mut().find(|saved| saved.id == run.id).unwrap();
        saved.active_attempt_id = run.active_attempt_id.clone();
        saved.root_node_id = run.root_node_id.clone();
        saved.status = run.status.clone();
        let resume = resumable_native(
            &kernel,
            &p,
            &run,
            if fence == "cwd" {
                other_cwd.path()
            } else {
                cwd.path()
            }
            .to_str()
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            resume.as_deref(),
            (fence == "exact").then_some("accepted-restart-native"),
            "{fence}"
        );
    }
}

#[tokio::test]
async fn restart_recovers_only_unconfirmed_steering_not_the_accepted_root_or_other_steers() {
    use crate::orchestration::{
        command::{Command, Operation},
        effects::{EffectRequest, EffectStatus},
        steering,
        threads::planner::{Send, ThreadOperation},
    };
    let cwd = tempfile::tempdir().unwrap();
    let (db, kernel, _) = fixture(cwd.path());
    let source = start_text(
        &kernel,
        "source",
        "accepted-root",
        "Already accepted root input.",
    )
    .await;
    accept(&kernel, &source, cwd.path(), "native-steering").await;
    for (id, text) in [
        ("accepted-steer", "Already accepted steering."),
        ("untold-steer", "Unconfirmed steering 日本語 🧪."),
    ] {
        let receipt = kernel
            .dispatch(
                &Command {
                    id: format!("steer:{id}").into(),
                    thread_id: "source".into(),
                    operation: Operation::Thread(Box::new(ThreadOperation::Send(Send {
                        message_id: id.into(),
                        text: text.into(),
                        mode: zeron_proto::orchestration_mcp::T3ThreadSendInputMode::Steer,
                        driver: "mock".into(),
                        sender: "source".into(),
                        target_run: Some(source.id.clone()),
                        metadata: None,
                    }))),
                },
                crate::now_ms(),
            )
            .await
            .unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Accepted, "{receipt:?}");
    }
    let effects: Vec<_> = kernel
        .store
        .effects()
        .unwrap()
        .into_iter()
        .filter(|e| matches!(&e.request, EffectRequest::ProviderTurnSteer { .. }))
        .collect();
    let accepted = effects
        .iter()
        .find(|e| {
            matches!(&e.request,
        EffectRequest::ProviderTurnSteer {message_id,..} if message_id.0 == "accepted-steer")
        })
        .unwrap();
    let unknown = effects
        .iter()
        .find(|e| {
            matches!(&e.request,
        EffectRequest::ProviderTurnSteer {message_id,..} if message_id.0 == "untold-steer")
        })
        .unwrap();
    let p = kernel.store.thread(&"source".into()).unwrap().unwrap();
    kernel
        .store
        .write(|tx| {
            steering::confirm(tx, &p, accepted, crate::now_ms())?;
            tx.execute(
                "UPDATE orchestration_effect_outbox SET status='running',dispatch_started=1
            WHERE effect_id IN (?1,?2)",
                rusqlite::params![accepted.id, unknown.id],
            )?;
            Ok(())
        })
        .unwrap();
    let reopened = Kernel::open(Arc::new(DocsStore::open(db.path()).unwrap()), "host").unwrap();
    reopened.recover(crate::now_ms()).await.unwrap();
    assert_eq!(
        reopened.store.effect(&accepted.id).unwrap().unwrap().status,
        EffectStatus::Succeeded
    );
    assert_eq!(
        reopened.store.effect(&unknown.id).unwrap().unwrap().status,
        EffectStatus::Cancelled
    );
    reopened.store.rebuild().unwrap();
    let next = start(&reopened, "source", "current-input").await;
    let mut input = request(cwd.path());
    prepare(&reopened, &next, &mut input).await.unwrap();
    assert!(input.prompt.contains("Unconfirmed steering 日本語 🧪."));
    assert!(
        input
            .prompt
            .contains("Steering acceptance was not confirmed")
    );
    assert!(!input.prompt.contains("Already accepted root input."));
    assert!(!input.prompt.contains("Already accepted steering."));
    assert!(input.prompt.ends_with("Current request stays intact.\n🧪"));
    let mut retry = request(cwd.path());
    let before = reopened.store.thread_transfers(&"source".into()).unwrap();
    assert!(
        prepare(&reopened, &next, &mut retry)
            .await
            .unwrap_err()
            .to_string()
            .contains(UNCERTAIN_ERROR),
        "an unacknowledged native context preparation cannot be blindly injected again"
    );
    assert_eq!(
        reopened.store.thread_transfers(&"source".into()).unwrap(),
        before
    );
    accept(&reopened, &next, cwd.path(), "native-steering").await;
    observe(&reopened, &next, done(zeron_proto::DoneStatus::Completed)).await;
    let later = start(&reopened, "source", "later-input").await;
    let mut later_input = request(cwd.path());
    prepare(&reopened, &later, &mut later_input).await.unwrap();
    assert!(
        !later_input
            .prompt
            .contains("Unconfirmed steering 日本語 🧪."),
        "settled coverage prevents replay of recovered steering"
    );
}

async fn prepare(
    kernel: &Kernel,
    run: &OrchestrationV2Run,
    request: &mut zeron_proto::RunRequest,
) -> Result<()> {
    let harness = zeron_harness::mock::MockHarness { script: vec![] };
    prepare_run(
        kernel,
        &run.thread_id,
        run,
        request,
        &harness,
        &crate::orchestration::assembly::capabilities(&harness),
        Default::default(),
    )
    .await
}

fn session_started(cwd: &std::path::Path, native: &str) -> zeron_proto::AgentEvent {
    zeron_proto::AgentEvent::SessionStarted {
        instance_id: None,
        harness: zeron_proto::HarnessId::Mock,
        model: "mock".into(),
        tools: vec![],
        cwd: cwd.to_string_lossy().into_owned(),
        session_id: native.into(),
        assistant_message_id: String::new(),
    }
}

async fn accept(kernel: &Kernel, run: &OrchestrationV2Run, cwd: &std::path::Path, native: &str) {
    observe(kernel, run, session_started(cwd, native)).await;
    observe(kernel, run, zeron_proto::AgentEvent::InputAccepted).await;
}

fn done(status: zeron_proto::DoneStatus) -> zeron_proto::AgentEvent {
    zeron_proto::AgentEvent::Done {
        status,
        result: None,
        error: None,
        session_id: None,
    }
}

struct ForkHarness {
    ambiguous: bool,
    calls: std::sync::atomic::AtomicUsize,
}
#[async_trait::async_trait]
impl zeron_harness::Harness for ForkHarness {
    fn id(&self) -> zeron_proto::HarnessId {
        zeron_proto::HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Fork fixture"
    }
    fn supports_steering(&self) -> bool {
        false
    }
    fn steering_mode(&self) -> zeron_proto::SteeringMode {
        zeron_proto::SteeringMode::StepBoundary
    }
    fn reasoning_levels(&self) -> &[zeron_proto::ReasoningLevel] {
        &[]
    }
    fn session_lifecycle(&self) -> Option<&dyn zeron_harness::session_lifecycle::SessionLifecycle> {
        Some(self)
    }
    async fn models(
        &self,
    ) -> std::result::Result<Vec<zeron_proto::Model>, zeron_harness::HarnessError> {
        Ok(vec![])
    }
    async fn run(
        &self,
        _: zeron_proto::RunRequest,
        _: zeron_harness::RunControls,
    ) -> std::result::Result<
        futures::stream::BoxStream<
            'static,
            std::result::Result<zeron_proto::AgentEvent, zeron_harness::HarnessError>,
        >,
        zeron_harness::HarnessError,
    > {
        Err(zeron_harness::HarnessError::Protocol("Not used".into()))
    }
}
#[async_trait::async_trait]
impl zeron_harness::session_lifecycle::SessionLifecycle for ForkHarness {
    fn can_fork_from_turn(&self) -> bool {
        true
    }
    async fn fork_thread(
        &self,
        input: zeron_harness::session_lifecycle::NativeForkRequest,
    ) -> std::result::Result<String, zeron_harness::HarnessError> {
        assert_eq!(input.source_thread_id, "native:source");
        assert_eq!(input.source_turn_id.as_deref(), Some("native-turn"));
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.ambiguous {
            Err(zeron_harness::HarnessError::Protocol(
                "Response lost after acceptance".into(),
            ))
        } else {
            Ok("native-fork".into())
        }
    }
}

#[tokio::test]
async fn native_fork_receipt_recovery_and_missing_cursor_portable_fallback() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    for (ambiguous, cursor) in [(false, true), (true, true), (false, false)] {
        let cwd = tempfile::tempdir().unwrap();
        let (db, kernel, source) = fixture(cwd.path());
        if cursor {
            let mut turn = sample("OrchestrationV2ProviderTurn");
            turn["id"] = json!("source-turn");
            turn["providerThreadId"] = json!("provider:source");
            turn["runAttemptId"] = json!("attempt:source");
            turn["nativeTurnRef"] =
                json!({"driver":"codex","nativeId":"native-turn","strength":"strong"});
            kernel.store.write(|tx| {
                tx.execute("INSERT INTO orchestration_projection_records(thread_id,kind,id,payload_json,last_sequence)
                    VALUES('source','provider-turn','source-turn',?1,1)",[turn.to_string()])?;Ok(())
            }).unwrap();
        }
        kernel
            .transfer_command(
                &"source".into(),
                "fork:native".into(),
                TransferOperation::Fork {
                    target: "native-child".into(),
                    source: SourcePoint::Run { run_id: source.id },
                    title: None,
                },
            )
            .await
            .unwrap();
        let run = start(&kernel, "native-child", "native-input").await;
        let harness = ForkHarness {
            ambiguous,
            calls: AtomicUsize::new(0),
        };
        let caps = crate::orchestration::assembly::capabilities(&harness);
        let mut input = request(cwd.path());
        let first = prepare_run(
            &kernel,
            &run.thread_id,
            &run,
            &mut input,
            &harness,
            &caps,
            Default::default(),
        )
        .await;
        let reopened = Kernel::open(Arc::new(DocsStore::open(db.path()).unwrap()), "host").unwrap();
        let mut retry = request(cwd.path());
        let second = prepare_run(
            &reopened,
            &run.thread_id,
            &run,
            &mut retry,
            &harness,
            &caps,
            Default::default(),
        )
        .await;
        if ambiguous {
            assert!(first.unwrap_err().to_string().contains("Response lost"));
            assert!(
                second
                    .unwrap_err()
                    .to_string()
                    .contains("Native fork acceptance is uncertain")
            );
        } else {
            first.unwrap();
            second.unwrap();
            assert_eq!(input.resume, retry.resume);
            let transfer = &reopened.store.thread_transfers(&run.thread_id).unwrap()[0];
            assert_eq!(transfer["status"], "consumed");
            assert_eq!(
                transfer["resolution"]["strategy"],
                if cursor {
                    "native_fork"
                } else {
                    "portable_context"
                }
            );
            if cursor {
                assert_eq!(input.resume.as_deref(), Some("native-fork"));
                let p = reopened.store.thread(&run.thread_id).unwrap().unwrap();
                assert_eq!(
                    p.records["provider-thread"][0]["forkedFrom"]["providerThreadId"],
                    "provider:source"
                );
            }
        }
        assert_eq!(harness.calls.load(Ordering::SeqCst), usize::from(cursor));
    }
}

#[tokio::test]
async fn portable_fork_merge_and_failed_start_retry_preserve_prompt_and_consume_once() {
    let cwd = tempfile::tempdir().unwrap();
    let (db, kernel, source_run) = fixture(cwd.path());
    let item = json!({"id":"source-history","threadId":"source","runId":source_run.id,
        "providerThreadId":"provider:source","type":"assistant_message","text":"Source history only.",
        "status":"completed","ordinal":1});
    kernel.store.write(|tx| {
        tx.execute("INSERT INTO orchestration_projection_records(thread_id,kind,id,payload_json,last_sequence)
            VALUES('source','turn-item','source-history',?1,1)",[item.to_string()])?;Ok(())
    }).unwrap();
    kernel
        .transfer_command(
            &"source".into(),
            "fork:portable".into(),
            TransferOperation::Fork {
                target: "child".into(),
                source: SourcePoint::LatestStable,
                title: None,
            },
        )
        .await
        .unwrap();
    let child = start(&kernel, "child", "child-input").await;
    let mut input = request(cwd.path());
    prepare(&kernel, &child, &mut input).await.unwrap();
    assert!(input.prompt.contains("Source history only."));
    assert!(input.prompt.ends_with("Current request stays intact.\n🧪"));
    let source_items = kernel
        .store
        .thread(&"source".into())
        .unwrap()
        .unwrap()
        .records;
    let reopened = Kernel::open(Arc::new(DocsStore::open(db.path()).unwrap()), "host").unwrap();
    let mut replay = request(cwd.path());
    prepare(&reopened, &child, &mut replay).await.unwrap();
    assert_eq!(input.prompt, replay.prompt);
    assert_eq!(
        reopened
            .store
            .thread_transfers(&"child".into())
            .unwrap()
            .len(),
        1
    );
    observe(&reopened, &child, done(zeron_proto::DoneStatus::Errored)).await;
    let retry = start(&reopened, "child", "retry-input").await;
    let mut retry_input = request(cwd.path());
    prepare(&reopened, &retry, &mut retry_input).await.unwrap();
    assert!(retry_input.prompt.contains("Source history only."));
    accept(&reopened, &retry, cwd.path(), "native-child").await;
    let projection = reopened.store.thread(&"child".into()).unwrap().unwrap();
    assert_eq!(
        projection.records["context-handoff"][0]["delivery"]["status"],
        "inline"
    );
    assert_eq!(
        reopened.store.thread_transfers(&"child".into()).unwrap()[0]["targetRunId"],
        child.id.0
    );
    observe(&reopened, &retry, done(zeron_proto::DoneStatus::Completed)).await;
    let merge = reopened
        .transfer_command(
            &"child".into(),
            "merge:portable".into(),
            TransferOperation::MergeBack {
                target: "source".into(),
                source: SourcePoint::Run { run_id: retry.id },
            },
        )
        .await
        .unwrap();
    assert_eq!(merge.status, ReceiptStatus::Accepted, "{merge:?}");
    let parent_run = start(&reopened, "source", "merge-input").await;
    let mut merged = request(cwd.path());
    merged.resume = Some("native-parent".into());
    prepare(&reopened, &parent_run, &mut merged).await.unwrap();
    assert!(merged.prompt.contains("merge_back / fork_delta_summary"));
    assert!(
        !merged.prompt.contains("Source history only."),
        "merge delta must exclude inherited parent history"
    );
    assert!(merged.prompt.ends_with("Current request stays intact.\n🧪"));
    // Context imports never append messages to the source historical transcript.
    assert_eq!(
        source_items["turn-item"],
        reopened
            .store
            .thread(&"source".into())
            .unwrap()
            .unwrap()
            .records["turn-item"]
    );
    assert!(
        std::fs::read_dir(cwd.path()).unwrap().next().is_none(),
        "context merge must perform no Git/filesystem mutation"
    );
    let mut uncertain = request(cwd.path());
    uncertain.resume = Some("native-parent".into());
    assert!(
        prepare(&reopened, &parent_run, &mut uncertain)
            .await
            .unwrap_err()
            .to_string()
            .contains(UNCERTAIN_ERROR)
    );
    accept(&reopened, &parent_run, cwd.path(), "native-parent").await;
    let mut accepted = request(cwd.path());
    accepted.resume = Some("native-parent".into());
    prepare(&reopened, &parent_run, &mut accepted)
        .await
        .unwrap();
    assert_eq!(
        accepted.prompt, "Current request stays intact.\n🧪",
        "accepted inline context cannot be delivered twice"
    );
}

#[tokio::test]
async fn provider_switch_and_telemetry_use_only_accepted_root_native_identity() {
    let cwd = tempfile::tempdir().unwrap();
    let (_db, kernel, _run) = fixture(cwd.path());
    let first = start(&kernel, "source", "occupancy-input").await;
    accept(&kernel, &first, cwd.path(), "native-a").await;
    // Unknown window/threshold must be omitted, not invalid null PositiveInts.
    observe(
        &kernel,
        &first,
        zeron_proto::AgentEvent::ContextUsageSnapshot {
            usage: zeron_proto::ContextUsage {
                tokens: Some(23_000),
                window: None,
                compact_at: None,
                session: None,
            },
        },
    )
    .await;
    let mut projection = kernel.store.thread(&"source".into()).unwrap().unwrap();
    let provider = projection.records["provider-thread"]
        .iter()
        .find(|p| p["id"] == json!(first.provider_thread_id))
        .unwrap()
        .clone();
    assert_eq!(
        latest_native_context_usage(&projection, &provider)
            .unwrap()
            .0
            .used_tokens,
        23_000
    );
    let mut stale = provider.clone();
    stale["nativeThreadRef"]["nativeId"] = json!("native-b");
    assert!(latest_native_context_usage(&projection, &stale).is_none());
    let turn = projection
        .records
        .get_mut("provider-turn")
        .unwrap()
        .first_mut()
        .unwrap();
    turn["nodeId"] = json!("nested-not-root");
    assert!(latest_native_context_usage(&projection, &provider).is_none());
    observe(&kernel, &first, done(zeron_proto::DoneStatus::Completed)).await;
    let mut thread = json!(
        kernel
            .store
            .thread(&"source".into())
            .unwrap()
            .unwrap()
            .thread
    );
    thread["modelSelection"]["instanceId"] = json!("another-instance");
    thread["providerInstanceId"] = json!("another-instance");
    kernel
        .store
        .write(|tx| {
            tx.execute(
                "UPDATE orchestration_projection_threads SET payload_json=?1 WHERE id='source'",
                [thread.to_string()],
            )?;
            Ok(())
        })
        .unwrap();
    let switched = start(&kernel, "source", "switch-input").await;
    let mut input = request(cwd.path());
    input.resume = Some("native-a".into());
    prepare(&kernel, &switched, &mut input).await.unwrap();
    assert!(
        input.resume.is_none(),
        "native IDs cannot cross provider instances"
    );
    assert!(input.prompt.contains("full_thread_summary"));
    let transfer = kernel
        .store
        .thread_transfers(&"source".into())
        .unwrap()
        .into_iter()
        .find(|t| t["type"] == "provider_handoff")
        .unwrap();
    assert_eq!(transfer["status"], "consumed");
    assert_eq!(transfer["targetProviderInstanceId"], "another-instance");
    let run = kernel
        .store
        .thread(&"source".into())
        .unwrap()
        .unwrap()
        .runs
        .last()
        .unwrap()
        .clone();
    assert!(run.context_handoff_id.is_some());
}

async fn select_instance(kernel: &Kernel, instance: &str, model: Option<&str>) {
    let mut thread = kernel
        .store
        .thread(&"source".into())
        .unwrap()
        .unwrap()
        .thread;
    thread.provider_instance_id = instance.into();
    thread.model_selection.instance_id = instance.into();
    if let Some(model) = model {
        thread.model_selection.model = model.into();
    }
    let command = Command {
        id: format!("select:{}", uuid::Uuid::new_v4()).into(),
        thread_id: thread.id.clone(),
        operation: Operation::SessionBinding(Box::new(thread)),
    };
    assert_eq!(
        kernel
            .dispatch(&command, crate::now_ms())
            .await
            .unwrap()
            .status,
        ReceiptStatus::Accepted
    );
}

#[tokio::test]
async fn returning_provider_resumes_its_own_native_history_and_only_receives_the_missed_delta() {
    for restart in [false, true] {
        let cwd = tempfile::tempdir().unwrap();
        let (db, kernel, _) = fixture(cwd.path());
        let original = kernel
            .store
            .thread(&"source".into())
            .unwrap()
            .unwrap()
            .thread
            .model_selection;
        let first = start(&kernel, "source", "a-input").await;
        accept(&kernel, &first, cwd.path(), "native-a").await;
        observe(
            &kernel,
            &first,
            zeron_proto::AgentEvent::TextDelta {
                text: "A already knows this decision.".into(),
            },
        )
        .await;
        observe(&kernel, &first, done(zeron_proto::DoneStatus::Completed)).await;
        select_instance(&kernel, "provider-b", None).await;
        let second = start(&kernel, "source", "b-input").await;
        assert_ne!(first.provider_thread_id, second.provider_thread_id);
        let mut input = request(cwd.path());
        input.resume = Some("native-a".into());
        prepare(&kernel, &second, &mut input).await.unwrap();
        assert_eq!(
            input.resume, None,
            "a different instance cannot receive A's native ID"
        );
        assert!(input.prompt.contains("full_thread_summary"));
        assert!(input.prompt.contains("A already knows this decision."));
        accept(&kernel, &second, cwd.path(), "native-b").await;
        observe(
            &kernel,
            &second,
            zeron_proto::AgentEvent::TextDelta {
                text: "B changed the implementation and verified it.".into(),
            },
        )
        .await;
        observe(&kernel, &second, done(zeron_proto::DoneStatus::Completed)).await;
        let kernel = if restart {
            let reopened =
                Kernel::open(Arc::new(DocsStore::open(db.path()).unwrap()), "host").unwrap();
            reopened.recover(crate::now_ms()).await.unwrap();
            reopened
        } else {
            kernel
        };
        select_instance(&kernel, &original.instance_id.0, None).await;
        let returned = start(&kernel, "source", "return-to-a").await;
        assert_eq!(returned.provider_thread_id, first.provider_thread_id);
        let mut input = request(cwd.path());
        input.resume = Some("native-b".into());
        prepare(&kernel, &returned, &mut input).await.unwrap();
        assert_eq!(
            input.resume.as_deref(),
            Some("native-a"),
            "restart={restart}"
        );
        assert!(input.prompt.contains("delta_since_target_last_seen"));
        assert!(
            input
                .prompt
                .contains("B changed the implementation and verified it.")
        );
        assert!(!input.prompt.contains("A already knows this decision."));
        let p = kernel.store.thread(&returned.thread_id).unwrap().unwrap();
        let handoff = p.records["context-handoff"]
            .iter()
            .find(|h| h["targetRunId"] == returned.id.0)
            .unwrap();
        assert_eq!(
            handoff["coveredRunOrdinals"],
            json!({"from":second.ordinal,"to":second.ordinal})
        );
        let provider_a = provider_for_run(&p, &first).unwrap();
        let provider_b = provider_for_run(&p, &second).unwrap();
        assert_eq!(provider_a["nativeThreadRef"]["nativeId"], "native-a");
        assert_eq!(provider_b["nativeThreadRef"]["nativeId"], "native-b");
        assert!(
            provider_a["handoffIds"]
                .as_array()
                .unwrap()
                .contains(&handoff["id"])
        );
        accept(&kernel, &returned, cwd.path(), "native-a").await;
        let mut accepted = request(cwd.path());
        prepare(&kernel, &returned, &mut accepted).await.unwrap();
        assert_eq!(
            accepted.prompt,
            request(cwd.path()).prompt,
            "accepted context cannot be delivered twice"
        );
        assert_eq!(
            accepted.resume.as_deref(),
            Some("native-a"),
            "preparing an already accepted run retains its native identity"
        );
    }
}

#[tokio::test]
async fn changed_model_options_or_checkout_uses_full_context_and_never_a_foreign_native_delta() {
    for changed in ["model", "options", "checkout"] {
        let cwd = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let (_db, kernel, _) = fixture(cwd.path());
        let first = start(&kernel, "source", "initial-input").await;
        accept(&kernel, &first, cwd.path(), "native-original").await;
        observe(
            &kernel,
            &first,
            zeron_proto::AgentEvent::TextDelta {
                text: "The original complete decision.".into(),
            },
        )
        .await;
        observe(&kernel, &first, done(zeron_proto::DoneStatus::Completed)).await;
        if changed == "model" {
            select_instance(
                &kernel,
                &first.provider_instance_id.0,
                Some("different-model"),
            )
            .await;
        } else if changed == "options" {
            let mut thread = kernel
                .store
                .thread(&"source".into())
                .unwrap()
                .unwrap()
                .thread;
            let mut selection = json!(thread.model_selection);
            selection["options"] = json!([{"id":"reasoningEffort","value":"high"}]);
            thread.model_selection = serde_json::from_value(selection).unwrap();
            kernel
                .dispatch(
                    &Command {
                        id: "changed-options".into(),
                        thread_id: thread.id.clone(),
                        operation: Operation::SessionBinding(Box::new(thread)),
                    },
                    crate::now_ms(),
                )
                .await
                .unwrap();
        }
        let next = start(&kernel, "source", "changed-input").await;
        let mut input = request(if changed == "checkout" {
            other.path()
        } else {
            cwd.path()
        });
        input.resume = Some("unrelated-native".into());
        prepare(&kernel, &next, &mut input).await.unwrap();
        assert!(input.resume.is_none());
        assert!(input.prompt.contains("full_thread_summary"));
        assert!(input.prompt.contains("The original complete decision."));
        assert!(!input.prompt.contains("delta_since_target_last_seen"));
    }
}

#[tokio::test]
async fn same_provider_followup_preserves_its_native_handle_and_replaces_a_stale_caller_resume() {
    let cwd = tempfile::tempdir().unwrap();
    let (_db, kernel, _) = fixture(cwd.path());
    let first = start(&kernel, "source", "same-provider-first").await;
    accept(&kernel, &first, cwd.path(), "native-owned").await;
    observe(&kernel, &first, done(zeron_proto::DoneStatus::Completed)).await;
    let next = start(&kernel, "source", "same-provider-next").await;
    assert_eq!(first.provider_thread_id, next.provider_thread_id);
    let mut input = request(cwd.path());
    input.resume = Some("native-from-another-conversation".into());
    prepare(&kernel, &next, &mut input).await.unwrap();
    assert_eq!(input.resume.as_deref(), Some("native-owned"));
    assert_eq!(input.prompt, request(cwd.path()).prompt);
    assert!(
        kernel
            .store
            .thread_transfers(&"source".into())
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn imported_runless_history_survives_portable_handoffs_without_duplicate_current_input() {
    let cwd = tempfile::tempdir().unwrap();
    let (_db, kernel, _) = fixture(cwd.path());
    let mut thread = kernel
        .store
        .thread(&"source".into())
        .unwrap()
        .unwrap()
        .thread;
    thread.history_origin = Optional::Present(OrchestrationV2ThreadHistoryOrigin::V1Import);
    let command = Command {
        id: "import-marker".into(),
        thread_id: thread.id.clone(),
        operation: Operation::SessionBinding(Box::new(thread)),
    };
    kernel.dispatch(&command, crate::now_ms()).await.unwrap();
    let imported = json!({"id":"legacy-item","threadId":"source","runId":null,
        "messageId":"legacy-message","type":"assistant_message","text":"Legacy goal and constraints.",
        "status":"completed","ordinal":0});
    kernel.store.write(|tx| {
        tx.execute("INSERT INTO orchestration_projection_records(thread_id,kind,id,payload_json,last_sequence)
            VALUES('source','turn-item','legacy-item',?1,1)",[imported.to_string()])?;
        Ok(())
    }).unwrap();
    select_instance(&kernel, "new-provider", None).await;
    let run = start(&kernel, "source", "new-input").await;
    let mut input = request(cwd.path());
    prepare(&kernel, &run, &mut input).await.unwrap();
    assert!(input.prompt.contains("Legacy goal and constraints."));
    assert_eq!(
        input
            .prompt
            .matches("Current request stays intact.")
            .count(),
        1
    );
}

#[tokio::test]
async fn adopted_user_item_keeps_identity_and_unaccepted_root_retry_history() {
    for accepted in [false, true] {
        let cwd = tempfile::tempdir().unwrap();
        let (_db, kernel, _) = fixture(cwd.path());
        let mut thread = kernel
            .store
            .thread(&"source".into())
            .unwrap()
            .unwrap()
            .thread;
        thread.history_origin = Optional::Present(OrchestrationV2ThreadHistoryOrigin::V1Import);
        kernel
            .dispatch(
                &Command {
                    id: "adopted-history-marker".into(),
                    thread_id: thread.id.clone(),
                    operation: Operation::SessionBinding(Box::new(thread)),
                },
                crate::now_ms(),
            )
            .await
            .unwrap();
        let text = "Adopted user input must not silently disappear from retry history.";
        let run = start_text(&kernel, "source", "adopted-input", text).await;
        let now = crate::now_ms();
        let time = crate::orchestration::event::iso(now).unwrap();
        let item = json!({"id":"adopted-item","threadId":"source","runId":null,
            "nodeId":null,"providerThreadId":null,"providerTurnId":null,
            "nativeItemRef":null,"parentItemId":null,"type":"user_message",
            "status":"completed","ordinal":0,"title":null,
            "startedAt":time,"completedAt":time,"updatedAt":time,
            "createdBy":"user","creationSource":"server","messageId":"adopted-input",
            "inputIntent":"turn_start","text":text,"attachments":[]});
        kernel
            .store
            .write(|tx| {
                kernel.store.append_event(
                    tx,
                    None,
                    crate::orchestration::event::make(
                        "adopted-item-event".into(),
                        &run.thread_id,
                        "turn-item.updated",
                        &item,
                        now,
                    )?,
                )?;
                Ok(())
            })
            .unwrap();
        observe(&kernel, &run, session_started(cwd.path(), "adopted-native")).await;
        let p = kernel.store.thread(&run.thread_id).unwrap().unwrap();
        let history = local_items(&p, run.ordinal);
        let attributed: Vec<_> = history
            .iter()
            .filter(|i| i["messageId"] == "adopted-input")
            .collect();
        assert_eq!(attributed.len(), 1);
        assert_eq!(attributed[0]["id"], "adopted-item");
        assert_eq!(attributed[0]["runId"], run.id.0);
        assert!(attributed[0]["providerTurnId"].is_null());
        assert!(
            p.records["provider-turn"].is_empty(),
            "history attribution must not manufacture provider acceptance"
        );
        if accepted {
            observe(&kernel, &run, zeron_proto::AgentEvent::InputAccepted).await;
            let p = kernel.store.thread(&run.thread_id).unwrap().unwrap();
            let bound = p.records["turn-item"]
                .iter()
                .find(|i| i["id"] == "adopted-item")
                .unwrap();
            assert_eq!(bound["runId"], run.id.0);
            assert_eq!(bound["nodeId"], run.root_node_id.as_ref().unwrap().0);
            assert_eq!(
                bound["providerThreadId"],
                run.provider_thread_id.as_ref().unwrap().0
            );
            assert!(bound["providerTurnId"].is_string());
            assert_eq!(bound["ordinal"], 0);
        }
        observe(&kernel, &run, done(zeron_proto::DoneStatus::Interrupted)).await;
        let next = start(&kernel, "source", "after-adopted-input").await;
        let mut input = request(cwd.path());
        prepare(&kernel, &next, &mut input).await.unwrap();
        assert_eq!(input.prompt.contains(text), !accepted);
        if !accepted {
            assert_eq!(input.prompt.matches(text).count(), 1);
            assert!(input.prompt.contains("item=adopted-item"));
        }
    }
}

#[tokio::test]
async fn mcp_project_scope_refusal_order_and_passive_transfer_shapes_match_t3() {
    use crate::orchestration::transfer_service::TransferService;
    let cwd = tempfile::tempdir().unwrap();
    let (_db, kernel, run) = fixture(cwd.path());
    let thread = kernel
        .store
        .thread(&"source".into())
        .unwrap()
        .unwrap()
        .thread;
    let caller = crate::orchestration::service::CallerScope {
        thread_id: thread.id.clone(),
        run_id: run.id.clone(),
        session_id: "credential".into(),
        project_id: thread.project_id.clone(),
        workspace_root: cwd.path().into(),
        runtime_mode: thread.runtime_mode,
        interaction_mode: thread.interaction_mode,
        provider_instance_id: thread.provider_instance_id.clone(),
    };
    let service = super::mcp::EngineTransferService {
        kernel: kernel.clone(),
    };
    let error = service
        .fork(
            caller.clone(),
            serde_json::from_value(json!({
                "sourcePoint":{"type":"run","runId":run.id}
            }))
            .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        json!(error.into_failure()),
        json!({"_tag":"OrchestratorMcpFailure",
        "code":"parent_not_active","message":"The calling provider no longer owns an active thread run."})
    );
    // Target project lookup precedes the inactive-caller refusal.
    let error = service
        .merge_back(
            caller.clone(),
            serde_json::from_value(json!({
                "targetThreadId":"outside-project","sourcePoint":{"type":"run","runId":run.id}
            }))
            .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(json!(error.into_failure())["code"], "thread_not_found");
    assert!(
        service
            .transfers(caller.clone(), serde_json::from_value(json!({})).unwrap())
            .await
            .unwrap()
            .transfers
            .is_empty()
    );
    let _active = start(&kernel, "source", "mcp-active").await;
    let result = service
        .fork(
            caller.clone(),
            serde_json::from_value(json!({
                "sourcePoint":{"type":"run","runId":run.id},"title":"MCP fork"
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    assert!(result.target_thread_id.0.starts_with("mcp:"));
    assert!(result.target_thread_id.0.ends_with(":fork"));
    let result = json!(
        service
            .transfers(caller.clone(), serde_json::from_value(json!({})).unwrap())
            .await
            .unwrap()
    );
    let row = result["transfers"][0].as_object().unwrap();
    assert_eq!(
        row.keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        ["id", "sourceThreadId", "targetThreadId", "status"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
    let error = service
        .fork(
            caller,
            serde_json::from_value(json!({
                "sourcePoint":{"type":"run","runId":"nonexistent"}
            }))
            .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        json!(error.into_failure()),
        json!({"_tag":"OrchestratorMcpFailure",
        "code":"orchestration_error","message":"The operation could not be completed."})
    );
}

#[tokio::test]
async fn fresh_native_acceptance_receipt_records_budgeted_selection_not_original_history() {
    let cwd = tempfile::tempdir().unwrap();
    let (_db, kernel, source) = fixture(cwd.path());
    kernel.store.write(|tx| {
        for (id,text) in [("small","Keep this.".into()),("huge","x".repeat(8_000))] {
            let item = json!({"id":id,"threadId":"source","runId":source.id,"providerThreadId":"provider:source",
                "type":"assistant_message","text":text,"status":"completed","ordinal":1});
            tx.execute("INSERT INTO orchestration_projection_records(thread_id,kind,id,payload_json,last_sequence)
                VALUES('source','turn-item',?1,?2,1)",rusqlite::params![id,item.to_string()])?;
        } Ok(())
    }).unwrap();
    kernel
        .transfer_command(
            &"source".into(),
            "fork:budgeted".into(),
            TransferOperation::Fork {
                target: "budgeted".into(),
                source: SourcePoint::LatestStable,
                title: None,
            },
        )
        .await
        .unwrap();
    let run = start(&kernel, "budgeted", "budgeted-input").await;
    let mut input = request(cwd.path());
    input.attachments = vec!["/attachment.png".into(); 11];
    prepare(&kernel, &run, &mut input).await.unwrap();
    assert!(input.prompt.contains("Keep this."));
    assert!(!input.prompt.contains(&"x".repeat(8_000)));
    accept(&kernel, &run, cwd.path(), "fresh-native").await;
    let p = kernel.store.thread(&run.thread_id).unwrap().unwrap();
    let h = &p.records["context-handoff"][0];
    assert_eq!(h["history"]["messages"].as_array().unwrap().len(), 2);
    assert_eq!(h["delivery"]["itemIds"], json!(["small"]));
    assert_eq!(h["delivery"]["omittedItemIds"], json!(["huge"]));
}

#[test]
fn oracle_short_history_verbatim_and_oversized_multilingual_whole() {
    let messages = messages();
    let selected = select_history(&messages, "History", 0, 16_000);
    assert_eq!(selected.messages, messages);
    assert_eq!(selected.omitted_items, 0);
    let candidates = vec![
        messages[0].clone(),
        message("huge", "assistant", &"界🧪".repeat(20_000)),
        message("old", "assistant", &"a".repeat(4_000)),
        messages[1].clone(),
    ];
    let selected = select_history(&candidates, "thread:handoff runs 1-4", 0, 3_000);
    assert_eq!(selected.messages, messages);
    assert_eq!(selected.omitted_items, 2);
    assert!(history_cost(&selected.messages, &selected.context) <= 3_000);
}
#[test]
fn oracle_json_utf8_escaping_and_digit_boundaries() {
    let candidates: Vec<_> = (0..500)
        .map(|i| {
            message(
                &format!("item:{i}"),
                if i % 2 == 0 { "user" } else { "assistant" },
                &"\0\\\"🧪界".repeat(15),
            )
        })
        .collect();
    for budget in [1_024, 4_000, 16_000] {
        let selected = select_history(&candidates, "Retrieve omitted history", 0, budget);
        assert!(history_cost(&selected.messages, &selected.context) <= budget);
        assert!(selected.omitted_items > 0);
    }
    let candidates: Vec<_> = (0..20)
        .map(|i| message(&format!("boundary:{i}"), "user", "Short request"))
        .collect();
    for budget in 4_000..=9_000 {
        let selected = select_history(&candidates, "Recover history", 90, budget);
        assert!(history_cost(&selected.messages, &selected.context) <= budget);
    }
}
#[test]
fn oracle_occupancy_input_attachment_budget() {
    let base = handoff_budget(16_000, "Continue", &[], None, 0, Some(32_000));
    assert_eq!(base, 15_990);
    assert!(handoff_budget(16_000, "Continue", &[], None, 8_000, Some(32_000)) < base);
    assert_eq!(
        handoff_budget(16_000, &"界".repeat(30_000), &[], None, 0, Some(32_000)),
        0
    );
    let usage = ContextUsage {
        used_tokens: 23_000,
        max_tokens: Some(24_000),
        auto_compact_threshold: None,
    };
    assert_eq!(
        handoff_budget(16_000, "Continue", &[], Some(&usage), 0, Some(32_000)),
        0
    );
    for size in [100_000, 10 * 1024 * 1024] {
        let image = json!({"type":"image","sizeBytes":size});
        let budget = handoff_budget(16_000, "Continue", &[image.clone()], None, 0, Some(32_000));
        assert!(budget > 4_000 && budget < base);
        assert_eq!(
            handoff_budget(
                16_000,
                "Continue",
                &[image.clone(), image.clone()],
                None,
                0,
                Some(32_000)
            ),
            0
        );
        assert_eq!(
            handoff_budget(
                64_000,
                &"x".repeat(70_000),
                &[image],
                None,
                0,
                Some(1_000_000)
            ),
            64_000
        );
    }
    assert_eq!(
        handoff_budget(2_000, "Continue", &[], None, 0, Some(32_000)),
        2_000
    );
    let previous = ContextUsage {
        used_tokens: 37_321,
        max_tokens: Some(258_400),
        auto_compact_threshold: Some(32_000),
    };
    assert_eq!(
        context_usage_for_handoff(false, false, false, Some(&previous), None),
        None
    );
    assert_eq!(
        context_usage_for_handoff(true, true, true, Some(&previous), None),
        Some(previous.clone())
    );
    assert_eq!(
        context_usage_for_handoff(true, false, false, Some(&previous), Some(1_000_000)),
        Some(ContextUsage {
            used_tokens: 37_321,
            max_tokens: Some(1_000_000),
            auto_compact_threshold: None
        })
    );
    for count in 1..=16 {
        let images = vec![json!({"type":"image","sizeBytes":100_000}); count];
        let budget = handoff_budget(16_000, "Compare these screenshots", &images, None, 0, None);
        if count <= 8 {
            assert_eq!(budget, 16_000);
        }
        if count == 10 {
            assert!(budget > 0 && budget < 16_000);
        }
        if count == 16 {
            assert_eq!(budget, 0);
        }
        assert_eq!(
            handoff_budget(
                16_000,
                "Compare these screenshots",
                &images,
                None,
                0,
                Some(20_000)
            ),
            0
        );
    }
}
#[test]
fn oracle_command_outcomes_and_private_reasoning() {
    let command = historical_message(&json!({"id":"cmd","type":"command_execution","input":"vp test","output":"Failure near end","exitCode":1,
        "threadId":"t","runId":"r","providerThreadId":null,"status":"failed"})).unwrap();
    assert_eq!(command["role"], "assistant");
    assert!(command["text"].as_str().unwrap().contains("Exit code: 1"));
    assert!(historical_message(&json!({"type":"reasoning","text":"private"})).is_none());
}

#[tokio::test]
async fn stable_status_matrix_lineage_and_crash_atomicity() {
    let cwd = tempfile::tempdir().unwrap();
    let (_dir, kernel, run) = fixture(cwd.path());
    for status in [
        "preparing",
        "queued",
        "starting",
        "running",
        "rolled_back",
        "completed",
        "waiting",
        "failed",
        "interrupted",
        "cancelled",
    ] {
        let status_value: OrchestrationV2RunStatus = serde_json::from_value(json!(status)).unwrap();
        assert_eq!(
            forkable(&status_value),
            matches!(
                status,
                "completed" | "waiting" | "failed" | "interrupted" | "cancelled"
            )
        );
        kernel.store.write(|tx| {
            let mut value = json!(run); value["status"] = json!(status);
            tx.execute("UPDATE orchestration_projection_runs SET payload_json=?1 WHERE id='run:source'",[value.to_string()])?; Ok(())
        }).unwrap();
        let receipt = kernel
            .transfer_command(
                &"source".into(),
                CommandId(format!("fork:{status}")),
                TransferOperation::Fork {
                    target: format!("child:{status}").into(),
                    source: SourcePoint::Run {
                        run_id: run.id.clone(),
                    },
                    title: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(
            receipt.status == ReceiptStatus::Accepted,
            forkable(&status_value),
            "{status}: {receipt:?}"
        );
        if !forkable(&status_value) {
            assert!(
                receipt
                    .error
                    .unwrap()
                    .contains("in-progress and rolled-back runs cannot be forked")
            );
        }
    }
    let child = kernel
        .store
        .thread(&"child:completed".into())
        .unwrap()
        .unwrap();
    assert_eq!(child.thread.title, "Source fork");
    assert_eq!(child.thread.lineage.parent_thread_id, Some("source".into()));
    assert_eq!(child.thread.lineage.root_thread_id, "source".into());
    assert!(child.runs.is_empty());
    assert!(kernel.store.thread_transfers(&child.thread.id).unwrap()[0]["basePoint"].is_null());
    kernel.store.inject_failure(WriteBoundary::BeforeCommit, 1);
    assert!(
        kernel
            .transfer_command(
                &"source".into(),
                "crash:fork".into(),
                TransferOperation::Fork {
                    target: "never-created".into(),
                    source: SourcePoint::Run {
                        run_id: run.id.clone()
                    },
                    title: None
                }
            )
            .await
            .is_err()
    );
    assert!(
        kernel
            .store
            .thread(&"never-created".into())
            .unwrap()
            .is_none()
    );
}

#[test]
fn latest_stable_requires_completed_checkpoint_and_native_matrix() {
    let cwd = tempfile::tempdir().unwrap();
    let (_dir, kernel, run) = fixture(cwd.path());
    let mut projection = kernel.store.thread(&"source".into()).unwrap().unwrap();
    kernel
        .store
        .read(|conn| {
            assert_eq!(
                source_run(conn, &projection, &SourcePoint::LatestStable)?
                    .unwrap()
                    .id,
                run.id
            );
            projection.runs[0].checkpoint_id = None;
            assert!(source_run(conn, &projection, &SourcePoint::LatestStable)?.is_none());
            projection.runs[0].checkpoint_id = Some("cp".into());
            projection.runs[0].status = OrchestrationV2RunStatus::Waiting;
            assert!(source_run(conn, &projection, &SourcePoint::LatestStable)?.is_none());
            Ok(())
        })
        .unwrap();
    let mut caps =
        crate::orchestration::assembly::capabilities(&zeron_harness::mock::MockHarness {
            script: vec![],
        });
    caps.threads.can_fork_thread = true;
    caps.threads.can_fork_from_turn = true;
    caps.identity.native_thread_ids = OrchestrationV2NativeRefStrength::Strong;
    for strength in ["strong", "weak", "none"] {
        for status in ["completed", "waiting", "failed", "interrupted", "cancelled"] {
            let mut run = run.clone();
            run.status = serde_json::from_value(json!(status)).unwrap();
            let transfer = json!({"sourceProviderInstanceId":run.provider_instance_id,"sourcePoint":{"providerThreadRef":{"strength":strength}}});
            assert_eq!(
                native_fork_eligible(&transfer, &run, &run.provider_instance_id, &caps),
                strength == "strong" && matches!(status, "completed" | "waiting")
            );
            assert!(!native_fork_eligible(
                &transfer,
                &run,
                &"another-provider".into(),
                &caps
            ));
        }
    }
}

struct RecordingDelivery {
    native: bool,
    handoffs: Mutex<Vec<Value>>,
    calls: Mutex<usize>,
}
#[async_trait::async_trait]
impl HandoffDelivery for RecordingDelivery {
    async fn inject(&self, _: &str, _: &[Value], _: &str) -> Result<bool> {
        *self.calls.lock().unwrap() += 1;
        Ok(self.native)
    }
    async fn persist(&self, h: &Value) -> Result<()> {
        self.handoffs.lock().unwrap().push(h.clone());
        Ok(())
    }
}
fn handoff() -> Value {
    json!({"id":"h","strategy":"full_thread_summary","threadId":"t","summaryText":"",
        "coveredRunOrdinals":{"from":1,"to":2},"history":{"messages":messages(),"coverage":"History","omittedItems":0}})
}
#[tokio::test]
async fn oracle_native_inline_receipts_retry_and_uncertain_acceptance() {
    for native in [true, false] {
        let delivery = RecordingDelivery {
            native,
            handoffs: Mutex::new(vec![]),
            calls: Mutex::new(0),
        };
        let prepared = deliver_handoffs(
            &[handoff()],
            Some("native"),
            &"t".into(),
            16_000,
            &Default::default(),
            true,
            false,
            &delivery,
        )
        .await
        .unwrap();
        let rows = delivery.handoffs.lock().unwrap().clone();
        assert_eq!(rows[0]["delivery"]["status"], "pending");
        if native {
            assert!(prepared.context.is_empty());
            assert_eq!(rows[1]["delivery"]["status"], "injected");
            deliver_handoffs(
                &[rows[1].clone()],
                Some("native"),
                &"t".into(),
                16_000,
                &Default::default(),
                true,
                false,
                &delivery,
            )
            .await
            .unwrap();
            assert_eq!(*delivery.calls.lock().unwrap(), 1);
        } else {
            assert!(!prepared.context.is_empty());
            prepared.accepted(&delivery).await.unwrap();
            assert_eq!(
                delivery.handoffs.lock().unwrap().last().unwrap()["delivery"]["status"],
                "inline"
            );
        }
        let uncertain = deliver_handoffs(
            &[rows[0].clone()],
            Some("native"),
            &"t".into(),
            16_000,
            &Default::default(),
            true,
            false,
            &delivery,
        )
        .await
        .err()
        .unwrap();
        assert!(uncertain.to_string().contains(UNCERTAIN_ERROR));
    }
}
#[tokio::test]
async fn oracle_omissions_deferred_inline_and_accumulated_coverage() {
    let delivery = RecordingDelivery {
        native: false,
        handoffs: Mutex::new(vec![]),
        calls: Mutex::new(0),
    };
    let mut h = handoff();
    h["history"]["messages"]
        .as_array_mut()
        .unwrap()
        .push(message("huge", "user", &"x".repeat(20_000)));
    h["history"]["omittedItems"] = json!(1);
    h["history"]["omittedItemIds"] = json!(["earlier"]);
    let prepared = deliver_handoffs(
        &[h.clone()],
        Some("native"),
        &"t".into(),
        16_000,
        &Default::default(),
        true,
        false,
        &delivery,
    )
    .await
    .unwrap();
    prepared.accepted(&delivery).await.unwrap();
    assert_eq!(
        delivery.handoffs.lock().unwrap().last().unwrap()["delivery"]["omittedItemIds"],
        json!(["earlier", "huge"])
    );
    let deferred = deliver_handoffs(
        &[h],
        Some("native"),
        &"t".into(),
        16_000,
        &Default::default(),
        true,
        true,
        &delivery,
    )
    .await
    .unwrap();
    assert!(deferred.context.is_empty());
    assert!(
        delivery
            .handoffs
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .get("delivery")
            .is_none()
    );
    let many = vec![handoff(); 100];
    let bounded = deliver_handoffs(
        &many,
        Some("native"),
        &"t".into(),
        2_500,
        &Default::default(),
        true,
        false,
        &delivery,
    )
    .await
    .unwrap();
    assert!(
        bounded
            .context
            .contains("detailed coverage references omitted")
    );
}
#[test]
fn queue_and_multiple_merge_refusals_are_exact() {
    let pending = json!({"targetThreadId":"t","sourceThreadId":"fork:one","type":"merge_back","status":"pending"});
    let error = ensure_start_allowed(&[pending.clone()], &"t".into(), true).unwrap_err();
    assert_eq!(
        error.to_string(),
        "orchestration invariant: Thread t has a pending merge-back transfer; queued merge-back consumption is not implemented yet."
    );
    let mut other = pending.clone();
    other["sourceThreadId"] = json!("fork:two");
    assert_eq!(
        ensure_start_allowed(&[pending, other], &"t".into(), false)
            .unwrap_err()
            .to_string(),
        "orchestration invariant: Thread t has pending merge-back transfers from multiple forks."
    );
}

#[tokio::test]
async fn merge_parent_lineage_supersession_reopen_consumption_and_wire_privacy() {
    let cwd = tempfile::tempdir().unwrap();
    let (db, kernel, run) = fixture(cwd.path());
    let fork = kernel
        .transfer_command(
            &"source".into(),
            "fork:test".into(),
            TransferOperation::Fork {
                target: "fork".into(),
                source: SourcePoint::Run {
                    run_id: run.id.clone(),
                },
                title: Some("Research".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(fork.status, ReceiptStatus::Accepted);
    kernel.store.write(|tx| {
        let mut child = json!(run); child["id"] = json!("run:fork"); child["threadId"] = json!("fork");
        tx.execute("INSERT INTO orchestration_projection_runs(id,thread_id,ordinal,status,provider_instance_id,payload_json,last_sequence)
            VALUES('run:fork','fork',1,'completed',?1,?2,1)",rusqlite::params![child["providerInstanceId"].as_str(),child.to_string()])?;
        Ok(())
    }).unwrap();
    let wrong = kernel
        .transfer_command(
            &"source".into(),
            "merge:wrong".into(),
            TransferOperation::MergeBack {
                target: "fork".into(),
                source: SourcePoint::Run {
                    run_id: run.id.clone(),
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(
        wrong.error.as_deref(),
        Some("Thread source is not a fork of fork.")
    );
    for command in ["merge:one", "merge:two"] {
        assert_eq!(
            kernel
                .transfer_command(
                    &"fork".into(),
                    command.into(),
                    TransferOperation::MergeBack {
                        target: "source".into(),
                        source: SourcePoint::Run {
                            run_id: "run:fork".into()
                        }
                    }
                )
                .await
                .unwrap()
                .status,
            ReceiptStatus::Accepted
        );
    }
    let rows = kernel.store.thread_transfers(&"source".into()).unwrap();
    let merges: Vec<_> = rows.iter().filter(|t| t["type"] == "merge_back").collect();
    assert_eq!(merges[0]["status"], "superseded");
    assert_eq!(merges[1]["status"], "pending");
    assert_eq!(merges[1]["basePoint"]["runId"], "run:source");
    let mut transfer = merges[1].clone();
    transfer["status"] = json!("consumed");
    transfer["targetRunId"] = json!("target-run");
    transfer["consumedAt"] = json!("2026-10-04T00:00:00Z");
    let handoff = json!({"id":"handoff:merge","transferId":transfer["id"],"threadId":"source","targetRunId":"target-run",
        "fromProviderThreadIds":[],"toProviderThreadId":"provider:source","coveredRunOrdinals":{"from":1,"to":1},
        "strategy":"fork_delta_summary","status":"ready","summaryMessageId":null,"summaryText":"private",
        "history":{"messages":messages(),"coverage":"Recover source","omittedItems":0,"omittedItemIds":[]},
        "delivery":{"nativeThreadId":"native","status":"pending","itemIds":[]},
        "createdByProviderInstanceId":null,"createdAt":"2026-10-04T00:00:00Z","updatedAt":"2026-10-04T00:00:00Z"});
    assert_eq!(
        kernel
            .transfer_command(
                &"source".into(),
                "consume".into(),
                TransferOperation::Update {
                    transfer: Box::new(transfer.clone()),
                    handoff: Some(Box::new(handoff.clone()))
                }
            )
            .await
            .unwrap()
            .status,
        ReceiptStatus::Accepted
    );
    let reopened = Kernel::open(Arc::new(DocsStore::open(db.path()).unwrap()), "host").unwrap();
    let mut conflicting = transfer.clone();
    conflicting["targetRunId"] = json!("another-run");
    assert_eq!(
        reopened
            .transfer_command(
                &"source".into(),
                "consume-again".into(),
                TransferOperation::Update {
                    transfer: Box::new(conflicting),
                    handoff: None
                }
            )
            .await
            .unwrap()
            .status,
        ReceiptStatus::Rejected
    );
    let read = reopened.store.transfer_ui_state(&"source".into()).unwrap();
    assert_eq!(read.handoffs[0]["summaryText"], "");
    assert!(read.handoffs[0].get("history").is_none());
    assert!(read.handoffs[0].get("delivery").is_none());
    let publication = reopened.store.read(|conn| {
        let value: String = conn.query_row("SELECT payload_json FROM orchestration_publication_batches ORDER BY rowid DESC LIMIT 1",[],|r| r.get(0))?;
        Ok(serde_json::from_str::<Value>(&value)?)
    }).unwrap();
    let h = &publication["documents"][0]["payload"]["records"]["context-handoff"][0];
    assert_eq!(h["summaryText"], "");
    assert!(h.get("history").is_none());
    assert!(h.get("delivery").is_none());
}
