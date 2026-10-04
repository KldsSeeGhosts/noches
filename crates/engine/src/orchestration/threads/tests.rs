//! Oracles: T3 ThreadManagementService.test.ts, OrchestratorMcpService.test.ts
//! and OrchestratorMcpService.activity.test.ts (pinned 20261003.2632).
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::*;
use zeron_proto::provider_instance::*;
use zeron_proto::{InteractionMode, RuntimeMode};
use zeron_sync::DocsStore;

use super::{KernelThreadService, wire};
use crate::orchestration::command::{Command, Operation};
use crate::orchestration::service::OrchestratorService;
use crate::orchestration::service::{CallerScope, ToolError};
use crate::orchestration::task::{
    DelegationService, DelegationTargets, ResolvedTarget, execution_seed,
};
use crate::orchestration::thread_service::ThreadService;
use crate::orchestration::{Kernel, ReceiptStatus};

struct Targets;
#[async_trait]
impl DelegationTargets for Targets {
    async fn resolve(
        &self,
        parent: &OrchestrationV2AppThread,
        target: Option<&DelegateTaskInputTarget>,
    ) -> Result<ResolvedTarget, ToolError> {
        if target.and_then(|t| t.model.as_ref()).map(String::as_str) == Some("missing") {
            return Err(ToolError::new(
                OrchestratorMcpFailureCode::ModelUnavailable,
                "Model missing is unavailable.",
            ));
        }
        let mut selection = json!(parent.model_selection);
        if let Some(target) = target {
            if let Some(instance) = target.provider_instance_id.as_ref() {
                selection["instanceId"] = json!(instance);
            }
            if let Some(model) = target.model.as_ref() {
                selection["model"] = json!(model);
            }
            if let Some(options) = target.options.as_ref() {
                selection["options"] = json!(options);
            }
        }
        Ok(ResolvedTarget {
            selection: ModelSelection::try_from(selection).unwrap(),
            driver: ProviderDriverKind("mock".into()),
        })
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    service: KernelThreadService,
    caller: CallerScope,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let kernel = Kernel::open(Arc::new(DocsStore::open(dir.path()).unwrap()), "host").unwrap();
        let command = Command::wire(serde_json::from_value(json!({
            "type":"thread.create","commandId":"create-parent","threadId":"parent","projectId":"project","title":"Parent",
            "createdBy":"user","creationSource":"web","modelSelection":{"instanceId":"mock","model":"mock-1"},
            "runtimeMode":"full-access","interactionMode":"default","branch":"main","worktreePath":dir.path()
        })).unwrap()).unwrap();
        assert_eq!(
            kernel
                .store
                .dispatch(&command, 1_800_000_000_000)
                .unwrap()
                .status,
            ReceiptStatus::Accepted
        );
        let parent = kernel.store.thread(&"parent".into()).unwrap().unwrap();
        let seed = execution_seed(
            &parent.thread,
            1,
            "parent-input",
            "starting",
            "mock",
            1_800_000_000_000,
        )
        .unwrap();
        assert_eq!(
            kernel
                .store
                .dispatch(
                    &Command {
                        id: "seed".into(),
                        thread_id: "parent".into(),
                        operation: Operation::CreateExecution(Box::new(seed.clone()))
                    },
                    1_800_000_000_000
                )
                .unwrap()
                .status,
            ReceiptStatus::Accepted
        );
        let caller = CallerScope {
            thread_id: "parent".into(),
            run_id: seed.run.id,
            session_id: "session/parent:1".into(),
            project_id: "project".into(),
            workspace_root: dir.path().into(),
            runtime_mode: RuntimeMode::FullAccess,
            interaction_mode: InteractionMode::Default,
            provider_instance_id: ProviderInstanceId("mock".into()),
        };
        let delegation = Arc::new(DelegationService {
            kernel: kernel.clone(),
            targets: Arc::new(Targets),
        });
        Self {
            _dir: dir,
            service: KernelThreadService { kernel, delegation },
            caller,
        }
    }

    async fn send(&self, input: Value) -> T3ThreadSendResult {
        self.service
            .send(self.caller.clone(), serde_json::from_value(input).unwrap())
            .await
            .unwrap()
    }

    fn emit(&self, thread: &str, kind: &str, payload: Value) {
        let event = crate::orchestration::event::make(
            EventId(format!("test:{}", uuid::Uuid::new_v4())),
            &thread.into(),
            kind,
            &payload,
            1_800_000_000_001,
        )
        .unwrap();
        self.service
            .kernel
            .store
            .write(|tx| {
                self.service.kernel.store.append_event(tx, None, event)?;
                Ok(())
            })
            .unwrap();
    }

    fn item(&self, thread: &str, kind: &str, ordinal: i64, text: &str) -> String {
        let id = format!("item:{thread}:{ordinal}");
        let run = self
            .service
            .kernel
            .store
            .thread(&thread.into())
            .unwrap()
            .unwrap()
            .runs
            .first()
            .map(|r| r.id.clone());
        let mut item = json!({"id":id,"type":kind,"threadId":thread,"runId":run,"nodeId":null,
            "providerThreadId":null,"providerTurnId":null,"nativeItemRef":null,"parentItemId":null,
            "ordinal":ordinal,"status":"completed","title":null,"startedAt":null,
            "completedAt":null,"updatedAt":"2026-10-03T00:00:00.000Z","text":text,"streaming":false,
            "createdBy":"user","creationSource":"web"});
        if matches!(kind, "user_message" | "assistant_message") {
            item["messageId"] = json!(format!("message:{id}"));
            item["streaming"] = json!(false);
            if kind == "user_message" {
                item["inputIntent"] = json!("turn_start");
                item["attachments"] = json!([]);
            }
            self.emit(thread,"message.updated",json!({"id":format!("message:{id}"),"threadId":thread,"runId":run,
                "nodeId":null,"role":if kind == "user_message" {"user"} else {"assistant"},"createdBy":"user",
                "creationSource":"web","text":text,"attachments":[],"streaming":false,
                "createdAt":"2026-10-03T00:00:00.000Z","updatedAt":"2026-10-03T00:00:00.000Z"}));
        }
        self.emit(thread, "turn-item.updated", item);
        id
    }

    fn running(&self, restart: bool) {
        let p = self
            .service
            .kernel
            .store
            .thread(&"parent".into())
            .unwrap()
            .unwrap();
        let caps =
            crate::orchestration::assembly::capabilities(&zeron_harness::mock::MockHarness {
                script: vec![],
            });
        let mut caps = json!(caps);
        caps["turns"]["supportsActiveSteering"] = json!(!restart);
        caps["turns"]["supportsSteeringByInterruptRestart"] = json!(restart);
        let run = &p.runs[0];
        self.service
            .kernel
            .store
            .dispatch(
                &Command {
                    id: format!("running:{}", uuid::Uuid::new_v4()).into(),
                    thread_id: "parent".into(),
                    operation: Operation::Task(Box::new(
                        crate::orchestration::task::TaskOperation::RunnerEvent {
                            run_id: run.id.clone(),
                            attempt_id: run.active_attempt_id.clone().unwrap(),
                            event: zeron_proto::AgentEvent::SessionStarted {
                                instance_id: None,
                                harness: zeron_proto::HarnessId::Mock,
                                model: "mock-1".into(),
                                tools: vec![],
                                cwd: self.caller.workspace_root.to_string_lossy().into_owned(),
                                session_id: "native".into(),
                                assistant_message_id: "assistant".into(),
                            },
                            capabilities: Some(Box::new(serde_json::from_value(caps).unwrap())),
                        },
                    )),
                },
                1_800_000_000_001,
            )
            .unwrap();
    }

    fn clone_thread(&self, id: &str, project: &str, lineage: Value) {
        let p = self
            .service
            .kernel
            .store
            .thread(&"parent".into())
            .unwrap()
            .unwrap();
        let mut thread = json!(p.thread);
        thread["id"] = json!(id);
        thread["projectId"] = json!(project);
        thread["lineage"] = lineage;
        self.emit(id, "thread.created", thread);
    }

    fn provider_event(&self, thread: &str, event: zeron_proto::AgentEvent) {
        let p = self
            .service
            .kernel
            .store
            .thread(&thread.into())
            .unwrap()
            .unwrap();
        let r = &p.runs[0];
        let receipt = self
            .service
            .kernel
            .store
            .dispatch(
                &Command {
                    id: format!("provider:{}", uuid::Uuid::new_v4()).into(),
                    thread_id: thread.into(),
                    operation: Operation::Task(Box::new(
                        crate::orchestration::task::TaskOperation::RunnerEvent {
                            run_id: r.id.clone(),
                            attempt_id: r.active_attempt_id.clone().unwrap(),
                            event,
                            capabilities: Some(Box::new(
                                crate::orchestration::assembly::capabilities(
                                    &zeron_harness::mock::MockHarness { script: vec![] },
                                ),
                            )),
                        },
                    )),
                },
                1_800_000_000_005,
            )
            .unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Accepted);
    }
}

#[test]
fn js_surrogate_slice_and_exact_wire_json() {
    let (safe, units, truncated) = wire::slice("A😀B", 1, 1);
    assert_eq!(safe, "�");
    assert_eq!(units, vec![0xd83d]);
    assert!(truncated);
    let page = json!({"items":[{"text":wire::text_value(&units)}]});
    assert_eq!(wire::read_json(&page), r#"{"items":[{"text":"\ud83d"}]}"#);
    let response = json!({"jsonrpc":"2.0","id":1,"result":wire::result(page)});
    let encoded = wire::response_json(&response);
    assert!(encoded.contains(r#""structuredContent":{"items":[{"text":"\ud83d"}]}"#));
    assert!(encoded.contains(r#"\"text\":\"\\ud83d\""#));
    assert_eq!(wire::slice("A😀B", 2, 2).1, vec![0xde00, 0x42]);
    assert_eq!(wire::slice("A😀B", 100, 2), ("".into(), vec![], false));
}

#[test]
fn activity_json_text_keeps_javascript_field_order_and_absent_versus_null() {
    assert_eq!(
        super::timeline::text(&json!({"type":"dynamic_tool","toolName":"echo","input":"hello"})),
        Some("{\n  \"toolName\": \"echo\",\n  \"input\": \"hello\"\n}".into())
    );
    assert_eq!(
        super::timeline::text(
            &json!({"type":"dynamic_tool","toolName":null,"input":{},"output":null})
        ),
        Some("{\n  \"toolName\": null,\n  \"input\": {},\n  \"output\": null\n}".into())
    );
    assert_eq!(
        super::timeline::text(&json!({"type":"file_search"})),
        Some("{}".into())
    );
    assert_eq!(
        super::timeline::text(&json!({"type":"file_search","results":[]})),
        Some("{\n  \"results\": []\n}".into())
    );
}

#[tokio::test]
async fn auto_before_steerable_queues_and_retry_ignores_changed_payload() {
    let f = Fixture::new();
    let first = f
        .send(json!({"threadId":"parent","message":"first","clientRequestId":"key"}))
        .await;
    let retry = f
        .send(json!({"threadId":"parent","message":"changed","clientRequestId":"key"}))
        .await;
    assert_eq!(first, retry);
    assert_eq!(
        first.delivery,
        OrchestratorMcpThreadSendResultDelivery::Queued
    );
    let p = f
        .service
        .kernel
        .store
        .thread(&"parent".into())
        .unwrap()
        .unwrap();
    assert_eq!(p.runs.len(), 2);
    let m = crate::orchestration::task::records(&p, "message")
        .iter()
        .find(|m| m["id"] == first.message_id.0)
        .unwrap();
    assert_eq!(m["text"], "first");
}

#[tokio::test]
async fn wait_timeout_is_observation_not_interrupt_or_acknowledgement() {
    let f = Fixture::new();
    let before = f.service.kernel.store.events().unwrap().len();
    let result = f
        .service
        .wait(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent","timeoutMs":1})).unwrap(),
        )
        .await
        .unwrap();
    assert!(result.timed_out);
    assert_eq!(result.run_id, Some(f.caller.run_id.clone()));
    assert_eq!(before, f.service.kernel.store.events().unwrap().len());
}

#[tokio::test]
async fn explicit_steer_refuses_before_any_receipt_or_side_effect() {
    let f = Fixture::new();
    let before = f.service.kernel.store.events().unwrap().len();
    let error = f
        .service
        .send(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent","message":"hello","mode":"steer"}))
                .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ThreadNotSendable);
    assert_eq!(
        error.message,
        "Thread parent has no running turn that can be steered."
    );
    assert_eq!(before, f.service.kernel.store.events().unwrap().len());
}

#[tokio::test]
async fn batch_checkout_and_top_level_identity_are_not_delegation() {
    let f = Fixture::new();
    let result = f
        .service
        .create(
            f.caller.clone(),
            serde_json::from_value(
                json!({"threads":[{"title":"Empty"},{"prompt":"Task"}],"clientRequestId":"batch"}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(result.threads.len(), 2);
    let first = f
        .service
        .kernel
        .store
        .thread(&result.threads[0].thread_id)
        .unwrap()
        .unwrap();
    assert_eq!(first.thread.lineage.parent_thread_id, None);
    assert_eq!(
        first.thread.worktree_path,
        Some(f.caller.workspace_root.to_string_lossy().into_owned())
    );
    assert!(first.runs.is_empty());
    let result2 = f.service.create(f.caller.clone(),serde_json::from_value(json!({"threads":[{"title":"Changed"},{"prompt":"Changed"}],"clientRequestId":"batch"})).unwrap()).await.unwrap();
    assert_eq!(result, result2);
}

#[tokio::test]
async fn messages_activity_item_continuation_and_utf16_offsets() {
    let f = Fixture::new();
    let first = f.item("parent", "user_message", 1, "Hello");
    f.item("parent", "reasoning", 2, "Thinking");
    let emoji = f.item("parent", "assistant_message", 3, "A😀B");
    let read = |v| {
        f.service
            .read(f.caller.clone(), serde_json::from_value(v).unwrap())
    };
    let page = read(json!({"threadId":"parent","limit":1})).await.unwrap();
    assert_eq!(page.result.items.len(), 1);
    assert_eq!(page.result.items[0].item_id.0, first);
    assert_eq!(page.result.next_position, Some(0));
    assert!(page.result.has_more);
    let page = read(json!({"threadId":"parent","afterPosition":0,"limit":1,"maxCharsPerItem":2}))
        .await
        .unwrap();
    assert_eq!(page.result.items[0].position, 2);
    assert_eq!(
        page.result.items[0].next_text_offset.as_ref(),
        Some(&Some(2))
    );
    assert_eq!(page.text_units[&0], vec![0x41, 0xd83d]);
    let continuation = read(json!({"threadId":"parent","itemId":emoji,"textOffset":2,"afterPosition":100,"view":"messages","maxCharsPerItem":2})).await.unwrap();
    assert_eq!(continuation.text_units[&0], vec![0xde00, 0x42]);
    assert!(!continuation.result.items[0].text_truncated);
    let activity = read(json!({"threadId":"parent","view":"activity"}))
        .await
        .unwrap();
    assert_eq!(activity.result.items.len(), 3);
    assert_eq!(activity.result.thread.item_count, 3);
}

#[tokio::test]
async fn fully_active_auto_steers_but_queue_remains_separate() {
    let f = Fixture::new();
    f.running(false);
    let auto = f
        .send(json!({"threadId":"parent","message":"update","clientRequestId":"auto"}))
        .await;
    assert_eq!(
        auto.delivery,
        OrchestratorMcpThreadSendResultDelivery::Steered
    );
    assert_eq!(auto.run_id, f.caller.run_id);
    let queue = f
        .send(json!({"threadId":"parent","message":"later","mode":"queue"}))
        .await;
    assert_eq!(
        queue.delivery,
        OrchestratorMcpThreadSendResultDelivery::Queued
    );
    assert_ne!(queue.run_id, auto.run_id);
}

#[tokio::test]
async fn restart_supersedes_attempt_without_reopening_a_task() {
    let f = Fixture::new();
    f.running(true);
    let old = f
        .service
        .kernel
        .store
        .thread(&"parent".into())
        .unwrap()
        .unwrap()
        .runs[0]
        .active_attempt_id
        .clone();
    let restart = f.send(json!({"threadId":"parent","message":"new direction","mode":"restart","clientRequestId":"restart"})).await;
    assert_eq!(
        restart.delivery,
        OrchestratorMcpThreadSendResultDelivery::Restarted
    );
    assert_eq!(restart.run_id, f.caller.run_id);
    let p = f
        .service
        .kernel
        .store
        .thread(&"parent".into())
        .unwrap()
        .unwrap();
    assert_ne!(p.runs[0].active_attempt_id, old);
    assert_eq!(
        p.attempts[0].status,
        OrchestrationV2RunAttemptStatus::Superseded
    );
    assert_eq!(
        p.attempts[1].reason,
        OrchestrationV2RunAttemptReason::SteeringRestart
    );
    assert_eq!(p.runs.len(), 1);
}

#[tokio::test]
async fn explicit_wait_missing_run_and_interrupt_before_start_hold_queue() {
    let f = Fixture::new();
    let queued = f
        .send(json!({"threadId":"parent","message":"later","mode":"queue"}))
        .await;
    let error = f
        .service
        .wait(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent","runId":"missing"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.message,
        "Run missing does not belong to thread parent."
    );
    assert_eq!(error.code, OrchestratorMcpFailureCode::RunNotFound);
    let error = f
        .service
        .interrupt(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent","runId":queued.run_id})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.code,
        OrchestratorMcpFailureCode::ThreadNotInterruptible
    );
    let interrupted = f
        .service
        .interrupt(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent","clientRequestId":"stop"})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        interrupted.status,
        OrchestratorMcpThreadInterruptResultStatus::InterruptRequested
    );
    let p = f
        .service
        .kernel
        .store
        .thread(&"parent".into())
        .unwrap()
        .unwrap();
    assert_eq!(p.runs[0].status, OrchestrationV2RunStatus::Interrupted);
    assert_eq!(p.runs[1].queue_held.as_ref(), Some(&true));
    let again = f
        .service
        .interrupt(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent","runId":f.caller.run_id})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        again.status,
        OrchestratorMcpThreadInterruptResultStatus::Interrupted
    );
}

#[tokio::test]
async fn user_attached_context_can_be_read_cross_project_but_never_written() {
    let f = Fixture::new();
    f.clone_thread(
        "context",
        "other",
        json!({"parentThreadId":null,"relationshipToParent":null,"rootThreadId":"context"}),
    );
    f.item("context", "assistant_message", 1, "Other project history");
    let input = json!({"threadId":"context"});
    let error = f
        .service
        .read(
            f.caller.clone(),
            serde_json::from_value(input.clone()).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ThreadNotFound);
    let mut m = json!({"id":"context-message","threadId":"parent","runId":f.caller.run_id,"nodeId":null,
        "role":"user","createdBy":"agent","creationSource":"mcp","text":"context","attachments":[],
        "streaming":false,"createdAt":"2026-10-03T00:00:00.000Z","updatedAt":"2026-10-03T00:00:00.000Z",
        "context":{"version":1,"records":[{"version":1,"contextId":"attached","label":"Context","kind":"thread",
            "environmentId":"host","threadId":"context","title":"Context"}]}});
    f.emit("parent", "message.updated", m.clone());
    assert!(
        f.service
            .read(
                f.caller.clone(),
                serde_json::from_value(input.clone()).unwrap()
            )
            .await
            .is_err()
    );
    m["createdBy"] = json!("user");
    f.emit("parent", "message.updated", m);
    assert_eq!(
        f.service
            .read(f.caller.clone(), serde_json::from_value(input).unwrap())
            .await
            .unwrap()
            .result
            .items
            .len(),
        1
    );
    assert!(
        f.service
            .send(
                f.caller.clone(),
                serde_json::from_value(json!({"threadId":"context","message":"not permitted"}))
                    .unwrap()
            )
            .await
            .is_err()
    );
    assert!(
        f.service
            .configuration(
                f.caller.clone(),
                serde_json::from_value(json!({"threadId":"context"})).unwrap()
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn native_child_refusal_is_exact_and_leaves_parking_unchanged() {
    let f = Fixture::new();
    f.clone_thread("native","project",json!({"parentThreadId":"parent","relationshipToParent":"subagent","rootThreadId":"parent"}));
    let mut thread = json!(
        f.service
            .kernel
            .store
            .thread(&"native".into())
            .unwrap()
            .unwrap()
            .thread
    );
    thread["settledOverride"] = json!("settled");
    f.emit("native", "thread.metadata-updated", thread);
    let before = f.service.kernel.store.events().unwrap().len();
    let error = f
        .service
        .send(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"native","message":"hello"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::OrchestrationError);
    assert_eq!(
        error.message,
        "Unable to send to thread native: This subagent is run by its provider and cannot take messages. Message the parent thread instead."
    );
    assert_eq!(before, f.service.kernel.store.events().unwrap().len());
}

#[tokio::test]
async fn list_and_read_prefer_active_run_over_newer_cancelled_queue() {
    let f = Fixture::new();
    f.running(false);
    let queued = f
        .send(json!({"threadId":"parent","message":"queue","mode":"queue"}))
        .await;
    let p = f
        .service
        .kernel
        .store
        .thread(&"parent".into())
        .unwrap()
        .unwrap();
    let mut q = json!(p.runs.iter().find(|r| r.id == queued.run_id).unwrap());
    q["status"] = json!("cancelled");
    f.emit("parent", "run.updated", q);
    let read = f
        .service
        .read(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent","runLimit":1})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        read.result.thread.status,
        OrchestratorMcpThreadStatus::Running
    );
    assert_eq!(read.result.recent_runs.len(), 1);
    assert_eq!(read.result.recent_runs[0].run_id, queued.run_id);
    let list = f
        .service
        .list(
            f.caller.clone(),
            serde_json::from_value(json!({"statuses":["running"]})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.total, 1);
    assert_eq!(list.threads[0].latest_run_id, Some(queued.run_id));
}

#[tokio::test]
async fn only_complete_direct_child_terminal_result_acknowledges_delivery() {
    let f = Fixture::new();
    let delegated = f
        .service
        .delegation
        .delegate_task(
            f.caller.clone(),
            serde_json::from_value(json!({
                "task":"Return a result","mode":"async","clientRequestId":"ack-child"
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    let id = &delegated.child_thread_id.0;
    f.provider_event(
        id,
        zeron_proto::AgentEvent::SessionStarted {
            instance_id: None,
            harness: zeron_proto::HarnessId::Mock,
            model: "mock-1".into(),
            tools: vec![],
            cwd: f.caller.workspace_root.to_string_lossy().into_owned(),
            session_id: "native-child".into(),
            assistant_message_id: "assistant".into(),
        },
    );
    f.provider_event(
        id,
        zeron_proto::AgentEvent::Done {
            status: zeron_proto::DoneStatus::Completed,
            result: Some("A😀 useful result".into()),
            error: None,
            session_id: None,
        },
    );
    f.service
        .kernel
        .reconcile_ancestors(&delegated.child_thread_id)
        .await
        .unwrap();
    let state = || {
        let p = f
            .service
            .kernel
            .store
            .thread(&"parent".into())
            .unwrap()
            .unwrap();
        crate::orchestration::task::records(&p, "subagent")
            .iter()
            .find(|t| t["id"] == delegated.task_id.0)
            .unwrap()["completionDelivery"]["state"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let read = |v| {
        f.service
            .read(f.caller.clone(), serde_json::from_value(v).unwrap())
    };
    let pending = state();
    assert!(matches!(pending.as_str(), "pending" | "claimed"));
    read(json!({"threadId":id,"limit":1})).await.unwrap();
    assert_eq!(state(), pending);
    read(json!({"threadId":id,"maxCharsPerItem":2}))
        .await
        .unwrap();
    assert_eq!(state(), pending);
    let passive = f
        .service
        .kernel
        .store
        .ui_thread_timeline(serde_json::from_value(json!({"threadId":id})).unwrap())
        .unwrap();
    let result_item = &passive.page.items.last().unwrap().item_id;
    assert_eq!(state(), pending);
    read(json!({"threadId":id,"itemId":result_item,"textOffset":1}))
        .await
        .unwrap();
    assert_eq!(state(), pending);
    f.service
        .wait(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":id,"timeoutMs":1})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(state(), pending);
    let full = read(json!({"threadId":id})).await.unwrap();
    let result_id = &full.result.items.last().unwrap().item_id.0;
    assert_eq!(state(), "acknowledged");
    assert!(
        f.service
            .kernel
            .store
            .events()
            .unwrap()
            .iter()
            .any(|event| event.command_id.as_ref().is_some_and(|id| id
                .0
                .starts_with("command:mcp:session%2Fparent%3A1:thread-read-acknowledge:")))
    );
    // A second round of thread messages is ordinary activity: it cannot
    // reopen or replace the immutable delegated task's result.
    f.send(json!({"threadId":id,"message":"follow up","clientRequestId":"later-child-run"}))
        .await;
    assert_eq!(state(), "acknowledged");
    let original = f
        .service
        .delegation
        .task_status(
            f.caller.clone(),
            serde_json::from_value(json!({"taskId":delegated.task_id})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(original.summary, Some("A😀 useful result".into()));
    assert!(original.has_pending_child_runs);
    assert!(!result_id.is_empty());
}

#[tokio::test]
async fn passive_ui_read_does_not_mutate_and_configuration_requires_live_owner() {
    let f = Fixture::new();
    f.item("parent", "assistant_message", 1, "hello");
    let before = f.service.kernel.store.events().unwrap().len();
    let ui = f
        .service
        .kernel
        .store
        .ui_thread_timeline(serde_json::from_value(json!({"threadId":"parent"})).unwrap())
        .unwrap();
    assert_eq!(ui.page.items.len(), 1);
    assert_eq!(before, f.service.kernel.store.events().unwrap().len());
    let config = f
        .service
        .configuration(f.caller.clone(), serde_json::from_value(json!({})).unwrap())
        .await
        .unwrap();
    assert_eq!(config.model_selection.model, "mock-1");
    let configured = f
        .service
        .configure(
            f.caller.clone(),
            serde_json::from_value(
                json!({"modelSelection":{"instanceId":"mock","model":"mock-2"}}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert!(configured.sequence > 0);
    assert_eq!(
        f.service
            .kernel
            .store
            .thread(&"parent".into())
            .unwrap()
            .unwrap()
            .thread
            .model_selection
            .model,
        "mock-2"
    );
    let mut stale = f.caller.clone();
    stale.provider_instance_id = ProviderInstanceId("other".into());
    let error = f
        .service
        .configure(
            stale,
            serde_json::from_value(
                json!({"modelSelection":{"instanceId":"mock","model":"mock-1"}}),
            )
            .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.message,
        "The calling provider no longer owns an active thread run."
    );
}

#[tokio::test]
async fn real_mcp_http_keeps_surrogate_escapes_and_capability_refusal_order() {
    let f = Fixture::new();
    f.item("parent", "assistant_message", 1, "A😀B");
    let registry = Arc::new(crate::HarnessRegistry::new());
    registry.register(Arc::new(zeron_harness::mock::MockHarness {
        script: vec![],
    }));
    let server = Arc::new(crate::mcp::McpServer::new(registry));
    server
        .toolkit
        .set_thread_service(Arc::new(f.service.clone()));
    let scope = crate::mcp::auth::InvocationScope {
        caller: f.caller.clone(),
        environment_id: "host".into(),
        selection: serde_json::from_value(json!({"instanceId":"mock","model":"mock-1"})).unwrap(),
        capabilities: ["orchestration".into()].into_iter().collect(),
        issued_at: 0,
        task_id: None,
    };
    let credential = server.credentials.issue(scope.clone()).unwrap();
    let endpoint = server.endpoint().await.unwrap();
    let reply = reqwest::Client::new()
        .post(endpoint)
        .header("authorization", &credential.authorization)
        .json(
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
                "name":"t3_thread_read","arguments":{"threadId":"parent","maxCharsPerItem":2}
            }}),
        )
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(reply.contains(r#""text":"\u0041\ud83d""#), "{reply}");
    assert!(reply.contains(r#""nextTextOffset":2"#));
    assert!(!reply.contains("noches.thread.utf16"));
    let mut denied = scope;
    denied.capabilities.clear();
    let credential = server.credentials.issue(denied).unwrap();
    let reply: Value = reqwest::Client::new()
        .post(endpoint)
        .header("authorization", &credential.authorization)
        .json(
            &json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                "name":"t3_thread_read","arguments":{"threadId":"missing"}
            }}),
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        reply["result"]["structuredContent"],
        json!({"_tag":"OrchestratorMcpFailure","code":"capability_denied",
        "message":"This MCP credential does not grant orchestration capabilities."})
    );
    assert_eq!(reply["result"]["isError"], false);
    server.shutdown();
}

#[tokio::test]
async fn fork_inherited_positions_synthetic_marker_and_visibility_match_t3() {
    let f = Fixture::new();
    f.item("parent", "user_message", 1, "Parent prompt");
    f.item("parent", "assistant_message", 2, "Parent result");
    f.clone_thread(
        "fork",
        "project",
        json!({"parentThreadId":"parent","relationshipToParent":"fork","rootThreadId":"parent"}),
    );
    let mut thread = json!(
        f.service
            .kernel
            .store
            .thread(&"fork".into())
            .unwrap()
            .unwrap()
            .thread
    );
    thread["forkedFrom"] = json!({"type":"run","threadId":"parent","runId":f.caller.run_id});
    f.emit("fork", "thread.metadata-updated", thread);
    f.item("fork", "assistant_message", 1, "Fork result");
    let read = f
        .service
        .read(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"fork","view":"activity"})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(read.result.items.len(), 4);
    assert_eq!(
        read.result.items[0].visibility,
        OrchestratorMcpThreadTimelineItemVisibility::Inherited
    );
    assert_eq!(
        read.result.items[2].visibility,
        OrchestratorMcpThreadTimelineItemVisibility::Synthetic
    );
    assert_eq!(read.result.items[2].item_id.0, "turn-item:fork:fork");
    assert_eq!(
        read.result.items[3].visibility,
        OrchestratorMcpThreadTimelineItemVisibility::Local
    );
    let messages = f
        .service
        .read(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"fork"})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(messages.result.items.len(), 3);
    assert_eq!(messages.result.items[2].position, 3);
}

#[tokio::test]
async fn late_steering_becomes_one_receipted_followup_with_the_same_message() {
    let f = Fixture::new();
    f.running(false);
    let steered = f.send(json!({"threadId":"parent","message":"accepted update","mode":"steer","clientRequestId":"late"})).await;
    let effect = f.service.kernel.store.effects().unwrap().into_iter().find(|e| matches!(&e.request,
        crate::orchestration::effects::EffectRequest::ProviderTurnSteer {message_id,..} if *message_id == steered.message_id)).unwrap();
    f.provider_event(
        "parent",
        zeron_proto::AgentEvent::Done {
            status: zeron_proto::DoneStatus::Completed,
            result: Some("Original result".into()),
            error: None,
            session_id: None,
        },
    );
    super::runner::late_steer(&f.service.kernel, &effect, &steered.message_id)
        .await
        .unwrap();
    super::runner::late_steer(&f.service.kernel, &effect, &steered.message_id)
        .await
        .unwrap();
    let p = f
        .service
        .kernel
        .store
        .thread(&"parent".into())
        .unwrap()
        .unwrap();
    assert_eq!(p.runs.len(), 2);
    assert_eq!(p.runs[1].user_message_id, steered.message_id);
    assert_eq!(
        crate::orchestration::task::records(&p, "message")
            .iter()
            .filter(|m| m["id"] == steered.message_id.0)
            .count(),
        1
    );
}

#[tokio::test]
async fn empty_wait_and_terminal_explicit_run_do_not_wait_for_a_newer_run() {
    let f = Fixture::new();
    f.clone_thread(
        "idle",
        "project",
        json!({"parentThreadId":null,"relationshipToParent":null,"rootThreadId":"idle"}),
    );
    let idle = f
        .service
        .wait(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"idle","timeoutMs":3600000})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(idle.run_id, None);
    assert_eq!(idle.status, OrchestratorMcpThreadStatus::Idle);
    assert!(!idle.timed_out);
    f.provider_event(
        "parent",
        zeron_proto::AgentEvent::Done {
            status: zeron_proto::DoneStatus::Completed,
            result: Some("Done".into()),
            error: None,
            session_id: None,
        },
    );
    f.send(json!({"threadId":"parent","message":"later"})).await;
    let explicit = f
        .service
        .wait(
            f.caller.clone(),
            serde_json::from_value(
                json!({"threadId":"parent","runId":f.caller.run_id,"timeoutMs":3600000}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(explicit.run_id, Some(f.caller.run_id));
    assert_eq!(explicit.status, OrchestratorMcpThreadStatus::Completed);
    assert!(!explicit.timed_out);
}

#[tokio::test]
async fn missing_foreign_and_deleted_threads_keep_t3_error_families() {
    let f = Fixture::new();
    let error = f
        .service
        .read(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"missing"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::OrchestrationError);
    assert_eq!(
        error.message,
        "Unable to load thread missing in project project."
    );
    let error = f
        .service
        .configuration(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"missing"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::OrchestrationError);
    assert_eq!(error.message, "The operation could not be completed.");
    f.clone_thread(
        "foreign",
        "other",
        json!({"parentThreadId":null,"relationshipToParent":null,"rootThreadId":"foreign"}),
    );
    let error = f
        .service
        .read(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"foreign"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ThreadNotFound);
    assert_eq!(
        error.message,
        "Thread foreign was not found in project project."
    );
    let parent = f
        .service
        .kernel
        .store
        .thread(&f.caller.thread_id)
        .unwrap()
        .unwrap();
    let mut thread = json!(parent.thread);
    thread["deletedAt"] = json!("2026-10-03T00:00:00.000Z");
    f.emit("parent", "thread.metadata-updated", thread);
    let error = f
        .service
        .read(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ThreadNotFound);
    assert_eq!(error.message, "Thread parent is no longer available.");
    let before = f.service.kernel.store.events().unwrap().len();
    let error = f
        .service
        .send(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent","message":"hello"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ThreadNotFound);
    assert_eq!(
        error.message,
        "Thread parent was not found in project project."
    );
    assert_eq!(before, f.service.kernel.store.events().unwrap().len());
}

#[tokio::test]
async fn batch_overrides_partial_acceptance_and_retry_preconditions_match_t3() {
    let f = Fixture::new();
    let input = json!({"clientRequestId":"partial","threads":[
        {"prompt":"First","target":{"providerInstanceId":"mock","model":"mock-2","options":{"thinking":true}},
        "runtimeMode":"auto","interactionMode":"plan"},
        {"title":"Refused","target":{"model":"missing"}}
    ]});
    let error = f
        .service
        .create(
            f.caller.clone(),
            serde_json::from_value(input.clone()).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ModelUnavailable);
    let accepted: ThreadId = "thread:mcp:session%2Fparent%3A1:partial:0".into();
    let refused: ThreadId = "thread:mcp:session%2Fparent%3A1:partial:1".into();
    let first = f.service.kernel.store.thread(&accepted).unwrap().unwrap();
    assert_eq!(first.thread.model_selection.model, "mock-2");
    assert_eq!(first.thread.runtime_mode, RuntimeMode::Auto);
    assert_eq!(first.thread.interaction_mode, InteractionMode::Plan);
    assert_eq!(
        first.thread.worktree_path,
        Some(f.caller.workspace_root.to_string_lossy().into_owned())
    );
    assert_eq!(
        json!(first.thread.model_selection.options),
        json!([{"id":"thinking","value":true}])
    );
    assert_eq!(first.runs.len(), 1);
    assert!(f.service.kernel.store.thread(&refused).unwrap().is_none());
    let error = f
        .service
        .create(f.caller.clone(), serde_json::from_value(input).unwrap())
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ModelUnavailable);
    assert_eq!(
        f.service
            .kernel
            .store
            .thread(&accepted)
            .unwrap()
            .unwrap()
            .runs
            .len(),
        1
    );
    let parent = f
        .service
        .kernel
        .store
        .thread(&f.caller.thread_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        crate::orchestration::task::records(&parent, "turn-item")
            .iter()
            .filter(|i| i["type"] == "thread_created")
            .count(),
        1
    );
    // Even a receipted request must pass the live-parent gate again.
    let mut run = json!(parent.runs[0]);
    run["status"] = json!("completed");
    f.emit("parent", "run.updated", run);
    let error = f
        .service
        .create(
            f.caller.clone(),
            serde_json::from_value(
                json!({"clientRequestId":"partial","threads":[{"title":"Retry"}]}),
            )
            .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ParentNotActive);
}

#[tokio::test]
async fn running_interrupt_targets_the_provider_turn_and_refuses_unsupported_capability() {
    let f = Fixture::new();
    f.running(false);
    let interrupted = f
        .service
        .interrupt(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent","clientRequestId":"interrupt"}))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        interrupted.status,
        OrchestratorMcpThreadInterruptResultStatus::InterruptRequested
    );
    let p = f
        .service
        .kernel
        .store
        .thread(&f.caller.thread_id)
        .unwrap()
        .unwrap();
    let turn = &crate::orchestration::task::records(&p, "provider-turn")[0];
    assert!(f.service.kernel.store.effects().unwrap().iter().any(|e| matches!(&e.request,crate::orchestration::effects::EffectRequest::ProviderTurnInterrupt { provider_turn_id,.. } if provider_turn_id.0 == turn["id"])));
    assert_eq!(p.runs[0].status, OrchestrationV2RunStatus::Running);
    let f = Fixture::new();
    f.running(false);
    let p = f
        .service
        .kernel
        .store
        .thread(&f.caller.thread_id)
        .unwrap()
        .unwrap();
    let mut session = crate::orchestration::task::records(&p, "provider-session")[0].clone();
    session["capabilities"]["turns"]["supportsInterrupt"] = json!(false);
    f.emit("parent", "provider-session.updated", session);
    let before = f.service.kernel.store.events().unwrap().len();
    let error = f
        .service
        .interrupt(
            f.caller.clone(),
            serde_json::from_value(json!({"threadId":"parent","clientRequestId":"unsupported"}))
                .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::OrchestrationError);
    assert_eq!(
        error.message,
        "Unable to interrupt thread parent: Failed to dispatch orchestration command run.interrupt (command:mcp:session%2Fparent%3A1:thread-interrupt:unsupported)."
    );
    assert_eq!(before, f.service.kernel.store.events().unwrap().len());
}

#[tokio::test]
async fn created_thread_record_stays_bound_to_the_accepting_parent_run_after_it_finishes() {
    let f = Fixture::new();
    f.running(false);
    f.clone_thread(
        "created",
        "project",
        json!({"parentThreadId":null,"relationshipToParent":null,"rootThreadId":"created"}),
    );
    let parent = f
        .service
        .kernel
        .store
        .thread(&f.caller.thread_id)
        .unwrap()
        .unwrap();
    let node = parent.runs[0].root_node_id.clone().unwrap();
    f.provider_event(
        "parent",
        zeron_proto::AgentEvent::Done {
            status: zeron_proto::DoneStatus::Completed,
            result: Some("Done".into()),
            error: None,
            session_id: None,
        },
    );
    f.send(json!({"threadId":"parent","message":"a newer conversation run"}))
        .await;
    f.service
        .kernel
        .task_command(
            &f.caller.thread_id,
            "record-after-completion".into(),
            crate::orchestration::task::TaskOperation::CreationRecord {
                parent_run_id: f.caller.run_id.clone(),
                parent_node_id: node.clone(),
                target_thread_id: "created".into(),
                target_run_id: None,
            },
        )
        .await
        .unwrap();
    let p = f
        .service
        .kernel
        .store
        .thread(&f.caller.thread_id)
        .unwrap()
        .unwrap();
    let row = crate::orchestration::task::records(&p, "turn-item")
        .iter()
        .find(|i| i["type"] == "thread_created")
        .unwrap();
    assert_eq!(row["runId"], f.caller.run_id.0);
    assert_eq!(row["nodeId"], node.0);
    assert_eq!(
        row["providerTurnId"],
        crate::orchestration::task::records(&parent, "provider-turn")[0]["id"]
    );
}
