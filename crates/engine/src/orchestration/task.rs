//! App-owned delegation. Provider-native observations never call `Delegate`.
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rusqlite::Connection;
use serde_json::{Value, json};
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::*;
use zeron_proto::provider_instance::{ModelSelection, ProviderDriverKind};
use zeron_proto::{AgentEvent, InteractionMode, RuntimeMode};

use super::command::{Command, ExecutionSeed, Operation, Plan, run_terminal};
use super::effects::EffectRequest;
use super::event::{encode_component, iso, mcp_command_id};
use super::projection::{self, ThreadProjection};
use super::service::{CallerScope, OrchestratorService, ToolError};
use super::{Error, Kernel, ReceiptStatus, Result};

/// Catalog resolution remains a shared live MCP/composer service, not a
/// hardcoded vendor list in the task domain. It runs BEFORE receipt replay.
#[async_trait]
pub trait DelegationTargets: Send + Sync {
    async fn resolve(
        &self,
        parent: &OrchestrationV2AppThread,
        target: Option<&DelegateTaskInputTarget>,
    ) -> std::result::Result<ResolvedTarget, ToolError>;
}

#[async_trait]
pub trait DelegationCatalog: Send + Sync {
    /// The same live configured-instance catalog exposed by capabilities and
    /// the composer. Availability/auth/adapter constraints are already joined.
    async fn providers(
        &self,
    ) -> std::result::Result<
        Vec<zeron_proto::provider_instance::OrchestratorMcpProviderCapability>,
        ToolError,
    >;
}

pub struct CatalogTargets(pub Arc<dyn DelegationCatalog>);

#[async_trait]
impl DelegationTargets for CatalogTargets {
    async fn resolve(
        &self,
        parent: &OrchestrationV2AppThread,
        target: Option<&DelegateTaskInputTarget>,
    ) -> std::result::Result<ResolvedTarget, ToolError> {
        use OrchestratorMcpFailureCode as Code;
        let providers = self.0.providers().await?;
        let instance = target.and_then(|target| target.provider_instance_id.as_ref());
        let driver = target.and_then(|target| target.driver_kind.as_ref());
        let available =
            |provider: &&zeron_proto::provider_instance::OrchestratorMcpProviderCapability| {
                provider.can_run_child_task && provider.constraints.is_empty()
            };
        let instance = if let Some(instance) = instance {
            instance.clone()
        } else if let Some(driver) = driver {
            let candidates: Vec<_> = providers
                .iter()
                .filter(|provider| provider.driver_kind.0 == *driver)
                .collect();
            if candidates.is_empty() {
                return Err(ToolError::new(
                    Code::ProviderUnavailable,
                    format!("No V2 provider adapter is registered for driver {driver}."),
                ));
            }
            candidates
                .iter()
                .copied()
                .filter(available)
                .find(|provider| {
                    provider.provider_instance_id == parent.model_selection.instance_id
                })
                .or_else(|| candidates.iter().copied().find(available))
                .map(|provider| provider.provider_instance_id.0.clone())
                .ok_or_else(|| {
                    ToolError::new(
                        Code::ProviderUnavailable,
                        format!("No available V2 provider instance for driver {driver}."),
                    )
                })?
        } else {
            parent.model_selection.instance_id.0.clone()
        };
        let provider = providers
            .iter()
            .find(|provider| provider.provider_instance_id.0 == instance)
            .ok_or_else(|| {
                ToolError::new(
                    Code::ProviderUnavailable,
                    format!("Provider instance {instance} is not registered."),
                )
            })?;
        if let Some(driver) = driver
            && provider.driver_kind.0 != *driver
        {
            return Err(ToolError::new(
                Code::InvalidRequest,
                format!(
                    "Provider instance {instance} uses driver {}, not {driver}.",
                    provider.driver_kind.0
                ),
            ));
        }
        if !provider.can_run_child_task || !provider.constraints.is_empty() {
            return Err(ToolError::new(
                Code::ProviderUnavailable,
                format!(
                    "Provider {instance} cannot run a child task: {}",
                    provider.constraints.join(" ")
                ),
            ));
        }
        let requested_model = target.and_then(|target| target.model.as_ref());
        let model = requested_model
            .cloned()
            .or_else(|| {
                if instance == parent.model_selection.instance_id.0 {
                    Some(parent.model_selection.model.clone())
                } else {
                    provider.models.first().map(|model| model.id.clone())
                }
            })
            .ok_or_else(|| {
                ToolError::new(
                    Code::ModelUnavailable,
                    format!("Provider {instance} has no model available for inheritance."),
                )
            })?;
        if requested_model.is_some()
            && !provider.models.is_empty()
            && !provider.models.iter().any(|entry| entry.id == model)
        {
            return Err(ToolError::new(
                Code::ModelUnavailable,
                format!("Model {model} is not advertised by provider {instance}."),
            ));
        }
        let options = target.and_then(|target| target.options.as_ref());
        if instance == parent.model_selection.instance_id.0
            && model == parent.model_selection.model
            && options.is_none()
        {
            return Ok(ResolvedTarget {
                selection: parent.model_selection.clone(),
                driver: provider.driver_kind.clone(),
            });
        }
        let mut selection = json!({"instanceId":instance,"model":model});
        if let Some(options) = options {
            let value = serde_json::to_value(options).map_err(tool_error)?;
            let options = if let Some(map) = value.as_object() {
                map.iter()
                    .map(|(id, value)| json!({"id":id,"value":value}))
                    .collect::<Vec<_>>()
            } else {
                value.as_array().unwrap().clone()
            };
            let descriptors = provider
                .models
                .iter()
                .find(|entry| entry.id == model)
                .and_then(|entry| entry.options.as_ref());
            let mut seen = std::collections::BTreeSet::new();
            let mut problems = vec![];
            for option in &options {
                let id = option["id"].as_str().unwrap_or("");
                if !seen.insert(id) {
                    problems.push(format!("Option {id} was specified more than once."));
                    continue;
                }
                if let Some(descriptors) = descriptors {
                    let descriptors = serde_json::to_value(descriptors).map_err(tool_error)?;
                    let descriptors = descriptors.as_array().unwrap();
                    if let Some(descriptor) =
                        descriptors.iter().find(|descriptor| descriptor["id"] == id)
                    {
                        if descriptor["type"] == "boolean" && !option["value"].is_boolean() {
                            problems.push(format!("Option {id} expects a boolean value."));
                        } else if descriptor["type"] == "select"
                            && !descriptor["options"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|choice| choice["id"] == option["value"])
                        {
                            let choices = descriptor["options"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .filter_map(|choice| choice["id"].as_str())
                                .collect::<Vec<_>>()
                                .join(", ");
                            problems.push(format!("Option {id} must be one of: {choices}."));
                        }
                    } else {
                        let known = descriptors
                            .iter()
                            .filter_map(|descriptor| descriptor["id"].as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        problems.push(format!(
                            "Unknown option {id}; supported options: {}.",
                            if known.is_empty() { "none" } else { &known }
                        ));
                    }
                }
            }
            if !problems.is_empty() {
                return Err(ToolError::new(
                    Code::InvalidRequest,
                    format!(
                        "Model {model} on provider {instance} rejected options: {}",
                        problems.join(" ")
                    ),
                ));
            }
            selection["options"] = json!(options);
        }
        Ok(ResolvedTarget {
            selection: serde_json::from_value(selection).map_err(tool_error)?,
            driver: provider.driver_kind.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    pub selection: ModelSelection,
    pub driver: ProviderDriverKind,
}

#[derive(Clone)]
pub struct DelegationService {
    pub kernel: Kernel,
    pub targets: Arc<dyn DelegationTargets>,
}

#[derive(Debug, Clone)]
pub struct DelegateRequest {
    pub parent_run_id: RunId,
    pub parent_node_id: NodeId,
    pub target: ResolvedTarget,
    pub prompt: String,
    pub title: Option<String>,
    pub runtime_mode: RuntimeMode,
    pub interaction_mode: InteractionMode,
    pub always_wake: bool,
    pub cwd: String,
}

/// Owner-only planner inputs; these are not additional public MCP schemas.
#[derive(Debug, Clone)]
pub enum TaskOperation {
    Delegate(DelegateRequest),
    Finalize {
        child_thread_id: ThreadId,
    },
    Observe {
        task_id: NodeId,
        observed_by: Option<RunId>,
        dispose: bool,
    },
    WakePolicy {
        task_id: NodeId,
    },
    Cancel {
        task_id: NodeId,
        child_thread_id: ThreadId,
        run_id: RunId,
    },
    RunnerEvent {
        run_id: RunId,
        attempt_id: RunAttemptId,
        event: AgentEvent,
        capabilities: Option<Box<OrchestrationV2ProviderCapabilities>>,
    },
    Reconcile,
    Delivery(super::mailbox::DeliveryCommand),
    /// The runner only advances queued notification work when no active turn
    /// exists, never by interrupting the parent.
    DrainQueue,
    StopCohort {
        run_id: RunId,
    },
    StartMessage {
        prompt: String,
        driver: ProviderDriverKind,
        message_id: MessageId,
    },
    CreationRecord {
        target_thread_id: ThreadId,
    },
}

impl TaskOperation {
    pub fn lock_threads(&self, command: &CommandId) -> Vec<ThreadId> {
        match self {
            Self::Delegate(_) => vec![delegated_thread_id(command)],
            Self::Finalize { child_thread_id }
            | Self::Cancel {
                child_thread_id, ..
            } => vec![child_thread_id.clone()],
            Self::CreationRecord { target_thread_id } => vec![target_thread_id.clone()],
            _ => vec![],
        }
    }

    pub fn command_type(&self) -> &'static str {
        match self {
            Self::Delegate(_) => "delegated_task.request",
            Self::Finalize { .. } => "delegated_task.finalize",
            Self::Observe { dispose: true, .. } => "delegated_task.completion-delivery.dispose",
            Self::Observe { .. } => "delegated_task.completion-delivery.acknowledge",
            Self::WakePolicy { .. } => "delegated_task.wake-policy",
            Self::Cancel { .. } => "run.interrupt",
            Self::RunnerEvent { .. } => "kernel.runner.event",
            Self::Reconcile => "delegated_task.reconcile",
            Self::Delivery(_) => "notification.delivery",
            Self::DrainQueue => "notification.queue.drain",
            Self::StopCohort { .. } => "delegated_task.cohort.stop",
            Self::StartMessage { .. } => "message.dispatch",
            Self::CreationRecord { .. } => "thread.created.record",
        }
    }
}

pub fn delegated_id(kind: &str, command: &CommandId) -> String {
    format!("{kind}:delegated-task:{}", encode_component(&command.0))
}
pub fn delegated_task_id(command: &CommandId) -> NodeId {
    NodeId(delegated_id("node", command))
}
pub fn delegated_thread_id(command: &CommandId) -> ThreadId {
    ThreadId(delegated_id("thread", command))
}

// JavaScript title budgets are UTF-16 units, not Rust scalars. A split
// surrogate cannot inhabit String; stop before it (documented wire limitation).
fn clipped_title(text: &str, maximum: usize) -> String {
    if text.encode_utf16().count() <= maximum {
        return text.into();
    }
    let mut units = 0;
    let prefix: String = text
        .chars()
        .take_while(|ch| {
            units += ch.len_utf16();
            units <= maximum - 3
        })
        .collect();
    format!("{prefix}...")
}

pub(crate) fn records<'a>(projection: &'a ThreadProjection, kind: &str) -> &'a [Value] {
    projection
        .records
        .get(kind)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

pub(crate) fn terminal(status: &str) -> bool {
    matches!(
        status,
        "completed" | "failed" | "cancelled" | "interrupted" | "rolled_back"
    )
}

pub(crate) fn active_run(projection: &ThreadProjection) -> Option<&OrchestrationV2Run> {
    projection
        .runs
        .iter()
        .rev()
        .find(|run| !run_terminal(&run.status) && run.status != OrchestrationV2RunStatus::Queued)
}

pub(crate) fn find_task(projection: &ThreadProjection, id: &NodeId) -> Result<Value> {
    records(projection, "subagent")
        .iter()
        .find(|task| {
            task["id"] == id.0
                && task["origin"] == "app_owned"
                && task["threadId"] == projection.thread.id.0
                && task["childThreadId"].is_string()
        })
        .cloned()
        .ok_or_else(|| {
            Error::Invariant(format!(
                "Delegated task {} does not belong to thread {}.",
                id.0, projection.thread.id.0
            ))
        })
}

/// Monitor and rolled-back runs do not count as work. Published results are
/// handled separately; later runs may change progress without reopening a task.
fn monitor_run(projection: &ThreadProjection, run: &OrchestrationV2Run) -> bool {
    records(projection, "message").iter().any(|message| {
        message["runId"] == run.id.0 && message["notification"]["source"]["kind"] == "monitor"
    })
}

pub fn progress(projection: &ThreadProjection) -> (&'static str, Option<&OrchestrationV2Run>) {
    let work: Vec<_> = projection
        .runs
        .iter()
        .filter(|run| {
            !monitor_run(projection, run) && run.status != OrchestrationV2RunStatus::RolledBack
        })
        .collect();
    let result = work
        .iter()
        .copied()
        .filter(|run| run_terminal(&run.status) && (run.started_at.is_some() || run.ordinal == 1))
        .max_by_key(|run| run.ordinal);
    let active = work.iter().any(|run| !run_terminal(&run.status));
    let children = records(projection, "subagent").iter().any(|task| {
        !terminal(task["status"].as_str().unwrap_or("running"))
            || matches!(
                task["completionDelivery"]["state"].as_str(),
                Some("pending" | "claimed")
            )
    }) || records(projection, "provider-thread")
        .iter()
        .any(|provider| {
            provider["pendingBackgroundTasks"]
                .as_array()
                .is_some_and(|tasks| !tasks.is_empty())
        });
    (
        if active || result.is_none() {
            "working"
        } else if children {
            "waiting_for_children"
        } else {
            "result_available"
        },
        result,
    )
}

pub(crate) fn result_text(projection: &ThreadProjection, run: &OrchestrationV2Run) -> String {
    if run.status == OrchestrationV2RunStatus::Failed
        && let Some(error) = records(projection, "turn-item")
            .iter()
            .rev()
            .find(|item| item["runId"] == run.id.0 && item["type"] == "error")
        && let Some(text) = error["failure"]["message"].as_str()
    {
        return text.into();
    }
    if let Some(message) = records(projection, "message").iter().rev().find(|message| {
        message["runId"] == run.id.0
            && message["role"] == "assistant"
            && message["text"]
                .as_str()
                .is_some_and(|text| !text.trim().is_empty())
    }) {
        return message["text"].as_str().unwrap().into();
    }
    if let Some(item) = records(projection, "turn-item").iter().rev().find(|item| {
        item["runId"] == run.id.0
            && item["type"] == "assistant_message"
            && item["text"]
                .as_str()
                .is_some_and(|text| !text.trim().is_empty())
    }) {
        return item["text"].as_str().unwrap().into();
    }
    if run.status == OrchestrationV2RunStatus::Completed {
        "Child task completed without an assistant result.".into()
    } else {
        format!(
            "Child task ended with status {}.",
            serde_json::to_value(&run.status).unwrap().as_str().unwrap()
        )
    }
}

pub(crate) fn message(
    thread: &ThreadId,
    run: Option<&RunId>,
    node: Option<&NodeId>,
    id: &str,
    text: &str,
    role: &str,
    now: i64,
) -> Result<Value> {
    Ok(json!({
        "id":id,"threadId":thread,"runId":run,"nodeId":node,"role":role,
        "createdBy":"agent","creationSource":"mcp","text":text,"attachments":[],
        "streaming":false,"createdAt":iso(now)?,"updatedAt":iso(now)?
    }))
}

pub(crate) fn execution_seed(
    thread: &OrchestrationV2AppThread,
    ordinal: i64,
    message_id: &str,
    status: &str,
    driver: &str,
    now: i64,
) -> Result<ExecutionSeed> {
    let run = format!("run:{}:{ordinal}", encode_component(&thread.id.0));
    let attempt = format!("run-attempt:{}:1", encode_component(&run));
    let root = format!("node:{}:1", encode_component(&run));
    let provider = format!("provider-thread:app:{}", encode_component(&thread.id.0));
    let time = iso(now)?;
    Ok(ExecutionSeed {
        run: serde_json::from_value(json!({
            "id":run,"threadId":thread.id,"ordinal":ordinal,"providerInstanceId":thread.provider_instance_id,
            "modelSelection":thread.model_selection,"providerThreadId":provider,"userMessageId":message_id,
            "rootNodeId":root,"activeAttemptId":attempt,"status":status,"requestedAt":time,
            "startedAt":null,"completedAt":null,"checkpointId":null,"contextHandoffId":null
        }))?,
        attempt: serde_json::from_value(json!({
            "id":attempt,"runId":run,"attemptOrdinal":1,"rootNodeId":root,
            "providerInstanceId":thread.provider_instance_id,"providerThreadId":provider,"providerTurnId":null,
            "reason":"initial","status":"pending","startedAt":null,"completedAt":null
        }))?,
        root: serde_json::from_value(json!({
            "id":root,"threadId":thread.id,"runId":run,"rootNodeId":root,"parentNodeId":null,
            "kind":"root_turn","status":"pending","countsForRun":true,"providerThreadId":provider,
            "providerTurnId":null,"nativeItemRef":null,"runtimeRequestId":null,
            "checkpointScopeId":null,"startedAt":null,"completedAt":null
        }))?,
        provider_thread: serde_json::from_value(json!({
            "id":provider,"driver":driver,"providerInstanceId":thread.provider_instance_id,
            "providerSessionId":null,"appThreadId":thread.id,"ownerNodeId":root,"nativeThreadRef":null,
            "nativeConversationHeadRef":null,"status":"not_loaded","firstRunOrdinal":1,"lastRunOrdinal":ordinal,
            "handoffIds":[],"forkedFrom":null,"pendingBackgroundTasks":[],"createdAt":time,"updatedAt":time
        }))?,
    })
}

pub(crate) fn emit_execution(
    plan: &mut Plan,
    command: &Command,
    thread: &ThreadId,
    seed: &ExecutionSeed,
    now: i64,
) -> Result<()> {
    plan.emit_on(
        command,
        thread,
        "provider-thread.updated",
        &seed.provider_thread,
        now,
    )?;
    plan.emit_on(command, thread, "run.created", &seed.run, now)?;
    plan.emit_on(command, thread, "run-attempt.created", &seed.attempt, now)?;
    plan.emit_on(command, thread, "node.updated", &seed.root, now)?;
    if seed.run.status == OrchestrationV2RunStatus::Starting {
        plan.routed_effects.push((
            thread.clone(),
            EffectRequest::ProviderTurnStart {
                run_id: seed.run.id.clone(),
            },
        ));
    }
    Ok(())
}

pub(crate) fn plan(
    conn: &Connection,
    command: &Command,
    operation: &TaskOperation,
    now: i64,
) -> Result<Plan> {
    let projection = projection::read_thread(conn, &command.thread_id)?.ok_or_else(|| {
        Error::Invariant(format!("Thread {} was not found.", command.thread_id.0))
    })?;
    let mut plan = Plan::default();
    match operation {
        TaskOperation::Delegate(request) => {
            let run = projection
                .runs
                .iter()
                .find(|run| {
                    run.id == request.parent_run_id
                        && !run_terminal(&run.status)
                        && run.status != OrchestrationV2RunStatus::Queued
                })
                .ok_or_else(|| Error::Invariant("Parent run is not active.".into()))?;
            if run.root_node_id.is_none()
                || !projection.nodes.iter().any(|node| {
                    node.id == request.parent_node_id && node.run_id.as_ref() == Some(&run.id)
                })
                || projection.thread.archived_at.is_some()
                || projection.thread.deleted_at.is_some()
                || !projection.thread.runtime_mode.permits(request.runtime_mode)
                || !projection
                    .thread
                    .interaction_mode
                    .permits(request.interaction_mode)
            {
                return Err(Error::Invariant(
                    "Invalid delegation parent or inherited policy.".into(),
                ));
            }
            let child_id = delegated_thread_id(&command.id);
            if projection::read_thread(conn, &child_id)?.is_some() {
                return Err(Error::Invariant(
                    "Delegated child identity already exists.".into(),
                ));
            }
            let task_id = delegated_task_id(&command.id);
            let child_message = delegated_id("message", &command.id);
            let item_id = delegated_id("turn-item", &command.id);
            let title = request
                .title
                .as_deref()
                .filter(|title| !title.trim().is_empty())
                .unwrap_or(&request.prompt)
                .trim();
            let title = clipped_title(title, 72);
            let mut child = serde_json::to_value(&projection.thread)?;
            child["id"] = json!(child_id);
            child["title"] = json!(title);
            child["createdBy"] = json!("agent");
            child["creationSource"] = json!("mcp");
            child["modelSelection"] = json!(request.target.selection);
            child["providerInstanceId"] = json!(request.target.selection.instance_id);
            child["runtimeMode"] = json!(request.runtime_mode);
            child["interactionMode"] = json!(request.interaction_mode);
            child["lineage"] = json!({"parentThreadId":projection.thread.id,"relationshipToParent":"subagent","rootThreadId":projection.thread.lineage.root_thread_id});
            child["forkedFrom"] = json!({"type":"node","nodeId":task_id});
            child["createdAt"] = json!(iso(now)?);
            child["updatedAt"] = json!(iso(now)?);
            for key in [
                "activeProviderThreadId",
                "archivedAt",
                "settledOverride",
                "settledAt",
                "snoozedUntil",
                "snoozedAt",
                "lastVisitedAt",
                "deletedAt",
            ] {
                child[key] = Value::Null;
            }
            child.as_object_mut().unwrap().remove("historyOrigin");
            // A bound checkout wins; otherwise persist the caller's resolved cwd
            // as the child binding, so the effect needs no ephemeral caller.
            if child["worktreePath"].is_null() {
                child["worktreePath"] = json!(request.cwd);
            }
            let child: OrchestrationV2AppThread = serde_json::from_value(child)?;
            let seed = execution_seed(
                &child,
                1,
                &child_message,
                "starting",
                &request.target.driver.0,
                now,
            )?;
            let task = json!({
                "id":task_id,"threadId":command.thread_id,"runId":run.id,"parentNodeId":request.parent_node_id,
                "origin":"app_owned","createdBy":"agent","driver":request.target.driver,
                "providerInstanceId":request.target.selection.instance_id,"providerThreadId":null,
                "childThreadId":child_id,"nativeTaskRef":null,"prompt":request.prompt,"title":request.title,
                "model":request.target.selection.model,"completionWake":if request.always_wake {"always"} else {"settled_only"},
                "status":"running","result":null,"startedAt":iso(now)?,"completedAt":null,"updatedAt":iso(now)?
            });
            let node = json!({
                "id":task_id,"threadId":command.thread_id,"runId":run.id,"parentNodeId":request.parent_node_id,
                "rootNodeId":run.root_node_id,"kind":"subagent","status":"running","countsForRun":false,
                "providerThreadId":null,"providerTurnId":null,"nativeItemRef":null,"runtimeRequestId":null,
                "checkpointScopeId":null,"startedAt":iso(now)?,"completedAt":null
            });
            let item = json!({
                "id":item_id,"threadId":command.thread_id,"runId":run.id,"nodeId":task_id,
                "providerThreadId":run.provider_thread_id,"providerTurnId":null,"nativeItemRef":null,
                "parentItemId":null,"ordinal":records(&projection,"turn-item").len()+1,"status":"running",
                "title":title,"startedAt":iso(now)?,"completedAt":null,"updatedAt":iso(now)?,"type":"subagent",
                "subagentId":task_id,"origin":"app_owned","driver":request.target.driver,
                "providerInstanceId":request.target.selection.instance_id,"childThreadId":child_id,
                "prompt":request.prompt,"result":null
            });
            plan.emit_on(command, &child_id, "thread.created", &child, now)?;
            plan.emit(command, "node.updated", &node, now)?;
            plan.emit(command, "subagent.updated", &task, now)?;
            plan.emit(command, "turn-item.updated", &item, now)?;
            emit_execution(&mut plan, command, &child_id, &seed, now)?;
            let mut initial = message(
                &child_id,
                Some(&seed.run.id),
                Some(&seed.root.id),
                &child_message,
                &request.prompt,
                "user",
                now,
            )?;
            initial["senderThreadId"] = json!(command.thread_id);
            plan.emit_on(command, &child_id, "message.updated", &initial, now)?;
            let transfer = transfer(
                command,
                "subagent_spawn",
                TransferEnd {
                    thread: &projection.thread.id,
                    run: Some(&run.id),
                    instance: &run.provider_instance_id.0,
                },
                TransferEnd {
                    thread: &child_id,
                    run: Some(&seed.run.id),
                    instance: &request.target.selection.instance_id.0,
                },
                now,
            )?;
            plan.emit_on(
                command,
                &child_id,
                "context-transfer.created",
                &transfer,
                now,
            )?;
        }
        TaskOperation::Finalize { child_thread_id } => {
            let child = projection::read_thread(conn, child_thread_id)?
                .ok_or_else(|| Error::Invariant("Child thread is missing.".into()))?;
            if child.thread.lineage.parent_thread_id.as_ref() != Some(&command.thread_id)
                || serde_json::to_value(&child.thread.lineage)?["relationshipToParent"]
                    != "subagent"
            {
                return Err(Error::Invariant(
                    "Child is not owned by this parent.".into(),
                ));
            }
            let (state, run) = progress(&child);
            if state == "result_available"
                && let Some(run) = run
                && let Some(task) = records(&projection, "subagent").iter().find(|task| {
                    task["childThreadId"] == child_thread_id.0
                        && task["origin"] == "app_owned"
                        && task["result"].is_null()
                })
            {
                let mut task = task.clone();
                task["status"] = serde_json::to_value(&run.status)?;
                task["result"] = json!(result_text(&child, run));
                task["providerThreadId"] = json!(run.provider_thread_id);
                task["completedAt"] = json!(iso(now)?);
                task["updatedAt"] = json!(iso(now)?);
                if !matches!(
                    task["completionDelivery"]["state"].as_str(),
                    Some("acknowledged" | "disposed" | "delivered")
                ) {
                    task["completionDelivery"] = json!({"state":"pending","observedByRunId":null});
                }
                for node in &projection.nodes {
                    if node.id.0 == task["id"].as_str().unwrap() {
                        let mut node = serde_json::to_value(node)?;
                        node["status"] = task["status"].clone();
                        node["completedAt"] = task["completedAt"].clone();
                        node["providerThreadId"] = task["providerThreadId"].clone();
                        plan.emit(command, "node.updated", &node, now)?;
                    }
                }
                for item in records(&projection, "turn-item") {
                    if item["subagentId"] == task["id"] {
                        let mut item = item.clone();
                        item["status"] = task["status"].clone();
                        item["result"] = task["result"].clone();
                        item["completedAt"] = task["completedAt"].clone();
                        item["updatedAt"] = task["updatedAt"].clone();
                        plan.emit(command, "turn-item.updated", &item, now)?;
                    }
                }
                let parent_run = task["runId"].as_str().map(|id| RunId(id.into()));
                let mut transfer = transfer(
                    command,
                    "subagent_result",
                    TransferEnd {
                        thread: child_thread_id,
                        run: Some(&run.id),
                        instance: &run.provider_instance_id.0,
                    },
                    TransferEnd {
                        thread: &command.thread_id,
                        run: parent_run.as_ref(),
                        instance: &projection.thread.provider_instance_id.0,
                    },
                    now,
                )?;
                if let Some(parent_run) = parent_run
                    .as_ref()
                    .and_then(|id| projection.runs.iter().find(|run| &run.id == id))
                    && let (Some(source_provider), Some(target_provider)) =
                        (&run.provider_thread_id, &parent_run.provider_thread_id)
                {
                    let handoff_id = format!(
                        "context-handoff:subagent-result:{}",
                        encode_component(&command.id.0)
                    );
                    let summary_message = records(&child, "message")
                        .iter()
                        .rev()
                        .find(|message| {
                            message["runId"] == run.id.0 && message["role"] == "assistant"
                        })
                        .map(|message| message["id"].clone())
                        .unwrap_or(Value::Null);
                    let handoff = json!({
                        "id":handoff_id,"transferId":transfer["id"],"threadId":command.thread_id,"targetRunId":parent_run.id,
                        "fromProviderThreadIds":[source_provider],"toProviderThreadId":target_provider,
                        "coveredRunOrdinals":{"from":run.ordinal,"to":run.ordinal},"strategy":"manual_context","status":"ready",
                        "summaryMessageId":summary_message,"summaryText":task["result"],"createdByProviderInstanceId":run.provider_instance_id,
                        "createdAt":iso(now)?,"updatedAt":iso(now)?
                    });
                    transfer["resolution"] =
                        json!({"strategy":"portable_context","contextHandoffId":handoff_id});
                    plan.emit(command, "context-handoff.updated", &handoff, now)?;
                }
                plan.emit(command, "context-transfer.created", &transfer, now)?;
                super::mailbox::reserve(&projection, vec![task], command, &mut plan, now)?;
            }
        }
        TaskOperation::Observe {
            task_id,
            observed_by,
            dispose,
        } => {
            let mut task = find_task(&projection, task_id)?;
            if *dispose || terminal(task["status"].as_str().unwrap_or("")) {
                let old = task["completionDelivery"]["state"].as_str();
                if old != Some("disposed") && (*dispose || old != Some("acknowledged")) {
                    task["completionDelivery"] = json!({"state":if *dispose {"disposed"} else {"acknowledged"},"observedByRunId":observed_by});
                    task["updatedAt"] = json!(iso(now)?);
                }
                plan.emit(command, "subagent.updated", &task, now)?;
                super::mailbox::remove_member(&projection, task_id, command, &mut plan, now)?;
            }
        }
        TaskOperation::WakePolicy { task_id } => {
            let mut task = find_task(&projection, task_id)?;
            task["completionWake"] = json!("always");
            task["updatedAt"] = json!(iso(now)?);
            super::mailbox::reserve(&projection, vec![task], command, &mut plan, now)?;
        }
        TaskOperation::Cancel {
            task_id,
            child_thread_id,
            run_id,
        } => {
            let mut task = find_task(&projection, task_id)?;
            if task["childThreadId"] != child_thread_id.0 || !task["result"].is_null() {
                return Err(Error::Invariant(
                    "Task cancellation ownership changed.".into(),
                ));
            }
            let child = projection::read_thread(conn, child_thread_id)?
                .ok_or_else(|| Error::Invariant("Child missing.".into()))?;
            if active_run(&child).is_none_or(|run| &run.id != run_id) {
                return Err(Error::Invariant("Child run is not interruptible.".into()));
            }
            let mut child_command = command.clone();
            child_command.thread_id = child_thread_id.clone();
            // Stop this run's cohort and hold its queued work atomically with
            // disposal in the parent. This is not recursive descendant kill.
            super::mailbox::stop(&child, &child_command, &mut plan, run_id, now)?;
            task["completionDelivery"] = json!({"state":"disposed","observedByRunId":null});
            plan.emit(command, "subagent.updated", &task, now)?;
            plan.cancel_threads.push(child_thread_id.clone());
            plan.routed_effects.push((
                child_thread_id.clone(),
                EffectRequest::ManagedRunInterrupt {
                    run_id: run_id.clone(),
                },
            ));
            super::mailbox::remove_member(&projection, task_id, command, &mut plan, now)?;
        }
        TaskOperation::RunnerEvent {
            run_id,
            attempt_id,
            event,
            capabilities,
        } => super::runner::plan_event(
            conn,
            &projection,
            command,
            &mut plan,
            run_id,
            attempt_id,
            event,
            capabilities.as_deref(),
            now,
        )?,
        TaskOperation::Reconcile => {
            super::mailbox::reserve(&projection, vec![], command, &mut plan, now)?
        }
        TaskOperation::Delivery(delivery) => {
            super::mailbox::plan_delivery(&projection, command, &mut plan, delivery, now)?
        }
        TaskOperation::DrainQueue => {
            super::continuation::drain(&projection, command, &mut plan, now)?
        }
        TaskOperation::StopCohort { run_id } => {
            super::mailbox::stop(&projection, command, &mut plan, run_id, now)?
        }
        TaskOperation::StartMessage {
            prompt,
            driver,
            message_id,
        } => {
            if active_run(&projection).is_some()
                || projection.thread.archived_at.is_some()
                || projection.thread.deleted_at.is_some()
            {
                return Err(Error::Invariant("Thread is not sendable.".into()));
            }
            let ordinal = projection
                .runs
                .iter()
                .map(|run| run.ordinal)
                .max()
                .unwrap_or(0)
                + 1;
            let seed = execution_seed(
                &projection.thread,
                ordinal,
                &message_id.0,
                "starting",
                &driver.0,
                now,
            )?;
            emit_execution(&mut plan, command, &command.thread_id, &seed, now)?;
            let initial = message(
                &command.thread_id,
                Some(&seed.run.id),
                Some(&seed.root.id),
                &message_id.0,
                prompt,
                "user",
                now,
            )?;
            plan.emit(command, "message.updated", &initial, now)?;
        }
        TaskOperation::CreationRecord { target_thread_id } => {
            let target = projection::read_thread(conn, target_thread_id)?
                .ok_or_else(|| Error::Invariant("Created thread missing.".into()))?;
            let parent_run = active_run(&projection)
                .ok_or_else(|| Error::Invariant("Creation parent no longer active.".into()))?;
            let item = json!({
                "id":format!("turn-item:created-thread:{}",encode_component(&command.id.0)),"type":"thread_created",
                "threadId":projection.thread.id,"runId":parent_run.id,"nodeId":parent_run.root_node_id,
                "providerThreadId":parent_run.provider_thread_id,"providerTurnId":null,"nativeItemRef":null,
                "parentItemId":null,"ordinal":records(&projection,"turn-item").len()+1,"status":"completed",
                "title":target.thread.title,"startedAt":iso(now)?,"completedAt":iso(now)?,"updatedAt":iso(now)?,
                "targetThreadId":target_thread_id,"targetRunId":target.runs.last().map(|run| &run.id),
                "targetProviderInstanceId":target.thread.provider_instance_id,"targetModel":target.thread.model_selection.model
            });
            plan.emit(command, "turn-item.updated", &item, now)?;
        }
    }
    // Stale notifications are durable successful no-ops, not a fresh wake.
    if plan.events.is_empty() {
        plan.emit(command, "thread.metadata-updated", &projection.thread, now)?;
    }
    Ok(plan)
}

struct TransferEnd<'a> {
    thread: &'a ThreadId,
    run: Option<&'a RunId>,
    instance: &'a str,
}

fn transfer(
    command: &Command,
    kind: &str,
    source: TransferEnd<'_>,
    target: TransferEnd<'_>,
    now: i64,
) -> Result<Value> {
    Ok(json!({
        "id":format!("context-transfer:{kind}:{}",encode_component(&command.id.0)),
        "type":kind,"sourceThreadId":source.thread,"targetThreadId":target.thread,
        "sourcePoint":{"threadId":source.thread,"runId":source.run},"basePoint":null,
        "sourceProviderInstanceId":source.instance,"targetProviderInstanceId":target.instance,
        "targetRunId":target.run,"status":"consumed","resolution":null,"createdBy":"agent","error":null,
        "createdAt":iso(now)?,"updatedAt":iso(now)?,"consumedAt":iso(now)?
    }))
}

impl Kernel {
    pub async fn reconcile_ancestors(&self, child: &ThreadId) -> Result<()> {
        let mut child_id = child.clone();
        let mut visited = std::collections::BTreeSet::new();
        while visited.insert(child_id.clone()) {
            let Some(child) = self.store.thread(&child_id)? else {
                break;
            };
            let Some(parent_id) = child.thread.lineage.parent_thread_id.as_ref() else {
                break;
            };
            if child.thread.creation_source != OrchestrationV2CreationSource::Mcp
                || serde_json::to_value(&child.thread.lineage)?["relationshipToParent"]
                    != "subagent"
                || progress(&child).0 != "result_available"
            {
                break;
            }
            let Some(parent) = self.store.thread(parent_id)? else {
                break;
            };
            if records(&parent, "subagent").iter().any(|task| {
                task["origin"] == "app_owned"
                    && task["childThreadId"] == child_id.0
                    && task["result"].is_null()
            }) {
                self.task_command(
                    parent_id,
                    CommandId(format!(
                        "command:child-finalize:{}:{}",
                        encode_component(&child_id.0),
                        uuid::Uuid::new_v4()
                    )),
                    TaskOperation::Finalize {
                        child_thread_id: child_id.clone(),
                    },
                )
                .await?;
            }
            child_id = parent_id.clone();
        }
        Ok(())
    }
    pub async fn task_command(
        &self,
        parent: &ThreadId,
        id: CommandId,
        operation: TaskOperation,
    ) -> Result<()> {
        let receipt = self
            .dispatch(
                &Command {
                    id,
                    thread_id: parent.clone(),
                    operation: Operation::Task(Box::new(operation)),
                },
                crate::now_ms(),
            )
            .await?;
        if receipt.status == ReceiptStatus::Rejected {
            return Err(Error::Invariant(receipt.error.unwrap_or_default()));
        }
        Ok(())
    }

    /// Reconcile terminal children and every open cohort after kernel recovery.
    /// No provider work occurs here; only durable requests are reserved.
    pub async fn reconcile_delegation(&self) -> Result<()> {
        let threads = self.store.read(|conn| {
            let mut stmt =
                conn.prepare("SELECT id FROM orchestration_projection_threads ORDER BY id")?;
            Ok(stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?)
        })?;
        for id in &threads {
            let child = self.store.thread(&ThreadId(id.clone()))?.unwrap();
            if child.thread.creation_source == OrchestrationV2CreationSource::Mcp
                && let Some(parent) = child.thread.lineage.parent_thread_id.as_ref()
                && serde_json::to_value(&child.thread.lineage)?["relationshipToParent"]
                    == "subagent"
            {
                self.task_command(
                    parent,
                    CommandId(format!(
                        "command:reconcile-result:{}:{}",
                        encode_component(id),
                        uuid::Uuid::new_v4()
                    )),
                    TaskOperation::Finalize {
                        child_thread_id: child.thread.id.clone(),
                    },
                )
                .await?;
            }
        }
        for id in threads {
            self.task_command(
                &ThreadId(id),
                CommandId(format!("command:reconcile-mail:{}", uuid::Uuid::new_v4())),
                TaskOperation::Reconcile,
            )
            .await?;
        }
        Ok(())
    }
}

fn tool_error(error: impl std::fmt::Display) -> ToolError {
    ToolError::new(
        OrchestratorMcpFailureCode::OrchestrationError,
        error.to_string(),
    )
}
fn key(input: &Optional<String>) -> String {
    input
        .as_ref()
        .cloned()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

impl DelegationService {
    /// Sequential, separately receipted batch creation. Failure at index N
    /// retains accepted entries < N; retry uses the same per-index IDs.
    pub async fn create_threads(
        &self,
        caller: CallerScope,
        input: CreateThreadsInput,
    ) -> std::result::Result<CreateThreadsResult, ToolError> {
        let parent = self.parent(&caller, true)?;
        if input.threads.is_empty() || input.threads.len() > 20 {
            return Err(ToolError::new(
                OrchestratorMcpFailureCode::InvalidRequest,
                "Thread batch must contain 1–20 entries.",
            ));
        }
        let key = key(&input.client_request_id);
        let mut created = vec![];
        for (index, request) in input.threads.iter().enumerate() {
            let value = serde_json::to_value(request).map_err(tool_error)?;
            let target = value
                .get("target")
                .cloned()
                .map(serde_json::from_value::<DelegateTaskInputTarget>)
                .transpose()
                .map_err(tool_error)?;
            let target = self
                .targets
                .resolve(&parent.thread, target.as_ref())
                .await?;
            let runtime = match request.runtime_mode.as_ref() {
                None | Some(OrchestratorMcpRuntimeMode::Inherit) => parent.thread.runtime_mode,
                Some(mode) => {
                    serde_json::from_value(serde_json::to_value(mode).map_err(tool_error)?)
                        .map_err(tool_error)?
                }
            };
            let interaction = match request.interaction_mode.as_ref() {
                None | Some(OrchestratorMcpInteractionMode::Inherit) => {
                    parent.thread.interaction_mode
                }
                Some(mode) => {
                    serde_json::from_value(serde_json::to_value(mode).map_err(tool_error)?)
                        .map_err(tool_error)?
                }
            };
            if !caller.runtime_mode.permits(runtime) || !parent.thread.runtime_mode.permits(runtime)
            {
                return Err(ToolError::new(
                    OrchestratorMcpFailureCode::RuntimeModeEscalationDenied,
                    format!(
                        "Child runtime mode {} is broader than parent mode {}.",
                        serde_json::to_value(runtime).unwrap().as_str().unwrap(),
                        serde_json::to_value(parent.thread.runtime_mode)
                            .unwrap()
                            .as_str()
                            .unwrap()
                    ),
                ));
            }
            if !caller.interaction_mode.permits(interaction)
                || !parent.thread.interaction_mode.permits(interaction)
            {
                return Err(ToolError::new(
                    OrchestratorMcpFailureCode::InteractionModeEscalationDenied,
                    format!(
                        "Child interaction mode {} is broader than parent mode {}.",
                        serde_json::to_value(interaction).unwrap().as_str().unwrap(),
                        serde_json::to_value(parent.thread.interaction_mode)
                            .unwrap()
                            .as_str()
                            .unwrap()
                    ),
                ));
            }
            let thread = ThreadId(format!(
                "thread:mcp:{}:{}:{index}",
                encode_component(&caller.session_id),
                encode_component(&key)
            ));
            let detail = request
                .title
                .as_ref()
                .map(|text| text.trim())
                .filter(|text| !text.is_empty())
                .or_else(|| {
                    request
                        .prompt
                        .as_ref()
                        .map(|text| text.trim())
                        .filter(|text| !text.is_empty())
                });
            let title = detail
                .map(|text| clipped_title(text, 80))
                .unwrap_or_else(|| format!("{} thread {}", parent.thread.title, index + 1));
            let command = Command::wire(serde_json::from_value(json!({
                "type":"thread.create","commandId":format!("{}:{index}",mcp_command_id(&caller.session_id,"create-thread",&key).0),
                "createdBy":"agent","creationSource":"mcp","threadId":thread,"projectId":parent.thread.project_id,
                "title":title,"modelSelection":target.selection,"runtimeMode":runtime,"interactionMode":interaction,
                "branch":parent.thread.branch,"worktreePath":parent.thread.worktree_path.as_ref().cloned().unwrap_or_else(|| caller.workspace_root.to_string_lossy().into_owned())
            })).map_err(tool_error)?).map_err(tool_error)?;
            let receipt = self
                .kernel
                .dispatch(&command, crate::now_ms())
                .await
                .map_err(tool_error)?;
            if receipt.status == ReceiptStatus::Rejected {
                return Err(tool_error(receipt.error.unwrap_or_default()));
            }
            if let Some(prompt) = request.prompt.as_ref() {
                self.kernel
                    .task_command(
                        &thread,
                        CommandId(format!(
                            "{}:{index}",
                            mcp_command_id(&caller.session_id, "dispatch-thread", &key).0
                        )),
                        TaskOperation::StartMessage {
                            prompt: prompt.clone(),
                            driver: target.driver,
                            message_id: MessageId(format!(
                                "message:mcp:{}:{}:{index}",
                                encode_component(&caller.session_id),
                                encode_component(&key)
                            )),
                        },
                    )
                    .await
                    .map_err(tool_error)?;
            }
            self.kernel
                .task_command(
                    &caller.thread_id,
                    CommandId(format!(
                        "{}:{index}",
                        mcp_command_id(&caller.session_id, "record-created-thread", &key).0
                    )),
                    TaskOperation::CreationRecord {
                        target_thread_id: thread.clone(),
                    },
                )
                .await
                .map_err(tool_error)?;
            let projection = self
                .kernel
                .store
                .thread(&thread)
                .map_err(tool_error)?
                .unwrap();
            created.push(json!({"threadId":thread,"runId":projection.runs.last().map(|run| &run.id),
                "status":projection.runs.last().map(|run| serde_json::to_value(&run.status).unwrap()).unwrap_or(json!("idle")),
                "title":projection.thread.title,"createdBy":projection.thread.created_by,"creationSource":projection.thread.creation_source,
                "providerInstanceId":projection.thread.provider_instance_id,"model":projection.thread.model_selection.model}));
        }
        serde_json::from_value(json!({"threads":created})).map_err(tool_error)
    }

    /// Read/wait integration hook: waits only observe run state. Timeout never
    /// sends an interrupt and never acknowledges completion mail.
    pub async fn wait_child_run(
        &self,
        caller: &CallerScope,
        task: &NodeId,
        timeout: Duration,
    ) -> std::result::Result<bool, ToolError> {
        let wait = async {
            loop {
                let result = self.read_task(caller, task, false, false).await?;
                if !matches!(
                    result.status,
                    OrchestratorMcpDelegatedTaskStatus::Queued
                        | OrchestratorMcpDelegatedTaskStatus::Running
                        | OrchestratorMcpDelegatedTaskStatus::Waiting
                ) {
                    return Ok::<_, ToolError>(true);
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        tokio::time::timeout(timeout, wait)
            .await
            .unwrap_or(Ok(false))
    }

    /// P4 calls this only after its complete, direct-child terminal-result read;
    /// a partial read or thread_wait does not consume the result.
    pub async fn acknowledge_child_read(
        &self,
        caller: &CallerScope,
        child: &ThreadId,
        offset: usize,
        truncated: bool,
        complete_terminal_result: bool,
    ) -> std::result::Result<(), ToolError> {
        if offset != 0 || truncated || !complete_terminal_result {
            return Ok(());
        }
        let parent = self.parent(caller, false)?;
        if let Some(task) = records(&parent, "subagent")
            .iter()
            .find(|task| task["origin"] == "app_owned" && task["childThreadId"] == child.0)
        {
            self.read_task(
                caller,
                &NodeId(task["id"].as_str().unwrap().into()),
                false,
                true,
            )
            .await?;
        }
        Ok(())
    }

    fn parent(
        &self,
        caller: &CallerScope,
        active: bool,
    ) -> std::result::Result<ThreadProjection, ToolError> {
        let parent = self
            .kernel
            .store
            .thread(&caller.thread_id)
            .map_err(tool_error)?
            .ok_or_else(|| {
                ToolError::new(
                    OrchestratorMcpFailureCode::ThreadNotFound,
                    "Caller thread was not found.",
                )
            })?;
        if parent.thread.project_id != caller.project_id {
            return Err(ToolError::new(
                OrchestratorMcpFailureCode::CapabilityDenied,
                "Caller project scope does not own this thread.",
            ));
        }
        if active
            && (parent.thread.archived_at.is_some()
                || parent.thread.deleted_at.is_some()
                || active_run(&parent).is_none_or(|run| {
                    run.id != caller.run_id
                        || run.root_node_id.is_none()
                        || run.provider_instance_id != caller.provider_instance_id
                }))
        {
            return Err(ToolError::new(
                OrchestratorMcpFailureCode::ParentNotActive,
                "Delegated tasks require an active run owned by this MCP provider session.",
            ));
        }
        Ok(parent)
    }

    pub async fn read_task(
        &self,
        caller: &CallerScope,
        task_id: &NodeId,
        timeout: bool,
        ack: bool,
    ) -> std::result::Result<TaskStatusResult, ToolError> {
        let mut parent = self.parent(caller, false)?;
        let mut task = find_task(&parent, task_id).map_err(|error| {
            ToolError::new(OrchestratorMcpFailureCode::TaskNotFound, error.to_string())
        })?;
        let child_id = ThreadId(task["childThreadId"].as_str().unwrap().into());
        let child = self
            .kernel
            .store
            .thread(&child_id)
            .map_err(tool_error)?
            .ok_or_else(|| {
                ToolError::new(
                    OrchestratorMcpFailureCode::ThreadNotFound,
                    "Child thread was not found.",
                )
            })?;
        // A read can race terminal observation before its result publication.
        // Publish under the same parent/child locks before acknowledging, so
        // that consuming a derived result cannot leave a later automatic wake.
        if task["result"].is_null() && progress(&child).0 == "result_available" {
            self.kernel
                .task_command(
                    &caller.thread_id,
                    CommandId(format!("command:status-finalize:{}", uuid::Uuid::new_v4())),
                    TaskOperation::Finalize {
                        child_thread_id: child_id.clone(),
                    },
                )
                .await
                .map_err(tool_error)?;
            parent = self.parent(caller, false)?;
            task = find_task(&parent, task_id).map_err(tool_error)?;
        }
        let spawn = records(&child, "context-transfer").iter().find(|transfer| {
            transfer["type"] == "subagent_spawn"
                && transfer["sourceThreadId"] == caller.thread_id.0
                && transfer["targetThreadId"] == child_id.0
        });
        let original = match spawn {
            Some(transfer) => child
                .runs
                .iter()
                .find(|run| transfer["targetRunId"] == run.id.0),
            None => child.runs.first(), // Only legacy projections lack a spawn.
        };
        let latest = child
            .runs
            .iter()
            .filter(|run| {
                run_terminal(&run.status)
                    && !monitor_run(&child, run)
                    && run.status != OrchestrationV2RunStatus::RolledBack
                    && (run.started_at.is_some()
                        || original.is_some_and(|original| original.id == run.id))
            })
            .max_by_key(|run| run.ordinal);
        let (progress_state, result_run) = progress(&child);
        let published = task["result"].is_string();
        let state = if published {
            "result_available"
        } else {
            progress_state
        };
        let status = if published {
            task["status"].as_str().unwrap_or("completed").to_owned()
        } else if state == "result_available" {
            result_run
                .map(|run| status(&run.status))
                .unwrap_or("running")
                .to_owned()
        } else if original.is_some_and(|run| run.status == OrchestrationV2RunStatus::Queued) {
            "queued".into()
        } else {
            "running".into()
        };
        let summary = if published {
            task["result"].clone()
        } else if terminal(&status) {
            result_run
                .map(|run| json!(result_text(&child, run)))
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        let transfers: Vec<_> = records(&parent, "context-transfer")
            .iter()
            .filter(|transfer| {
                transfer["type"] == "subagent_result"
                    && transfer["sourceThreadId"] == child_id.0
                    && transfer["targetThreadId"] == caller.thread_id.0
            })
            .collect();
        let response: TaskStatusResult = serde_json::from_value(json!({
            "taskId":task_id,"childThreadId":child_id,"childRunId":original.map(|run| &run.id),"childNodeId":task_id,
            "status":status,"workState":state,
            "hasPendingChildRuns":child.runs.iter().any(|run| !run_terminal(&run.status) && original.is_none_or(|original| run.ordinal > original.ordinal)),
            "providerInstanceId":task["providerInstanceId"],"model":task["model"],"summary":summary,
            "resultContextTransferId":transfers.first().map(|transfer| &transfer["id"]),
            "latestTerminalRunId":latest.map(|run| &run.id),"latestTerminalStatus":latest.map(|run| status_for_terminal(&run.status)),
            "latestTerminalSummary":latest.map(|run| if original.is_some_and(|original| original.id == run.id) {summary.clone()} else {json!(result_text(&child,run))}),
            "latestTerminalResultContextTransferId":latest.and_then(|run| transfers.iter().find(|transfer| transfer["sourcePoint"]["runId"] == run.id.0)
                .or_else(|| original.filter(|original| original.id == run.id).and_then(|_| transfers.iter().find(|transfer| transfer["sourcePoint"].get("runId").is_none())))).map(|transfer| &transfer["id"]),
            "waitTimedOut":timeout
        })).map_err(tool_error)?;
        if ack
            && terminal(&status)
            && !matches!(
                task["completionDelivery"]["state"].as_str(),
                Some("acknowledged" | "disposed")
            )
        {
            self.kernel
                .task_command(
                    &caller.thread_id,
                    CommandId(format!("command:task-status-ack:{}", uuid::Uuid::new_v4())),
                    TaskOperation::Observe {
                        task_id: task_id.clone(),
                        observed_by: active_run(&parent)
                            .filter(|run| run.provider_instance_id == caller.provider_instance_id)
                            .map(|run| run.id.clone()),
                        dispose: false,
                    },
                )
                .await
                .map_err(tool_error)?;
            self.kernel
                .reconcile_ancestors(&caller.thread_id)
                .await
                .map_err(tool_error)?;
        }
        Ok(response)
    }
}

pub(crate) fn status(status: &OrchestrationV2RunStatus) -> &'static str {
    match status {
        OrchestrationV2RunStatus::Queued => "queued",
        OrchestrationV2RunStatus::Waiting => "waiting",
        OrchestrationV2RunStatus::Completed => "completed",
        OrchestrationV2RunStatus::Failed => "failed",
        OrchestrationV2RunStatus::Cancelled | OrchestrationV2RunStatus::RolledBack => "cancelled",
        OrchestrationV2RunStatus::Interrupted => "interrupted",
        _ => "running",
    }
}
fn status_for_terminal(value: &OrchestrationV2RunStatus) -> &'static str {
    status(value)
}

#[async_trait]
impl OrchestratorService for DelegationService {
    async fn delegate_task(
        &self,
        caller: CallerScope,
        input: DelegateTaskInput,
    ) -> std::result::Result<DelegateTaskResult, ToolError> {
        let parent = self.parent(&caller, true)?;
        if input.task.trim().is_empty() {
            return Err(ToolError::new(
                OrchestratorMcpFailureCode::InvalidRequest,
                "Task must not be empty.",
            ));
        }
        let target = self
            .targets
            .resolve(&parent.thread, input.target.as_ref())
            .await?;
        let runtime = match input.runtime_mode.as_ref() {
            None | Some(OrchestratorMcpRuntimeMode::Inherit) => parent.thread.runtime_mode,
            Some(mode) => serde_json::from_value(serde_json::to_value(mode).map_err(tool_error)?)
                .map_err(tool_error)?,
        };
        let interaction = match input.interaction_mode.as_ref() {
            None | Some(OrchestratorMcpInteractionMode::Inherit) => parent.thread.interaction_mode,
            Some(mode) => serde_json::from_value(serde_json::to_value(mode).map_err(tool_error)?)
                .map_err(tool_error)?,
        };
        if !caller.runtime_mode.permits(runtime) || !parent.thread.runtime_mode.permits(runtime) {
            return Err(ToolError::new(
                OrchestratorMcpFailureCode::RuntimeModeEscalationDenied,
                format!(
                    "Child runtime mode {} is broader than parent mode {}.",
                    serde_json::to_value(runtime).unwrap().as_str().unwrap(),
                    serde_json::to_value(parent.thread.runtime_mode)
                        .unwrap()
                        .as_str()
                        .unwrap()
                ),
            ));
        }
        if !caller.interaction_mode.permits(interaction)
            || !parent.thread.interaction_mode.permits(interaction)
        {
            return Err(ToolError::new(
                OrchestratorMcpFailureCode::InteractionModeEscalationDenied,
                format!(
                    "Child interaction mode {} is broader than parent mode {}.",
                    serde_json::to_value(interaction).unwrap().as_str().unwrap(),
                    serde_json::to_value(parent.thread.interaction_mode)
                        .unwrap()
                        .as_str()
                        .unwrap()
                ),
            ));
        }
        let request_key = key(&input.client_request_id);
        let id = mcp_command_id(&caller.session_id, "delegate-task", &request_key);
        let task_id = delegated_task_id(&id);
        let role = input
            .role
            .as_ref()
            .map(|role| serde_json::to_value(role).unwrap());
        let prompt = if let Some(role) = role
            .as_ref()
            .and_then(Value::as_str)
            .filter(|role| *role != "general")
        {
            format!(
                "Act as the {role} sub-agent for this task.\n\n{}",
                input.task
            )
        } else {
            input.task
        };
        let wait = input.mode.as_ref() == Some(&DelegateTaskInputMode::Wait);
        self.kernel
            .task_command(
                &caller.thread_id,
                id,
                TaskOperation::Delegate(DelegateRequest {
                    parent_run_id: caller.run_id.clone(),
                    parent_node_id: active_run(&parent).unwrap().root_node_id.clone().unwrap(),
                    target,
                    prompt,
                    title: input.title.as_ref().cloned(),
                    runtime_mode: runtime,
                    interaction_mode: interaction,
                    always_wake: !wait,
                    cwd: caller.workspace_root.to_string_lossy().into_owned(),
                }),
            )
            .await
            .map_err(tool_error)?;
        if wait {
            let timeout = input
                .timeout_ms
                .as_ref()
                .and_then(|value| serde_json::to_value(value).ok())
                .and_then(|value| value.as_f64())
                .unwrap_or(600_000.0)
                .clamp(1.0, 3_600_000.0) as u64;
            let future = async {
                loop {
                    let response = self.read_task(&caller, &task_id, false, true).await?;
                    if matches!(
                        response.status,
                        OrchestratorMcpDelegatedTaskStatus::Completed
                            | OrchestratorMcpDelegatedTaskStatus::Failed
                            | OrchestratorMcpDelegatedTaskStatus::Cancelled
                            | OrchestratorMcpDelegatedTaskStatus::Interrupted
                    ) {
                        return Ok(response);
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            };
            if let Ok(response) = tokio::time::timeout(Duration::from_millis(timeout), future).await
            {
                return response;
            }
            if let Err(error) = self
                .kernel
                .task_command(
                    &caller.thread_id,
                    mcp_command_id(
                        &caller.session_id,
                        "delegate-task-wake-policy",
                        &request_key,
                    ),
                    TaskOperation::WakePolicy {
                        task_id: task_id.clone(),
                    },
                )
                .await
            {
                tracing::warn!(%error,"delegation wait timeout wake upgrade failed");
            }
            return self.read_task(&caller, &task_id, true, true).await;
        }
        self.read_task(&caller, &task_id, false, true).await
    }

    async fn task_status(
        &self,
        caller: CallerScope,
        input: TaskStatusInput,
    ) -> std::result::Result<TaskStatusResult, ToolError> {
        self.read_task(&caller, &NodeId(input.task_id), false, true)
            .await
    }

    async fn task_cancel(
        &self,
        caller: CallerScope,
        input: TaskCancelInput,
    ) -> std::result::Result<TaskCancelResult, ToolError> {
        let task_id = NodeId(input.task_id);
        let current = self.read_task(&caller, &task_id, false, false).await?;
        let request_key = key(&input.client_request_id);
        let terminal = !matches!(
            current.status,
            OrchestratorMcpDelegatedTaskStatus::Queued
                | OrchestratorMcpDelegatedTaskStatus::Running
                | OrchestratorMcpDelegatedTaskStatus::Waiting
        );
        if terminal {
            self.kernel
                .task_command(
                    &caller.thread_id,
                    mcp_command_id(
                        &caller.session_id,
                        "cancel-task-completion-delivery",
                        &request_key,
                    ),
                    TaskOperation::Observe {
                        task_id: task_id.clone(),
                        observed_by: None,
                        dispose: true,
                    },
                )
                .await
                .map_err(tool_error)?;
        } else {
            let child = self
                .kernel
                .store
                .thread(&current.child_thread_id)
                .map_err(tool_error)?
                .unwrap();
            let run = active_run(&child).ok_or_else(|| {
                ToolError::new(
                    OrchestratorMcpFailureCode::TaskNotCancellable,
                    format!(
                        "Delegated task {} has no interruptible child run.",
                        task_id.0
                    ),
                )
            })?;
            self.kernel
                .task_command(
                    &caller.thread_id,
                    mcp_command_id(&caller.session_id, "cancel-task", &request_key),
                    TaskOperation::Cancel {
                        task_id: task_id.clone(),
                        child_thread_id: current.child_thread_id,
                        run_id: run.id.clone(),
                    },
                )
                .await
                .map_err(|error| {
                    ToolError::new(
                        OrchestratorMcpFailureCode::TaskNotCancellable,
                        error.to_string(),
                    )
                })?;
        }
        serde_json::from_value(json!({"taskId":task_id,"status":if terminal {serde_json::to_value(current.status).map_err(tool_error)?} else {json!("cancel_requested")}})).map_err(tool_error)
    }
}
