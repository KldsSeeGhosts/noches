use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use zeron_proto::orchestration::*;
use zeron_sync::DocsStore;

use super::command::ExecutionSeed;
use super::effects::*;
use super::event::{encode_component, make, mcp_command_id};
use super::projection;
use super::sync_publish::*;
use super::*;

const NOW: i64 = 1_800_000_000_000;

struct Fixture {
    dir: tempfile::TempDir,
    kernel: Kernel,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let kernel = Kernel::open(Arc::new(DocsStore::open(dir.path()).unwrap()), "host").unwrap();
        Self { dir, kernel }
    }

    fn reopen(&mut self) {
        self.kernel =
            Kernel::open(Arc::new(DocsStore::open(self.dir.path()).unwrap()), "host").unwrap();
    }

    fn create(&self, thread: &str) {
        let receipt = self
            .kernel
            .store
            .dispatch(&create_thread(thread), NOW)
            .unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Accepted, "{receipt:?}");
    }
}

/// Contract samples supply required fields; these are pinned structural fixtures,
/// not represented as upstream execution traces.
fn sample(name: &str) -> Value {
    static CASES: OnceLock<BTreeMap<String, Vec<Value>>> = OnceLock::new();
    CASES.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../proto/tests/t3_oracle/fixtures/serde-cases.json"
        ))
        .unwrap()
    })[name][0]
        .clone()
}

fn create_thread(thread: &str) -> Command {
    let mut value = sample("OrchestrationV2Command");
    value["type"] = json!("thread.create");
    value["commandId"] = json!(format!("create:{thread}"));
    value["threadId"] = json!(thread);
    value["projectId"] = json!("project");
    value["title"] = json!("Kernel test");
    Command::wire(serde_json::from_value(value).unwrap()).unwrap()
}

fn seed(thread: &str, ordinal: i64, status: &str) -> Command {
    let run_id = format!("run:{thread}:{ordinal}");
    let attempt_id = format!("attempt:{run_id}:1");
    let node_id = format!("node:{run_id}:1");
    let provider_id = format!("provider:{run_id}");
    let mut run = sample("OrchestrationV2Run");
    run["id"] = json!(run_id);
    run["threadId"] = json!(thread);
    run["ordinal"] = json!(ordinal);
    run["rootNodeId"] = json!(node_id);
    run["activeAttemptId"] = json!(attempt_id);
    run["providerThreadId"] = json!(provider_id);
    run["status"] = json!(status);
    run["startedAt"] = Value::Null;
    run["completedAt"] = Value::Null;
    let mut attempt = sample("OrchestrationV2RunAttempt");
    attempt["id"] = json!(attempt_id);
    attempt["runId"] = json!(run_id);
    attempt["attemptOrdinal"] = json!(1);
    attempt["rootNodeId"] = json!(node_id);
    attempt["providerThreadId"] = json!(provider_id);
    attempt["providerInstanceId"] = run["providerInstanceId"].clone();
    attempt["providerTurnId"] = Value::Null;
    attempt["status"] = json!("pending");
    attempt["startedAt"] = Value::Null;
    attempt["completedAt"] = Value::Null;
    let mut node = sample("OrchestrationV2ExecutionNode");
    node["id"] = json!(node_id);
    node["threadId"] = json!(thread);
    node["runId"] = json!(run_id);
    node["rootNodeId"] = json!(node_id);
    node["parentNodeId"] = Value::Null;
    node["kind"] = json!("root_turn");
    node["status"] = json!("pending");
    node["countsForRun"] = json!(true);
    node["providerThreadId"] = json!(provider_id);
    node["providerTurnId"] = Value::Null;
    node["startedAt"] = Value::Null;
    node["completedAt"] = Value::Null;
    let mut provider = sample("OrchestrationV2ProviderThread");
    provider["id"] = json!(provider_id);
    provider["appThreadId"] = json!(thread);
    provider["providerInstanceId"] = run["providerInstanceId"].clone();
    provider["providerSessionId"] = Value::Null;
    provider["firstRunOrdinal"] = json!(ordinal);
    provider["lastRunOrdinal"] = json!(ordinal);
    Command {
        id: CommandId(format!("seed:{thread}:{ordinal}")),
        thread_id: ThreadId(thread.into()),
        operation: Operation::CreateExecution(Box::new(ExecutionSeed {
            run: serde_json::from_value(run).unwrap(),
            attempt: serde_json::from_value(attempt).unwrap(),
            root: serde_json::from_value(node).unwrap(),
            provider_thread: serde_json::from_value(provider).unwrap(),
        })),
    }
}

fn guard(command: &Command) -> ProviderGuard {
    let Operation::CreateExecution(seed) = &command.operation else {
        panic!()
    };
    ProviderGuard {
        run_id: seed.run.id.clone(),
        active_attempt_id: seed.attempt.id.clone(),
        provider_thread_id: seed.provider_thread.id.clone(),
        provider_session_id: None,
        provider_turn_id: None,
        expected_last_run_ordinal: seed.run.ordinal,
    }
}

fn progress(command: &Command, status: &str) -> Command {
    let Operation::CreateExecution(seed) = &command.operation else {
        panic!()
    };
    let mut run = serde_json::to_value(&seed.run).unwrap();
    run["status"] = json!(status);
    let event = make(
        EventId(format!("progress:{}:{status}", command.id.0)),
        &command.thread_id,
        "run.updated",
        &run,
        NOW + 1,
    )
    .unwrap();
    Command {
        id: CommandId(format!("progress:{}:{status}", command.id.0)),
        thread_id: command.thread_id.clone(),
        operation: Operation::ProviderEvents {
            guard: guard(command),
            events: vec![event],
        },
    }
}

fn accept(store: &Store, command: &Command) -> CommandReceipt {
    let receipt = store.dispatch(command, NOW).unwrap();
    assert_eq!(receipt.status, ReceiptStatus::Accepted, "{receipt:?}");
    receipt
}

fn snapshot(store: &Store) -> Value {
    store
        .read(|conn| {
            let mut result = serde_json::Map::new();
            for table in [
                "orchestration_events",
                "orchestration_command_receipts",
                "orchestration_projection_threads",
                "orchestration_projection_runs",
                "orchestration_projection_attempts",
                "orchestration_projection_nodes",
                "orchestration_projection_records",
                "orchestration_projection_metadata",
                "orchestration_adoptions",
                "orchestration_effect_outbox",
                "orchestration_publication_batches",
                "orchestration_publication_acks",
                "processed_commands",
            ] {
                let mut stmt = conn.prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))?;
                let columns = stmt.column_count();
                let rows = stmt
                    .query_map([], |row| {
                        (0..columns)
                            .map(|index| {
                                use rusqlite::types::ValueRef;
                                Ok(match row.get_ref(index)? {
                                    ValueRef::Null => Value::Null,
                                    ValueRef::Integer(value) => json!(value),
                                    ValueRef::Real(value) => json!(value),
                                    ValueRef::Text(value) => json!(String::from_utf8_lossy(value)),
                                    ValueRef::Blob(_) => panic!("kernel stores no blobs"),
                                })
                            })
                            .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result.insert(table.into(), json!(rows));
            }
            Ok(Value::Object(result))
        })
        .unwrap()
}

#[test]
fn replay_and_rebuild_are_equivalent_and_do_not_enqueue_work() {
    let mut fixture = Fixture::new();
    fixture.create("t");
    let command = seed("t", 1, "starting");
    accept(&fixture.kernel.store, &command);
    accept(&fixture.kernel.store, &progress(&command, "running"));
    let before = snapshot(&fixture.kernel.store);
    fixture.kernel.store.rebuild().unwrap();
    assert_eq!(snapshot(&fixture.kernel.store), before);
    fixture.reopen();
    assert!(fixture.kernel.store.verify_projections().unwrap());
    assert_eq!(snapshot(&fixture.kernel.store), before);
    assert_eq!(fixture.kernel.store.projection_frontier().unwrap(), 6);
}

#[test]
fn accepted_and_rejected_receipts_replay_even_with_different_payloads() {
    let fixture = Fixture::new();
    let mut command = create_thread("t");
    let accepted = accept(&fixture.kernel.store, &command);
    command.operation = Operation::Recover;
    assert_eq!(
        fixture.kernel.store.dispatch(&command, NOW + 99).unwrap(),
        accepted
    );
    let rejected_command = Command {
        id: CommandId("absent".into()),
        thread_id: ThreadId("missing".into()),
        operation: Operation::Recover,
    };
    let rejected = fixture
        .kernel
        .store
        .dispatch(&rejected_command, NOW)
        .unwrap();
    assert_eq!(rejected.status, ReceiptStatus::Rejected);
    let mut retry = create_thread("missing");
    retry.id = rejected_command.id;
    assert_eq!(
        fixture.kernel.store.dispatch(&retry, NOW + 100).unwrap(),
        rejected
    );
    assert_eq!(fixture.kernel.store.events().unwrap().len(), 1);
    assert_eq!(
        fixture.kernel.store.pending_publications().unwrap().len(),
        1
    );
}

#[test]
fn transaction_rolls_back_at_each_acceptance_write_boundary() {
    for boundary in [
        WriteBoundary::BeforeReceipt,
        WriteBoundary::ReceiptReserved,
        WriteBoundary::EventAppended,
        WriteBoundary::ProjectionApplied,
        WriteBoundary::ProjectionRow,
        WriteBoundary::BindingActivated,
        WriteBoundary::ActivityUpdated,
        WriteBoundary::FrontierUpdated,
        WriteBoundary::EffectEnqueued,
        WriteBoundary::PublicationEnqueued,
        WriteBoundary::ReceiptFinalized,
        WriteBoundary::BeforeCommit,
    ] {
        let count = if matches!(
            boundary,
            WriteBoundary::EventAppended
                | WriteBoundary::ProjectionApplied
                | WriteBoundary::ProjectionRow
                | WriteBoundary::ActivityUpdated
                | WriteBoundary::FrontierUpdated
        ) {
            4
        } else {
            1
        };
        for occurrence in 1..=count {
            let fixture = Fixture::new();
            fixture.create("t");
            let command = seed("t", 1, "starting");
            let before = snapshot(&fixture.kernel.store);
            fixture.kernel.store.inject_failure(boundary, occurrence);
            assert!(
                matches!(fixture.kernel.store.dispatch(&command, NOW), Err(Error::Injected(value)) if value == boundary),
                "{boundary:?} occurrence {occurrence}"
            );
            assert_eq!(
                snapshot(&fixture.kernel.store),
                before,
                "{boundary:?}:{occurrence}"
            );
            accept(&fixture.kernel.store, &command);
            assert_eq!(fixture.kernel.store.effects().unwrap().len(), 1);
        }
    }
}

#[test]
fn crash_after_commit_and_before_processed_mark_only_replays_source() {
    let mut fixture = Fixture::new();
    fixture.create("t");
    let command = seed("t", 1, "starting");
    fixture
        .kernel
        .store
        .inject_failure(WriteBoundary::AfterCommit, 1);
    assert!(fixture.kernel.store.dispatch(&command, NOW).is_err());
    fixture.reopen();
    let receipt = accept(&fixture.kernel.store, &command);
    assert_eq!(receipt.result_sequence, 5);
    assert_eq!(fixture.kernel.store.effects().unwrap().len(), 1);
    fixture
        .kernel
        .store
        .inject_failure(WriteBoundary::IntentProcessed, 1);
    assert!(
        fixture
            .kernel
            .store
            .mark_intent_processed(&command.id, NOW)
            .is_err()
    );
    fixture
        .kernel
        .store
        .mark_intent_processed(&command.id, NOW)
        .unwrap();
    assert!(
        fixture
            .kernel
            .store
            .mark_intent_processed(&CommandId("not accepted".into()), NOW)
            .is_err()
    );
    assert_eq!(fixture.kernel.store.events().unwrap().len(), 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_commands_and_receipt_races_have_one_authority() {
    let fixture = Fixture::new();
    fixture.create("t");
    let first = seed("t", 1, "starting");
    let mut second = seed("t", 1, "starting");
    second.id = CommandId("racing".into());
    let (one, two) = tokio::join!(
        fixture.kernel.dispatch(&first, NOW),
        fixture.kernel.dispatch(&second, NOW)
    );
    let statuses = [one.unwrap().status, two.unwrap().status];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == ReceiptStatus::Accepted)
            .count(),
        1
    );
    assert_eq!(fixture.kernel.store.effects().unwrap().len(), 1);
    let (one, two) = tokio::join!(
        fixture.kernel.dispatch(&first, NOW),
        fixture.kernel.dispatch(&first, NOW)
    );
    assert_eq!(one.unwrap(), two.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_kernel_handles_still_serialize_in_sqlite() {
    let fixture = Fixture::new();
    fixture.create("t");
    let other = Kernel::open(
        Arc::new(DocsStore::open(fixture.dir.path()).unwrap()),
        "host",
    )
    .unwrap();
    let command = seed("t", 1, "starting");
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let left = fixture.kernel.clone();
    let right = other.clone();
    let left_command = command.clone();
    let left_barrier = barrier.clone();
    let one = tokio::spawn(async move {
        left_barrier.wait().await;
        left.dispatch(&left_command, NOW).await
    });
    let two = tokio::spawn(async move {
        barrier.wait().await;
        right.dispatch(&command, NOW).await
    });
    assert_eq!(one.await.unwrap().unwrap(), two.await.unwrap().unwrap());
    assert_eq!(other.store.effects().unwrap().len(), 1);
}

#[test]
fn late_events_after_supersession_or_terminality_are_rejected() {
    let fixture = Fixture::new();
    fixture.create("t");
    let command = seed("t", 1, "starting");
    accept(&fixture.kernel.store, &command);
    accept(&fixture.kernel.store, &progress(&command, "running"));
    let Operation::CreateExecution(seed) = &command.operation else {
        panic!()
    };
    let mut attempt = seed.attempt.clone();
    attempt.id = RunAttemptId("attempt2".into());
    attempt.attempt_ordinal = 2;
    attempt.reason = OrchestrationV2RunAttemptReason::ProviderRecovery;
    let mut root = seed.root.clone();
    root.id = NodeId("root2".into());
    root.root_node_id = root.id.clone();
    attempt.root_node_id = root.id.clone();
    let replacement = Command {
        id: CommandId("replace".into()),
        thread_id: command.thread_id.clone(),
        operation: Operation::ReplaceAttempt {
            guard: guard(&command),
            attempt: Box::new(attempt.clone()),
            root: Box::new(root),
        },
    };
    let before = snapshot(&fixture.kernel.store);
    fixture
        .kernel
        .store
        .inject_failure(WriteBoundary::EffectsRetired, 1);
    assert!(fixture.kernel.store.dispatch(&replacement, NOW).is_err());
    assert_eq!(snapshot(&fixture.kernel.store), before);
    accept(&fixture.kernel.store, &replacement);
    let late = progress(&command, "completed");
    let rejection = fixture.kernel.store.dispatch(&late, NOW).unwrap();
    assert_eq!(rejection.status, ReceiptStatus::Rejected);
    assert_eq!(
        rejection.error.as_deref(),
        Some("Stale provider ownership.")
    );
    assert_eq!(
        fixture.kernel.store.dispatch(&late, NOW + 2).unwrap(),
        rejection
    );
    let mut next_guard = guard(&command);
    next_guard.active_attempt_id = attempt.id;
    let mut run = fixture
        .kernel
        .store
        .thread(&command.thread_id)
        .unwrap()
        .unwrap()
        .runs[0]
        .clone();
    run.status = OrchestrationV2RunStatus::Completed;
    let event = make(
        EventId("new completed".into()),
        &command.thread_id,
        "run.updated",
        &run,
        NOW,
    )
    .unwrap();
    let terminal = Command {
        id: CommandId("new completed".into()),
        thread_id: command.thread_id.clone(),
        operation: Operation::ProviderEvents {
            guard: next_guard.clone(),
            events: vec![event.clone()],
        },
    };
    accept(&fixture.kernel.store, &terminal);
    let mut replay = terminal;
    replay.id = CommandId("late new attempt".into());
    assert_eq!(
        fixture.kernel.store.dispatch(&replay, NOW).unwrap().status,
        ReceiptStatus::Rejected
    );
}

#[test]
fn session_and_ordinal_guards_reject_without_events_or_effects() {
    let fixture = Fixture::new();
    fixture.create("t");
    let seed = seed("t", 1, "starting");
    accept(&fixture.kernel.store, &seed);
    for index in 0..4 {
        let mut command = progress(&seed, "running");
        command.id = CommandId(format!("stale:{index}"));
        let Operation::ProviderEvents { guard, .. } = &mut command.operation else {
            panic!()
        };
        match index {
            0 => guard.provider_session_id = Some(ProviderSessionId("old".into())),
            1 => guard.expected_last_run_ordinal = 2,
            2 => guard.provider_thread_id = ProviderThreadId("old".into()),
            _ => guard.provider_turn_id = Some(ProviderTurnId("old".into())),
        }
        let before = fixture.kernel.store.events().unwrap();
        assert_eq!(
            fixture.kernel.store.dispatch(&command, NOW).unwrap().status,
            ReceiptStatus::Rejected
        );
        assert_eq!(fixture.kernel.store.events().unwrap(), before);
    }
}

#[test]
fn adoption_never_enqueues_effects_or_fabricates_task_lineage() {
    let fixture = Fixture::new();
    let temp = Fixture::new();
    temp.create("t");
    let thread = temp
        .kernel
        .store
        .thread(&ThreadId("t".into()))
        .unwrap()
        .unwrap()
        .thread;
    let adoption = Command {
        id: CommandId("adopt".into()),
        thread_id: thread.id.clone(),
        operation: Operation::Adopt {
            legacy_chat_id: "old-chat".into(),
            thread: Box::new(thread),
            messages: vec![],
        },
    };
    fixture
        .kernel
        .store
        .inject_failure(WriteBoundary::AdoptionRecorded, 1);
    assert!(fixture.kernel.store.dispatch(&adoption, NOW).is_err());
    assert!(!fixture.kernel.store.is_v2_managed("old-chat").unwrap());
    accept(&fixture.kernel.store, &adoption);
    assert!(fixture.kernel.store.is_v2_managed("old-chat").unwrap());
    assert!(fixture.kernel.store.effects().unwrap().is_empty());
}

fn enqueue_for_test(store: &Store, thread: &str, id: &str, request: &EffectRequest) {
    store
        .write(|tx| {
            effects::enqueue(
                tx,
                id,
                &CommandId(id.into()),
                &ThreadId(thread.into()),
                request,
                NOW,
            )
        })
        .unwrap();
}

#[derive(Default)]
struct FakeExecutor {
    outcomes: Mutex<VecDeque<EffectOutcome>>,
    calls: Mutex<Vec<String>>,
    accepted: Mutex<BTreeSet<String>>,
    probe: Option<Store>,
}

#[async_trait]
impl EffectExecutor for FakeExecutor {
    async fn execute(&self, effect: &Effect, _cancellation: CancellationToken) -> EffectOutcome {
        if let Some(store) = &self.probe {
            // A read here would deadlock if the worker retained a transaction.
            assert!(store.receipt(&effect.command_id).unwrap().is_some());
        }
        self.calls.lock().unwrap().push(effect.id.clone());
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(EffectOutcome::Succeeded);
        if outcome == EffectOutcome::Succeeded {
            self.accepted.lock().unwrap().insert(effect.id.clone());
        }
        outcome
    }
}

#[tokio::test]
async fn lane_retry_blocks_followers_but_not_titles_or_other_threads() {
    let fixture = Fixture::new();
    enqueue_for_test(
        &fixture.kernel.store,
        "a",
        "rollback",
        &EffectRequest::TerminalCleanup,
    );
    enqueue_for_test(
        &fixture.kernel.store,
        "a",
        "start",
        &EffectRequest::ProviderTurnStart {
            run_id: RunId("r".into()),
        },
    );
    enqueue_for_test(
        &fixture.kernel.store,
        "a",
        "title",
        &EffectRequest::ThreadTitleGenerate {
            kind: TitleKind::Regenerate,
        },
    );
    enqueue_for_test(
        &fixture.kernel.store,
        "b",
        "other",
        &EffectRequest::TerminalCleanup,
    );
    let executor = Arc::new(FakeExecutor::default());
    executor
        .outcomes
        .lock()
        .unwrap()
        .push_back(EffectOutcome::Retry);
    let worker = EffectWorker::new(
        fixture.kernel.store.clone(),
        executor.clone(),
        "worker".into(),
    );
    assert!(worker.step(NOW).await.unwrap());
    // The worker schedules retry from completion, including real elapsed SQL
    // and executor time. Do not assume execution finished in less than 1ms.
    let retry_at = fixture
        .kernel
        .store
        .effect("rollback")
        .unwrap()
        .unwrap()
        .available_at;
    assert!(retry_at >= NOW + retry_delay_ms(1));
    assert!(worker.step(NOW).await.unwrap());
    assert!(worker.step(NOW).await.unwrap());
    assert!(!worker.step(retry_at - 1).await.unwrap());
    assert!(worker.step(retry_at).await.unwrap());
    assert!(worker.step(retry_at).await.unwrap());
    assert_eq!(
        *executor.calls.lock().unwrap(),
        ["rollback", "title", "other", "rollback", "start"]
    );
}

#[test]
fn lease_expiry_fences_stale_worker_and_uncertain_acceptance_blocks_lane() {
    let fixture = Fixture::new();
    enqueue_for_test(
        &fixture.kernel.store,
        "a",
        "safe",
        &EffectRequest::TerminalCleanup,
    );
    let claim = fixture
        .kernel
        .store
        .claim_effect("same-worker", NOW, 30)
        .unwrap()
        .unwrap();
    fixture.kernel.store.begin_effect(&claim, NOW).unwrap();
    let newer = fixture
        .kernel
        .store
        .claim_effect("same-worker", NOW + 30, 30)
        .unwrap()
        .unwrap();
    assert!(
        !fixture
            .kernel
            .store
            .finish_effect(&claim, &EffectOutcome::Succeeded, NOW + 31, 5)
            .unwrap()
    );
    assert!(
        fixture
            .kernel
            .store
            .finish_effect(&newer, &EffectOutcome::Succeeded, NOW + 31, 5)
            .unwrap()
    );
    enqueue_for_test(
        &fixture.kernel.store,
        "a",
        "process",
        &EffectRequest::ProviderTurnStart {
            run_id: RunId("r".into()),
        },
    );
    enqueue_for_test(
        &fixture.kernel.store,
        "a",
        "next",
        &EffectRequest::TerminalCleanup,
    );
    let claim = fixture
        .kernel
        .store
        .claim_effect("worker", NOW + 40, 30)
        .unwrap()
        .unwrap();
    fixture.kernel.store.begin_effect(&claim, NOW + 40).unwrap();
    assert!(
        fixture
            .kernel
            .store
            .claim_effect("other", NOW + 70, 30)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fixture
            .kernel
            .store
            .effect("process")
            .unwrap()
            .unwrap()
            .status,
        EffectStatus::Uncertain
    );
    assert!(
        fixture
            .kernel
            .store
            .resolve_uncertain("process", true, NOW + 70)
            .unwrap()
    );
    assert_eq!(
        fixture
            .kernel
            .store
            .claim_effect("other", NOW + 70, 30)
            .unwrap()
            .unwrap()
            .id,
        "next"
    );
}

#[tokio::test]
async fn crashes_before_dispatch_and_after_acceptance_never_repeat_process_work() {
    for started in [false, true] {
        let mut fixture = Fixture::new();
        fixture.create("t");
        let command = seed("t", 1, "starting");
        accept(&fixture.kernel.store, &command);
        let effect = fixture
            .kernel
            .store
            .claim_effect("old", NOW, 30_000)
            .unwrap()
            .unwrap();
        let executor = FakeExecutor::default();
        if started {
            fixture.kernel.store.begin_effect(&effect, NOW).unwrap();
            assert_eq!(
                executor.execute(&effect, CancellationToken::new()).await,
                EffectOutcome::Succeeded
            );
            fixture
                .kernel
                .store
                .inject_failure(WriteBoundary::EffectAck, 1);
            assert!(
                fixture
                    .kernel
                    .store
                    .finish_effect(&effect, &EffectOutcome::Succeeded, NOW, 5)
                    .is_err()
            );
        }
        fixture.reopen();
        let summary = fixture.kernel.recover(NOW + 1).await.unwrap();
        assert_eq!(summary.uncertain_effects, usize::from(started));
        assert_eq!(summary.retired_process_effects, usize::from(!started));
        assert!(
            fixture
                .kernel
                .store
                .claim_effect("new", NOW + 1, 30_000)
                .unwrap()
                .is_none()
        );
        assert_eq!(executor.calls.lock().unwrap().len(), usize::from(started));
        let projection = fixture
            .kernel
            .store
            .thread(&command.thread_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            projection.runs[0].status,
            OrchestrationV2RunStatus::Cancelled
        );
        assert_eq!(
            fixture
                .kernel
                .store
                .receipt(&command.id)
                .unwrap()
                .unwrap()
                .status,
            ReceiptStatus::Accepted
        );
        assert!(
            fixture
                .kernel
                .store
                .events()
                .unwrap()
                .iter()
                .any(|event| event.command_id.as_ref() == Some(&command.id))
        );
    }
}

#[tokio::test]
async fn safe_effect_replay_reuses_identity_after_lost_ack() {
    let mut fixture = Fixture::new();
    enqueue_for_test(
        &fixture.kernel.store,
        "a",
        "safe",
        &EffectRequest::TerminalCleanup,
    );
    let executor = Arc::new(FakeExecutor::default());
    let worker = EffectWorker::new(fixture.kernel.store.clone(), executor.clone(), "old".into());
    fixture
        .kernel
        .store
        .inject_failure(WriteBoundary::EffectAck, 1);
    assert!(worker.step(NOW).await.is_err());
    fixture.reopen();
    assert_eq!(
        fixture
            .kernel
            .recover(NOW + 1)
            .await
            .unwrap()
            .requeued_effects,
        1
    );
    let worker = EffectWorker::new(fixture.kernel.store.clone(), executor.clone(), "new".into());
    assert!(worker.step(NOW + 1).await.unwrap());
    assert_eq!(*executor.calls.lock().unwrap(), ["safe", "safe"]);
    assert_eq!(executor.accepted.lock().unwrap().len(), 1);
}

struct BlockingExecutor {
    entered: Notify,
    accepted: Mutex<usize>,
}

#[async_trait]
impl EffectExecutor for BlockingExecutor {
    async fn execute(&self, _effect: &Effect, token: CancellationToken) -> EffectOutcome {
        self.entered.notify_one();
        token.cancelled().await;
        *self.accepted.lock().unwrap() += 1;
        EffectOutcome::Uncertain
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_races_executor_and_cancelled_claim_cannot_ack() {
    let fixture = Fixture::new();
    enqueue_for_test(
        &fixture.kernel.store,
        "a",
        "cancel",
        &EffectRequest::TerminalCleanup,
    );
    let executor = Arc::new(BlockingExecutor {
        entered: Notify::new(),
        accepted: Mutex::new(0),
    });
    let worker = EffectWorker::new(fixture.kernel.store.clone(), executor.clone(), "w".into());
    let running = tokio::spawn(async move { worker.step(NOW).await });
    executor.entered.notified().await;
    fixture
        .kernel
        .store
        .cancel_effect("cancel", NOW + 1)
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        fixture
            .kernel
            .store
            .effect("cancel")
            .unwrap()
            .unwrap()
            .status,
        EffectStatus::Cancelled
    );
    assert!(
        !fixture
            .kernel
            .store
            .cancel_effect("cancel", NOW + 1)
            .unwrap()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn four_start_workers_allow_more_than_four_live_sessions() {
    let fixture = Fixture::new();
    for index in 0..8 {
        let thread = format!("thread:{index}");
        fixture.create(&thread);
        accept(&fixture.kernel.store, &seed(&thread, 1, "starting"));
    }
    let executor = Arc::new(FakeExecutor {
        probe: Some(fixture.kernel.store.clone()),
        ..FakeExecutor::default()
    });
    for _ in 0..2 {
        let workers: Vec<_> = (0..DEFAULT_WORKER_CONCURRENCY)
            .map(|index| {
                EffectWorker::new(
                    fixture.kernel.store.clone(),
                    executor.clone(),
                    format!("w{index}"),
                )
            })
            .collect();
        let results =
            futures::future::join_all(workers.iter().map(|worker| worker.step(NOW))).await;
        assert!(results.into_iter().all(|result| result.unwrap()));
    }
    assert_eq!(executor.accepted.lock().unwrap().len(), 8);
    assert!(
        fixture
            .kernel
            .store
            .effects()
            .unwrap()
            .iter()
            .all(|effect| effect.status == EffectStatus::Succeeded)
    );
}

#[derive(Default)]
struct FakePublisher {
    documents: Mutex<BTreeMap<String, Value>>,
    versions: Mutex<BTreeMap<String, i64>>,
    writes: Mutex<usize>,
}

#[async_trait]
impl ProjectionPublisher for FakePublisher {
    async fn publish(
        &self,
        batch: &PublicationBatch,
        document: &PublicationDocument,
    ) -> Result<()> {
        assert_eq!(batch.host_id, "host");
        let mut versions = self.versions.lock().unwrap();
        let current = versions.entry(document.doc_id.clone()).or_insert(-1);
        if document.version > *current {
            self.documents
                .lock()
                .unwrap()
                .insert(document.doc_id.clone(), document.payload.clone());
            *current = document.version;
            *self.writes.lock().unwrap() += 1;
        }
        Ok(())
    }
}

#[tokio::test]
async fn publication_crash_after_apply_before_ack_replays_same_versions() {
    let mut fixture = Fixture::new();
    fixture.create("t");
    let batch = fixture.kernel.store.pending_publications().unwrap()[0].clone();
    let publisher = Arc::new(FakePublisher::default());
    let worker = PublicationWorker {
        store: fixture.kernel.store.clone(),
        publisher: publisher.clone(),
    };
    fixture
        .kernel
        .store
        .inject_failure(WriteBoundary::PublicationAck, 1);
    assert!(worker.step().await.is_err());
    assert_eq!(*publisher.writes.lock().unwrap(), 1);
    fixture.reopen();
    let worker = PublicationWorker {
        store: fixture.kernel.store.clone(),
        publisher: publisher.clone(),
    };
    assert!(worker.step().await.unwrap());
    assert_eq!(*publisher.writes.lock().unwrap(), 2);
    assert!(!worker.step().await.unwrap());
    for doc in batch.documents {
        assert_eq!(
            publisher.documents.lock().unwrap()[&doc.doc_id],
            doc.payload
        );
    }
    assert!(
        fixture
            .kernel
            .store
            .acknowledge_publication(&batch.batch_id, "foreign", 1)
            .is_err()
    );
}

#[test]
fn barrier_requires_every_document_and_rejects_foreign_and_old_epochs() {
    let fixture = Fixture::new();
    fixture.create("t");
    let batch = fixture.kernel.store.pending_publications().unwrap()[0].clone();
    let mut replica = ReplicaBarrier::new("host".into(), batch.host_epoch);
    assert!(!replica.ready(&batch));
    assert!(replica.observe(&batch, &batch.documents[1]));
    assert!(!replica.ready(&batch));
    assert!(replica.observe(&batch, &batch.documents[0]));
    assert!(replica.ready(&batch));
    assert!(!replica.observe(&batch, &batch.documents[0]));
    let mut newer = batch.clone();
    newer.host_epoch += 1;
    assert!(replica.observe(&newer, &newer.documents[0]));
    assert!(!replica.ready(&newer));
    assert!(!replica.observe(&batch, &batch.documents[1]));
    newer.host_id = "peer".into();
    assert!(!replica.observe(&newer, &newer.documents[1]));
    assert!(matches!(
        Kernel::open(
            Arc::new(DocsStore::open(fixture.dir.path()).unwrap()),
            "peer"
        ),
        Err(Error::NotOwner)
    ));
}

#[tokio::test]
async fn recovery_holds_queue_repairs_projection_and_is_restart_safe() {
    let mut fixture = Fixture::new();
    fixture.create("t");
    let queued = seed("t", 1, "queued");
    accept(&fixture.kernel.store, &queued);
    fixture
        .kernel
        .store
        .write(|tx| {
            tx.execute(
                "UPDATE orchestration_projection_metadata SET last_sequence=0",
                [],
            )?;
            tx.execute(
                "UPDATE orchestration_projection_runs SET payload_json='broken'",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    fixture.reopen();
    let summary = fixture.kernel.recover(NOW + 1).await.unwrap();
    assert!(summary.projection_rebuilt);
    let projection = fixture
        .kernel
        .store
        .thread(&queued.thread_id)
        .unwrap()
        .unwrap();
    assert_eq!(projection.runs[0].status, OrchestrationV2RunStatus::Queued);
    assert_eq!(projection.runs[0].queue_held.as_ref(), Some(&true));
    assert_eq!(
        projection.attempts[0].status,
        OrchestrationV2RunAttemptStatus::Pending
    );
    assert!(fixture.kernel.store.effects().unwrap().is_empty());
    fixture.kernel.recover(NOW + 2).await.unwrap();
    assert!(fixture.kernel.store.verify_projections().unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn keyed_lock_order_is_deterministic_and_duplicate_keys_do_not_deadlock() {
    let locks = Arc::new(ThreadLocks::default());
    let first = locks
        .acquire([
            ThreadId("b".into()),
            ThreadId("a".into()),
            ThreadId("a".into()),
        ])
        .await;
    let other = locks.clone();
    let second = tokio::spawn(async move {
        other
            .acquire([ThreadId("a".into()), ThreadId("b".into())])
            .await
    });
    tokio::task::yield_now().await;
    assert!(!second.is_finished());
    drop(first);
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), second)
            .await
            .unwrap()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn stable_command_scope_uses_js_component_escaping() {
    assert_eq!(encode_component(" /é?!*'()"), "%20%2F%C3%A9%3F!*'()");
    assert_eq!(
        mcp_command_id("session:a", "delegate-task", "round/1").0,
        "command:mcp:session%3Aa:delegate-task:round%2F1"
    );
    assert_ne!(
        mcp_command_id("a", "send", "key"),
        mcp_command_id("b", "send", "key")
    );
    assert_eq!(retry_delay_ms(1), 100);
    assert_eq!(retry_delay_ms(100), 30_000);
}

#[test]
fn effect_claim_begin_ack_cancel_boundaries_are_transactional() {
    for boundary in [
        WriteBoundary::EffectClaim,
        WriteBoundary::EffectBegin,
        WriteBoundary::EffectAck,
        WriteBoundary::EffectCancel,
    ] {
        let fixture = Fixture::new();
        enqueue_for_test(
            &fixture.kernel.store,
            "a",
            "e",
            &EffectRequest::TerminalCleanup,
        );
        let claim = if boundary != WriteBoundary::EffectClaim {
            Some(
                fixture
                    .kernel
                    .store
                    .claim_effect("w", NOW, 30_000)
                    .unwrap()
                    .unwrap(),
            )
        } else {
            None
        };
        let before = snapshot(&fixture.kernel.store);
        fixture.kernel.store.inject_failure(boundary, 1);
        let result = match boundary {
            WriteBoundary::EffectClaim => fixture
                .kernel
                .store
                .claim_effect("w", NOW, 30_000)
                .map(|_| ()),
            WriteBoundary::EffectBegin => fixture
                .kernel
                .store
                .begin_effect(claim.as_ref().unwrap(), NOW)
                .map(|_| ()),
            WriteBoundary::EffectAck => fixture
                .kernel
                .store
                .finish_effect(claim.as_ref().unwrap(), &EffectOutcome::Succeeded, NOW, 5)
                .map(|_| ()),
            _ => fixture.kernel.store.cancel_effect("e", NOW).map(|_| ()),
        };
        assert!(result.is_err());
        assert_eq!(snapshot(&fixture.kernel.store), before, "{boundary:?}");
    }
}

#[tokio::test]
async fn acknowledged_effect_is_not_reexecuted_after_restart() {
    let mut fixture = Fixture::new();
    fixture.create("t");
    accept(&fixture.kernel.store, &seed("t", 1, "starting"));
    let executor = Arc::new(FakeExecutor::default());
    let worker = EffectWorker::new(fixture.kernel.store.clone(), executor.clone(), "old".into());
    assert!(worker.step(NOW).await.unwrap());
    fixture.reopen();
    fixture.kernel.recover(NOW + 100).await.unwrap();
    let worker = EffectWorker::new(fixture.kernel.store.clone(), executor.clone(), "new".into());
    assert!(!worker.step(NOW + 100).await.unwrap());
    assert_eq!(executor.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn recovery_rolls_back_each_outbox_write_and_epoch_boundary() {
    for (boundary, count) in [
        (WriteBoundary::RecoveryEffects, 3),
        (WriteBoundary::EpochAdvanced, 1),
        (WriteBoundary::BeforeCommit, 1),
    ] {
        for occurrence in 1..=count {
            let fixture = Fixture::new();
            fixture.create("t");
            accept(&fixture.kernel.store, &seed("t", 1, "starting"));
            enqueue_for_test(
                &fixture.kernel.store,
                "b",
                "safe",
                &EffectRequest::TerminalCleanup,
            );
            let claim = fixture
                .kernel
                .store
                .claim_effect("old", NOW, 30_000)
                .unwrap()
                .unwrap();
            fixture.kernel.store.begin_effect(&claim, NOW).unwrap();
            let before = snapshot(&fixture.kernel.store);
            let epoch: i64 = fixture
                .kernel
                .store
                .read(|conn| {
                    Ok(conn
                        .query_row("SELECT epoch FROM orchestration_host", [], |row| row.get(0))?)
                })
                .unwrap();
            fixture.kernel.store.inject_failure(boundary, occurrence);
            assert!(fixture.kernel.recover(NOW + 1).await.is_err());
            assert_eq!(snapshot(&fixture.kernel.store), before);
            let unchanged: i64 = fixture
                .kernel
                .store
                .read(|conn| {
                    Ok(conn
                        .query_row("SELECT epoch FROM orchestration_host", [], |row| row.get(0))?)
                })
                .unwrap();
            assert_eq!(epoch, unchanged);
            fixture.kernel.recover(NOW + 2).await.unwrap();
        }
    }
}

#[tokio::test]
async fn final_publication_ack_failure_and_lost_response_do_not_duplicate_docs() {
    for after_commit in [false, true] {
        let mut fixture = Fixture::new();
        fixture.create("t");
        let batch = fixture.kernel.store.pending_publications().unwrap()[0].clone();
        let publisher = Arc::new(FakePublisher::default());
        publisher
            .publish(&batch, &batch.documents[0])
            .await
            .unwrap();
        fixture
            .kernel
            .store
            .acknowledge_publication(
                &batch.batch_id,
                &batch.documents[0].doc_id,
                batch.documents[0].version,
            )
            .unwrap();
        publisher
            .publish(&batch, &batch.documents[1])
            .await
            .unwrap();
        fixture.kernel.store.inject_failure(
            if after_commit {
                WriteBoundary::AfterCommit
            } else {
                WriteBoundary::PublicationBatchAck
            },
            1,
        );
        assert!(
            fixture
                .kernel
                .store
                .acknowledge_publication(
                    &batch.batch_id,
                    &batch.documents[1].doc_id,
                    batch.documents[1].version,
                )
                .is_err()
        );
        fixture.reopen();
        let worker = PublicationWorker {
            store: fixture.kernel.store.clone(),
            publisher: publisher.clone(),
        };
        assert_eq!(worker.step().await.unwrap(), !after_commit);
        assert_eq!(*publisher.writes.lock().unwrap(), 2);
        assert!(
            fixture
                .kernel
                .store
                .pending_publications()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn backward_transition_and_cross_thread_provider_batch_are_durably_refused() {
    let fixture = Fixture::new();
    fixture.create("t");
    let command = seed("t", 1, "starting");
    accept(&fixture.kernel.store, &command);
    accept(&fixture.kernel.store, &progress(&command, "running"));
    let backward = progress(&command, "queued");
    assert_eq!(
        fixture
            .kernel
            .store
            .dispatch(&backward, NOW)
            .unwrap()
            .status,
        ReceiptStatus::Rejected
    );
    let Operation::CreateExecution(seed) = &command.operation else {
        panic!()
    };
    let mut root = seed.root.clone();
    root.thread_id = ThreadId("foreign".into());
    let event = make(
        EventId("foreign".into()),
        &root.thread_id,
        "node.updated",
        &root,
        NOW,
    )
    .unwrap();
    let foreign = Command {
        id: CommandId("foreign".into()),
        thread_id: command.thread_id.clone(),
        operation: Operation::ProviderEvents {
            guard: guard(&command),
            events: vec![event],
        },
    };
    assert_eq!(
        fixture.kernel.store.dispatch(&foreign, NOW).unwrap().status,
        ReceiptStatus::Rejected
    );
    assert_eq!(fixture.kernel.store.effects().unwrap().len(), 1);
}

fn fixture_event(store: &Store, thread: &str, event_type: &str, payload: &Value) {
    use super::event::Envelope;
    store
        .write(|tx| {
            let sequence = super::store::latest_sequence(tx)? + 1;
            let stream: i64 = tx.query_row(
            "SELECT COALESCE(MAX(stream_version)+1,0) FROM orchestration_events WHERE stream_id=?1",
            [thread], |row| row.get(0),
        )?;
            let event_id = format!("fixture:{sequence}");
            let event = make(
                EventId(event_id.clone()),
                &ThreadId(thread.into()),
                event_type,
                payload,
                NOW,
            )?;
            let envelope = Envelope {
                application_event_version: 2,
                sequence,
                stream_version: stream,
                command_id: Some(CommandId(event_id.clone())),
                event,
            };
            tx.execute(
                "INSERT INTO orchestration_events VALUES(?1,2,?2,?2,'thread',?3,?4,?5,?6,?7)",
                rusqlite::params![
                    sequence,
                    event_id,
                    thread,
                    stream,
                    event_type,
                    super::event::iso(NOW)?,
                    serde_json::to_string(&envelope)?
                ],
            )?;
            projection::apply_checked(tx, &envelope, &|_| Ok(()))
        })
        .unwrap();
}

#[tokio::test]
async fn message_answerable_requests_survive_while_live_requests_expire() {
    let fixture = Fixture::new();
    fixture.create("t");
    let command = seed("t", 1, "starting");
    accept(&fixture.kernel.store, &command);
    let Operation::CreateExecution(seed) = &command.operation else {
        panic!()
    };
    for (id, capability) in [
        ("message", json!({"type":"message"})),
        (
            "live",
            json!({"type":"live","providerSessionId":"old-session"}),
        ),
    ] {
        let mut request = sample("OrchestrationV2RuntimeRequest");
        request["id"] = json!(id);
        request["nodeId"] = json!(seed.root.id);
        request["status"] = json!("pending");
        request["responseCapability"] = capability;
        fixture_event(
            &fixture.kernel.store,
            "t",
            "runtime-request.updated",
            &request,
        );
    }
    fixture.kernel.recover(NOW + 1).await.unwrap();
    let requests = fixture
        .kernel
        .store
        .read(|conn| projection::read_records(conn, "t", "runtime-request"))
        .unwrap();
    assert_eq!(
        requests
            .iter()
            .find(|request| request["id"] == "message")
            .unwrap()["status"],
        "pending"
    );
    assert_eq!(
        requests
            .iter()
            .find(|request| request["id"] == "live")
            .unwrap()["status"],
        "expired"
    );
    assert_eq!(
        fixture
            .kernel
            .store
            .thread(&command.thread_id)
            .unwrap()
            .unwrap()
            .nodes[0]
            .status,
        OrchestrationV2ExecutionNodeStatus::Pending
    );
    let before = snapshot(&fixture.kernel.store);
    fixture.kernel.store.rebuild().unwrap();
    assert_eq!(snapshot(&fixture.kernel.store), before);
    let replicated =
        serde_json::to_string(&fixture.kernel.store.pending_publications().unwrap()).unwrap();
    assert!(!replicated.contains("old-session"));
}

#[test]
fn omitted_run_updates_preserve_recorded_cohort_and_roster() {
    let fixture = Fixture::new();
    fixture.create("t");
    let command = seed("t", 1, "queued");
    accept(&fixture.kernel.store, &command);
    let Operation::CreateExecution(seed) = &command.operation else {
        panic!()
    };
    let mut first = serde_json::to_value(&seed.run).unwrap();
    let cohort = json!({"disposition":"open","nextGeneration":1,"delivery":null});
    first["delegatedCompletion"] = cohort.clone();
    first["restartCancelledBackgroundWork"] = json!([]);
    fixture_event(&fixture.kernel.store, "t", "run.updated", &first);
    first.as_object_mut().unwrap().remove("delegatedCompletion");
    first
        .as_object_mut()
        .unwrap()
        .remove("restartCancelledBackgroundWork");
    fixture_event(&fixture.kernel.store, "t", "run.updated", &first);
    let projected = fixture
        .kernel
        .store
        .thread(&command.thread_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&projected.runs[0]).unwrap()["delegatedCompletion"],
        cohort
    );
    fixture.kernel.store.rebuild().unwrap();
    assert_eq!(
        fixture
            .kernel
            .store
            .thread(&command.thread_id)
            .unwrap()
            .unwrap(),
        projected
    );
}

#[tokio::test]
async fn stale_external_input_is_dropped_without_receipting_and_replay_is_idempotent() {
    let fixture = Fixture::new();
    fixture.create("t");
    let command = seed("t", 1, "starting");
    accept(&fixture.kernel.store, &command);
    let progress = progress(&command, "running");
    let Operation::ProviderEvents { guard, events } = &progress.operation else {
        panic!()
    };
    let mut stale = guard.clone();
    stale.active_attempt_id = RunAttemptId("old".into());
    let before = snapshot(&fixture.kernel.store);
    assert!(
        !fixture
            .kernel
            .append_provider_events(&command.thread_id, &stale, events, NOW)
            .await
            .unwrap()
    );
    assert_eq!(snapshot(&fixture.kernel.store), before);
    assert!(
        fixture
            .kernel
            .append_provider_events(&command.thread_id, guard, events, NOW)
            .await
            .unwrap()
    );
    let after = snapshot(&fixture.kernel.store);
    assert!(
        !fixture
            .kernel
            .append_provider_events(&command.thread_id, guard, events, NOW)
            .await
            .unwrap()
    );
    assert_eq!(snapshot(&fixture.kernel.store), after);
    assert!(
        fixture
            .kernel
            .store
            .receipt(&progress.id)
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .kernel
            .store
            .events()
            .unwrap()
            .last()
            .unwrap()
            .command_id
            .is_none()
    );
    fixture.kernel.store.rebuild().unwrap();
    assert_eq!(snapshot(&fixture.kernel.store), after);
}

#[test]
fn projection_rebuild_failures_leave_the_previous_generation_intact() {
    for (boundary, count) in [
        (WriteBoundary::ProjectionCleared, 6),
        (WriteBoundary::ProjectionRow, 5),
        (WriteBoundary::ActivityUpdated, 4),
        (WriteBoundary::FrontierUpdated, 5),
        (WriteBoundary::ProjectionRebuilt, 1),
    ] {
        for occurrence in 1..=count {
            let fixture = Fixture::new();
            fixture.create("t");
            accept(&fixture.kernel.store, &seed("t", 1, "starting"));
            fixture
                .kernel
                .store
                .write(|tx| {
                    tx.execute(
                        "UPDATE orchestration_projection_metadata SET last_sequence=0",
                        [],
                    )?;
                    Ok(())
                })
                .unwrap();
            let before = snapshot(&fixture.kernel.store);
            fixture.kernel.store.inject_failure(boundary, occurrence);
            assert!(
                fixture.kernel.store.rebuild().is_err(),
                "{boundary:?}:{occurrence}"
            );
            assert_eq!(snapshot(&fixture.kernel.store), before);
            fixture.kernel.store.rebuild().unwrap();
            assert!(fixture.kernel.store.verify_projections().unwrap());
        }
    }
}

#[tokio::test]
async fn effect_timeout_records_process_uncertainty_not_a_second_start() {
    let fixture = Fixture::new();
    enqueue_for_test(
        &fixture.kernel.store,
        "a",
        "timeout",
        &EffectRequest::ProviderTurnStart {
            run_id: RunId("r".into()),
        },
    );
    let executor = Arc::new(BlockingExecutor {
        entered: Notify::new(),
        accepted: Mutex::new(0),
    });
    let mut worker = EffectWorker::new(fixture.kernel.store.clone(), executor, "w".into());
    worker.lease_ms = 5;
    assert!(worker.step(NOW).await.unwrap());
    assert_eq!(
        fixture
            .kernel
            .store
            .effect("timeout")
            .unwrap()
            .unwrap()
            .status,
        EffectStatus::Uncertain
    );
    assert!(!worker.step(NOW + 100).await.unwrap());
}

#[test]
fn native_thread_import_uses_t3_identity_and_never_starts_a_provider() {
    let fixture = Fixture::new();
    let mut command = create_thread("import");
    let Operation::Wire(wire) = &mut command.operation else {
        panic!()
    };
    let mut value = serde_json::to_value(wire.as_ref()).unwrap();
    let instance = value["modelSelection"]["instanceId"]
        .as_str()
        .unwrap()
        .to_string();
    value["importedNativeThread"] =
        json!({"ref":{"driver":"fake","nativeId":"native/1","strength":"strong"}});
    **wire = serde_json::from_value(value).unwrap();
    accept(&fixture.kernel.store, &command);
    assert!(fixture.kernel.store.effects().unwrap().is_empty());
    let projection = fixture
        .kernel
        .store
        .thread(&command.thread_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        projection.thread.active_provider_thread_id.unwrap().0,
        format!(
            "provider-thread:provider:fake:provider-instance:{instance}:native-thread:native%2F1"
        )
    );
    assert!(projection.thread.lineage.parent_thread_id.is_none());
}

#[test]
fn rejected_receipt_failure_does_not_partially_reserve_identity() {
    let fixture = Fixture::new();
    let command = Command {
        id: CommandId("reject".into()),
        thread_id: ThreadId("missing".into()),
        operation: Operation::Recover,
    };
    let before = snapshot(&fixture.kernel.store);
    fixture
        .kernel
        .store
        .inject_failure(WriteBoundary::ReceiptFinalized, 1);
    assert!(fixture.kernel.store.dispatch(&command, NOW).is_err());
    assert_eq!(snapshot(&fixture.kernel.store), before);
    assert_eq!(
        fixture.kernel.store.dispatch(&command, NOW).unwrap().status,
        ReceiptStatus::Rejected
    );
}

#[test]
fn cross_thread_execution_identity_collision_gets_a_stable_rejected_receipt() {
    let fixture = Fixture::new();
    fixture.create("a");
    fixture.create("b");
    let first = seed("a", 1, "starting");
    accept(&fixture.kernel.store, &first);
    let mut second = seed("b", 1, "starting");
    let Operation::CreateExecution(first_seed) = &first.operation else {
        panic!()
    };
    let Operation::CreateExecution(second_seed) = &mut second.operation else {
        panic!()
    };
    second_seed.run.id = first_seed.run.id.clone();
    let before = fixture.kernel.store.events().unwrap();
    let rejected = fixture.kernel.store.dispatch(&second, NOW).unwrap();
    assert_eq!(rejected.status, ReceiptStatus::Rejected);
    assert_eq!(
        fixture.kernel.store.dispatch(&second, NOW + 1).unwrap(),
        rejected
    );
    assert_eq!(fixture.kernel.store.events().unwrap(), before);
    assert_eq!(fixture.kernel.store.effects().unwrap().len(), 1);
}
