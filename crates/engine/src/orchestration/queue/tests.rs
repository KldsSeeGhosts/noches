use std::sync::Arc;

use serde_json::{Value, json};
use zeron_proto::orchestration::*;
use zeron_proto::provider_instance::ProviderInstanceId;
use zeron_proto::{InteractionMode, RuntimeMode};
use zeron_sync::DocsStore;

use super::*;
use crate::orchestration::queue_service::QueueService;

const NOW: i64 = 1_800_000_000_000;

#[tokio::test]
async fn disconnect_fences_cover_reattachment_attempt_and_generation() {
    use crate::orchestration::effects::EffectRequest;
    let fixture = Fixture::new();
    let run = fixture.start_target().await;
    let mut p = fixture
        .service
        .kernel
        .store
        .thread(&run.thread_id)
        .unwrap()
        .unwrap();
    p.records.remove("provider-session");
    let provider = run.provider_thread_id.clone().unwrap();
    let session = format!(
        "provider-session:{}",
        crate::orchestration::event::encode_component(&run.id.0)
    );
    let request = EffectRequest::ProviderSessionDisconnect {
        provider_session_id: session.clone().into(),
        run_id: run.id.clone(),
        run_attempt_id: run.active_attempt_id.clone().unwrap(),
        provider_thread_id: provider.clone(),
    };
    assert!(super::session_control::target_still_disconnected(
        &p, &request
    ));
    p.records
        .insert("provider-session".into(), vec![json!({"id":session})]);
    assert!(!super::session_control::target_still_disconnected(
        &p, &request
    ));
    p.records.remove("provider-session");
    let index = p
        .runs
        .iter()
        .position(|candidate| candidate.id == run.id)
        .unwrap();
    p.runs[index].active_attempt_id = Some("replacement-attempt".into());
    assert!(!super::session_control::target_still_disconnected(
        &p, &request
    ));
    p.runs[index].active_attempt_id = run.active_attempt_id;
    let mut replacement = p.runs[index].clone();
    replacement.id = "replacement-run".into();
    p.runs.push(replacement);
    assert!(!super::session_control::target_still_disconnected(
        &p, &request
    ));
    p.runs.pop();
    let row = p
        .records
        .get_mut("provider-thread")
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == provider.0)
        .unwrap();
    row["id"] = json!("replacement-provider-generation");
    assert!(!super::session_control::target_still_disconnected(
        &p, &request
    ));
}

#[tokio::test]
async fn reset_generation_predicate_marks_only_the_first_run_after_a_closed_conversation() {
    let fixture = Fixture::new();
    let run = fixture.start_target().await;
    let mut p = fixture
        .service
        .kernel
        .store
        .thread(&run.thread_id)
        .unwrap()
        .unwrap();
    let index = p.runs.iter().position(|r| r.id == run.id).unwrap();
    // An ordinary conversation, and the same one before any reset, resumes.
    assert!(!super::session_control::fresh_after_reset(
        &p,
        &p.runs[index]
    ));
    let provider = run.provider_thread_id.clone().unwrap();
    let mut fresh = p.runs[index].clone();
    fresh.id = "after-reset".into();
    fresh.ordinal += 1;
    fresh.provider_thread_id = Some("fresh-generation".into());
    let rows = p.records.get_mut("provider-thread").unwrap();
    let mut closed = rows
        .iter()
        .find(|row| row["id"] == provider.0)
        .unwrap()
        .clone();
    closed["status"] = json!("closed");
    let mut generation = closed.clone();
    generation["id"] = json!("fresh-generation");
    generation["status"] = json!("not_loaded");
    rows.retain(|row| row["id"] != provider.0);
    rows.push(closed);
    rows.push(generation);
    // The conversation a user reset closed never resumes natively...
    assert!(super::session_control::fresh_after_reset(
        &p,
        &p.runs[index]
    ));
    // ...the first turn on its replacement is rebuilt, a later one resumes it.
    p.runs.push(fresh.clone());
    assert!(super::session_control::fresh_after_reset(&p, &fresh));
    let mut later = fresh.clone();
    later.id = "later".into();
    later.ordinal += 1;
    p.runs.push(later.clone());
    assert!(!super::session_control::fresh_after_reset(&p, &later));
}

#[tokio::test]
async fn reset_plan_closes_started_conversations_and_moves_queued_runs_together() {
    use zeron_proto::orchestration::OrchestrationV2RunStatus::{Completed, Queued, Starting};
    let fixture = Fixture::new();
    let run = fixture.start_target().await;
    let mut p = fixture
        .service
        .kernel
        .store
        .thread(&run.thread_id)
        .unwrap()
        .unwrap();
    p.records.remove("provider-session");
    let index = p.runs.iter().position(|r| r.id == run.id).unwrap();
    p.runs[index].status = Completed;
    // Two queued follow-ups bound to the started conversation, one already
    // on its own fresh one: only the first two share a new generation.
    for (id, ordinal) in [("queued-a", 2), ("queued-b", 3)] {
        let mut queued = p.runs[index].clone();
        queued.id = id.into();
        queued.ordinal = ordinal;
        queued.status = Queued;
        queued.active_attempt_id = None;
        p.runs.push(queued);
    }
    let plan_for = |p: &super::ThreadProjection, observed: Option<&str>| {
        let command = Command {
            id: CommandId("reset".into()),
            thread_id: p.thread.id.clone(),
            operation: Operation::Recover,
        };
        let mut plan = crate::orchestration::command::Plan::default();
        let result = fixture.service.kernel.store.read(|conn| {
            super::session_control::plan_reset(
                conn,
                &mut plan,
                &command,
                p,
                &json!({"observedRunId":observed,"providerSessions":[]}),
                NOW,
            )
        });
        (result, plan)
    };
    let (stale, _) = plan_for(&p, Some("another-run"));
    assert!(stale.unwrap_err().to_string().contains("newer turn"));
    let (ok, plan) = plan_for(&p, Some(&run.id.0));
    ok.unwrap();
    let events: Vec<Value> = plan
        .events
        .iter()
        .map(|event| serde_json::to_value(event).unwrap())
        .collect();
    let of = |kind: &str| -> Vec<&Value> { events.iter().filter(|e| e["type"] == kind).collect() };
    let threads = of("provider-thread.updated");
    assert_eq!(
        threads.len(),
        2,
        "one closed conversation, one new generation"
    );
    assert_eq!(threads[0]["payload"]["status"], "closed");
    assert_eq!(
        threads[0]["payload"]["id"],
        run.provider_thread_id.as_ref().unwrap().0
    );
    let fresh = threads[1]["payload"]["id"].as_str().unwrap().to_owned();
    assert_ne!(fresh, run.provider_thread_id.as_ref().unwrap().0);
    assert!(threads[1]["payload"]["nativeThreadRef"].is_null());
    let moved: Vec<_> = of("run.updated")
        .into_iter()
        .map(|e| {
            (
                e["payload"]["id"].clone(),
                e["payload"]["providerThreadId"].clone(),
            )
        })
        .collect();
    assert_eq!(
        moved,
        vec![
            (json!("queued-a"), json!(fresh)),
            (json!("queued-b"), json!(fresh))
        ]
    );
    // The started run keeps its history; nothing else is rewritten.
    assert!(
        of("run.updated")
            .iter()
            .all(|e| e["payload"]["id"] != run.id.0)
    );

    // A turn that is still running refuses, and so does a thread that never
    // started a conversation.
    p.runs[index].status = Starting;
    let (active, _) = plan_for(&p, Some(&run.id.0));
    assert!(
        active
            .unwrap_err()
            .to_string()
            .contains("Stop the current run")
    );
    p.runs[index].status = Completed;
    p.records.remove("provider-thread");
    let (empty, _) = plan_for(&p, Some(&run.id.0));
    assert!(empty.unwrap_err().to_string().contains("no agent session"));
}

struct Fixture {
    _dir: tempfile::TempDir,
    service: QueueDomain,
    caller: CallerScope,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let kernel = Kernel::open(Arc::new(DocsStore::open(dir.path()).unwrap()), "host").unwrap();
        for (id, project) in [
            ("parent", "project"),
            ("target", "project"),
            ("foreign", "other"),
        ] {
            let command=Command::wire(serde_json::from_value(json!({
                "type":"thread.create","commandId":format!("create-{id}"),"threadId":id,"projectId":project,"title":id,
                "createdBy":"user","creationSource":"web","modelSelection":{"instanceId":"mock","model":"mock-1"},
                "runtimeMode":"full-access","interactionMode":"default","branch":"main","worktreePath":dir.path()
            })).unwrap()).unwrap();
            assert_eq!(
                kernel.store.dispatch(&command, NOW).unwrap().status,
                ReceiptStatus::Accepted
            );
        }
        let p = kernel
            .store
            .thread(&ThreadId("parent".into()))
            .unwrap()
            .unwrap();
        let seed =
            task::execution_seed(&p.thread, 1, "parent-input", "starting", "mock", NOW).unwrap();
        let run_id = seed.run.id.clone();
        kernel
            .store
            .dispatch(
                &Command {
                    id: CommandId("seed-parent".into()),
                    thread_id: p.thread.id,
                    operation: Operation::CreateExecution(Box::new(seed)),
                },
                NOW,
            )
            .unwrap();
        let caller = CallerScope {
            thread_id: ThreadId("parent".into()),
            run_id,
            session_id: "session/a:1".into(),
            project_id: ProjectId("project".into()),
            workspace_root: dir.path().into(),
            runtime_mode: RuntimeMode::FullAccess,
            interaction_mode: InteractionMode::Default,
            provider_instance_id: ProviderInstanceId("mock".into()),
        };
        Self {
            _dir: dir,
            service: QueueDomain::new(kernel),
            caller,
        }
    }
    async fn call(&self, name: &str, input: Value) -> std::result::Result<Value, ToolError> {
        self.service.call(self.caller.clone(), name, input).await
    }
    async fn sync(&self, text: &str) -> String {
        let rows = vec![
            zeron_doc::QueuedMessage::new("queue-one", text, "host"),
            zeron_doc::QueuedMessage::new("queue-two", "second", "phone"),
        ];
        let result = self
            .service
            .mutate(
                None,
                ThreadId("target".into()),
                "host.sync_loro_queue",
                json!({"items":rows,"driver":"mock"}),
                CommandId(format!("sync-{}", uuid::Uuid::new_v4())),
                NOW,
            )
            .await
            .unwrap();
        assert_eq!(result.status, ReceiptStatus::Accepted, "{:?}", result.error);
        self.call("t3_queue_list", json!({"threadId":"target"}))
            .await
            .unwrap()["items"][0]["queuedRunId"]
            .as_str()
            .unwrap()
            .into()
    }

    fn seed(&self, thread: &str, event: &str, payload: Value) {
        self.service
            .kernel
            .store
            .write(|tx| {
                self.service.kernel.store.append_event(
                    tx,
                    None,
                    super::super::event::make(
                        EventId(format!("test:{}", uuid::Uuid::new_v4())),
                        &ThreadId(thread.into()),
                        event,
                        &payload,
                        NOW,
                    )?,
                )?;
                Ok(())
            })
            .unwrap();
    }

    fn request(&self, id: &str, kind: &str, capability: Value) {
        self.seed("target","runtime-request.updated",json!({"id":id,"nodeId":"request-node",
            "providerTurnId":null,"nativeRequestRef":null,"kind":kind,"status":"pending",
            "responseCapability":capability,"createdAt":super::super::event::iso(NOW).unwrap(),"resolvedAt":null}));
        if kind == "user_input" {
            self.seed("target","turn-item.updated",json!({"id":format!("item:{id}"),"threadId":"target",
                "runId":null,"nodeId":null,"providerThreadId":null,"providerTurnId":null,
                "nativeItemRef":null,"parentItemId":null,"ordinal":1,"status":"running","title":null,
                "startedAt":null,"completedAt":null,"updatedAt":super::super::event::iso(NOW).unwrap(),
                "type":"user_input_request","requestId":id,
                "questions":[{"id":"answer","header":"Choice","question":"Which one?","options":[],
                    "allowCustomAnswer":true,"required":true}]}));
        }
    }

    async fn start_target(&self) -> OrchestrationV2Run {
        let thread = ThreadId("target".into());
        let p = self.service.kernel.store.thread(&thread).unwrap().unwrap();
        let seed = task::execution_seed(&p.thread, 1, "active", "starting", "mock", NOW).unwrap();
        let run = seed.run.clone();
        self.service
            .kernel
            .dispatch(
                &Command {
                    id: CommandId("start-target".into()),
                    thread_id: thread.clone(),
                    operation: Operation::CreateExecution(Box::new(seed)),
                },
                NOW,
            )
            .await
            .unwrap();
        let mut capabilities: Value = serde_json::from_str::<Value>(include_str!(
            "../../../../proto/tests/t3_oracle/fixtures/serde-cases.json"
        ))
        .unwrap()["OrchestrationV2ProviderCapabilities"][0]
            .clone();
        capabilities["turns"]["supportsActiveSteering"] = json!(true);
        self.service
            .kernel
            .task_command(
                &thread,
                CommandId("session-start".into()),
                super::super::task::TaskOperation::RunnerEvent {
                    run_id: run.id.clone(),
                    attempt_id: run.active_attempt_id.clone().unwrap(),
                    event: zeron_proto::AgentEvent::SessionStarted {
                        instance_id: None,
                        session_id: "native-target".into(),
                        harness: zeron_proto::HarnessId::Mock,
                        model: "mock-1".into(),
                        tools: vec![],
                        cwd: "/fixture".into(),
                        assistant_message_id: "assistant".into(),
                    },
                    capabilities: Some(Box::new(serde_json::from_value(capabilities).unwrap())),
                },
            )
            .await
            .unwrap();
        self.service
            .kernel
            .task_command(
                &thread,
                CommandId("input-accepted".into()),
                super::super::task::TaskOperation::RunnerEvent {
                    run_id: run.id.clone(),
                    attempt_id: run.active_attempt_id.clone().unwrap(),
                    event: zeron_proto::AgentEvent::InputAccepted,
                    capabilities: None,
                },
            )
            .await
            .unwrap();
        run
    }
}

fn host_for_repair(f: &Fixture) -> super::host::HostQueue {
    super::host::HostQueue {
        domain: Arc::new(QueueDomain::new(f.service.kernel.clone())),
        docs: crate::DocHost::new(
            f.service.kernel.store.docs.clone(),
            crate::DocHostConfig {
                device_id: "host".into(),
                default_harness: zeron_proto::HarnessId::Mock,
                edge: None,
            },
        ),
        registry: Arc::new(crate::HarnessRegistry::new()),
    }
}

#[tokio::test]
async fn idle_queue_repair_compares_intents_before_loading_retained_history() {
    let f = Fixture::new();
    let host = host_for_repair(&f);
    let handle = host.docs.open("target").unwrap();
    let store = &f.service.kernel.store;
    let before = store.access_counts();
    host.repair_all().await.unwrap();
    let after = store.access_counts();
    assert_eq!(
        after[0] - before[0],
        2,
        "patch IDs and the small intent row only"
    );
    assert_eq!(after[1], before[1], "unchanged queues must not dispatch");

    // A real Loro change still enters the authority/reconciliation path.
    handle
        .doc()
        .push_queued(&zeron_doc::QueuedMessage::new("new", "new prompt", "host"))
        .unwrap();
    host.repair_all().await.unwrap();
    let p = store.thread(&ThreadId("target".into())).unwrap().unwrap();
    assert_eq!(p.runs.len(), 1);
    let before = store.access_counts();
    host.repair_all().await.unwrap();
    assert_eq!(store.access_counts(), [before[0] + 2, before[1]]);
}

#[tokio::test]
async fn idle_empty_legacy_queue_does_not_trigger_passive_admission() {
    let f = Fixture::new();
    let host = host_for_repair(&f);
    host.docs.open("legacy").unwrap();
    host.repair_all().await.unwrap();
    assert!(
        f.service
            .kernel
            .store
            .thread(&ThreadId("legacy".into()))
            .unwrap()
            .is_none()
    );
    assert!(!f.service.kernel.store.is_v2_managed("legacy").unwrap());
}

#[tokio::test]
#[ignore = "comparative optimized-test benchmark; run without concurrent builds"]
async fn profile_unchanged_queue_repair() {
    let f = Fixture::new();
    let host = host_for_repair(&f);
    let handle = host.docs.open("target").unwrap();
    let store = &f.service.kernel.store;
    let raw = serde_json::to_string(&json!({"text": "x".repeat(2_160)})).unwrap();
    store
        .write(|tx| {
            let mut insert = tx.prepare(
                "INSERT INTO orchestration_projection_records
                 (kind,id,thread_id,last_sequence,payload_json) VALUES('message',?1,'target',0,?2)",
            )?;
            for index in 0..10_000 {
                insert.execute(rusqlite::params![format!("retained-{index}"), raw])?;
            }
            Ok(())
        })
        .unwrap();
    // Reference the former repair order against the SAME store/indexes:
    // full projection, lock, then equality check. No mutation in either path.
    let start = std::time::Instant::now();
    for _ in 0..50 {
        let p = store.thread(&ThreadId("target".into())).unwrap().unwrap();
        let _guard = handle.orchestration_queue_lock().await;
        host.synchronize(&p, &handle).await.unwrap();
    }
    let legacy_us = start.elapsed().as_micros();
    let start = std::time::Instant::now();
    for _ in 0..50 {
        host.repair_all().await.unwrap();
    }
    let equality_first_us = start.elapsed().as_micros();
    eprintln!(
        "queue_repair_profile retained_records=10000 history_bytes=21600000 \
         repairs=50 legacy_full_projection_us={legacy_us} equality_first_us={equality_first_us}"
    );
}

#[tokio::test]
async fn unicode_code_points_pages_and_stale_mutations() {
    let f = Fixture::new();
    let id = f.sync(&"😀".repeat(16001)).await;
    let list = f
        .call("t3_queue_list", json!({"threadId":"target","limit":1}))
        .await
        .unwrap();
    assert_eq!(
        list["items"][0]["text"].as_str().unwrap().chars().count(),
        1000
    );
    assert_eq!(list["nextCursor"], 1);
    let read = f
        .call(
            "t3_queue_read",
            json!({"threadId":"target","queuedRunId":id}),
        )
        .await
        .unwrap();
    assert_eq!(read["text"].as_str().unwrap().chars().count(), 16000);
    assert_eq!(read["truncated"], true);
    assert!(
        f.call(
            "t3_queue_edit",
            json!({"threadId":"target","queuedRunId":id,"text":"\u{feff} \t"})
        )
        .await
        .is_err()
    );
    f.call(
        "t3_queue_cancel",
        json!({"threadId":"target","queuedRunId":id}),
    )
    .await
    .unwrap();
    let error = f
        .call(
            "t3_queue_edit",
            json!({"threadId":"target","queuedRunId":id,"text":"changed"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.message, "The operation could not be completed.");
    let error = f
        .call(
            "t3_queue_read",
            json!({"threadId":"target","queuedRunId":id}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.message, "The queued message was not found.");
}

#[tokio::test]
async fn reorder_self_refuses_and_target_is_revalidated_without_interrupt() {
    let f = Fixture::new();
    let id = f.sync("first").await;
    assert!(
        f.call(
            "t3_queue_reorder",
            json!({"threadId":"target","queuedRunId":id,"beforeRunId":id})
        )
        .await
        .is_err()
    );
    let frontier = f.service.kernel.store.projection_frontier().unwrap();
    assert!(
        f.call(
            "t3_queue_promote_to_steer",
            json!({"threadId":"target","queuedRunId":id,"targetRunId":"stale"})
        )
        .await
        .is_err()
    );
    assert_eq!(
        f.service.kernel.store.projection_frontier().unwrap(),
        frontier
    );
    assert_eq!(
        f.call(
            "t3_queue_read",
            json!({"threadId":"target","queuedRunId":id})
        )
        .await
        .unwrap()["text"],
        "first"
    );
}

#[tokio::test]
async fn metadata_idempotency_scope_and_project_access() {
    let f = Fixture::new();
    let input =
        json!({"threadId":"target","action":"rename","title":"First","clientRequestId":"key"});
    let first = f.call("t3_thread_update", input).await.unwrap();
    let replay=f.call("t3_thread_update",json!({"threadId":"target","action":"rename","title":"Changed","clientRequestId":"key"})).await.unwrap();
    assert_eq!(first, replay);
    assert_eq!(first["title"], "First");
    assert!(
        first["commandId"]
            .as_str()
            .unwrap()
            .contains("session%2Fa%3A1:thread-update:target:rename:key")
    );
    let error = f
        .call("t3_queue_list", json!({"threadId":"foreign"}))
        .await
        .unwrap_err();
    assert_eq!(
        error.message,
        "The thread was not found in the calling project."
    );
}

#[tokio::test]
async fn lifecycle_due_wake_pin_promotion_and_ack_are_durable() {
    let f = Fixture::new();
    let snooze = json!({"threadId":"target","action":"snooze","snoozedUntil":super::super::event::iso(NOW+1000).unwrap()});
    let r = f
        .service
        .mutate(
            Some(f.caller.clone()),
            ThreadId("target".into()),
            "t3_thread_organize",
            snooze,
            CommandId("snooze".into()),
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(r.status, ReceiptStatus::Accepted, "{:?}", r.error);
    assert_eq!(f.service.wake_due(NOW + 999).await.unwrap(), 0);
    assert_eq!(f.service.wake_due(NOW + 1000).await.unwrap(), 1);
    let state = f
        .service
        .kernel
        .store
        .queue_ui_state(&ThreadId("target".into()))
        .unwrap();
    assert_eq!(
        state.lifecycle.woke_at.unwrap().timestamp_millis(),
        NOW + 1000
    );
    f.service
        .mutate(
            None,
            ThreadId("target".into()),
            "host.acknowledge_woke",
            json!({}),
            CommandId("ack".into()),
            NOW + 1001,
        )
        .await
        .unwrap();
    assert!(
        f.service
            .kernel
            .store
            .queue_ui_state(&ThreadId("target".into()))
            .unwrap()
            .lifecycle
            .woke_at
            .is_none()
    );
    f.call(
        "t3_thread_organize",
        json!({"threadId":"target","action":"settle"}),
    )
    .await
    .unwrap();
    let settled = f
        .service
        .kernel
        .store
        .queue_ui_state(&ThreadId("target".into()))
        .unwrap()
        .lifecycle;
    assert!(settled.settled_at.is_some());
    assert_eq!(settled.settled_by, Some(zeron_proto::SettleSource::User));
    f.call(
        "t3_thread_organize",
        json!({"threadId":"target","action":"pin"}),
    )
    .await
    .unwrap();
    let state = f
        .service
        .kernel
        .store
        .queue_ui_state(&ThreadId("target".into()))
        .unwrap();
    assert!(state.lifecycle.pinned_at.is_some());
    assert!(state.lifecycle.settled_at.is_none());
}

#[tokio::test]
async fn active_and_queued_work_are_never_hidden() {
    let f = Fixture::new();
    let error = f
        .call("t3_thread_organize", json!({"action":"settle"}))
        .await
        .unwrap_err();
    assert_eq!(error.message, "The operation could not be completed.");
    f.call(
        "t3_thread_organize",
        json!({"action":"snooze","snoozedUntil":"2099-01-01T00:00:00.000Z"}),
    )
    .await
    .unwrap();
    assert!(
        f.service
            .kernel
            .store
            .queue_ui_state(&f.caller.thread_id)
            .unwrap()
            .lifecycle
            .snoozed_until
            .is_none()
    );
    f.sync("queued").await;
    assert!(
        f.call(
            "t3_thread_organize",
            json!({"threadId":"target","action":"settle"})
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn only_questions_are_answerable_and_async_validation_rolls_back() {
    let f = Fixture::new();
    f.request(
        "approval",
        "command",
        json!({"type":"not_resumable","reason":"expired callback"}),
    );
    f.request("question", "user_input", json!({"type":"message"}));
    assert_eq!(
        f.call("t3_pending_request_list", json!({"threadId":"target"}))
            .await
            .unwrap(),
        json!({"requestIds":["question"]})
    );
    for name in ["t3_pending_request_read", "t3_pending_request_respond"] {
        let error = f
            .call(
                name,
                json!({"threadId":"target","requestId":"approval","answers":{}}),
            )
            .await
            .unwrap_err();
        assert_eq!(
            error.message,
            "The pending user-input request was not found."
        );
    }
    let frontier = f.service.kernel.store.projection_frontier().unwrap();
    assert!(f.call("t3_pending_request_respond",json!({"threadId":"target","requestId":"question","answers":{"answer":["not a string"]}})).await.is_err());
    assert_eq!(
        f.service.kernel.store.projection_frontier().unwrap(),
        frontier
    );
    let result = f
        .call(
            "t3_pending_request_respond",
            json!({"threadId":"target","requestId":"question","answers":{"answer":"  first  "}}),
        )
        .await
        .unwrap();
    assert!(result["sequence"].as_i64().unwrap() > frontier);
    assert_eq!(
        f.call("t3_pending_request_list", json!({"threadId":"target"}))
            .await
            .unwrap()["requestIds"],
        json!([])
    );
    let p = f
        .service
        .kernel
        .store
        .thread(&ThreadId("target".into()))
        .unwrap()
        .unwrap();
    let message = task::records(&p, "message")
        .iter()
        .find(|m| m["id"] == "async-answer:question")
        .unwrap();
    assert_eq!(message["text"], "Which one?\nfirst");
}

#[tokio::test]
async fn nonresumable_question_is_readable_but_cannot_be_answered() {
    let f = Fixture::new();
    f.request(
        "question",
        "user_input",
        json!({"type":"not_resumable","reason":"callback expired"}),
    );
    assert!(
        f.call(
            "t3_pending_request_read",
            json!({"threadId":"target","requestId":"question"})
        )
        .await
        .is_ok()
    );
    let frontier = f.service.kernel.store.projection_frontier().unwrap();
    let error = f
        .call(
            "t3_pending_request_respond",
            json!({"threadId":"target","requestId":"question","answers":{"answer":"yes"}}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.message, "The operation could not be completed.");
    assert_eq!(
        f.service.kernel.store.projection_frontier().unwrap(),
        frontier
    );
}

#[tokio::test]
async fn pre_attachment_runtime_requests_do_not_brick_observation_or_invent_authority() {
    use crate::orchestration::task::TaskOperation;
    use zeron_proto::{AgentEvent, DoneStatus, HarnessId, PermissionRequest, UserInputQuestion};

    for approval in [false, true] {
        for attach in [false, true] {
            let f = Fixture::new();
            let thread = &f.caller.thread_id;
            let p = f.service.kernel.store.thread(thread).unwrap().unwrap();
            let run = &p.runs[0];
            let capabilities: Value = serde_json::from_str::<Value>(include_str!(
                "../../../../proto/tests/t3_oracle/fixtures/serde-cases.json"
            ))
            .unwrap()["OrchestrationV2ProviderCapabilities"][0]
                .clone();
            let mut permission = PermissionRequest::standard("Exec", "cargo test", true);
            permission.id = "early".into();
            let incoming = if approval {
                AgentEvent::PermissionRequested {
                    request: permission,
                }
            } else {
                AgentEvent::InputRequested {
                    request_id: "early".into(),
                    questions: vec![UserInputQuestion {
                        id: "q".into(),
                        header: "Question".into(),
                        question: "Proceed?".into(),
                        options: vec!["yes".into()],
                        multi_select: false,
                    }],
                }
            };
            for (index, event) in [
                Some(incoming),
                attach.then(|| AgentEvent::SessionStarted {
                    instance_id: None,
                    session_id: "real-native".into(),
                    harness: HarnessId::Mock,
                    model: "mock-1".into(),
                    tools: vec![],
                    cwd: "/fixture".into(),
                    assistant_message_id: "assistant".into(),
                }),
                Some(AgentEvent::Done {
                    status: DoneStatus::Interrupted,
                    result: None,
                    error: None,
                    session_id: None,
                }),
            ]
            .into_iter()
            .enumerate()
            {
                let Some(event) = event else { continue };
                f.service
                    .kernel
                    .task_command(
                        thread,
                        CommandId(format!("early-event:{index}")),
                        TaskOperation::RunnerEvent {
                            run_id: run.id.clone(),
                            attempt_id: run.active_attempt_id.clone().unwrap(),
                            event,
                            capabilities: Some(Box::new(
                                serde_json::from_value(capabilities.clone()).unwrap(),
                            )),
                        },
                    )
                    .await
                    .unwrap();
                let current = f.service.kernel.store.thread(thread).unwrap().unwrap();
                let request = &current.records["runtime-request"][0];
                if index == 0 {
                    assert_eq!(current.runs[0].status, OrchestrationV2RunStatus::Waiting);
                    assert_eq!(
                        request["responseCapability"]["type"], "not_resumable",
                        "a callback with no native binding cannot manufacture live authority"
                    );
                    assert!(current.records["provider-session"].is_empty());
                } else if index == 1 {
                    assert_eq!(current.runs[0].status, OrchestrationV2RunStatus::Waiting);
                    assert_eq!(request["responseCapability"]["type"], "live");
                    assert_eq!(
                        request["responseCapability"]["providerSessionId"],
                        current.records["provider-session"][0]["id"]
                    );
                } else {
                    assert_eq!(
                        current.runs[0].status,
                        OrchestrationV2RunStatus::Interrupted
                    );
                    assert_eq!(request["status"], "expired");
                }
                assert!(
                    current.records["provider-turn"].is_empty(),
                    "a callback, attachment or interruption is not root-input acceptance"
                );
                assert!(current.attempts[0].provider_turn_id.is_none());
            }
            // A stale late attachment cannot make an expired request live.
            let receipt = f
                .service
                .kernel
                .task_command(
                    thread,
                    CommandId("late-attachment".into()),
                    TaskOperation::RunnerEvent {
                        run_id: run.id.clone(),
                        attempt_id: run.active_attempt_id.clone().unwrap(),
                        event: AgentEvent::SessionStarted {
                            instance_id: None,
                            session_id: "stale-native".into(),
                            harness: HarnessId::Mock,
                            model: "mock-1".into(),
                            tools: vec![],
                            cwd: "/fixture".into(),
                            assistant_message_id: "late-assistant".into(),
                        },
                        capabilities: Some(Box::new(
                            serde_json::from_value(capabilities.clone()).unwrap(),
                        )),
                    },
                )
                .await;
            receipt.unwrap();
            let current = f.service.kernel.store.thread(thread).unwrap().unwrap();
            assert_eq!(current.records["runtime-request"][0]["status"], "expired");
            assert_eq!(
                current.runs[0].status,
                OrchestrationV2RunStatus::Interrupted
            );
        }
    }
}

#[tokio::test]
async fn live_question_dispatch_and_runtime_projection_preserve_capability() {
    let f = Fixture::new();
    let p = f
        .service
        .kernel
        .store
        .thread(&f.caller.thread_id)
        .unwrap()
        .unwrap();
    let capabilities: Value = serde_json::from_str::<Value>(include_str!(
        "../../../../proto/tests/t3_oracle/fixtures/serde-cases.json"
    ))
    .unwrap()["OrchestrationV2ProviderCapabilities"][0]
        .clone();
    for event in [
        zeron_proto::AgentEvent::SessionStarted {
            instance_id: None,
            session_id: "native".into(),
            harness: zeron_proto::HarnessId::Mock,
            model: "mock-1".into(),
            tools: vec![],
            cwd: "/fixture".into(),
            assistant_message_id: "assistant".into(),
        },
        zeron_proto::AgentEvent::InputRequested {
            request_id: "live".into(),
            questions: vec![zeron_proto::UserInputQuestion {
                id: "q".into(),
                header: "Question".into(),
                question: "Proceed?".into(),
                options: vec!["yes".into()],
                multi_select: false,
            }],
        },
    ] {
        f.service
            .kernel
            .task_command(
                &f.caller.thread_id,
                CommandId(format!("event:{}", uuid::Uuid::new_v4())),
                super::super::task::TaskOperation::RunnerEvent {
                    run_id: p.runs[0].id.clone(),
                    attempt_id: p.runs[0].active_attempt_id.clone().unwrap(),
                    event,
                    capabilities: Some(Box::new(
                        serde_json::from_value(capabilities.clone()).unwrap(),
                    )),
                },
            )
            .await
            .unwrap();
    }
    let read = f
        .call("t3_pending_request_read", json!({"requestId":"live"}))
        .await
        .unwrap();
    assert_eq!(
        read["questions"][0]["options"][0],
        json!({"label":"yes","description":""})
    );
    f.call(
        "t3_pending_request_respond",
        json!({"requestId":"live","answers":{"q":"yes"}}),
    )
    .await
    .unwrap();
    let ui = f
        .service
        .kernel
        .store
        .queue_ui_state(&f.caller.thread_id)
        .unwrap();
    assert!(ui.pending_questions.is_empty());
}

#[tokio::test]
async fn search_global_top_matches_filter_after_limit_and_escape_wildcards() {
    let f = Fixture::new();
    for (thread, id, text, role) in [
        ("foreign", "foreign-user", "Needle 100%_literal", "user"),
        ("target", "target-assistant", "Needle other", "assistant"),
    ] {
        f.seed(
            thread,
            "message.updated",
            task::message(&ThreadId(thread.into()), None, None, id, text, role, NOW).unwrap(),
        );
    }
    assert_eq!(
        f.call("t3_thread_search", json!({"query":"needle","limit":1}))
            .await
            .unwrap()["matches"],
        json!([])
    );
    let all = f
        .call("t3_thread_search", json!({"query":"needle","limit":50}))
        .await
        .unwrap();
    assert_eq!(all["matches"].as_array().unwrap().len(), 1);
    assert_eq!(all["matches"][0]["threadId"], "target");
    assert_eq!(
        f.call("t3_thread_search", json!({"query":"%_","limit":50}))
            .await
            .unwrap()["matches"],
        json!([])
    );
    assert!(
        f.call("t3_thread_search", json!({"query":"x"}))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn owner_queue_edits_preserve_paths_and_respect_remote_edit_leases() {
    let f = Fixture::new();
    let docs = crate::DocHost::new(
        f.service.kernel.store.docs.clone(),
        crate::DocHostConfig {
            device_id: "host".into(),
            default_harness: zeron_proto::HarnessId::Mock,
            edge: None,
        },
    );
    let handle = docs.open("target").unwrap();
    let mut row = zeron_doc::QueuedMessage::new("attached", "first", "phone");
    row.attachments = vec!["/fixture/upload.png".into()];
    row.hold_for_turn_end = true;
    handle.doc().push_queued(&row).unwrap();
    let registry = Arc::new(crate::HarnessRegistry::new());
    let host = super::host::HostQueue {
        domain: Arc::new(QueueDomain::new(f.service.kernel.clone())),
        docs: docs.clone(),
        registry,
    };
    let list = host
        .call(
            f.caller.clone(),
            "t3_queue_list",
            json!({"threadId":"target"}),
        )
        .await
        .unwrap();
    let id = list["items"][0]["queuedRunId"].as_str().unwrap();
    let lease = docs
        .begin_queued_message_edit("target", "attached", "phone", "other-instance")
        .await
        .unwrap();
    assert!(matches!(
        lease,
        crate::doc_host::BeginQueueEditOutcome::Acquired { .. }
    ));
    assert!(
        host.call(
            f.caller.clone(),
            "t3_queue_edit",
            json!({"threadId":"target","queuedRunId":id,"text":"changed"})
        )
        .await
        .is_err()
    );
    assert_eq!(handle.doc().read_queue().unwrap()[0].text, "first");
    handle
        .doc()
        .set_queued_delivery_gate("attached", None)
        .unwrap();
    host.call(
        f.caller.clone(),
        "t3_queue_edit",
        json!({"threadId":"target","queuedRunId":id,"text":"changed"}),
    )
    .await
    .unwrap();
    let row = handle.doc().read_queue().unwrap().remove(0);
    assert_eq!(row.text, "changed");
    assert_eq!(row.attachments, vec!["/fixture/upload.png"]);
    assert!(row.hold_for_turn_end);
    // Both device edits still refer to one canonical run/message.
    let p = f
        .service
        .kernel
        .store
        .thread(&ThreadId("target".into()))
        .unwrap()
        .unwrap();
    assert_eq!(p.runs.len(), 1);
}

#[tokio::test]
async fn loro_queue_has_one_drainer_and_cancelled_intents_do_not_resurrect() {
    let f = Fixture::new();
    f.sync("first").await;
    f.service
        .kernel
        .task_command(
            &ThreadId("target".into()),
            CommandId("drain".into()),
            super::super::task::TaskOperation::DrainQueue,
        )
        .await
        .unwrap();
    let p = f
        .service
        .kernel
        .store
        .thread(&ThreadId("target".into()))
        .unwrap()
        .unwrap();
    assert!(
        p.runs
            .iter()
            .all(|r| r.status == OrchestrationV2RunStatus::Queued)
    );
    let result = f
        .service
        .mutate(
            None,
            ThreadId("target".into()),
            "host.sync_loro_queue",
            json!({"items":[],"driver":"mock"}),
            CommandId("remove-intents".into()),
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(result.status, ReceiptStatus::Accepted);
    assert_eq!(
        f.call("t3_queue_list", json!({"threadId":"target"}))
            .await
            .unwrap()["items"],
        json!([])
    );
}

#[tokio::test]
async fn successful_promotion_preserves_attachments_and_only_enqueues_steering() {
    let f = Fixture::new();
    let active = f.start_target().await;
    assert!(super::can_promote_to_steer(
        &f.service
            .kernel
            .store
            .thread(&"target".into())
            .unwrap()
            .unwrap()
    ));
    let queued = f.sync("steering").await;
    let mut intents = f
        .service
        .kernel
        .store
        .read(|conn| crate::orchestration::ui_queue::loro_intents(conn, &ThreadId("target".into())))
        .unwrap();
    intents[0].attachments = vec!["/fixture/promoted.png".into()];
    f.service
        .mutate(
            None,
            ThreadId("target".into()),
            "host.sync_loro_queue",
            json!({"items":intents,"driver":"mock"}),
            CommandId("promotion-paths".into()),
            NOW,
        )
        .await
        .unwrap();
    let p = f
        .service
        .kernel
        .store
        .thread(&ThreadId("target".into()))
        .unwrap()
        .unwrap();
    let mut message = task::records(&p, "message")
        .iter()
        .find(|m| m["id"] == "queue-one")
        .unwrap()
        .clone();
    message["attachments"] = json!([{"type":"image","id":"upload","name":"image.png","mimeType":"image/png","sizeBytes":12}]);
    f.seed("target", "message.updated", message.clone());
    let frontier = f.service.kernel.store.projection_frontier().unwrap();
    f.call(
        "t3_queue_promote_to_steer",
        json!({"threadId":"target","queuedRunId":queued,"targetRunId":active.id}),
    )
    .await
    .unwrap();
    let p = f
        .service
        .kernel
        .store
        .thread(&ThreadId("target".into()))
        .unwrap()
        .unwrap();
    assert_eq!(
        p.runs.iter().find(|r| r.id.0 == queued).unwrap().status,
        OrchestrationV2RunStatus::Cancelled
    );
    let promoted = task::records(&p, "message")
        .iter()
        .find(|m| m["id"] == "queue-one")
        .unwrap();
    assert_eq!(promoted["runId"], active.id.0);
    assert_eq!(promoted["attachments"], message["attachments"]);
    let item = task::records(&p, "turn-item")
        .iter()
        .find(|i| i["messageId"] == "queue-one")
        .unwrap();
    assert_eq!(item["inputIntent"], "promoted_queued_to_steer");
    let events: Vec<_> = f
        .service
        .kernel
        .store
        .events()
        .unwrap()
        .into_iter()
        .filter(|e| e.sequence > frontier)
        .map(|e| {
            serde_json::to_value(e.event).unwrap()["type"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(
        events,
        vec![
            "run.updated",
            "run-attempt.updated",
            "node.updated",
            "message.updated",
            "turn-item.updated"
        ]
    );
    let effects=f.service.kernel.store.read(|conn|{
        let mut statement=conn.prepare("SELECT effect_type FROM orchestration_effect_outbox WHERE thread_id='target' ORDER BY ordinal")?;
        let rows=statement.query_map([],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(rows)
    }).unwrap();
    assert_eq!(effects, vec!["provider-turn.start", "provider-turn.steer"]);
    let promoted_effect = f
        .service
        .kernel
        .store
        .effects()
        .unwrap()
        .into_iter()
        .find(|effect| {
            matches!(&effect.request, crate::orchestration::effects::EffectRequest::ProviderTurnSteer { message_id, .. } if message_id.0 == "queue-one")
        })
        .unwrap();
    assert!(super::effects::is_promotion(&f.service.kernel.store, &promoted_effect).unwrap());
    // The same wire effect from ordinary ThreadService send must still use
    // that slice's late-steer recovery, never the strict promotion adapter.
    let receipt = f
        .service
        .kernel
        .dispatch(
            &Command {
                id: "ordinary-steer".into(),
                thread_id: "target".into(),
                operation: Operation::Thread(Box::new(
                    crate::orchestration::threads::planner::ThreadOperation::Send(
                        crate::orchestration::threads::planner::Send {
                            message_id: "ordinary-steering-message".into(),
                            text: "ordinary".into(),
                            mode: zeron_proto::orchestration_mcp::T3ThreadSendInputMode::Steer,
                            driver: "mock".into(),
                            sender: "parent".into(),
                            target_run: Some(active.id.clone()),
                            metadata: None,
                        },
                    ),
                )),
            },
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(
        receipt.status,
        ReceiptStatus::Accepted,
        "{:?}",
        receipt.error
    );
    let ordinary_effect = f
        .service
        .kernel
        .store
        .effects()
        .unwrap()
        .into_iter()
        .find(|effect| effect.command_id.0 == "ordinary-steer")
        .unwrap();
    assert!(!super::effects::is_promotion(&f.service.kernel.store, &ordinary_effect).unwrap());
    // The host can remove/sync the promoted intent before a delayed effect.
    f.service
        .mutate(
            None,
            ThreadId("target".into()),
            "host.sync_loro_queue",
            json!({"items":[],"driver":"mock"}),
            CommandId("promotion-intents-consumed".into()),
            NOW,
        )
        .await
        .unwrap();
    let paths = f
        .service
        .kernel
        .store
        .read(|conn| {
            crate::orchestration::ui_queue::attachment_paths(
                conn,
                &ThreadId("target".into()),
                "queue-one",
            )
        })
        .unwrap();
    assert_eq!(paths, vec!["/fixture/promoted.png"]);
}

fn set_promotion_capabilities(f: &Fixture, active: bool, interrupt: bool, restart: bool) {
    let p = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let mut session = task::records(&p, "provider-session")[0].clone();
    session["capabilities"]["turns"]["supportsActiveSteering"] = json!(active);
    session["capabilities"]["turns"]["supportsInterrupt"] = json!(interrupt);
    session["capabilities"]["turns"]["supportsSteeringByInterruptRestart"] = json!(restart);
    f.seed("target", "provider-session.attached", session);
}

#[tokio::test]
async fn queued_promotion_restarts_one_logical_run_with_original_message_and_exact_control_target()
{
    use crate::orchestration::{effects::EffectRequest, steering::RuntimeTarget};
    use zeron_proto::QueuePromotionMode;
    let f = Fixture::new();
    let active = f.start_target().await;
    set_promotion_capabilities(&f, false, true, true);
    let queued = f.sync("restart direction").await;
    let before = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let runtime = RuntimeTarget::for_run(&before.runs[0]).unwrap();
    f.service
        .kernel
        .store
        .write(|conn| {
            crate::orchestration::steering::bind_runtime(
                conn,
                &"target".into(),
                &runtime,
                "original-process",
            )
        })
        .unwrap();
    let hint = f
        .service
        .kernel
        .store
        .queue_ui_state(&"target".into())
        .unwrap();
    assert_eq!(
        hint.promotion_mode,
        Some(QueuePromotionMode::InterruptRestart)
    );
    assert!(
        !hint.can_promote_to_steer,
        "older UIs must not call an interrupting action Steer"
    );
    let mut message = task::records(&before, "message")
        .iter()
        .find(|m| m["id"] == "queue-one")
        .unwrap()
        .clone();
    message["attachments"] = json!([{"type":"file","id":"upload","name":"notes.txt","mimeType":"text/plain","sizeBytes":12}]);
    message["context"] = json!({"version":1,"records":[{"version":1,"contextId":"attached",
        "label":"Context","kind":"thread","environmentId":"host","threadId":"parent","title":"Parent"}]});
    message["scheduledTaskId"] = json!("schedule");
    message["senderThreadId"] = json!("parent");
    f.seed("target", "message.updated", message.clone());
    let input = json!({"queuedRunId":queued,"targetRunId":active.id,"expectedExecution":"interrupt_restart"});
    let id: CommandId = "promote-restart-once".into();
    let first = f
        .service
        .mutate(
            None,
            "target".into(),
            "t3_queue_promote_to_steer",
            input.clone(),
            id.clone(),
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(first.status, ReceiptStatus::Accepted, "{:?}", first.error);
    let after = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    assert_eq!(
        after.runs.len(),
        before.runs.len(),
        "promotion must not create another logical run"
    );
    let restarted = after.runs.iter().find(|r| r.id == active.id).unwrap();
    assert_eq!(restarted.status, OrchestrationV2RunStatus::Starting);
    assert_eq!(restarted.user_message_id.0, "queue-one");
    assert_ne!(restarted.active_attempt_id, active.active_attempt_id);
    let new_attempt = after
        .attempts
        .iter()
        .find(|a| Some(&a.id) == restarted.active_attempt_id.as_ref())
        .unwrap();
    assert_eq!(
        new_attempt.reason,
        OrchestrationV2RunAttemptReason::SteeringRestart
    );
    assert_eq!(new_attempt.status, OrchestrationV2RunAttemptStatus::Pending);
    assert_eq!(new_attempt.attempt_ordinal, 2);
    assert_eq!(
        after
            .attempts
            .iter()
            .find(|a| Some(&a.id) == active.active_attempt_id.as_ref())
            .unwrap()
            .status,
        OrchestrationV2RunAttemptStatus::Superseded
    );
    assert_eq!(
        after
            .nodes
            .iter()
            .find(|n| Some(&n.id) == active.root_node_id.as_ref())
            .unwrap()
            .status,
        OrchestrationV2ExecutionNodeStatus::Interrupted
    );
    assert_eq!(
        after.runs.iter().find(|r| r.id.0 == queued).unwrap().status,
        OrchestrationV2RunStatus::Cancelled
    );
    let other = after
        .runs
        .iter()
        .find(|r| r.user_message_id.0 == "queue-two")
        .unwrap();
    assert_eq!(other.status, OrchestrationV2RunStatus::Queued);
    let promoted = task::records(&after, "message")
        .iter()
        .find(|m| m["id"] == "queue-one")
        .unwrap();
    assert_eq!(promoted["runId"], active.id.0);
    assert_eq!(promoted["nodeId"], json!(restarted.root_node_id));
    for field in [
        "text",
        "attachments",
        "context",
        "createdBy",
        "creationSource",
        "scheduledTaskId",
        "senderThreadId",
    ] {
        assert_eq!(promoted[field], message[field], "{field}");
    }
    let item = task::records(&after, "turn-item")
        .iter()
        .find(|i| i["messageId"] == "queue-one")
        .unwrap();
    assert_eq!(item["inputIntent"], "promoted_queued_to_steer");
    assert_eq!(item["nodeId"], json!(restarted.root_node_id));
    assert!(
        item["providerTurnId"].is_null(),
        "the new attempt has not accepted input"
    );
    let effects = f.service.kernel.store.effects().unwrap();
    let effect = effects.iter().find(|e| e.command_id == id).unwrap();
    assert!(
        matches!(&effect.request, EffectRequest::ProviderTurnRestart { interrupted_attempt_id, run_id, .. }
        if Some(interrupted_attempt_id) == active.active_attempt_id.as_ref() && run_id == &active.id)
    );
    assert!(!effects.iter().any(
        |e| e.command_id == id && matches!(e.request, EffectRequest::ProviderTurnSteer { .. })
    ));
    let saved: Value = f
        .service
        .kernel
        .store
        .read(|conn| {
            let raw: String = conn.query_row(
                "SELECT target_json FROM orchestration_control_targets WHERE effect_id=?1",
                [&effect.id],
                |r| r.get(0),
            )?;
            Ok(serde_json::from_str(&raw)?)
        })
        .unwrap();
    assert_eq!(saved["runtime"]["runtime_id"], "original-process");
    assert_eq!(
        saved["runtime"]["attempt_id"],
        json!(active.active_attempt_id)
    );
    assert_eq!(
        saved["replacement_attempt_id"],
        json!(restarted.active_attempt_id)
    );
    assert!(f.service.kernel.store.verify_projections().unwrap());
    f.service.kernel.store.rebuild().unwrap();
    let replay = f
        .service
        .mutate(
            None,
            "target".into(),
            "t3_queue_promote_to_steer",
            input,
            id,
            NOW + 1,
        )
        .await
        .unwrap();
    assert_eq!(replay.result_sequence, first.result_sequence);
    assert_eq!(
        f.service.kernel.store.effects().unwrap().len(),
        effects.len()
    );
}

/// Save the next-turn selection on the thread, as `thread.model-selection.set`
/// or `provider.switch` would.
fn save_selection(f: &Fixture, instance: &str, model: &str) {
    let p = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let mut thread = json!(p.thread);
    thread["providerInstanceId"] = json!(instance);
    thread["modelSelection"] = json!({"instanceId":instance,"model":model});
    f.seed("target", "thread.provider-switched", thread);
}

fn set_driver(f: &Fixture, driver: &str) {
    let p = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let mut provider = task::records(&p, "provider-thread")[0].clone();
    provider["driver"] = json!(driver);
    f.seed("target", "provider-thread.updated", provider);
}

fn selection(instance: &str, model: &str) -> Value {
    json!({"instanceId":instance,"model":model})
}

async fn promote(f: &Fixture, id: &str, input: Value) -> super::super::CommandReceipt {
    f.service
        .mutate(
            None,
            "target".into(),
            "t3_queue_promote_to_steer",
            input,
            id.into(),
            NOW,
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn promotion_hint_follows_the_complete_selection_and_transition_policy() {
    use zeron_proto::QueuePromotionMode::*;
    // (driver, saved selection, active, interrupt+restart, mode, deferred, blocked)
    let cases = [
        ("codex", ("mock", "mock-1"), true, true, Some(ActiveSteering), false, false),
        ("codex", ("mock", "mock-2"), true, true, Some(InterruptRestart), false, false),
        ("codex", ("mock", "mock-2"), true, false, Some(ActiveSteering), true, false),
        ("codex", ("mock", "mock-2"), false, true, Some(InterruptRestart), false, false),
        ("codex", ("mock", "mock-2"), false, false, None, false, false),
        // A same-instance change the adapter cannot absorb is a new generation.
        ("mock", ("mock", "mock-2"), true, true, Some(InterruptRestartWithHandoff), false, false),
        ("mock", ("mock", "mock-2"), true, false, None, false, true),
        ("codex", ("other", "other-1"), true, true, Some(InterruptRestartWithHandoff), false, false),
        ("codex", ("other", "other-1"), true, false, None, false, true),
    ];
    for (driver, (instance, model), active, restart, mode, deferred, blocked) in cases {
        let f = Fixture::new();
        f.start_target().await;
        set_driver(&f, driver);
        set_promotion_capabilities(&f, active, restart, restart);
        f.sync("direction").await;
        save_selection(&f, instance, model);
        let hint = f
            .service
            .kernel
            .store
            .queue_ui_state(&"target".into())
            .unwrap();
        let label = format!("{driver} {instance}/{model} active={active} restart={restart}");
        assert_eq!(hint.promotion_mode, mode, "{label}");
        assert_eq!(hint.promotion_selection_deferred, deferred, "{label}");
        assert_eq!(hint.promotion_blocked.is_some(), blocked, "{label}");
        let changed = model != "mock-1";
        assert_eq!(
            hint.promotion_selection.is_some(),
            changed && mode.is_some(),
            "{label}: the displayed selection is echoed only when it matters"
        );
    }
}

#[tokio::test]
async fn same_instance_model_change_restarts_the_native_generation_and_fences_the_old_process() {
    use crate::orchestration::{effects::EffectRequest, steering::RuntimeTarget};
    let f = Fixture::new();
    let active = f.start_target().await;
    set_driver(&f, "codex");
    set_promotion_capabilities(&f, true, true, true);
    let queued = f.sync("switch models now").await;
    save_selection(&f, "mock", "mock-2");
    let before = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let runtime = RuntimeTarget::for_run(&before.runs[0]).unwrap();
    f.service
        .kernel
        .store
        .write(|conn| {
            crate::orchestration::steering::bind_runtime(
                conn,
                &"target".into(),
                &runtime,
                "original-process",
            )
        })
        .unwrap();
    let input = json!({"queuedRunId":queued,"targetRunId":active.id,
        "expectedExecution":"interrupt_restart","expectedSelection":selection("mock","mock-2"),
        "resolvedSelection":selection("mock","mock-2"),"targetDriver":"codex"});
    let first = promote(&f, "promote-model", input.clone()).await;
    assert_eq!(first.status, ReceiptStatus::Accepted, "{:?}", first.error);
    let after = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let run = after.runs.iter().find(|r| r.id == active.id).unwrap();
    assert_eq!(run.model_selection.model.to_string(), "mock-2");
    assert_eq!(run.user_message_id.0, "queue-one");
    assert_eq!(
        run.provider_thread_id, active.provider_thread_id,
        "an absorbed model change keeps the native generation"
    );
    let attempt = after
        .attempts
        .iter()
        .find(|a| Some(&a.id) == run.active_attempt_id.as_ref())
        .unwrap();
    assert_eq!(attempt.attempt_ordinal, 2);
    assert_eq!(attempt.provider_thread_id, active.provider_thread_id.clone().unwrap());
    let effects = f.service.kernel.store.effects().unwrap();
    let effect = effects
        .iter()
        .find(|e| e.command_id.0 == "promote-model")
        .unwrap();
    assert!(matches!(&effect.request, EffectRequest::ProviderTurnRestart { provider_thread_id, .. }
        if Some(provider_thread_id) == active.provider_thread_id.as_ref()));
    let saved: Value = f
        .service
        .kernel
        .store
        .read(|conn| {
            let raw: String = conn.query_row(
                "SELECT target_json FROM orchestration_control_targets WHERE effect_id=?1",
                [&effect.id],
                |r| r.get(0),
            )?;
            Ok(serde_json::from_str(&raw)?)
        })
        .unwrap();
    assert_eq!(saved["runtime"]["runtime_id"], "original-process");
    assert_eq!(
        saved["runtime"]["attempt_id"],
        json!(active.active_attempt_id)
    );
    assert_eq!(saved["replacement_attempt_id"], json!(attempt.id));
    let other = after
        .runs
        .iter()
        .find(|r| r.user_message_id.0 == "queue-two")
        .unwrap();
    assert_eq!(other.status, OrchestrationV2RunStatus::Queued);
    assert!(f.service.kernel.store.verify_projections().unwrap());
    f.service.kernel.store.rebuild().unwrap();
    let replay = promote(&f, "promote-model", input).await;
    assert_eq!(replay.result_sequence, first.result_sequence);
    assert_eq!(
        f.service.kernel.store.effects().unwrap().len(),
        effects.len()
    );
}

#[tokio::test]
async fn cross_instance_promotion_rebinds_the_replacement_attempt_and_fences_the_old_process() {
    use crate::orchestration::{effects::EffectRequest, steering::RuntimeTarget};
    let f = Fixture::new();
    let active = f.start_target().await;
    set_driver(&f, "codex");
    set_promotion_capabilities(&f, true, true, true);
    let queued = f.sync("continue on the other provider").await;
    save_selection(&f, "other", "other-1");
    let before = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let old_attempt = before.runs[0].active_attempt_id.clone().unwrap();
    let runtime = RuntimeTarget::for_run(&before.runs[0]).unwrap();
    f.service
        .kernel
        .store
        .write(|conn| {
            crate::orchestration::steering::bind_runtime(
                conn,
                &"target".into(),
                &runtime,
                "original-process",
            )
        })
        .unwrap();
    let input = json!({"queuedRunId":queued,"targetRunId":active.id,
        "expectedExecution":"interrupt_restart_with_handoff",
        "expectedSelection":selection("other","other-1"),
        "resolvedSelection":selection("other","other-1"),"targetDriver":"claudeAgent"});
    let first = promote(&f, "promote-cross", input.clone()).await;
    assert_eq!(first.status, ReceiptStatus::Accepted, "{:?}", first.error);
    let after = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    assert_eq!(after.runs.len(), before.runs.len(), "one logical run");
    let run = after.runs.iter().find(|r| r.id == active.id).unwrap();
    assert_eq!(run.provider_instance_id.0, "other");
    assert_eq!(run.model_selection.model.to_string(), "other-1");
    assert_eq!(run.user_message_id.0, "queue-one");
    assert_eq!(run.status, OrchestrationV2RunStatus::Starting);
    let generation = run.provider_thread_id.clone().unwrap();
    assert_ne!(Some(&generation), active.provider_thread_id.as_ref());
    assert_eq!(
        generation.0,
        "provider-thread:app:target:other:1:attempt:2",
        "a per-attempt generation never collides with the original"
    );
    let provider = task::records(&after, "provider-thread")
        .iter()
        .find(|p| p["id"] == generation.0)
        .unwrap()
        .clone();
    assert_eq!(provider["driver"], "claudeAgent");
    assert_eq!(provider["providerInstanceId"], "other");
    assert!(provider["nativeThreadRef"].is_null());
    assert_eq!(provider["lastRunOrdinal"], run.ordinal);
    let attempt = after
        .attempts
        .iter()
        .find(|a| Some(&a.id) == run.active_attempt_id.as_ref())
        .unwrap();
    assert_eq!(attempt.provider_instance_id.0, "other");
    assert_eq!(attempt.provider_thread_id, generation);
    assert!(
        json!(attempt)["nativeThreadId"].is_null(),
        "the new generation never inherits the old native identity"
    );
    let old = after.attempts.iter().find(|a| a.id == old_attempt).unwrap();
    assert_eq!(old.status, OrchestrationV2RunAttemptStatus::Superseded);
    assert_eq!(old.provider_thread_id, active.provider_thread_id.clone().unwrap());
    assert_eq!(old.provider_instance_id.0, "mock");
    let root = after
        .nodes
        .iter()
        .find(|n| Some(&n.id) == run.root_node_id.as_ref())
        .unwrap();
    assert_eq!(root.provider_thread_id.as_ref(), Some(&generation));
    // Teardown names the OLD exact provider/process, independent of the binding.
    let effects = f.service.kernel.store.effects().unwrap();
    let effect = effects
        .iter()
        .find(|e| e.command_id.0 == "promote-cross")
        .unwrap();
    assert!(matches!(&effect.request, EffectRequest::ProviderTurnRestart { provider_thread_id, interrupted_attempt_id, .. }
        if Some(provider_thread_id) == active.provider_thread_id.as_ref() && interrupted_attempt_id == &old_attempt));
    let saved: Value = f
        .service
        .kernel
        .store
        .read(|conn| {
            let raw: Option<String> = conn.query_row(
                "SELECT target_json FROM orchestration_control_targets WHERE effect_id=?1",
                [&effect.id],
                |r| r.get(0),
            )?;
            Ok(serde_json::from_str(&raw.expect("a frozen target"))?)
        })
        .unwrap();
    assert_eq!(saved["runtime"]["runtime_id"], "original-process");
    assert_eq!(saved["runtime"]["attempt_id"], json!(old_attempt));
    assert_eq!(
        saved["runtime"]["provider_thread_id"],
        json!(active.provider_thread_id)
    );
    assert_eq!(saved["replacement_attempt_id"], json!(attempt.id));
    let other = after
        .runs
        .iter()
        .find(|r| r.user_message_id.0 == "queue-two")
        .unwrap();
    assert_eq!(other.status, OrchestrationV2RunStatus::Queued);
    assert!(f.service.kernel.store.verify_projections().unwrap());
    f.service.kernel.store.rebuild().unwrap();
    let rebuilt = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    assert_eq!(json!(rebuilt.runs), json!(after.runs));
    let replay = promote(&f, "promote-cross", input).await;
    assert_eq!(replay.result_sequence, first.result_sequence);
    assert_eq!(
        f.service.kernel.store.effects().unwrap().len(),
        effects.len()
    );
}

#[tokio::test]
async fn same_instance_handoff_change_restarts_on_a_new_generation_and_fences_the_old_process() {
    use crate::orchestration::{effects::EffectRequest, steering::RuntimeTarget};
    let f = Fixture::new();
    let active = f.start_target().await;
    // The test adapter cannot switch models inside a native session.
    set_driver(&f, "mock");
    set_promotion_capabilities(&f, true, true, true);
    let queued = f.sync("continue on the next model").await;
    save_selection(&f, "mock", "mock-2");
    let before = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let old_attempt = before.runs[0].active_attempt_id.clone().unwrap();
    let runtime = RuntimeTarget::for_run(&before.runs[0]).unwrap();
    f.service
        .kernel
        .store
        .write(|conn| {
            crate::orchestration::steering::bind_runtime(
                conn,
                &"target".into(),
                &runtime,
                "original-process",
            )
        })
        .unwrap();
    let input = json!({"queuedRunId":queued,"targetRunId":active.id,
        "expectedExecution":"interrupt_restart_with_handoff",
        "expectedSelection":selection("mock","mock-2"),
        "resolvedSelection":selection("mock","mock-2"),"targetDriver":"mock"});
    // The non-handoff click for the same change is a stale action, not a restart.
    let stale = promote(
        &f,
        "promote-stale",
        json!({"queuedRunId":queued,"targetRunId":active.id,
            "expectedExecution":"interrupt_restart",
            "expectedSelection":selection("mock","mock-2"),
            "resolvedSelection":selection("mock","mock-2"),"targetDriver":"mock"}),
    )
    .await;
    assert_eq!(stale.status, ReceiptStatus::Rejected);
    let first = promote(&f, "promote-same", input.clone()).await;
    assert_eq!(first.status, ReceiptStatus::Accepted, "{:?}", first.error);
    let after = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    assert_eq!(after.runs.len(), before.runs.len(), "one logical run");
    let run = after.runs.iter().find(|r| r.id == active.id).unwrap();
    assert_eq!(run.model_selection.model.to_string(), "mock-2");
    assert_eq!(run.user_message_id.0, "queue-one");
    let generation = run.provider_thread_id.clone().unwrap();
    assert_eq!(
        generation.0, "provider-thread:app:target:mock:1:attempt:2",
        "a handoff on the same instance is a new generation, never the old one"
    );
    let old = after.attempts.iter().find(|a| a.id == old_attempt).unwrap();
    assert_eq!(old.status, OrchestrationV2RunAttemptStatus::Superseded);
    assert_eq!(
        old.provider_thread_id,
        active.provider_thread_id.clone().unwrap()
    );
    let effects = f.service.kernel.store.effects().unwrap();
    let effect = effects
        .iter()
        .find(|e| e.command_id.0 == "promote-same")
        .unwrap();
    assert!(
        matches!(&effect.request, EffectRequest::ProviderTurnRestart { provider_thread_id, interrupted_attempt_id, .. }
        if Some(provider_thread_id) == active.provider_thread_id.as_ref() && interrupted_attempt_id == &old_attempt)
    );
    assert!(f.service.kernel.store.verify_projections().unwrap());
    let replay = promote(&f, "promote-same", input).await;
    assert_eq!(replay.result_sequence, first.result_sequence);
    assert_eq!(
        f.service.kernel.store.effects().unwrap().len(),
        effects.len()
    );
}

#[tokio::test]
async fn stopping_during_a_cross_instance_restart_cancels_it_and_binds_nothing_late() {
    use crate::orchestration::effects::{EffectRequest, EffectStatus};
    let f = Fixture::new();
    let active = f.start_target().await;
    set_driver(&f, "codex");
    set_promotion_capabilities(&f, true, true, true);
    let queued = f.sync("continue elsewhere").await;
    save_selection(&f, "other", "other-1");
    let receipt = promote(
        &f,
        "promote-then-stop",
        json!({"queuedRunId":queued,"targetRunId":active.id,
            "expectedExecution":"interrupt_restart_with_handoff",
            "expectedSelection":selection("other","other-1"),
            "resolvedSelection":selection("other","other-1"),"targetDriver":"claudeAgent"}),
    )
    .await;
    assert_eq!(receipt.status, ReceiptStatus::Accepted, "{:?}", receipt.error);
    let restarting = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let run = restarting.runs.iter().find(|r| r.id == active.id).unwrap();
    assert_eq!(run.status, OrchestrationV2RunStatus::Starting);
    let stop = f
        .service
        .kernel
        .store
        .dispatch(
            &Command {
                id: "stop-mid-restart".into(),
                thread_id: "target".into(),
                operation: Operation::Thread(Box::new(
                    crate::orchestration::threads::planner::ThreadOperation::Interrupt {
                        run_id: active.id.clone(),
                        reason: None,
                    },
                )),
            },
            NOW + 1,
        )
        .unwrap();
    assert_eq!(stop.status, ReceiptStatus::Accepted, "{:?}", stop.error);
    let effect = f
        .service
        .kernel
        .store
        .effects()
        .unwrap()
        .into_iter()
        .find(|e| e.command_id.0 == "promote-then-stop")
        .unwrap();
    assert!(matches!(effect.request, EffectRequest::ProviderTurnRestart { .. }));
    assert_eq!(
        effect.status,
        EffectStatus::Cancelled,
        "the pending replacement start cannot outlive the stop"
    );
    let after = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let run = after.runs.iter().find(|r| r.id == active.id).unwrap();
    assert!(crate::orchestration::command::run_terminal(&run.status));
    assert_eq!(
        run.provider_instance_id.0, "other",
        "the binding stays exact for the records that already name it"
    );
    assert!(f.service.kernel.store.verify_projections().unwrap());
}

#[tokio::test]
async fn selection_promotion_refuses_stale_unresolved_and_mismatched_requests_atomically() {
    for fence in [
        "stale-selection",
        "missing-selection",
        "unresolved-instance",
        "resolution-for-another-selection",
        "steer-click",
        "no-interrupt-for-handoff",
    ] {
        let f = Fixture::new();
        let active = f.start_target().await;
        set_driver(&f, "codex");
        set_promotion_capabilities(&f, true, fence != "no-interrupt-for-handoff", fence != "no-interrupt-for-handoff");
        let queued = f.sync("direction").await;
        save_selection(&f, "other", "other-1");
        let mut input = json!({"queuedRunId":queued,"targetRunId":active.id,
            "expectedExecution":"interrupt_restart_with_handoff",
            "expectedSelection":selection("other","other-1"),
            "resolvedSelection":selection("other","other-1"),"targetDriver":"claudeAgent"});
        match fence {
            "stale-selection" => input["expectedSelection"] = selection("other", "other-2"),
            "missing-selection" => input["expectedSelection"] = Value::Null,
            "unresolved-instance" => {
                input["resolvedSelection"] = Value::Null;
                input["targetDriver"] = Value::Null;
            }
            "resolution-for-another-selection" => {
                input["resolvedSelection"] = selection("other", "other-2")
            }
            "steer-click" => input["expectedExecution"] = json!("active_steering"),
            _ => {}
        }
        let frontier = f.service.kernel.store.projection_frontier().unwrap();
        let receipt = promote(&f, &format!("refused-{fence}"), input).await;
        assert_eq!(receipt.status, ReceiptStatus::Rejected, "{fence}");
        assert_eq!(
            f.service.kernel.store.projection_frontier().unwrap(),
            frontier,
            "{fence}: a refusal writes nothing"
        );
        let after = f
            .service
            .kernel
            .store
            .thread(&"target".into())
            .unwrap()
            .unwrap();
        assert_eq!(
            after.runs.iter().find(|r| r.id.0 == queued).unwrap().status,
            OrchestrationV2RunStatus::Queued,
            "{fence}"
        );
        let run = after.runs.iter().find(|r| r.id == active.id).unwrap();
        assert_eq!(run.provider_instance_id.0, "mock", "{fence}");
        assert_eq!(run.active_attempt_id, active.active_attempt_id, "{fence}");
    }
}

#[tokio::test]
async fn a_selection_that_waits_for_the_next_turn_steers_without_moving_the_run() {
    use crate::orchestration::effects::EffectRequest;
    let f = Fixture::new();
    let active = f.start_target().await;
    set_driver(&f, "codex");
    set_promotion_capabilities(&f, true, false, false);
    let queued = f.sync("steer on the old model").await;
    save_selection(&f, "mock", "mock-2");
    let input = json!({"queuedRunId":queued,"targetRunId":active.id,
        "expectedExecution":"active_steering","expectedSelection":selection("mock","mock-2")});
    // Both the displayed selection and the mode are fenced.
    let mut stale = input.clone();
    stale["expectedSelection"] = selection("mock", "mock-3");
    assert_eq!(
        promote(&f, "waiting-stale", stale).await.status,
        ReceiptStatus::Rejected
    );
    let mut unsure = input.clone();
    unsure["expectedSelection"] = Value::Null;
    assert_eq!(
        promote(&f, "waiting-unreviewed", unsure).await.status,
        ReceiptStatus::Rejected,
        "the user must have been shown that the selection waits"
    );
    let receipt = promote(&f, "waiting-ok", input).await;
    assert_eq!(receipt.status, ReceiptStatus::Accepted, "{:?}", receipt.error);
    let after = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let run = after.runs.iter().find(|r| r.id == active.id).unwrap();
    assert_eq!(run.model_selection.model.to_string(), "mock-1");
    assert_eq!(run.active_attempt_id, active.active_attempt_id);
    assert_eq!(
        after.thread.model_selection.model.to_string(),
        "mock-2",
        "the saved selection stays for the next turn"
    );
    let effects = f.service.kernel.store.effects().unwrap();
    assert!(effects.iter().any(
        |e| e.command_id.0 == "waiting-ok" && matches!(e.request, EffectRequest::ProviderTurnSteer { .. })
    ));
    assert!(!effects.iter().any(
        |e| e.command_id.0 == "waiting-ok" && matches!(e.request, EffectRequest::ProviderTurnRestart { .. })
    ));
}

#[tokio::test]
async fn restart_promotion_refuses_stale_modes_capabilities_bindings_and_maintenance_atomically() {
    for fence in [
        "steer-click",
        "no-interrupt",
        "no-restart",
        "completed",
        "turn-root",
        "provider-ordinal",
        "provider-instance",
        "selection",
        "maintenance",
    ] {
        let f = Fixture::new();
        let active = f.start_target().await;
        set_promotion_capabilities(&f, false, fence != "no-interrupt", fence != "no-restart");
        let queued = f
            .sync(if fence == "maintenance" {
                "/compact"
            } else {
                "queued direction"
            })
            .await;
        let p = f
            .service
            .kernel
            .store
            .thread(&"target".into())
            .unwrap()
            .unwrap();
        match fence {
            "completed" => {
                let mut run = json!(p.runs[0]);
                run["status"] = json!("completed");
                f.seed("target", "run.updated", run);
            }
            "turn-root" => {
                let mut turn = task::records(&p, "provider-turn")[0].clone();
                turn["nodeId"] = json!("different-root");
                f.seed("target", "provider-turn.updated", turn);
            }
            "provider-ordinal" | "provider-instance" => {
                let mut provider = task::records(&p, "provider-thread")[0].clone();
                if fence == "provider-ordinal" {
                    provider["lastRunOrdinal"] = json!(999);
                } else {
                    provider["providerInstanceId"] = json!("different-instance");
                }
                f.seed("target", "provider-thread.updated", provider);
            }
            "selection" => {
                let mut thread = json!(p.thread);
                thread["modelSelection"]["model"] = json!("changed-model");
                f.seed("target", "thread.model-selection-updated", thread);
            }
            _ => {}
        }
        let frontier = f.service.kernel.store.projection_frontier().unwrap();
        let result = f.service.mutate(None, "target".into(), "t3_queue_promote_to_steer",
            json!({"queuedRunId":queued,"targetRunId":active.id,
                "expectedExecution":if fence == "steer-click" {"active_steering"} else {"interrupt_restart"}}),
            format!("refused-{fence}").into(), NOW).await.unwrap();
        assert_eq!(
            result.status,
            ReceiptStatus::Rejected,
            "{fence}: {:?}",
            result.error
        );
        assert_eq!(
            f.service.kernel.store.projection_frontier().unwrap(),
            frontier,
            "{fence}"
        );
        let after = f
            .service
            .kernel
            .store
            .thread(&"target".into())
            .unwrap()
            .unwrap();
        assert_eq!(
            after.runs.iter().find(|r| r.id.0 == queued).unwrap().status,
            OrchestrationV2RunStatus::Queued,
            "{fence}"
        );
        assert_eq!(
            after.runs[0].active_attempt_id, active.active_attempt_id,
            "{fence}"
        );
    }
}

#[tokio::test]
async fn postcommit_crash_repairs_intent_patch_without_duplicate_delivery() {
    let f = Fixture::new();
    let docs = crate::DocHost::new(
        f.service.kernel.store.docs.clone(),
        crate::DocHostConfig {
            device_id: "host".into(),
            default_harness: zeron_proto::HarnessId::Mock,
            edge: None,
        },
    );
    let handle = docs.open("target").unwrap();
    handle
        .doc()
        .push_queued(&zeron_doc::QueuedMessage::new(
            "intent", "original", "phone",
        ))
        .unwrap();
    let host = super::host::HostQueue {
        domain: Arc::new(QueueDomain::new(f.service.kernel.clone())),
        docs: docs.clone(),
        registry: Arc::new(crate::HarnessRegistry::new()),
    };
    let list = host
        .call(
            f.caller.clone(),
            "t3_queue_list",
            json!({"threadId":"target"}),
        )
        .await
        .unwrap();
    let id = list["items"][0]["queuedRunId"].as_str().unwrap();
    f.service
        .kernel
        .store
        .inject_failure(super::super::WriteBoundary::AfterCommit, 1);
    assert!(
        host.call(
            f.caller.clone(),
            "t3_queue_edit",
            json!({"threadId":"target","queuedRunId":id,"text":"accepted"})
        )
        .await
        .is_err()
    );
    assert_eq!(handle.doc().read_queue().unwrap()[0].text, "original");
    host.repair(&ThreadId("target".into())).await.unwrap();
    assert_eq!(handle.doc().read_queue().unwrap()[0].text, "accepted");
    assert_eq!(
        f.service
            .kernel
            .store
            .thread(&ThreadId("target".into()))
            .unwrap()
            .unwrap()
            .runs
            .len(),
        1
    );
}

#[tokio::test]
async fn metadata_regeneration_link_unlink_and_noop_reorder() {
    let f = Fixture::new();
    let queued = f.sync("first").await;
    f.call(
        "t3_queue_reorder",
        json!({"threadId":"target","queuedRunId":queued,"beforeRunId":null}),
    )
    .await
    .unwrap();
    let frontier = f.service.kernel.store.projection_frontier().unwrap();
    let result = f
        .call(
            "t3_queue_reorder",
            json!({"threadId":"target","queuedRunId":queued,"beforeRunId":null}),
        )
        .await
        .unwrap();
    assert_eq!(result["sequence"], frontier);
    let regen = f
        .call(
            "t3_thread_update",
            json!({"threadId":"target","action":"regenerate_title","clientRequestId":"key"}),
        )
        .await
        .unwrap();
    assert_eq!(regen["titleRegeneration"]["requestId"], regen["commandId"]);
    let linked=f.call("t3_thread_update",json!({"threadId":"target","action":"link_pull_request","clientRequestId":"key",
        "pullRequest":{"repository":"KldsSeeGhosts/noches","number":41,"url":"https://github.com/KldsSeeGhosts/noches/pull/41"}})).await.unwrap();
    assert_eq!(linked["linkedPullRequest"]["projectId"], "project");
    assert_ne!(linked["commandId"], regen["commandId"]);
    let unlinked = f
        .call(
            "t3_thread_update",
            json!({"threadId":"target","action":"unlink_pull_request","clientRequestId":"key"}),
        )
        .await
        .unwrap();
    assert_eq!(unlinked["linkedPullRequest"], Value::Null);
}

#[tokio::test]
async fn metadata_pr_authority_is_atomic_replayable_and_preserves_sibling_and_watch() {
    use crate::orchestration::pull_requests::identity::parse_url;
    use crate::orchestration::store::WriteBoundary;
    let f = Fixture::new();
    let target = ThreadId("target".into());
    let url = "https://github.com/KldsSeeGhosts/noches/pull/41";
    let links = &f.service.links;
    links
        .link(
            &target,
            parse_url("https://github.com/KldsSeeGhosts/noches/pull/40").unwrap(),
            ThreadPullRequestLinkSource::Stack,
        )
        .await
        .unwrap();
    links
        .set_watching(&target, parse_url(url).unwrap(), true)
        .await
        .unwrap();
    let input = json!({"threadId":"target","action":"link_pull_request","clientRequestId":"durable",
        "pullRequest":{"repository":"KldsSeeGhosts/noches","number":41,"url":url}});
    let before = links.links(&target).unwrap();
    f.service
        .kernel
        .store
        .inject_failure(WriteBoundary::BeforeCommit, 1);
    assert!(f.call("t3_thread_update", input.clone()).await.is_err());
    assert_eq!(links.links(&target).unwrap(), before);
    f.service
        .kernel
        .store
        .inject_failure(WriteBoundary::AfterCommit, 1);
    assert!(f.call("t3_thread_update", input.clone()).await.is_err());
    let committed = links.links(&target).unwrap();
    assert_eq!(committed.len(), 2);
    assert_eq!(committed[1].source, ThreadPullRequestLinkSource::Manual);
    assert!(committed[1].watch.as_ref().is_some());
    let accepted = f.call("t3_thread_update", input.clone()).await.unwrap();
    assert_eq!(
        accepted["linkedPullRequest"]["repository"],
        "KldsSeeGhosts/noches"
    );
    let unlinked = f
        .call(
            "t3_thread_update",
            json!({"threadId":"target",
        "action":"unlink_pull_request","clientRequestId":"remove"}),
        )
        .await
        .unwrap();
    assert_eq!(unlinked["linkedPullRequest"], Value::Null);
    assert_eq!(links.links(&target).unwrap().len(), 1);
    assert_eq!(
        links.links(&target).unwrap()[0].source,
        ThreadPullRequestLinkSource::Stack
    );
    // The old command result is stable; replay must not resurrect its link.
    assert_eq!(f.call("t3_thread_update", input).await.unwrap(), accepted);
    assert_eq!(links.links(&target).unwrap().len(), 1);
}

#[tokio::test]
async fn mcp_uses_pinned_schemas_trims_search_and_returns_named_failures() {
    let f = Fixture::new();
    let scope = crate::mcp::auth::InvocationScope {
        caller: f.caller.clone(),
        environment_id: "host".into(),
        selection: serde_json::from_value(json!({"instanceId":"mock","model":"mock-1"})).unwrap(),
        capabilities: ["orchestration"].into_iter().map(str::to_owned).collect(),
        task_id: None,
        issued_at: 0,
    };
    let toolkit = crate::mcp::toolkit::Toolkit::new(Arc::new(crate::HarnessRegistry::new()));
    toolkit.set_queue_service(Arc::new(f.service));
    for (name, args, expected) in [
        (
            "t3_queue_read",
            json!({"queuedRunId":"missing"}),
            json!({"_tag":"OrchestratorMcpFailure","code":"invalid_request","message":"The queued message was not found."}),
        ),
        (
            "t3_pending_request_respond",
            json!({"requestId":"approval","answers":{}}),
            json!({"_tag":"OrchestratorMcpFailure","code":"invalid_request","message":"The pending user-input request was not found."}),
        ),
        (
            "t3_thread_organize",
            json!({"action":"snooze"}),
            json!({"_tag":"OrchestratorMcpFailure","code":"invalid_request","message":"snooze requires snoozedUntil."}),
        ),
        (
            "t3_thread_search",
            json!({"query":"  needle  ","limit":50}),
            json!({"matches":[]}),
        ),
    ] {
        let response=toolkit.request(scope.clone(),json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}})).await.unwrap();
        assert_eq!(
            response["result"]["structuredContent"], expected,
            "{response}"
        );
    }
    for args in [
        json!({"query":" x "}),
        json!({"query":"needle","limit":51}),
        json!({"query":"needle","limit":0}),
    ] {
        let response = toolkit
            .request(
                scope.clone(),
                json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"t3_thread_search","arguments":args}}),
            )
            .await
            .unwrap();
        assert_eq!(response["result"]["structuredContent"]["_tag"], "AiError");
    }
}

#[tokio::test]
async fn auto_settle_source_and_reopen_are_synced_without_hiding_failed_work() {
    let f = Fixture::new();
    f.service
        .settle_for_host(
            ThreadId("target".into()),
            zeron_proto::SettleSource::Auto,
            NOW,
        )
        .await
        .unwrap();
    let state = f
        .service
        .kernel
        .store
        .queue_ui_state(&ThreadId("target".into()))
        .unwrap();
    assert_eq!(
        state.lifecycle.settled_by,
        Some(zeron_proto::SettleSource::Auto)
    );
    assert!(state.lifecycle.settled_at.is_some());
    f.call(
        "t3_thread_organize",
        json!({"threadId":"target","action":"unsettle"}),
    )
    .await
    .unwrap();
    let state = f
        .service
        .kernel
        .store
        .queue_ui_state(&ThreadId("target".into()))
        .unwrap();
    assert!(state.lifecycle.settled_at.is_none());
    assert!(state.lifecycle.settled_by.is_none());
    let result = f
        .service
        .settle_for_host(
            ThreadId("target".into()),
            zeron_proto::SettleSource::Auto,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(
        result.status,
        ReceiptStatus::Rejected,
        "an explicit reopen must prevent an automatic sweep from re-parking it"
    );
}

#[tokio::test]
async fn shared_thread_send_clears_synced_settlement_source_and_snooze() {
    let f = Fixture::new();
    f.service
        .settle_for_host("target".into(), zeron_proto::SettleSource::Auto, NOW)
        .await
        .unwrap();
    let receipt = f.service.mutate(
        Some(f.caller.clone()),
        "target".into(),
        "t3_thread_organize",
        json!({"threadId":"target","action":"snooze","snoozedUntil":super::super::event::iso(NOW+1000).unwrap()}),
        "shared-send-snooze".into(),
        NOW,
    ).await.unwrap();
    assert_eq!(receipt.status, ReceiptStatus::Accepted);
    let receipt = f
        .service
        .kernel
        .dispatch(
            &Command {
                id: "shared-send-reengages".into(),
                thread_id: "target".into(),
                operation: Operation::Thread(Box::new(
                    crate::orchestration::threads::planner::ThreadOperation::Send(
                        crate::orchestration::threads::planner::Send {
                            message_id: "shared-send-input".into(),
                            text: "continue".into(),
                            mode: zeron_proto::orchestration_mcp::T3ThreadSendInputMode::Auto,
                            driver: "mock".into(),
                            sender: "parent".into(),
                            target_run: None,
                            metadata: None,
                        },
                    ),
                )),
            },
            NOW + 1,
        )
        .await
        .unwrap();
    assert_eq!(
        receipt.status,
        ReceiptStatus::Accepted,
        "{:?}",
        receipt.error
    );
    // Check after successful completion, not only while active work masks
    // stale lifecycle fields.
    let p = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let mut run = p.runs.last().unwrap().clone();
    run.status = OrchestrationV2RunStatus::Completed;
    run.completed_at = Some(super::super::event::iso(NOW + 2).unwrap());
    f.seed("target", "run.updated", serde_json::to_value(run).unwrap());
    let lifecycle = f
        .service
        .kernel
        .store
        .queue_ui_state(&"target".into())
        .unwrap()
        .lifecycle;
    assert!(lifecycle.settled_at.is_none());
    assert!(lifecycle.settled_by.is_none());
    assert!(lifecycle.snoozed_until.is_none());
}

#[tokio::test]
async fn delayed_promotion_effect_cannot_become_a_late_send() {
    use crate::orchestration::effects::{EffectExecutor, EffectOutcome, EffectRequest};
    let f = Fixture::new();
    let active = f.start_target().await;
    let queued = f.sync("strict promotion").await;
    f.call(
        "t3_queue_promote_to_steer",
        json!({"threadId":"target","queuedRunId":queued,"targetRunId":active.id}),
    )
    .await
    .unwrap();
    let effect = f.service.kernel.store.effects().unwrap().into_iter().find(|e| {
        matches!(&e.request, EffectRequest::ProviderTurnSteer { message_id, .. } if message_id.0 == "queue-one")
    }).unwrap();
    let p = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let mut ended = p.runs.iter().find(|r| r.id == active.id).unwrap().clone();
    ended.status = OrchestrationV2RunStatus::Completed;
    ended.completed_at = Some(super::super::event::iso(NOW + 1).unwrap());
    f.seed(
        "target",
        "run.updated",
        serde_json::to_value(ended).unwrap(),
    );
    // Use the production executor, but no live provider is needed: the stale
    // target must be refused before any session dispatch or fallback planning.
    let runtime_dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(crate::HarnessRegistry::new());
    registry.register(Arc::new(zeron_harness::mock::MockHarness {
        script: vec![],
    }));
    let core = crate::EngineCore::assemble(
        runtime_dir.path(),
        registry,
        zeron_proto::HarnessId::Mock,
        None,
    )
    .unwrap();
    let mut bridge = core
        .orchestration_host
        .as_ref()
        .unwrap()
        .bridge
        .as_ref()
        .clone();
    bridge.kernel = f.service.kernel.clone();
    let frontier = bridge.kernel.store.projection_frontier().unwrap();
    assert_eq!(
        bridge
            .execute(&effect, tokio_util::sync::CancellationToken::new())
            .await,
        EffectOutcome::Failed,
    );
    assert_eq!(bridge.kernel.store.projection_frontier().unwrap(), frontier);
    let after = bridge
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    assert_eq!(after.runs.len(), p.runs.len());
    assert_eq!(
        task::records(&after, "message").len(),
        task::records(&p, "message").len()
    );
    assert!(!bridge.sessions.turn_in_flight("target"));
}

#[tokio::test]
async fn settling_idle_work_detaches_provider_and_archive_disposes_after_metadata() {
    let f = Fixture::new();
    let active = f.start_target().await;
    let mut completed = serde_json::to_value(active).unwrap();
    completed["status"] = json!("completed");
    completed["completedAt"] = json!(super::super::event::iso(NOW).unwrap());
    f.seed("target", "run.updated", completed);
    f.call(
        "t3_thread_organize",
        json!({"threadId":"target","action":"settle"}),
    )
    .await
    .unwrap();
    let p = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    assert!(task::records(&p, "provider-session").is_empty());
    assert!(
        f.service
            .kernel
            .store
            .effects()
            .unwrap()
            .iter()
            .any(|effect| {
                matches!(
                    effect.request,
                    super::super::effects::EffectRequest::ProviderSessionDetach { .. }
                )
            })
    );
    f.call(
        "t3_thread_organize",
        json!({"threadId":"target","action":"unsettle"}),
    )
    .await
    .unwrap();
    f.sync("waiting").await;
    let frontier = f.service.kernel.store.projection_frontier().unwrap();
    f.call(
        "t3_thread_organize",
        json!({"threadId":"target","action":"archive"}),
    )
    .await
    .unwrap();
    let events = f.service.kernel.store.events().unwrap();
    let types: Vec<_> = events
        .into_iter()
        .filter(|e| e.sequence > frontier)
        .map(|e| {
            serde_json::to_value(e.event).unwrap()["type"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(types.first().map(String::as_str), Some("thread.archived"));
    assert!(types.iter().any(|t| t == "run.updated"));
    assert_eq!(
        f.call("t3_queue_list", json!({"threadId":"target"}))
            .await
            .unwrap()["items"],
        json!([])
    );
}

#[tokio::test]
async fn cross_instance_promotion_refuses_without_cancelling_the_intent() {
    let f = Fixture::new();
    let active = f.start_target().await;
    let queued = f.sync("steer").await;
    let p = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let mut thread = serde_json::to_value(p.thread).unwrap();
    thread["providerInstanceId"] = json!("other");
    thread["modelSelection"]["instanceId"] = json!("other");
    f.seed("target", "thread.provider-switched", thread);
    let frontier = f.service.kernel.store.projection_frontier().unwrap();
    assert!(
        f.call(
            "t3_queue_promote_to_steer",
            json!({"threadId":"target","queuedRunId":queued,"targetRunId":active.id})
        )
        .await
        .is_err()
    );
    assert_eq!(
        f.service.kernel.store.projection_frontier().unwrap(),
        frontier
    );
    let p = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    assert_eq!(
        p.runs.iter().find(|r| r.id.0 == queued).unwrap().status,
        OrchestrationV2RunStatus::Queued
    );
}

#[tokio::test]
async fn failed_work_stays_visible_after_later_queue_cancellation() {
    let f = Fixture::new();
    let active = f.start_target().await;
    let mut failed = serde_json::to_value(active).unwrap();
    failed["status"] = json!("failed");
    failed["completedAt"] = json!(super::super::event::iso(NOW).unwrap());
    f.seed("target", "run.updated", failed);
    let p = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let mut thread = serde_json::to_value(p.thread).unwrap();
    thread["settledOverride"] = json!("settled");
    thread["settledAt"] = json!(super::super::event::iso(NOW).unwrap());
    thread["snoozedUntil"] = json!(super::super::event::iso(NOW + 10000).unwrap());
    f.seed("target", "thread.metadata-updated", thread);
    f.sync("waiting").await;
    let list = f
        .call("t3_queue_list", json!({"threadId":"target"}))
        .await
        .unwrap();
    for entry in list["items"].as_array().unwrap() {
        f.call(
            "t3_queue_cancel",
            json!({"threadId":"target","queuedRunId":entry["queuedRunId"]}),
        )
        .await
        .unwrap();
    }
    let lifecycle = f
        .service
        .kernel
        .store
        .queue_ui_state(&"target".into())
        .unwrap()
        .lifecycle;
    assert!(lifecycle.settled_at.is_none());
    assert!(lifecycle.snoozed_until.is_none());
    let projection = f
        .service
        .kernel
        .store
        .thread(&"target".into())
        .unwrap()
        .unwrap();
    let summary = crate::orchestration::threads::timeline::summary(&projection, 0);
    assert_eq!(summary["settled"], false);
    assert!(summary["settledAt"].is_null());
    let (page, _) = crate::orchestration::threads::timeline::page(
        &f.service.kernel.store,
        &projection,
        &serde_json::from_value(json!({"threadId":"target"})).unwrap(),
    )
    .unwrap();
    assert!(!page.result.thread.settled);
    assert!(page.result.thread.settled_at.is_none());
}
