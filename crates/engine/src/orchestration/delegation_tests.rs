use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use futures::{StreamExt, stream::BoxStream};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use zeron_harness::{Harness, HarnessError, RunControls, mock::MockHarness};
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::*;
use zeron_proto::provider_instance::*;
use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, InteractionMode, Model, ReasoningLevel, RunRequest,
    RuntimeMode, SteeringMode,
};
use zeron_sync::DocsStore;

use super::command::{Command, Operation};
use super::effects::{EffectStatus, EffectWorker};
use super::mailbox::{DeliveryAction, DeliveryCommand, cohort};
use super::runner::{RunnerBridge, RunnerInstances, RunnerMcp, RunnerProvider};
use super::service::{CallerScope, OrchestratorService, ToolError};
use super::task::*;
use super::{Kernel, ReceiptStatus, WriteBoundary};

const NOW: i64 = 1_800_000_000_000;

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

struct Catalog(Mutex<Vec<OrchestratorMcpProviderCapability>>);

fn provider(instance: &str, driver: &str, model: &str) -> OrchestratorMcpProviderCapability {
    serde_json::from_value(json!({
        "providerInstanceId":instance,"driverKind":driver,"displayName":instance,
        "models":[{"id":model,"label":model}],"canRunChildTask":true,"canRunCrossProviderChildTask":true,"constraints":[]
    })).unwrap()
}

#[async_trait]
impl DelegationCatalog for Catalog {
    async fn providers(
        &self,
    ) -> std::result::Result<Vec<OrchestratorMcpProviderCapability>, ToolError> {
        Ok(self.0.lock().unwrap().clone())
    }
}

struct Fixture {
    dir: tempfile::TempDir,
    service: DelegationService,
    caller: CallerScope,
    catalog: Arc<Catalog>,
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
            kernel.store.dispatch(&command, NOW).unwrap().status,
            ReceiptStatus::Accepted
        );
        let parent = kernel
            .store
            .thread(&ThreadId("parent".into()))
            .unwrap()
            .unwrap();
        let seed =
            execution_seed(&parent.thread, 1, "parent-input", "starting", "mock", NOW).unwrap();
        assert_eq!(
            kernel
                .store
                .dispatch(
                    &Command {
                        id: CommandId("seed-parent".into()),
                        thread_id: parent.thread.id.clone(),
                        operation: Operation::CreateExecution(Box::new(seed.clone())),
                    },
                    NOW
                )
                .unwrap()
                .status,
            ReceiptStatus::Accepted
        );
        let catalog = Arc::new(Catalog(Mutex::new(vec![
            provider("mock", "mock", "mock-1"),
            provider("custom-claude", "claude-code", "claude-model"),
        ])));
        let service = DelegationService {
            kernel,
            targets: Arc::new(CatalogTargets(catalog.clone())),
        };
        let caller = CallerScope {
            thread_id: parent.thread.id,
            run_id: seed.run.id,
            session_id: "session/parent:1".into(),
            project_id: ProjectId("project".into()),
            workspace_root: dir.path().into(),
            runtime_mode: RuntimeMode::FullAccess,
            interaction_mode: InteractionMode::Default,
            provider_instance_id: ProviderInstanceId("mock".into()),
        };
        Self {
            dir,
            service,
            caller,
            catalog,
        }
    }

    fn kernel(&self) -> &Kernel {
        &self.service.kernel
    }
    fn parent(&self) -> super::projection::ThreadProjection {
        self.kernel()
            .store
            .thread(&self.caller.thread_id)
            .unwrap()
            .unwrap()
    }
    fn child(&self, task: &TaskStatusResult) -> super::projection::ThreadProjection {
        self.kernel()
            .store
            .thread(&task.child_thread_id)
            .unwrap()
            .unwrap()
    }
    async fn delegate(&self, key: &str) -> TaskStatusResult {
        self.service
            .delegate_task(
                self.caller.clone(),
                serde_json::from_value(json!({
                    "task":"Return a useful result.","clientRequestId":key
                }))
                .unwrap(),
            )
            .await
            .unwrap()
    }
    async fn op(&self, thread: &ThreadId, op: TaskOperation) {
        self.kernel()
            .task_command(
                thread,
                CommandId(format!("test:{}", uuid::Uuid::new_v4())),
                op,
            )
            .await
            .unwrap();
    }
    async fn event(&self, thread: &ThreadId, run: &OrchestrationV2Run, event: AgentEvent) {
        let mut capabilities = sample("OrchestrationV2ProviderCapabilities");
        capabilities["turns"]["supportsActiveSteering"] = json!(true);
        self.op(
            thread,
            TaskOperation::RunnerEvent {
                run_id: run.id.clone(),
                attempt_id: run.active_attempt_id.clone().unwrap(),
                event,
                capabilities: Some(Box::new(serde_json::from_value(capabilities).unwrap())),
            },
        )
        .await;
    }
    fn seed_event(&self, thread: &ThreadId, kind: &str, payload: Value) {
        self.kernel()
            .store
            .write(|tx| {
                self.kernel().store.append_event(
                    tx,
                    None,
                    super::event::make(
                        EventId(format!("fixture:{}", uuid::Uuid::new_v4())),
                        thread,
                        kind,
                        &payload,
                        NOW,
                    )?,
                )?;
                Ok(())
            })
            .unwrap();
    }
    async fn complete(&self, task: &TaskStatusResult, result: &str) {
        let child = self.child(task);
        self.event(
            &child.thread.id,
            &child.runs[0],
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: Some(result.into()),
                error: None,
                session_id: None,
            },
        )
        .await;
        self.op(
            &self.caller.thread_id,
            TaskOperation::Finalize {
                child_thread_id: child.thread.id,
            },
        )
        .await;
    }
    async fn status(&self, task: &TaskStatusResult) -> TaskStatusResult {
        self.service
            .task_status(
                self.caller.clone(),
                TaskStatusInput {
                    task_id: task.task_id.0.clone(),
                },
            )
            .await
            .unwrap()
    }
    fn delivery(&self, action: DeliveryAction) -> DeliveryCommand {
        let parent = self.parent();
        let run = parent
            .runs
            .iter()
            .find(|run| run.id == self.caller.run_id)
            .unwrap();
        let delivery = cohort(run)["delivery"].clone();
        DeliveryCommand {
            parent_run_id: run.id.clone(),
            generation: delivery["generation"].as_i64().unwrap(),
            message_id: MessageId(delivery["messageId"].as_str().unwrap().into()),
            action,
        }
    }
}

#[tokio::test]
async fn native_app_owned_and_top_level_are_distinct_and_spawn_atomic() {
    let fixture = Fixture::new();
    let task = fixture.delegate("one").await;
    let parent = fixture.parent();
    let child = fixture.child(&task);
    let node = parent
        .nodes
        .iter()
        .find(|node| node.id == task.task_id)
        .unwrap();
    assert!(!node.counts_for_run);
    assert_eq!(records(&parent, "subagent")[0]["origin"], "app_owned");
    assert_eq!(
        records(&child, "context-transfer")[0]["type"],
        "subagent_spawn"
    );
    assert_eq!(
        child.thread.lineage.parent_thread_id,
        Some(parent.thread.id.clone())
    );
    assert_eq!(records(&child, "message").len(), 1); // no copied parent history
    let batches = fixture.kernel().store.pending_publications().unwrap();
    assert_eq!(batches.last().unwrap().documents.len(), 3); // parent + child + registry
    assert!(
        batches
            .last()
            .unwrap()
            .documents
            .iter()
            .any(|doc| doc.doc_id.ends_with(&task.child_thread_id.0))
    );
    fixture
        .event(
            &fixture.caller.thread_id,
            &parent.runs[0],
            AgentEvent::Subagent {
                parent_tool_use_id: "vendor-task".into(),
                event: Box::new(AgentEvent::TextDelta {
                    text: "observed".into(),
                }),
            },
        )
        .await;
    let native = fixture.parent();
    assert_eq!(records(&native, "subagent")[1]["origin"], "provider_native");
    assert!(records(&native, "subagent")[1]["childThreadId"].is_null());
    assert_eq!(
        fixture
            .kernel()
            .store
            .effects()
            .unwrap()
            .iter()
            .filter(|effect| effect.thread_id != parent.thread.id)
            .count(),
        1
    );
    let top = fixture
        .service
        .create_threads(
            fixture.caller.clone(),
            serde_json::from_value(json!({
                "threads":[{"title":"Independent"}],"clientRequestId":"top"
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    let top = fixture
        .kernel()
        .store
        .thread(&top.threads[0].thread_id)
        .unwrap()
        .unwrap();
    assert!(top.thread.lineage.parent_thread_id.is_none());
    assert!(records(&top, "context-transfer").is_empty());
    assert_eq!(records(&fixture.parent(), "subagent").len(), 2);
    let before = fixture.kernel().store.events().unwrap();
    fixture.kernel().store.rebuild().unwrap();
    assert_eq!(
        fixture
            .kernel()
            .store
            .thread(&task.child_thread_id)
            .unwrap()
            .unwrap(),
        child
    );
    assert_eq!(fixture.kernel().store.events().unwrap(), before);
}

#[tokio::test]
async fn rollback_every_spawn_boundary_then_stable_receipt_replay() {
    for boundary in [
        WriteBoundary::ReceiptReserved,
        WriteBoundary::EventAppended,
        WriteBoundary::ProjectionRow,
        WriteBoundary::EffectEnqueued,
        WriteBoundary::PublicationEnqueued,
        WriteBoundary::ReceiptFinalized,
        WriteBoundary::BeforeCommit,
    ] {
        let fixture = Fixture::new();
        let before = fixture.kernel().store.events().unwrap().len();
        fixture.kernel().store.inject_failure(boundary, 1);
        assert!(
            fixture
                .service
                .delegate_task(
                    fixture.caller.clone(),
                    serde_json::from_value(json!({
                        "task":"task","clientRequestId":"retry"
                    }))
                    .unwrap()
                )
                .await
                .is_err()
        );
        assert_eq!(
            fixture.kernel().store.events().unwrap().len(),
            before,
            "{boundary:?}"
        );
        let first = fixture.delegate("retry").await;
        let second = fixture.service.delegate_task(fixture.caller.clone(),serde_json::from_value(json!({
            "task":"changed payload does not replace accepted work","clientRequestId":"retry"
        })).unwrap()).await.unwrap();
        assert_eq!(first.task_id, second.task_id);
        assert_eq!(records(&fixture.parent(), "subagent").len(), 1);
    }
}

#[tokio::test]
async fn role_inheritance_catalog_preconditions_and_session_scoped_keys() {
    let fixture = Fixture::new();
    let task = fixture
        .service
        .delegate_task(
            fixture.caller.clone(),
            serde_json::from_value(json!({
                "task":"Review code","role":"review","clientRequestId":"retry"
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        records(&fixture.child(&task), "message")[0]["text"],
        "Act as the review sub-agent for this task.\n\nReview code"
    );
    assert_eq!(
        fixture.child(&task).thread.worktree_path.as_ref(),
        Some(&fixture.caller.workspace_root.to_string_lossy().into_owned())
    );
    assert_eq!(task.provider_instance_id.0, "mock");
    fixture.catalog.0.lock().unwrap()[0]
        .constraints
        .push("disabled".into());
    let error = fixture
        .service
        .delegate_task(
            fixture.caller.clone(),
            serde_json::from_value(json!({
                "task":"same retry","clientRequestId":"retry"
            }))
            .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ProviderUnavailable);
    fixture.catalog.0.lock().unwrap()[0].constraints.clear();
    let mut caller = fixture.caller.clone();
    caller.session_id = "other-session".into();
    let other = fixture
        .service
        .delegate_task(
            caller,
            serde_json::from_value(json!({"task":"Review code","clientRequestId":"retry"}))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(other.task_id, task.task_id);
    let parent = fixture.parent();
    fixture
        .event(
            &fixture.caller.thread_id,
            &parent.runs[0],
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: Some("done".into()),
                error: None,
                session_id: None,
            },
        )
        .await;
    let error = fixture
        .service
        .delegate_task(
            fixture.caller.clone(),
            serde_json::from_value(json!({"task":"retry","clientRequestId":"retry"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ParentNotActive);
}

#[tokio::test]
async fn permission_and_interaction_escalation_refuse_before_creation() {
    let fixture = Fixture::new();
    let command = Command::wire(serde_json::from_value(json!({
        "type":"thread.runtime-mode.set","commandId":"narrow-runtime","threadId":"parent","runtimeMode":"approval-required"
    })).unwrap()).unwrap();
    assert_eq!(
        fixture
            .kernel()
            .store
            .dispatch(&command, NOW)
            .unwrap()
            .status,
        ReceiptStatus::Accepted
    );
    let error = fixture
        .service
        .delegate_task(
            fixture.caller.clone(),
            serde_json::from_value(json!({"task":"bad","runtimeMode":"full-access"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.code,
        OrchestratorMcpFailureCode::RuntimeModeEscalationDenied
    );
    assert_eq!(
        error.message,
        "Child runtime mode full-access is broader than parent mode approval-required."
    );
    let command = Command::wire(serde_json::from_value(json!({
        "type":"thread.interaction-mode.set","commandId":"narrow-plan","threadId":"parent","interactionMode":"plan"
    })).unwrap()).unwrap();
    fixture.kernel().store.dispatch(&command, NOW).unwrap();
    let error = fixture
        .service
        .delegate_task(
            fixture.caller.clone(),
            serde_json::from_value(json!({"task":"bad","interactionMode":"default"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.code,
        OrchestratorMcpFailureCode::InteractionModeEscalationDenied
    );
    assert!(records(&fixture.parent(), "subagent").is_empty());
}

#[tokio::test]
async fn batch_keeps_partially_accepted_threads_and_retry_reconciles() {
    let fixture = Fixture::new();
    let input: CreateThreadsInput = serde_json::from_value(json!({
        "threads":[{"title":"First"},{"target":{"providerInstanceId":"missing"}}],"clientRequestId":"batch"
    })).unwrap();
    assert_eq!(
        fixture
            .service
            .create_threads(fixture.caller.clone(), input.clone())
            .await
            .unwrap_err()
            .code,
        OrchestratorMcpFailureCode::ProviderUnavailable
    );
    let id = ThreadId(format!(
        "thread:mcp:{}:batch:0",
        super::event::encode_component(&fixture.caller.session_id)
    ));
    assert!(fixture.kernel().store.thread(&id).unwrap().is_some());
    assert_eq!(records(&fixture.parent(), "turn-item").len(), 1);
    fixture
        .catalog
        .0
        .lock()
        .unwrap()
        .push(provider("missing", "mock", "new-model"));
    let result = fixture
        .service
        .create_threads(fixture.caller.clone(), input.clone())
        .await
        .unwrap();
    assert_eq!(result.threads.len(), 2);
    assert_eq!(
        fixture
            .service
            .create_threads(fixture.caller.clone(), input)
            .await
            .unwrap(),
        result
    );
    assert_eq!(records(&fixture.parent(), "turn-item").len(), 2);
}

#[tokio::test]
async fn original_result_is_immutable_across_later_runs_and_terminal_cancel() {
    let fixture = Fixture::new();
    let task = fixture.delegate("original").await;
    fixture.complete(&task, "original result").await;
    let first = fixture.status(&task).await;
    assert_eq!(first.summary.as_deref(), Some("original result"));
    let child = fixture.child(&task);
    let seed = execution_seed(&child.thread, 2, "later-input", "starting", "mock", NOW).unwrap();
    assert_eq!(
        fixture
            .kernel()
            .store
            .dispatch(
                &Command {
                    id: CommandId("later-run".into()),
                    thread_id: child.thread.id.clone(),
                    operation: Operation::CreateExecution(Box::new(seed.clone())),
                },
                NOW
            )
            .unwrap()
            .status,
        ReceiptStatus::Accepted
    );
    let later = fixture.status(&task).await;
    assert!(later.has_pending_child_runs);
    assert_eq!(
        later.work_state,
        OrchestratorMcpDelegateTaskResultWorkState::ResultAvailable
    );
    let cancel = fixture
        .service
        .task_cancel(
            fixture.caller.clone(),
            serde_json::from_value(json!({"taskId":task.task_id})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(serde_json::to_value(cancel).unwrap()["status"], "completed");
    assert_eq!(
        fixture.child(&task).runs.last().unwrap().status,
        OrchestrationV2RunStatus::Starting
    );
    fixture
        .event(
            &child.thread.id,
            &seed.run,
            AgentEvent::SessionStarted {
                instance_id: None,
                harness: HarnessId::Mock,
                model: "mock-1".into(),
                tools: vec![],
                cwd: fixture.dir.path().display().to_string(),
                session_id: "later-session".into(),
                assistant_message_id: "later-assistant".into(),
            },
        )
        .await;
    fixture
        .event(
            &child.thread.id,
            &seed.run,
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: Some("later result".into()),
                error: None,
                session_id: None,
            },
        )
        .await;
    fixture.kernel().reconcile_delegation().await.unwrap();
    let later = fixture.status(&task).await;
    assert_eq!(later.summary.as_deref(), Some("original result"));
    assert_eq!(
        later.latest_terminal_summary.as_deref(),
        Some("later result")
    );
    assert_ne!(later.latest_terminal_run_id, later.child_run_id);
    assert_eq!(records(&fixture.parent(), "context-transfer").len(), 1);
}

#[tokio::test]
async fn wait_timeout_upgrades_wake_without_cancellation_and_wait_does_not_ack() {
    let fixture = Fixture::new();
    let task = fixture
        .service
        .delegate_task(
            fixture.caller.clone(),
            serde_json::from_value(json!({
                "task":"slow task","mode":"wait","timeoutMs":1,"clientRequestId":"slow"
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    assert!(task.wait_timed_out);
    assert_eq!(
        fixture.child(&task).runs[0].status,
        OrchestrationV2RunStatus::Starting
    );
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionWake"],
        "always"
    );
    assert!(
        !fixture
            .service
            .wait_child_run(&fixture.caller, &task.task_id, Duration::from_millis(1))
            .await
            .unwrap()
    );
    assert!(!fixture.status(&task).await.wait_timed_out);
    fixture.complete(&task, "done").await;
    assert!(
        fixture
            .service
            .wait_child_run(&fixture.caller, &task.task_id, Duration::from_millis(1))
            .await
            .unwrap()
    );
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "claimed"
    );
    fixture
        .service
        .acknowledge_child_read(&fixture.caller, &task.child_thread_id, 1, false, true)
        .await
        .unwrap();
    fixture
        .service
        .acknowledge_child_read(&fixture.caller, &task.child_thread_id, 0, true, true)
        .await
        .unwrap();
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "claimed"
    );
    fixture
        .service
        .acknowledge_child_read(&fixture.caller, &task.child_thread_id, 0, false, true)
        .await
        .unwrap();
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "acknowledged"
    );
}

#[tokio::test]
async fn cancel_active_is_request_then_disposal_not_terminal_confirmation() {
    let fixture = Fixture::new();
    let task = fixture.delegate("active").await;
    let result = fixture
        .service
        .task_cancel(
            fixture.caller.clone(),
            serde_json::from_value(json!({"taskId":task.task_id,"clientRequestId":"stop"}))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["status"],
        "cancel_requested"
    );
    assert_eq!(
        fixture.child(&task).runs[0].status,
        OrchestrationV2RunStatus::Starting
    );
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "disposed"
    );
    let effects = fixture.kernel().store.effects().unwrap();
    assert!(
        effects
            .iter()
            .any(|effect| effect.thread_id == task.child_thread_id
                && effect.status == EffectStatus::Cancelled)
    );
    assert!(effects.iter().any(|effect| matches!(
        effect.request,
        super::effects::EffectRequest::ManagedRunInterrupt { .. }
    )));
}

#[tokio::test]
async fn user_stop_cancels_under_parent_authority_without_acknowledging_a_result() {
    let fixture = Fixture::new();
    let task = fixture.delegate("user stop").await;
    let result = fixture
        .service
        .cancel_for_user(&fixture.caller.thread_id, task.task_id.to_string())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["status"],
        "cancel_requested"
    );
    assert!(
        fixture
            .kernel()
            .store
            .effects()
            .unwrap()
            .iter()
            .any(|effect| matches!(
                effect.request,
                super::effects::EffectRequest::ManagedRunInterrupt { .. }
            ))
    );
    // A stranger's task id is refused, never cancelled.
    let refused = fixture
        .service
        .cancel_for_user(&fixture.caller.thread_id, "node:missing".into())
        .await
        .unwrap_err();
    assert_eq!(refused.code, OrchestratorMcpFailureCode::TaskNotFound);
    // And an unknown parent chat is a clean error.
    assert!(
        fixture
            .service
            .cancel_for_user(&"nope".into(), task.task_id.to_string())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn completion_coalescing_arrivals_during_delivery_and_stale_generation() {
    let fixture = Fixture::new();
    let a = fixture.delegate("a").await;
    let b = fixture.delegate("b").await;
    let c = fixture.delegate("c").await;
    fixture.complete(&a, "A").await;
    fixture.complete(&b, "B").await;
    let delivery = fixture.delivery(DeliveryAction::Queue);
    assert_eq!(
        cohort(&fixture.parent().runs[0])["delivery"]["taskIds"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delivery(delivery.clone()),
        )
        .await;
    let queued = fixture.parent().runs.last().unwrap().clone();
    assert_eq!(queued.status, OrchestrationV2RunStatus::Queued);
    // Queued siblings still join; delivery becomes active only at actual start.
    let parent = fixture.parent();
    fixture
        .event(
            &fixture.caller.thread_id,
            &parent.runs[0],
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: Some("parent done".into()),
                error: None,
                session_id: None,
            },
        )
        .await;
    fixture
        .op(&fixture.caller.thread_id, TaskOperation::DrainQueue)
        .await;
    fixture.complete(&c, "C").await;
    assert_eq!(
        records(&fixture.parent(), "subagent")
            .iter()
            .find(|task| task["id"] == c.task_id.0)
            .unwrap()["completionDelivery"]["state"],
        "pending"
    );
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delivery(DeliveryCommand {
                action: DeliveryAction::Completed { cancelled: false },
                ..delivery.clone()
            }),
        )
        .await;
    let successor = cohort(&fixture.parent().runs[0]);
    assert_eq!(successor["delivery"]["generation"], 2);
    assert_eq!(successor["delivery"]["taskIds"], json!([c.task_id]));
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delivery(DeliveryCommand {
                action: DeliveryAction::Accepted,
                ..delivery
            }),
        )
        .await;
    assert_eq!(cohort(&fixture.parent().runs[0]), successor);
    assert_eq!(
        records(&fixture.parent(), "subagent")
            .iter()
            .find(|task| task["id"] == a.task_id.0)
            .unwrap()["completionDelivery"]["state"],
        "delivered"
    );
    fixture.status(&a).await;
    assert_eq!(
        records(&fixture.parent(), "subagent")
            .iter()
            .find(|task| task["id"] == a.task_id.0)
            .unwrap()["completionDelivery"]["state"],
        "acknowledged"
    );
}

#[tokio::test]
async fn native_background_and_nested_work_delay_result() {
    let fixture = Fixture::new();
    let task = fixture.delegate("outer").await;
    let child = fixture.child(&task);
    fixture
        .event(
            &child.thread.id,
            &child.runs[0],
            AgentEvent::Subagent {
                parent_tool_use_id: "native".into(),
                event: Box::new(AgentEvent::TextDelta {
                    text: "background".into(),
                }),
            },
        )
        .await;
    fixture.complete(&task, "root done").await;
    assert_eq!(
        fixture.status(&task).await.work_state,
        OrchestratorMcpDelegateTaskResultWorkState::WaitingForChildren
    );
    fixture
        .event(
            &child.thread.id,
            &child.runs[0],
            AgentEvent::Subagent {
                parent_tool_use_id: "native".into(),
                event: Box::new(AgentEvent::Done {
                    status: DoneStatus::Completed,
                    result: Some("native done".into()),
                    error: None,
                    session_id: None,
                }),
            },
        )
        .await;
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Finalize {
                child_thread_id: child.thread.id,
            },
        )
        .await;
    assert_eq!(
        fixture.status(&task).await.summary.as_deref(),
        Some("root done")
    );
}

#[tokio::test]
async fn nested_app_owned_task_waits_for_child_completion_consumption() {
    let fixture = Fixture::new();
    let outer = fixture.delegate("outer").await;
    let child = fixture.child(&outer);
    let nested_scope = CallerScope {
        thread_id: child.thread.id.clone(),
        run_id: child.runs[0].id.clone(),
        session_id: "nested-session".into(),
        project_id: child.thread.project_id.clone(),
        workspace_root: fixture.dir.path().into(),
        runtime_mode: child.thread.runtime_mode,
        interaction_mode: child.thread.interaction_mode,
        provider_instance_id: child.thread.provider_instance_id.clone(),
    };
    let nested = fixture
        .service
        .delegate_task(
            nested_scope.clone(),
            serde_json::from_value(json!({
                "task":"Nested work","clientRequestId":"nested"
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    fixture.complete(&outer, "outer turn result").await;
    assert_eq!(
        fixture.status(&outer).await.work_state,
        OrchestratorMcpDelegateTaskResultWorkState::WaitingForChildren
    );
    let grandchild = fixture.child(&nested);
    fixture
        .event(
            &grandchild.thread.id,
            &grandchild.runs[0],
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: Some("nested result".into()),
                error: None,
                session_id: None,
            },
        )
        .await;
    fixture
        .op(
            &child.thread.id,
            TaskOperation::Finalize {
                child_thread_id: grandchild.thread.id,
            },
        )
        .await;
    assert_eq!(
        fixture.status(&outer).await.work_state,
        OrchestratorMcpDelegateTaskResultWorkState::WaitingForChildren
    );
    fixture
        .service
        .task_status(
            nested_scope,
            TaskStatusInput {
                task_id: nested.task_id.0,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        fixture.status(&outer).await.summary.as_deref(),
        Some("outer turn result")
    );
}

#[tokio::test]
async fn catalog_model_options_and_driver_only_healthy_fallback() {
    let fixture = Fixture::new();
    let targets = CatalogTargets(fixture.catalog.clone());
    let parent = fixture.parent();
    fixture
        .catalog
        .0
        .lock()
        .unwrap()
        .push(provider("mock-custom", "mock", "custom-model"));
    fixture.catalog.0.lock().unwrap()[0]
        .constraints
        .push("disabled".into());
    let target = targets
        .resolve(
            &parent.thread,
            Some(&serde_json::from_value(json!({"driverKind":"mock"})).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(target.selection.instance_id.0, "mock-custom");
    assert_eq!(target.selection.model, "custom-model");
    assert!(target.selection.options.is_absent());
    let error = targets
        .resolve(
            &parent.thread,
            Some(
                &serde_json::from_value(
                    json!({"providerInstanceId":"mock-custom","model":"missing"}),
                )
                .unwrap(),
            ),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ModelUnavailable);
    let error = targets
        .resolve(
            &parent.thread,
            Some(
                &serde_json::from_value(
                    json!({"providerInstanceId":"mock-custom","driverKind":"claude-code"}),
                )
                .unwrap(),
            ),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::InvalidRequest);
    fixture.catalog.0.lock().unwrap()[2].models[0].options = Optional::Present(vec![
        serde_json::from_value(json!({"type":"select","id":"effort","label":"Effort","options":[{"id":"high","label":"High"}]})).unwrap()
    ]);
    let error = targets
        .resolve(
            &parent.thread,
            Some(
                &serde_json::from_value(json!({
                    "providerInstanceId":"mock-custom","options":[{"id":"effort","value":"low"}]
                }))
                .unwrap(),
            ),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.message,
        "Model custom-model on provider mock-custom rejected options: Option effort must be one of: high."
    );
    let target = targets
        .resolve(
            &parent.thread,
            Some(
                &serde_json::from_value(json!({
                    "providerInstanceId":"mock-custom","options":{"effort":"high"}
                }))
                .unwrap(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(target.selection).unwrap()["options"],
        json!([{"id":"effort","value":"high"}])
    );
}

#[tokio::test]
async fn repeated_ack_and_dispose_keep_first_observation_and_dispose_is_final() {
    let fixture = Fixture::new();
    let task = fixture.delegate("stable").await;
    fixture.complete(&task, "done").await;
    fixture.status(&task).await;
    let first = records(&fixture.parent(), "subagent")[0].clone();
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Observe {
                task_id: task.task_id.clone(),
                observed_by: None,
                dispose: false,
            },
        )
        .await;
    assert_eq!(records(&fixture.parent(), "subagent")[0], first);
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Observe {
                task_id: task.task_id.clone(),
                observed_by: None,
                dispose: true,
            },
        )
        .await;
    let disposed = records(&fixture.parent(), "subagent")[0].clone();
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Observe {
                task_id: task.task_id,
                observed_by: Some(fixture.caller.run_id.clone()),
                dispose: false,
            },
        )
        .await;
    assert_eq!(records(&fixture.parent(), "subagent")[0], disposed);
}

#[tokio::test]
async fn wait_policy_is_settled_only_until_timeout_or_parent_settlement() {
    let fixture = Fixture::new();
    let parent = fixture.parent();
    let target = CatalogTargets(fixture.catalog.clone())
        .resolve(&parent.thread, None)
        .await
        .unwrap();
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delegate(DelegateRequest {
                parent_run_id: fixture.caller.run_id.clone(),
                parent_node_id: parent.runs[0].root_node_id.clone().unwrap(),
                target,
                prompt: "wait".into(),
                title: None,
                runtime_mode: RuntimeMode::FullAccess,
                interaction_mode: InteractionMode::Default,
                always_wake: false,
                cwd: fixture.dir.path().display().to_string(),
            }),
        )
        .await;
    let task_id = records(&fixture.parent(), "subagent")[0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let task = fixture
        .service
        .read_task(&fixture.caller, &NodeId(task_id), false, false)
        .await
        .unwrap();
    fixture.complete(&task, "done").await;
    assert!(cohort(&fixture.parent().runs[0])["delivery"].is_null());
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "pending"
    );
    fixture
        .event(
            &fixture.caller.thread_id,
            &parent.runs[0],
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: Some("done".into()),
                error: None,
                session_id: None,
            },
        )
        .await;
    fixture
        .op(&fixture.caller.thread_id, TaskOperation::Reconcile)
        .await;
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "claimed"
    );
}

struct Instances {
    harness: Arc<dyn Harness>,
}

fn mock_bridge(
    fixture: &Fixture,
    harness: Arc<dyn Harness>,
) -> (crate::EngineCore, Arc<RunnerBridge>) {
    let registry = crate::registry::HarnessRegistry::new();
    registry.register(harness.clone());
    let core = crate::EngineCore::assemble(
        &fixture
            .dir
            .path()
            .join(format!("engine-{}", uuid::Uuid::new_v4())),
        Arc::new(registry),
        HarnessId::Mock,
        None,
    )
    .unwrap();
    let bridge = Arc::new(RunnerBridge {
        kernel: fixture.kernel().clone(),
        sessions: core.sessions.clone(),
        doc_host: core.doc_host.clone(),
        workspace: core.workspace.clone(),
        device_id: core.device_id.clone(),
        instances: Arc::new(Instances { harness }),
        mcp: Arc::new(TestMcp),
    });
    bridge.attach_recovery_gate();
    (core, bridge)
}
#[async_trait]
impl RunnerInstances for Instances {
    async fn resolve(
        &self,
        _instance: &ProviderInstanceId,
    ) -> std::result::Result<RunnerProvider, ToolError> {
        Ok(RunnerProvider {
            harness: self.harness.clone(),
            capabilities: serde_json::from_value(sample("OrchestrationV2ProviderCapabilities"))
                .unwrap(),
        })
    }
}

struct TestMcp;
#[async_trait]
impl RunnerMcp for TestMcp {
    async fn bind(
        &self,
        scope: CallerScope,
        sessions: &crate::sessions::SessionsEngine,
    ) -> std::result::Result<(), ToolError> {
        let credential = sessions
            .mcp_server()
            .credentials
            .issue(crate::mcp::auth::InvocationScope {
                environment_id: "test".into(),
                selection: serde_json::from_value(json!({
                    "instanceId": scope.provider_instance_id, "model": "mock-1"
                }))
                .unwrap(),
                caller: scope.clone(),
                capabilities: Default::default(),
                issued_at: 0,
                task_id: None,
            })
            .unwrap();
        let entry = zeron_harness::mcp::McpServerEntry::http(
            crate::mcp::SERVER_NAME,
            "http://127.0.0.1:1/mcp",
            [("Authorization".into(), credential.authorization)].into(),
        );
        sessions
            .register_session_mcp(
                &scope.thread_id.0,
                vec![entry],
                "Test orchestration session.".into(),
            )
            .map_err(|error| {
                ToolError::new(
                    OrchestratorMcpFailureCode::OrchestrationError,
                    error.to_string(),
                )
            })?;
        Ok(())
    }
}

#[tokio::test]
async fn startup_recovers_unreceipted_provider_acceptance_with_same_message_identity() {
    let fixture = Fixture::new();
    let task = fixture.delegate("recover-mail").await;
    fixture.complete(&task, "recover result").await;
    let delivery = fixture.delivery(DeliveryAction::BeginSteer);
    // Seed a real running parent then fence an offered steer. Simulate the
    // provider accepting it immediately before the receipt write/process loss.
    let parent = fixture.parent();
    fixture
        .event(
            &parent.thread.id,
            &parent.runs[0],
            AgentEvent::SessionStarted {
                instance_id: None,
                harness: HarnessId::Mock,
                model: "mock-1".into(),
                tools: vec![],
                cwd: fixture.dir.path().display().to_string(),
                session_id: "native-parent".into(),
                assistant_message_id: "parent-assistant".into(),
            },
        )
        .await;
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delivery(delivery.clone()),
        )
        .await;
    assert_eq!(
        records(&fixture.parent(), "message")
            .iter()
            .find(|message| message["id"] == delivery.message_id.0)
            .unwrap()["streaming"],
        true
    );
    let (core, bridge) = mock_bridge(&fixture, Arc::new(MockHarness { script: vec![] }));
    bridge.recover_mailbox().await.unwrap();
    let parent = fixture.parent();
    let messages: Vec<_> = records(&parent, "message")
        .iter()
        .filter(|message| message["id"] == delivery.message_id.0)
        .collect();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["streaming"], false);
    let wake = parent
        .runs
        .iter()
        .find(|run| run.user_message_id == delivery.message_id)
        .unwrap();
    assert_eq!(wake.status, OrchestrationV2RunStatus::Starting);
    assert_eq!(records(&parent, "subagent")[0]["result"], "recover result");
    let snapshots =
        serde_json::to_string(&fixture.kernel().store.pending_publications().unwrap()).unwrap();
    assert!(!snapshots.contains("\"provider-session\":"));
    assert!(!snapshots.contains("Authorization"));
    core.sessions.shutdown().await;
}

#[tokio::test]
async fn crash_at_child_start_acceptance_disposes_process_effect_without_relaunch() {
    let fixture = Fixture::new();
    let task = fixture.delegate("uncertain-start").await;
    let first = fixture.kernel().store.effects().unwrap()[0].id.clone();
    fixture
        .kernel()
        .store
        .cancel_effect(&first, crate::now_ms())
        .unwrap();
    let effect = fixture
        .kernel()
        .store
        .claim_effect("crashing-worker", crate::now_ms(), 30_000)
        .unwrap()
        .unwrap();
    assert_eq!(effect.thread_id, task.child_thread_id);
    fixture
        .kernel()
        .store
        .begin_effect(&effect, crate::now_ms())
        .unwrap();
    let child = fixture.child(&task);
    fixture
        .event(
            &child.thread.id,
            &child.runs[0],
            AgentEvent::SessionStarted {
                instance_id: None,
                harness: HarnessId::Mock,
                model: "mock-1".into(),
                tools: vec![],
                cwd: fixture.dir.path().display().to_string(),
                session_id: "accepted-child".into(),
                assistant_message_id: "assistant".into(),
            },
        )
        .await;
    let (core, bridge) = mock_bridge(&fixture, Arc::new(MockHarness { script: vec![] }));
    let summary = bridge.recover_mailbox().await.unwrap();
    assert_eq!(summary.uncertain_effects, 1);
    assert_eq!(
        fixture
            .kernel()
            .store
            .effect(&effect.id)
            .unwrap()
            .unwrap()
            .status,
        EffectStatus::Cancelled
    );
    assert_eq!(
        fixture.child(&task).runs[0].status,
        OrchestrationV2RunStatus::Cancelled
    );
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["status"],
        "cancelled"
    );
    assert!(!core.sessions.any_active());
    core.sessions.shutdown().await;
}

struct SteeringMock {
    offers: tokio::sync::mpsc::UnboundedSender<String>,
    starts: Arc<AtomicUsize>,
}
#[async_trait]
impl Harness for SteeringMock {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Active steering mock"
    }
    fn supports_steering(&self) -> bool {
        true
    }
    fn steering_mode(&self) -> SteeringMode {
        SteeringMode::StepBoundary
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[]
    }
    fn deterministic_turn_end(&self) -> bool {
        true
    }
    async fn models(&self) -> std::result::Result<Vec<Model>, HarnessError> {
        Ok(vec![])
    }
    async fn run(
        &self,
        request: RunRequest,
        controls: RunControls,
    ) -> std::result::Result<
        BoxStream<'static, std::result::Result<AgentEvent, HarnessError>>,
        HarnessError,
    > {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let started = AgentEvent::SessionStarted {
            instance_id: None,
            harness: HarnessId::Mock,
            model: "mock-1".into(),
            tools: vec![],
            cwd: request.cwd,
            session_id: "steering-parent".into(),
            assistant_message_id: "parent-assistant".into(),
        };
        let offers = self.offers.clone();
        Ok(futures::stream::once(async {Ok(started)}).chain(futures::stream::unfold((offers,controls.steering,controls.interrupt),|(offers,mut steering,interrupt)| async move {
            tokio::select! {
                _ = interrupt.cancelled() => None,
                message = steering.recv() => {
                    let message = message?;
                    if let Some(receipt) = message.notification_acceptance {let _ = receipt.send(true);}
                    let _ = offers.send(message.message_id.unwrap());
                    Some((Ok(AgentEvent::ReasoningDelta {text:message.prompt}),(offers,steering,interrupt)))
                }
            }
        })).boxed())
    }
}

#[tokio::test]
async fn live_parent_steer_is_noninterrupting_and_acceptance_does_not_acknowledge() {
    let fixture = Fixture::new();
    let task = fixture.delegate("live-steer").await;
    fixture.complete(&task, "child result").await;
    let (offers, mut received) = tokio::sync::mpsc::unbounded_channel();
    let starts = Arc::new(AtomicUsize::new(0));
    let harness = Arc::new(SteeringMock {
        offers,
        starts: starts.clone(),
    });
    let (core, bridge) = mock_bridge(&fixture, harness);
    core.workspace
        .create_chat(
            "parent",
            None,
            Some(&core.device_id),
            None,
            Some(fixture.dir.path().display().to_string()),
        )
        .unwrap();
    let parent = fixture.parent();
    bridge
        .mcp
        .bind(fixture.caller.clone(), &core.sessions)
        .await
        .unwrap();
    core.sessions.dispatch("parent",HarnessId::Mock,serde_json::from_value(json!({
        "prompt":"Parent is still busy.","model":"mock-1","reasoning":null,"cwd":fixture.dir.path(),"sandbox":"workspace-write","resume":null
    })).unwrap(),Some("live-parent-input".into())).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while starts.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    fixture
        .event(
            &parent.thread.id,
            &parent.runs[0],
            AgentEvent::SessionStarted {
                instance_id: None,
                harness: HarnessId::Mock,
                model: "mock-1".into(),
                tools: vec![],
                cwd: fixture.dir.path().display().to_string(),
                session_id: "steering-parent".into(),
                assistant_message_id: "parent-assistant".into(),
            },
        )
        .await;
    fixture
        .event(
            &parent.thread.id,
            &parent.runs[0],
            AgentEvent::InputAccepted,
        )
        .await;
    let delivery = fixture.delivery(DeliveryAction::Queue);
    for effect in fixture.kernel().store.effects().unwrap() {
        if matches!(
            effect.request,
            super::effects::EffectRequest::ProviderTurnStart { .. }
        ) {
            fixture
                .kernel()
                .store
                .cancel_effect(&effect.id, crate::now_ms())
                .unwrap();
        }
    }
    let worker = EffectWorker::new(
        fixture.kernel().store.clone(),
        bridge,
        "steer-worker".into(),
    );
    assert!(worker.step(crate::now_ms()).await.unwrap());
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), received.recv())
            .await
            .unwrap()
            .unwrap(),
        delivery.message_id.0
    );
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    assert!(core.sessions.turn_in_flight("parent"));
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "delivered"
    );
    assert_eq!(fixture.parent().runs.len(), 1);
    fixture.status(&task).await;
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "acknowledged"
    );
    core.sessions.shutdown().await;
}

struct GatedMock {
    entered: Arc<AtomicUsize>,
    release: Arc<tokio::sync::Notify>,
    requests: Arc<Mutex<Vec<RunRequest>>>,
}

#[async_trait]
impl Harness for GatedMock {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Gated mock"
    }
    fn supports_steering(&self) -> bool {
        false
    }
    fn steering_mode(&self) -> SteeringMode {
        SteeringMode::TurnBoundary
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[]
    }
    fn deterministic_turn_end(&self) -> bool {
        true
    }
    async fn models(&self) -> std::result::Result<Vec<Model>, HarnessError> {
        Ok(vec![])
    }
    async fn run(
        &self,
        request: RunRequest,
        controls: RunControls,
    ) -> std::result::Result<
        BoxStream<'static, std::result::Result<AgentEvent, HarnessError>>,
        HarnessError,
    > {
        assert!(
            controls
                .mcp
                .instructions()
                .contains("Test orchestration session.")
        );
        self.requests.lock().unwrap().push(request.clone());
        let started = AgentEvent::SessionStarted {
            instance_id: None,
            harness: HarnessId::Mock,
            model: request.model.clone().unwrap(),
            tools: vec![],
            cwd: request.cwd.clone(),
            session_id: format!("gated-{}", self.entered.fetch_add(1, Ordering::SeqCst)),
            assistant_message_id: uuid::Uuid::new_v4().to_string(),
        };
        let entered = self.entered.clone();
        let release = self.release.clone();
        if entered.load(Ordering::SeqCst) >= 8 {
            release.notify_waiters();
        }
        let mock = MockHarness {
            script: vec![
                AgentEvent::TextDelta {
                    text: "A child result.".into(),
                },
                AgentEvent::Done {
                    status: DoneStatus::Completed,
                    result: Some("A child result.".into()),
                    error: None,
                    session_id: None,
                },
            ],
        }
        .run(request, controls)
        .await?;
        Ok(futures::stream::once(async { Ok(started) })
            .chain(futures::stream::once(async move {
                loop {
                    let notified = release.notified();
                    if entered.load(Ordering::SeqCst) >= 8 {
                        break;
                    }
                    notified.await;
                }
                Ok(AgentEvent::ReasoningDelta {
                    text: "All eight children accepted.".into(),
                })
            }))
            .chain(mock)
            .boxed())
    }
}

#[tokio::test]
async fn eight_mock_children_execute_on_normal_transcripts_without_worker_cap() {
    let fixture = Fixture::new();
    let entered = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(Mutex::new(vec![]));
    let harness = Arc::new(GatedMock {
        entered: entered.clone(),
        release: Arc::new(tokio::sync::Notify::new()),
        requests: requests.clone(),
    });
    let registry = crate::registry::HarnessRegistry::new();
    registry.register(harness.clone());
    let core = crate::EngineCore::assemble(
        &fixture.dir.path().join("engine"),
        Arc::new(registry),
        HarnessId::Mock,
        None,
    )
    .unwrap();
    let bridge = Arc::new(RunnerBridge {
        kernel: fixture.kernel().clone(),
        sessions: core.sessions.clone(),
        doc_host: core.doc_host.clone(),
        workspace: core.workspace.clone(),
        device_id: core.device_id.clone(),
        instances: Arc::new(Instances { harness }),
        mcp: Arc::new(TestMcp),
    });
    bridge.attach_recovery_gate();
    // The seeded parent has no runtime in this fixture: retire its unrelated
    // start so the acceptance workers exercise only the actual children.
    let first = fixture.kernel().store.effects().unwrap()[0].id.clone();
    fixture
        .kernel()
        .store
        .cancel_effect(&first, crate::now_ms())
        .unwrap();
    let stop = CancellationToken::new();
    let workers = bridge.spawn_workers(stop.clone());
    let mut tasks = vec![];
    for index in 0..8 {
        tasks.push(fixture.delegate(&format!("child-{index}")).await);
    }
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if records(&fixture.parent(), "subagent")
                .iter()
                .all(|task| task["result"].is_string())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    for task in &tasks {
        assert_eq!(
            fixture.status(task).await.summary.as_deref(),
            Some("A child result.")
        );
        let entries = core
            .doc_host
            .open(&task.child_thread_id.0)
            .unwrap()
            .doc()
            .read_entries()
            .unwrap();
        assert!(
            entries
                .iter()
                .any(|entry| entry.role == zeron_doc::MessageRole::Assistant)
        );
        assert!(
            core.workspace
                .chat(&task.child_thread_id.0)
                .unwrap()
                .is_some()
        );
        let request = core.sessions.last_request(&task.child_thread_id.0).unwrap();
        let error = core
            .sessions
            .dispatch(&task.child_thread_id.0, HarnessId::Mock, request, None)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("read-only"));
    }
    assert_eq!(entered.load(Ordering::SeqCst), 8);
    assert!(requests.lock().unwrap().iter().all(|request| request.cwd
        == fixture.caller.workspace_root.display().to_string()
        && request.model.as_deref() == Some("mock-1")
        && request.runtime_mode == RuntimeMode::FullAccess));
    stop.cancel();
    for worker in workers {
        worker.await.unwrap();
    }
    core.sessions.shutdown().await;
}

#[tokio::test]
async fn parent_starting_running_waiting_idle_stopped_archived_deleted_matrix() {
    for state in [
        "starting", "running", "waiting", "idle", "stopped", "archived", "deleted",
    ] {
        let fixture = Fixture::new();
        let task = fixture.delegate(state).await;
        let parent = fixture.parent();
        match state {
            "running" | "waiting" => {
                fixture
                    .event(
                        &parent.thread.id,
                        &parent.runs[0],
                        AgentEvent::SessionStarted {
                            instance_id: None,
                            harness: HarnessId::Mock,
                            model: "mock-1".into(),
                            tools: vec![],
                            cwd: fixture.dir.path().display().to_string(),
                            session_id: "session".into(),
                            assistant_message_id: "assistant".into(),
                        },
                    )
                    .await;
                if state == "waiting" {
                    fixture
                        .event(
                            &parent.thread.id,
                            &parent.runs[0],
                            AgentEvent::InputRequested {
                                request_id: "question".into(),
                                questions: vec![],
                            },
                        )
                        .await;
                }
            }
            "idle" => {
                fixture
                    .event(
                        &parent.thread.id,
                        &parent.runs[0],
                        AgentEvent::Done {
                            status: DoneStatus::Completed,
                            result: Some("done".into()),
                            error: None,
                            session_id: None,
                        },
                    )
                    .await
            }
            "stopped" => {
                fixture
                    .op(
                        &parent.thread.id,
                        TaskOperation::StopCohort {
                            run_id: parent.runs[0].id.clone(),
                        },
                    )
                    .await
            }
            "archived" | "deleted" => {
                // Lifecycle planner belongs to P4. Seed the persisted source
                // event through the reducer rather than inventing a public API.
                fixture
                    .kernel()
                    .store
                    .write(|tx| {
                        let mut value = serde_json::to_value(&parent.thread)?;
                        value[if state == "archived" {
                            "archivedAt"
                        } else {
                            "deletedAt"
                        }] = json!(super::event::iso(NOW)?);
                        let event = super::event::make(
                            EventId(format!("fixture:{state}")),
                            &parent.thread.id,
                            if state == "archived" {
                                "thread.archived"
                            } else {
                                "thread.deleted"
                            },
                            &value,
                            NOW,
                        )?;
                        fixture.kernel().store.append_event(tx, None, event)?;
                        Ok(())
                    })
                    .unwrap();
            }
            _ => {}
        }
        fixture.complete(&task, "done").await;
        let parent = fixture.parent();
        let task = records(&parent, "subagent")
            .iter()
            .find(|candidate| candidate["id"] == task.task_id.0)
            .unwrap();
        assert_eq!(
            task["completionDelivery"]["state"],
            if matches!(state, "stopped" | "archived" | "deleted") {
                "disposed"
            } else {
                "claimed"
            },
            "{state}"
        );
        if !matches!(state, "stopped" | "archived" | "deleted") {
            fixture
                .op(
                    &fixture.caller.thread_id,
                    TaskOperation::Delivery(fixture.delivery(DeliveryAction::Queue)),
                )
                .await;
            assert_eq!(
                fixture.parent().runs.last().unwrap().status,
                if state == "idle" {
                    OrchestrationV2RunStatus::Starting
                } else {
                    OrchestrationV2RunStatus::Queued
                },
                "{state}"
            );
        }
    }
}

#[tokio::test]
async fn status_monitor_rollback_ordinal_and_spawn_binding_match_t3() {
    let fixture = Fixture::new();
    let task = fixture.delegate("status-oracle").await;
    fixture.complete(&task, "original").await;
    let child = fixture.child(&task);
    for (ordinal, run_status) in [(0, "starting"), (2, "completed"), (3, "rolled_back")] {
        let mut run = serde_json::to_value(&child.runs[0]).unwrap();
        run["id"] = json!(format!("extra-{ordinal}"));
        run["ordinal"] = json!(ordinal);
        run["status"] = json!(run_status);
        run["startedAt"] = json!(super::event::iso(NOW).unwrap());
        run["userMessageId"] = json!(format!("extra-message-{ordinal}"));
        fixture.seed_event(&child.thread.id, "run.created", run);
    }
    let mut monitor = message(
        &child.thread.id,
        Some(&RunId("extra-2".into())),
        None,
        "monitor",
        "monitor",
        "user",
        NOW,
    )
    .unwrap();
    monitor["notification"] =
        json!({"source":{"kind":"monitor"},"outcome":"updated","summary":"Monitor progress"});
    fixture.seed_event(&child.thread.id, "message.updated", monitor);
    let status = fixture.status(&task).await;
    assert!(
        !status.has_pending_child_runs,
        "an earlier ordinal is not later child work"
    );
    assert_eq!(status.latest_terminal_run_id, status.child_run_id);
    assert_eq!(status.latest_terminal_summary.as_deref(), Some("original"));
    assert_eq!(
        progress(&fixture.child(&task)).0,
        "working",
        "the earlier active work still affects progress"
    );
    let mut projection = fixture.child(&task);
    projection.runs.retain(|run| run.ordinal != 0);
    assert_eq!(progress(&projection).1.unwrap().id, child.runs[0].id);
    let mut spawn = records(&child, "context-transfer")[0].clone();
    spawn["targetRunId"] = Value::Null;
    fixture.seed_event(&child.thread.id, "context-transfer.updated", spawn);
    assert!(
        fixture.status(&task).await.child_run_id.is_none(),
        "an explicit null spawn binding is not legacy absence"
    );
}

#[tokio::test]
async fn status_racing_terminal_publication_acknowledges_without_later_wake() {
    let fixture = Fixture::new();
    let task = fixture.delegate("status-before-finalize").await;
    let child = fixture.child(&task);
    fixture
        .event(
            &child.thread.id,
            &child.runs[0],
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: Some("raced result".into()),
                error: None,
                session_id: None,
            },
        )
        .await;
    assert!(records(&fixture.parent(), "context-transfer").is_empty());
    let result = fixture.status(&task).await;
    assert_eq!(result.summary.as_deref(), Some("raced result"));
    assert!(result.result_context_transfer_id.is_some());
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "acknowledged"
    );
    assert!(cohort(&fixture.parent().runs[0])["delivery"].is_null());
    fixture.kernel().reconcile_delegation().await.unwrap();
    assert!(cohort(&fixture.parent().runs[0])["delivery"].is_null());
}

#[tokio::test]
async fn steering_requires_always_wake_live_turn_capability_and_not_maintenance() {
    let fixture = Fixture::new();
    let task = fixture.delegate("steer-gates").await;
    let parent = fixture.parent();
    fixture
        .event(
            &parent.thread.id,
            &parent.runs[0],
            AgentEvent::SessionStarted {
                instance_id: None,
                harness: HarnessId::Mock,
                model: "mock-1".into(),
                tools: vec![],
                cwd: fixture.dir.path().display().to_string(),
                session_id: "live".into(),
                assistant_message_id: "assistant".into(),
            },
        )
        .await;
    fixture
        .event(
            &parent.thread.id,
            &parent.runs[0],
            AgentEvent::InputAccepted,
        )
        .await;
    fixture.complete(&task, "done").await;
    let delivery = fixture.delivery(DeliveryAction::Queue);
    let parent = fixture.parent();
    assert!(super::mailbox::can_steer_delivery(&parent, &delivery));
    let mut narrowed = parent.clone();
    narrowed.records.get_mut("subagent").unwrap()[0]["completionWake"] = json!("settled_only");
    assert!(!super::mailbox::can_steer_delivery(&narrowed, &delivery));
    for change in ["capability", "turn", "session", "compact", "logout"] {
        let mut projection = parent.clone();
        match change {
            "capability" => {
                projection.records.get_mut("provider-session").unwrap()[0]["capabilities"]["turns"]
                    ["supportsActiveSteering"] = json!(false)
            }
            "turn" => {
                projection.records.get_mut("provider-turn").unwrap()[0]["status"] =
                    json!("completed")
            }
            "session" => {
                projection.records.get_mut("provider-session").unwrap()[0]["status"] =
                    json!("stopped")
            }
            _ => {
                projection.records.get_mut("message").unwrap().push(
                    message(
                        &projection.thread.id,
                        Some(&projection.runs[0].id),
                        None,
                        &projection.runs[0].user_message_id.0,
                        if change == "compact" {
                            " /COMPACT "
                        } else {
                            "/logout"
                        },
                        "user",
                        NOW,
                    )
                    .unwrap(),
                );
            }
        }
        assert!(
            !super::mailbox::can_steer_delivery(&projection, &delivery),
            "{change}"
        );
    }
}

#[tokio::test]
async fn stale_queued_delivery_does_not_starve_next_eligible_notification() {
    let fixture = Fixture::new();
    let first = fixture.delegate("stale-first").await;
    let second = fixture.delegate("fresh-second").await;
    fixture.complete(&first, "first").await;
    let delivery = fixture.delivery(DeliveryAction::Queue);
    fixture
        .op(&fixture.caller.thread_id, TaskOperation::Delivery(delivery))
        .await;
    fixture.status(&first).await;
    fixture.complete(&second, "second").await;
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delivery(fixture.delivery(DeliveryAction::Queue)),
        )
        .await;
    let parent = fixture.parent();
    fixture
        .event(
            &parent.thread.id,
            &parent.runs[0],
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: Some("parent done".into()),
                error: None,
                session_id: None,
            },
        )
        .await;
    fixture
        .op(&fixture.caller.thread_id, TaskOperation::DrainQueue)
        .await;
    let parent = fixture.parent();
    assert_eq!(parent.runs[1].status, OrchestrationV2RunStatus::Cancelled);
    assert_eq!(parent.runs[2].status, OrchestrationV2RunStatus::Starting);
}

#[tokio::test]
async fn recovery_keeps_cancelled_background_roster_and_validates_new_records() {
    let fixture = Fixture::new();
    let task = fixture.delegate("recovery-roster").await;
    let child = fixture.child(&task);
    fixture
        .event(
            &child.thread.id,
            &child.runs[0],
            AgentEvent::SessionStarted {
                instance_id: None,
                harness: HarnessId::Mock,
                model: "mock-1".into(),
                tools: vec![],
                cwd: fixture.dir.path().display().to_string(),
                session_id: "child".into(),
                assistant_message_id: "assistant".into(),
            },
        )
        .await;
    let mut provider = records(&fixture.child(&task), "provider-thread")[0].clone();
    provider["pendingBackgroundTasks"] =
        json!([{"taskId":"shell-1","kind":"command","description":"long shell"}]);
    fixture.seed_event(&child.thread.id, "provider-thread.updated", provider);
    fixture.complete(&task, "root settled").await;
    assert!(fixture.kernel().store.verify_projections().unwrap());
    let summary = fixture.kernel().recover(NOW + 100).await.unwrap();
    assert!(!summary.projection_rebuilt);
    fixture.kernel().reconcile_delegation().await.unwrap();
    let child = fixture.child(&task);
    let work = child.runs[0]
        .restart_cancelled_background_work
        .as_ref()
        .unwrap();
    assert_eq!(work.len(), 1);
    assert_eq!(
        work[0].kind,
        OrchestrationV2RestartCancelledBackgroundWorkKind::Shell
    );
    assert_eq!(
        records(&child, "provider-thread")[0]["pendingBackgroundTasks"],
        json!([])
    );
    assert!(fixture.kernel().store.verify_projections().unwrap());
    fixture.kernel().store.rebuild().unwrap();
    assert_eq!(
        fixture.child(&task).runs[0]
            .restart_cancelled_background_work
            .as_ref()
            .unwrap(),
        work
    );
}

#[tokio::test]
async fn active_cancel_executor_interrupts_owned_run_and_stops_its_cohort() {
    let fixture = Fixture::new();
    let task = fixture.delegate("cancel-live").await;
    let (offers, _received) = tokio::sync::mpsc::unbounded_channel();
    let starts = Arc::new(AtomicUsize::new(0));
    let (core, bridge) = mock_bridge(&fixture, Arc::new(SteeringMock { offers, starts }));
    fixture
        .kernel()
        .store
        .cancel_effect(
            &fixture.kernel().store.effects().unwrap()[0].id,
            crate::now_ms(),
        )
        .unwrap();
    let worker = EffectWorker::new(
        fixture.kernel().store.clone(),
        bridge,
        "cancel-worker".into(),
    );
    assert!(worker.step(crate::now_ms()).await.unwrap());
    assert!(core.sessions.turn_in_flight(&task.child_thread_id.0));
    let child = fixture.child(&task);
    fixture
        .event(
            &child.thread.id,
            &child.runs[0],
            AgentEvent::Subagent {
                parent_tool_use_id: "owned-native-background".into(),
                event: Box::new(AgentEvent::TextDelta {
                    text: "running".into(),
                }),
            },
        )
        .await;
    let result = fixture
        .service
        .task_cancel(
            fixture.caller.clone(),
            serde_json::from_value(json!({
                "taskId":task.task_id,"clientRequestId":"cancel-live"
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["status"],
        "cancel_requested"
    );
    assert_eq!(
        cohort(&fixture.child(&task).runs[0])["disposition"],
        "stopped"
    );
    assert!(worker.step(crate::now_ms()).await.unwrap());
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if fixture.status(&task).await.status == OrchestratorMcpDelegatedTaskStatus::Interrupted
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(!core.sessions.turn_in_flight(&task.child_thread_id.0));
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "disposed"
    );
    core.sessions.shutdown().await;
}

#[tokio::test]
async fn background_only_task_is_not_cancellable_without_an_interruptible_run() {
    let fixture = Fixture::new();
    let task = fixture.delegate("background-only").await;
    let child = fixture.child(&task);
    fixture
        .event(
            &child.thread.id,
            &child.runs[0],
            AgentEvent::Subagent {
                parent_tool_use_id: "native-background".into(),
                event: Box::new(AgentEvent::TextDelta {
                    text: "working".into(),
                }),
            },
        )
        .await;
    fixture.complete(&task, "root done").await;
    let error = fixture
        .service
        .task_cancel(
            fixture.caller.clone(),
            serde_json::from_value(json!({
                "taskId":task.task_id
            }))
            .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::TaskNotCancellable);
}

struct RefuseMcp;
#[async_trait]
impl RunnerMcp for RefuseMcp {
    async fn bind(
        &self,
        _scope: CallerScope,
        _sessions: &crate::sessions::SessionsEngine,
    ) -> std::result::Result<(), ToolError> {
        Err(ToolError::new(
            OrchestratorMcpFailureCode::CapabilityDenied,
            "Session MCP binding refused.",
        ))
    }
}

#[tokio::test]
async fn required_mcp_failure_refuses_provider_start_and_publishes_failure() {
    let fixture = Fixture::new();
    let task = fixture.delegate("mcp-refusal").await;
    let (offers, _received) = tokio::sync::mpsc::unbounded_channel();
    let starts = Arc::new(AtomicUsize::new(0));
    let (core, bridge) = mock_bridge(
        &fixture,
        Arc::new(SteeringMock {
            offers,
            starts: starts.clone(),
        }),
    );
    let mut bridge = (*bridge).clone();
    bridge.mcp = Arc::new(RefuseMcp);
    fixture
        .kernel()
        .store
        .cancel_effect(
            &fixture.kernel().store.effects().unwrap()[0].id,
            crate::now_ms(),
        )
        .unwrap();
    let worker = EffectWorker::new(
        fixture.kernel().store.clone(),
        Arc::new(bridge),
        "refuse-worker".into(),
    );
    assert!(worker.step(crate::now_ms()).await.unwrap());
    let result = fixture.status(&task).await;
    assert_eq!(result.status, OrchestratorMcpDelegatedTaskStatus::Failed);
    assert!(
        result
            .summary
            .unwrap()
            .contains("Session MCP binding refused.")
    );
    assert!(result.result_context_transfer_id.is_some());
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    core.sessions.shutdown().await;
}

#[tokio::test]
async fn cancelled_delivery_run_reclaims_mail_with_successor_identity() {
    let fixture = Fixture::new();
    let task = fixture.delegate("cancelled-delivery").await;
    fixture.complete(&task, "done").await;
    let first = fixture.delivery(DeliveryAction::Queue);
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delivery(first.clone()),
        )
        .await;
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delivery(DeliveryCommand {
                action: DeliveryAction::Completed { cancelled: true },
                ..first.clone()
            }),
        )
        .await;
    let next = fixture.delivery(DeliveryAction::Queue);
    assert!(next.generation > first.generation);
    assert_ne!(next.message_id, first.message_id);
    assert_eq!(
        records(&fixture.parent(), "subagent")[0]["completionDelivery"]["state"],
        "claimed"
    );
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delivery(DeliveryCommand {
                action: DeliveryAction::Accepted,
                ..first
            }),
        )
        .await;
    assert_eq!(
        fixture.delivery(DeliveryAction::Queue).message_id,
        next.message_id
    );
}

#[tokio::test]
async fn acknowledging_all_active_batch_members_keeps_successor_delivery_fence() {
    let fixture = Fixture::new();
    let first = fixture.delegate("observed-active").await;
    let second = fixture.delegate("pending-successor").await;
    fixture.complete(&first, "first").await;
    let delivery = fixture.delivery(DeliveryAction::Queue);
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delivery(delivery.clone()),
        )
        .await;
    let parent = fixture.parent();
    fixture
        .event(
            &parent.thread.id,
            &parent.runs[0],
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: Some("parent done".into()),
                error: None,
                session_id: None,
            },
        )
        .await;
    fixture
        .op(&fixture.caller.thread_id, TaskOperation::DrainQueue)
        .await;
    fixture.complete(&second, "second").await;
    fixture.status(&first).await;
    assert_eq!(
        cohort(&fixture.parent().runs[0])["delivery"]["taskIds"],
        json!([])
    );
    fixture
        .op(
            &fixture.caller.thread_id,
            TaskOperation::Delivery(DeliveryCommand {
                action: DeliveryAction::Completed { cancelled: false },
                ..delivery
            }),
        )
        .await;
    assert_eq!(
        cohort(&fixture.parent().runs[0])["delivery"]["taskIds"],
        json!([second.task_id])
    );
    assert_eq!(
        cohort(&fixture.parent().runs[0])["delivery"]["generation"],
        2
    );
}

fn managed_interrupt_threads(fixture: &Fixture) -> std::collections::BTreeSet<String> {
    fixture
        .kernel()
        .store
        .effects()
        .unwrap()
        .into_iter()
        .filter(|effect| {
            matches!(
                effect.request,
                super::effects::EffectRequest::ManagedRunInterrupt { .. }
            )
        })
        .map(|effect| effect.thread_id.0)
        .collect()
}

fn scope_for(fixture: &Fixture, thread: &super::projection::ThreadProjection) -> CallerScope {
    CallerScope {
        thread_id: thread.thread.id.clone(),
        run_id: thread.runs[0].id.clone(),
        session_id: format!("session/{}:1", thread.thread.id.0),
        project_id: thread.thread.project_id.clone(),
        workspace_root: fixture.dir.path().into(),
        runtime_mode: thread.thread.runtime_mode,
        interaction_mode: thread.thread.interaction_mode,
        provider_instance_id: thread.thread.provider_instance_id.clone(),
    }
}

/// An unrelated top-level thread with its own active run and delegated task.
async fn foreign_parent(fixture: &Fixture) -> (CallerScope, TaskStatusResult) {
    let command = Command::wire(serde_json::from_value(json!({
        "type":"thread.create","commandId":"create-foreign","threadId":"foreign","projectId":"project","title":"Foreign",
        "createdBy":"user","creationSource":"web","modelSelection":{"instanceId":"mock","model":"mock-1"},
        "runtimeMode":"full-access","interactionMode":"default","branch":"main","worktreePath":fixture.dir.path()
    })).unwrap()).unwrap();
    assert_eq!(
        fixture
            .kernel()
            .store
            .dispatch(&command, NOW)
            .unwrap()
            .status,
        ReceiptStatus::Accepted
    );
    let foreign = fixture
        .kernel()
        .store
        .thread(&ThreadId("foreign".into()))
        .unwrap()
        .unwrap();
    let seed =
        execution_seed(&foreign.thread, 1, "foreign-input", "starting", "mock", NOW).unwrap();
    assert_eq!(
        fixture
            .kernel()
            .store
            .dispatch(
                &Command {
                    id: CommandId("seed-foreign".into()),
                    thread_id: foreign.thread.id.clone(),
                    operation: Operation::CreateExecution(Box::new(seed)),
                },
                NOW
            )
            .unwrap()
            .status,
        ReceiptStatus::Accepted
    );
    let scope = scope_for(
        fixture,
        &fixture
            .kernel()
            .store
            .thread(&ThreadId("foreign".into()))
            .unwrap()
            .unwrap(),
    );
    let task = fixture
        .service
        .delegate_task(
            scope.clone(),
            serde_json::from_value(json!({"task":"Foreign work","clientRequestId":"foreign-task"}))
                .unwrap(),
        )
        .await
        .unwrap();
    (scope, task)
}

#[tokio::test]
async fn whole_thread_stop_walks_owned_tasks_leaves_first_and_replays_its_frozen_targets() {
    let fixture = Fixture::new();
    let outer = fixture.delegate("outer").await;
    let sibling = fixture.delegate("sibling").await;
    let done = fixture.delegate("done").await;
    fixture.complete(&done, "kept reply").await;
    let child = fixture.child(&outer);
    let nested = fixture
        .service
        .delegate_task(
            scope_for(&fixture, &child),
            serde_json::from_value(json!({"task":"Nested work","clientRequestId":"nested"}))
                .unwrap(),
        )
        .await
        .unwrap();
    let (_foreign_scope, foreign) = foreign_parent(&fixture).await;
    // A persistent monitor belongs to the foreground run's provider, not to an
    // app-owned task: whole-thread Stop must leave it as plain Stop does.
    let parent = fixture.parent();
    fixture.seed_event(&parent.thread.id, "turn-item.updated", json!({
        "id":"persistent-monitor","threadId":parent.thread.id,"runId":parent.runs[0].id,"nodeId":parent.runs[0].root_node_id,
        "providerThreadId":parent.runs[0].provider_thread_id,"providerTurnId":null,"nativeItemRef":null,"parentItemId":null,
        "ordinal":900,"status":"running","title":"monitor","startedAt":null,"completedAt":null,
        "updatedAt":"2026-10-07T04:00:00.000Z","type":"dynamic_tool","input":{"persistent":true},"output":"",
        "origin":null,"toolName":"monitor"
    }));

    let first =
        super::stop_all::stop_thread_work(fixture.kernel(), &fixture.caller.thread_id, "stop-1")
            .await
            .unwrap();
    // nested, outer, sibling and the foreground run; the settled task is spared.
    assert_eq!((first.stopped_runs, first.skipped), (4, 0));
    assert!(first.refusal.is_none());
    let interrupted = managed_interrupt_threads(&fixture);
    for task in [&outer, &sibling, &nested] {
        assert!(interrupted.contains(&task.child_thread_id.0), "{task:?}");
    }
    assert!(!interrupted.contains(&done.child_thread_id.0));
    assert!(!interrupted.contains(&foreign.child_thread_id.0));
    assert!(!interrupted.contains("foreign"));
    // Descendants first: the nested task is step 0 under its own owner.
    let receipt = |index| {
        fixture
            .kernel()
            .store
            .receipt(&super::stop_all::step_id(
                &fixture.caller.thread_id,
                "stop-1",
                index,
            ))
            .unwrap()
            .unwrap()
    };
    assert_eq!(receipt(0).thread_id, child.thread.id);
    assert_eq!(receipt(1).thread_id, fixture.caller.thread_id);
    assert_eq!(receipt(3).command_type, "run.interrupt");

    let after = fixture.parent();
    let delivery = |id: &NodeId| {
        records(&after, "subagent")
            .iter()
            .find(|task| task["id"] == id.0)
            .unwrap()["completionDelivery"]["state"]
            .clone()
    };
    assert_eq!(delivery(&outer.task_id), "disposed");
    assert_eq!(delivery(&sibling.task_id), "disposed");
    // The settled task keeps its terminal status and reply. Its wake belongs
    // to the interrupted foreground run's cohort, exactly as under plain Stop.
    assert_eq!(
        records(&after, "subagent")
            .iter()
            .find(|task| task["id"] == done.task_id.0)
            .unwrap()["status"],
        "completed"
    );
    assert_eq!(
        records(&after, "turn-item")
            .iter()
            .find(|item| item["id"] == "persistent-monitor")
            .unwrap()["status"],
        "running"
    );
    assert_eq!(
        fixture.status(&done).await.summary.as_deref(),
        Some("kept reply")
    );
    let foreign_child = fixture
        .kernel()
        .store
        .thread(&foreign.child_thread_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        foreign_child.runs[0].status,
        OrchestrationV2RunStatus::Starting
    );

    // Work started after the request is outside its frozen target set, and a
    // retry (response loss) repeats the first answer without new effects.
    let late = fixture
        .service
        .delegate_task(
            scope_for(&fixture, &fixture.child(&sibling)),
            serde_json::from_value(json!({"task":"Late work","clientRequestId":"late"})).unwrap(),
        )
        .await
        .unwrap();
    let effects = fixture.kernel().store.effects().unwrap().len();
    let replay =
        super::stop_all::stop_thread_work(fixture.kernel(), &fixture.caller.thread_id, "stop-1")
            .await
            .unwrap();
    assert_eq!((replay.stopped_runs, replay.skipped), (4, 0));
    assert_eq!(fixture.kernel().store.effects().unwrap().len(), effects);
    assert!(!managed_interrupt_threads(&fixture).contains(&late.child_thread_id.0));
    assert_eq!(
        fixture.child(&late).runs[0].status,
        OrchestrationV2RunStatus::Starting
    );
}

#[tokio::test]
async fn whole_thread_stop_refuses_delegated_children_and_unknown_threads() {
    let fixture = Fixture::new();
    let task = fixture.delegate("owned").await;
    let kernel = fixture.kernel();
    let refused = super::stop_all::stop_thread_work(kernel, &task.child_thread_id, "from-child")
        .await
        .unwrap_err();
    assert!(refused.to_string().contains("owns it"), "{refused}");
    assert!(
        super::stop_all::stop_thread_work(kernel, &ThreadId("nope".into()), "unknown")
            .await
            .is_err()
    );
    // Nothing was stopped by either refusal.
    assert!(managed_interrupt_threads(&fixture).is_empty());
}

#[tokio::test]
async fn panel_stop_and_whole_thread_stop_tear_down_the_exact_live_child_runtime() {
    for whole_thread in [false, true] {
        let fixture = Fixture::new();
        let task = fixture.delegate("live-stop").await;
        let (offers, _received) = tokio::sync::mpsc::unbounded_channel();
        let starts = Arc::new(AtomicUsize::new(0));
        let (core, bridge) = mock_bridge(&fixture, Arc::new(SteeringMock { offers, starts }));
        // The parent's own provider start is not under test.
        fixture
            .kernel()
            .store
            .cancel_effect(
                &fixture.kernel().store.effects().unwrap()[0].id,
                crate::now_ms(),
            )
            .unwrap();
        let worker =
            EffectWorker::new(fixture.kernel().store.clone(), bridge, "stop-worker".into());
        assert!(worker.step(crate::now_ms()).await.unwrap());
        assert!(core.sessions.turn_in_flight(&task.child_thread_id.0));
        if whole_thread {
            let result = super::stop_all::stop_thread_work(
                fixture.kernel(),
                &fixture.caller.thread_id,
                "stop-live",
            )
            .await
            .unwrap();
            assert_eq!((result.stopped_runs, result.skipped), (2, 0));
        } else {
            fixture
                .service
                .cancel_for_user(&fixture.caller.thread_id, task.task_id.to_string())
                .await
                .unwrap();
        }
        while worker.step(crate::now_ms()).await.unwrap() {}
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if fixture.status(&task).await.status
                    == OrchestratorMcpDelegatedTaskStatus::Interrupted
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(!core.sessions.turn_in_flight(&task.child_thread_id.0));
        core.sessions.shutdown().await;
    }
}

#[tokio::test]
async fn whole_thread_stop_freezes_the_child_process_and_ignores_a_replacement() {
    use super::steering::RuntimeTarget;
    let fixture = Fixture::new();
    let task = fixture.delegate("replaced").await;
    let child = fixture.child(&task);
    let runtime = RuntimeTarget::for_run(&child.runs[0]).unwrap();
    let bind = |process: &str| {
        fixture
            .kernel()
            .store
            .write(|conn| super::steering::bind_runtime(conn, &child.thread.id, &runtime, process))
            .unwrap()
    };
    bind("original-process");
    super::stop_all::stop_thread_work(fixture.kernel(), &fixture.caller.thread_id, "frozen")
        .await
        .unwrap();
    let effect = fixture
        .kernel()
        .store
        .effects()
        .unwrap()
        .into_iter()
        .find(|effect| {
            effect.thread_id == child.thread.id
                && matches!(
                    effect.request,
                    super::effects::EffectRequest::ManagedRunInterrupt { .. }
                )
        })
        .unwrap();
    let frozen: Option<String> = fixture
        .kernel()
        .store
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT target_json FROM orchestration_control_targets WHERE effect_id=?1",
                [&effect.id],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert!(frozen.unwrap().contains("original-process"));
    // A replacement process on the same logical attempt is never repaired by
    // the original Stop's settlement.
    bind("replacement-process");
    fixture
        .op(
            &child.thread.id,
            TaskOperation::ControlSettlement {
                effect_id: effect.id.clone(),
            },
        )
        .await;
    let after = fixture.child(&task);
    assert_eq!(after.runs[0].status, child.runs[0].status);
    assert_eq!(after.attempts[0].status, child.attempts[0].status);
}
