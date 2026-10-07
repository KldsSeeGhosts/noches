//! Thread-local commands plan against the locked, current SQL projection.
use rusqlite::Connection;
use serde_json::{Value, json};
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::T3ThreadSendInputMode;

use crate::orchestration::command::{Command, Plan, run_terminal};
use crate::orchestration::effects::EffectRequest;
use crate::orchestration::event::{encode_component, iso};
use crate::orchestration::projection::{self, ThreadProjection};
use crate::orchestration::task::{active_run, emit_execution, message, records};
use crate::orchestration::{Error, Result};

#[derive(Debug, Clone)]
pub struct Send {
    pub message_id: MessageId,
    pub text: String,
    pub mode: T3ThreadSendInputMode,
    pub driver: String,
    pub sender: ThreadId,
    pub target_run: Option<RunId>,
    pub metadata: Option<SendMetadata>,
}

#[derive(Debug, Clone)]
pub struct SendMetadata {
    pub scheduled_task_id: Option<ScheduledTaskId>,
    pub sender_thread_id: Option<ThreadId>,
    pub attachments: Vec<Value>,
    pub model_selection: Option<zeron_proto::provider_instance::ModelSelection>,
    pub created_by: OrchestrationV2Actor,
    pub creation_source: OrchestrationV2CreationSource,
}

fn message_metadata(message: &mut Value, input: &Send) {
    if let Some(meta) = &input.metadata {
        if let Some(sender) = &meta.sender_thread_id {
            message["senderThreadId"] = json!(sender);
        }
        if let Some(task) = &meta.scheduled_task_id {
            message["scheduledTaskId"] = json!(task);
        }
        message["attachments"] = json!(meta.attachments);
        message["createdBy"] = json!(meta.created_by);
        message["creationSource"] = json!(meta.creation_source);
    } else {
        message["senderThreadId"] = json!(input.sender);
    }
}

#[derive(Debug, Clone)]
pub enum ThreadOperation {
    Send(Send),
    /// Host-only late-steer recovery. Validation and dispatch planning share
    /// one SQL snapshot; this cannot silently retarget an ordinary Send.
    SteerFollowUp {
        effect: Box<crate::orchestration::effects::Effect>,
        message_id: MessageId,
    },
    Interrupt {
        run_id: RunId,
        reason: Option<String>,
    },
}

impl ThreadOperation {
    pub fn command_type(&self) -> &'static str {
        match self {
            Self::Send(_) => "message.dispatch",
            Self::SteerFollowUp { .. } => "message.steer-follow-up",
            Self::Interrupt { .. } => "run.interrupt",
        }
    }
}

pub(crate) fn assistant_message_id(run: &RunId, attempt: &RunAttemptId) -> String {
    let first = format!("run-attempt:{}:1", encode_component(&run.0));
    if attempt.0 == first {
        format!("message:assistant:{}", encode_component(&run.0))
    } else {
        format!("message:assistant:{}", encode_component(&attempt.0))
    }
}

pub(crate) fn steerable(projection: &ThreadProjection) -> Option<&OrchestrationV2Run> {
    projection
        .runs
        .iter()
        .filter(|r| {
            r.status == OrchestrationV2RunStatus::Running
                && r.active_attempt_id.as_ref().is_some_and(|id| {
                    records(projection, "provider-turn")
                        .iter()
                        .any(|t| t["runAttemptId"] == id.0 && t["status"] == "running")
                })
        })
        .max_by_key(|r| r.ordinal)
}

pub(crate) fn user_item(
    projection: &ThreadProjection,
    run: &OrchestrationV2Run,
    message: &Value,
    intent: &str,
    now: i64,
) -> Result<Value> {
    let mut item = json!({"id":format!("turn-item:user:{}",encode_component(message["id"].as_str().unwrap())),
        "threadId":projection.thread.id,"runId":run.id,"nodeId":run.root_node_id,
        "providerThreadId":run.provider_thread_id,"providerTurnId":projection.attempts.iter().find(|a| Some(&a.id) == run.active_attempt_id.as_ref()).and_then(|a| a.provider_turn_id.as_ref()),
        "nativeItemRef":null,"parentItemId":null,
        "ordinal":records(projection,"turn-item").iter().filter_map(|i| i["ordinal"].as_i64()).max().unwrap_or(0)+1,
        "status":"completed","title":null,"startedAt":iso(now)?,"completedAt":iso(now)?,"updatedAt":iso(now)?,
        "type":"user_message","messageId":message["id"],"inputIntent":intent,"text":message["text"],
        "attachments":message["attachments"],"createdBy":message["createdBy"],"creationSource":message["creationSource"]});
    for key in ["scheduledTaskId", "senderThreadId"] {
        if let Some(value) = message.get(key) {
            item[key] = value.clone();
        }
    }
    Ok(item)
}

pub(crate) fn next_item_ordinal(projection: &ThreadProjection, plan: &Plan) -> Result<i64> {
    let mut last = records(projection, "turn-item")
        .iter()
        .filter_map(|item| item["ordinal"].as_i64())
        .max()
        .unwrap_or(0);
    for event in &plan.events {
        if let OrchestrationV2DomainEvent::TurnItemUpdated(event) = event
            && event.thread_id == projection.thread.id
        {
            let item = serde_json::to_value(&event.payload)?;
            last = last.max(item["ordinal"].as_i64().unwrap_or(0));
        }
    }
    Ok(last + 1)
}

pub(crate) fn assistant_item(
    projection: &ThreadProjection,
    command: &Command,
    plan: &mut Plan,
    message: &Value,
    attempt: &Value,
    now: i64,
) -> Result<()> {
    let id = format!(
        "turn-item:assistant:{}",
        encode_component(message["id"].as_str().unwrap())
    );
    let previous = records(projection, "turn-item")
        .iter()
        .find(|i| i["id"] == id);
    let time = iso(now)?;
    let ordinal = match previous.and_then(|i| i["ordinal"].as_i64()) {
        Some(ordinal) => ordinal,
        None => next_item_ordinal(projection, plan)?,
    };
    let item = json!({"id":id,"type":"assistant_message","threadId":projection.thread.id,
        "runId":message["runId"],"nodeId":message["nodeId"],"providerThreadId":attempt["providerThreadId"],
        "providerTurnId":attempt["providerTurnId"],"nativeItemRef":null,"parentItemId":null,
        "ordinal":ordinal,
        "status":if message["streaming"] == true {"running"} else {"completed"},"title":null,
        "startedAt":previous.map(|i| &i["startedAt"]).cloned().unwrap_or(json!(time)),
        "completedAt":if message["streaming"] == true {Value::Null} else {json!(time)},"updatedAt":time,
        "messageId":message["id"],"text":message["text"],"attachments":message["attachments"],"streaming":message["streaming"]});
    plan.emit(command, "turn-item.updated", &item, now)
}

fn unsupported(command: &Command, detail: impl Into<String>) -> Error {
    // OrchestratorDispatchError intentionally exposes no internal Defect.
    let _ = detail.into();
    Error::Invariant(format!(
        "Failed to dispatch orchestration command {} ({}).",
        command.command_type().unwrap(),
        command.id
    ))
}

pub(crate) fn native_child(conn: &Connection, projection: &ThreadProjection) -> Result<bool> {
    if projection.thread.lineage.relationship_to_parent
        != Some(OrchestrationV2AppThreadLineageRelationshipToParent::Subagent)
    {
        return Ok(false);
    }
    let Some(parent) = &projection.thread.lineage.parent_thread_id else {
        return Ok(false);
    };
    Ok(projection::read_thread(conn, parent)?.is_none_or(|p| {
        !records(&p, "subagent")
            .iter()
            .any(|s| s["origin"] == "app_owned" && s["childThreadId"] == projection.thread.id.0)
    }))
}

fn clear_parking(
    projection: &ThreadProjection,
    command: &Command,
    plan: &mut Plan,
    now: i64,
) -> Result<()> {
    let mut thread = serde_json::to_value(&projection.thread)?;
    if !thread["settledOverride"].is_null() {
        if thread["settledOverride"] != "active" {
            thread["unsettledAt"] = json!(iso(now)?);
        }
        thread["settledOverride"] = Value::Null;
        thread["settledAt"] = Value::Null;
        thread["updatedAt"] = json!(iso(now)?);
        plan.emit(command, "thread.unsettled", &thread, now)?;
    }
    if !thread["snoozedUntil"].is_null() {
        thread["snoozedUntil"] = Value::Null;
        thread["snoozedAt"] = Value::Null;
        thread["updatedAt"] = json!(iso(now)?);
        plan.emit(command, "thread.unsnoozed", &thread, now)?;
    }
    Ok(())
}

pub(crate) fn plan(
    conn: &Connection,
    command: &Command,
    op: &ThreadOperation,
    now: i64,
) -> Result<Plan> {
    let projection = projection::read_thread(conn, &command.thread_id)?
        .ok_or_else(|| unsupported(command, "Missing thread"))?;
    let mut plan = Plan::default();
    match op {
        ThreadOperation::SteerFollowUp { effect, message_id } => {
            if effect.thread_id != command.thread_id {
                return Err(unsupported(command, "Foreign steering effect"));
            }
            let saved = crate::orchestration::effects::get(conn, &effect.id)?
                .filter(|saved| {
                    saved.request == effect.request && saved.command_id == effect.command_id
                })
                .ok_or_else(|| unsupported(command, "Steering effect changed"))?;
            if crate::orchestration::steering::confirmed(conn, &effect.id)? {
                // The kernel requires a receipted event even for a no-op.
                plan.emit(command, "thread.metadata-updated", &projection.thread, now)?;
                return Ok(plan);
            }
            let EffectRequest::ProviderTurnSteer {
                message_id: expected,
                ..
            } = &saved.request
            else {
                return Err(unsupported(command, "Not a steering effect"));
            };
            if expected != message_id {
                return Err(unsupported(command, "Steering message changed"));
            }
            if !matches!(
                saved.status,
                crate::orchestration::effects::EffectStatus::Pending
                    | crate::orchestration::effects::EffectStatus::Running
            ) {
                return Err(unsupported(
                    command,
                    "Steering effect is no longer dispatchable",
                ));
            }
            let (run, turn) =
                crate::orchestration::steering::target(&projection, &saved.request)
                    .ok_or_else(|| unsupported(command, "Recorded steering target changed"))?;
            if let Some(expected) =
                crate::orchestration::steering::admitted_runtime(conn, &effect.id)?
                && crate::orchestration::steering::runtime_id(
                    conn,
                    &command.thread_id,
                    &crate::orchestration::steering::RuntimeTarget::for_run(run).unwrap(),
                )?
                .as_ref()
                    != Some(&expected)
            {
                return Err(unsupported(command, "Steering runtime was replaced"));
            }
            // Only normal completion authorizes T3's late follow-up. Stop,
            // cancellation, supersession, failure and missing turns do not.
            if turn["status"] != "completed"
                || !matches!(
                    run.status,
                    OrchestrationV2RunStatus::Running
                        | OrchestrationV2RunStatus::Waiting
                        | OrchestrationV2RunStatus::Completed
                )
            {
                return Err(unsupported(command, "Steering target did not complete"));
            }
            let message = records(&projection, "message")
                .iter()
                .find(|m| m["id"] == message_id.0)
                .unwrap();
            let delegated = message["delegatedCompletion"].is_object();
            let provider_instance = if delegated {
                &run.provider_instance_id
            } else {
                &projection.thread.provider_instance_id
            };
            let driver = records(&projection, "provider-thread")
                .iter()
                .filter(|p| p["providerInstanceId"] == provider_instance.0)
                .max_by_key(|p| p["generation"].as_i64().unwrap_or(0))
                .and_then(|p| p["driver"].as_str())
                .ok_or_else(|| unsupported(command, "Follow-up provider missing"))?;
            let mut follow_up = self::plan(
                conn,
                command,
                &ThreadOperation::Send(Send {
                    message_id: message_id.clone(),
                    text: message["text"].as_str().unwrap_or_default().into(),
                    mode: T3ThreadSendInputMode::Auto,
                    driver: driver.into(),
                    sender: message["senderThreadId"]
                        .as_str()
                        .unwrap_or(&command.thread_id.0)
                        .into(),
                    target_run: None,
                    metadata: Some(SendMetadata {
                        scheduled_task_id: serde_json::from_value(
                            message["scheduledTaskId"].clone(),
                        )
                        .ok()
                        .flatten(),
                        sender_thread_id: serde_json::from_value(message["senderThreadId"].clone())
                            .ok()
                            .flatten(),
                        attachments: serde_json::from_value(message["attachments"].clone())
                            .unwrap_or_default(),
                        model_selection: delegated.then(|| run.model_selection.clone()),
                        created_by: serde_json::from_value(message["createdBy"].clone())
                            .unwrap_or(OrchestrationV2Actor::Agent),
                        creation_source: serde_json::from_value(message["creationSource"].clone())
                            .unwrap_or(OrchestrationV2CreationSource::Mcp),
                    }),
                }),
                now,
            )?;
            // Keep composer references and automatic-mail ownership, not just
            // the text/attachments. The accepted message identity is unchanged.
            for event in &mut follow_up.events {
                if let OrchestrationV2DomainEvent::MessageUpdated(updated) = event {
                    let mut payload = serde_json::to_value(&updated.payload)?;
                    if payload["id"] == message_id.0 {
                        for key in ["context", "delegatedCompletion", "notification"] {
                            if let Some(value) = message.get(key) {
                                payload[key] = value.clone();
                            }
                        }
                        updated.payload = serde_json::from_value(payload)?;
                    }
                }
            }
            return Ok(follow_up);
        }
        ThreadOperation::Send(input) => {
            // Refuse native children BEFORE unparking or writing any message.
            if native_child(conn, &projection)? {
                return Err(Error::Invariant("This subagent is run by its provider and cannot take messages. Message the parent thread instead.".into()));
            }
            if projection.thread.archived_at.is_some() || projection.thread.deleted_at.is_some() {
                return Err(unsupported(command, "Unavailable thread"));
            }
            clear_parking(&projection, command, &mut plan, now)?;
            let steer = input
                .target_run
                .as_ref()
                .and_then(|id| projection.runs.iter().find(|r| r.id == *id));
            let steer = steer.filter(|r| {
                r.status != OrchestrationV2RunStatus::Completed
                    && !(matches!(
                        r.status,
                        OrchestrationV2RunStatus::Running | OrchestrationV2RunStatus::Waiting
                    ) && records(&projection, "provider-turn").iter().any(|t| {
                        r.active_attempt_id
                            .as_ref()
                            .is_some_and(|a| t["runAttemptId"] == a.0)
                            && r.root_node_id.as_ref().is_some_and(|n| t["nodeId"] == n.0)
                            && t["status"] == "completed"
                    }))
            });
            if let Some(run) = steer {
                if run.status != OrchestrationV2RunStatus::Running {
                    return Err(unsupported(command, "Target is not running"));
                }
                let maintenance = |text: &str| {
                    matches!(text.trim().to_lowercase().as_str(), "/compact" | "/logout")
                };
                if maintenance(&input.text)
                    || records(&projection, "message")
                        .iter()
                        .find(|m| m["id"] == run.user_message_id.0)
                        .and_then(|m| m["text"].as_str())
                        .is_some_and(maintenance)
                {
                    return Err(unsupported(command, "Maintenance cannot steer"));
                }
                let provider = records(&projection, "provider-thread")
                    .iter()
                    .find(|p| {
                        Some(p["id"].as_str().unwrap_or_default())
                            == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
                    })
                    .ok_or_else(|| unsupported(command, "Missing provider thread"))?;
                let session_id = provider["providerSessionId"]
                    .as_str()
                    .ok_or_else(|| unsupported(command, "Missing session"))?;
                let session = records(&projection, "provider-session")
                    .iter()
                    .find(|s| s["id"] == session_id)
                    .ok_or_else(|| unsupported(command, "Inactive session"))?;
                let turn = records(&projection, "provider-turn")
                    .iter()
                    .find(|t| {
                        run.active_attempt_id
                            .as_ref()
                            .is_some_and(|id| t["runAttemptId"] == id.0)
                            && t["status"] == "running"
                    })
                    .ok_or_else(|| unsupported(command, "Missing running turn"))?;
                let caps = &session["capabilities"]["turns"];
                let direct = input.mode != T3ThreadSendInputMode::Restart
                    && caps["supportsActiveSteering"] == true;
                if !direct
                    && !(caps["supportsInterrupt"] == true
                        && caps["supportsSteeringByInterruptRestart"] == true)
                {
                    return Err(unsupported(command, "Unsupported steering policy"));
                }
                let mut run = run.clone();
                if !direct {
                    let old = projection
                        .attempts
                        .iter()
                        .find(|a| Some(&a.id) == run.active_attempt_id.as_ref())
                        .ok_or_else(|| unsupported(command, "Missing attempt"))?;
                    let ordinal = projection
                        .attempts
                        .iter()
                        .filter(|a| a.run_id == run.id)
                        .map(|a| a.attempt_ordinal)
                        .max()
                        .unwrap_or(0)
                        + 1;
                    let attempt_id = RunAttemptId(format!(
                        "run-attempt:{}:{ordinal}",
                        encode_component(&run.id.0)
                    ));
                    let root_id = NodeId(format!("node:{}:{ordinal}", encode_component(&run.id.0)));
                    let mut previous = old.clone();
                    previous.status = OrchestrationV2RunAttemptStatus::Superseded;
                    previous.completed_at = Some(iso(now)?);
                    plan.emit(command, "run-attempt.updated", &previous, now)?;
                    if let Some(root) = projection
                        .nodes
                        .iter()
                        .find(|n| Some(&n.id) == run.root_node_id.as_ref())
                    {
                        let mut root = root.clone();
                        root.status = OrchestrationV2ExecutionNodeStatus::Interrupted;
                        root.completed_at = Some(iso(now)?);
                        plan.emit(command, "node.updated", &root, now)?;
                    }
                    let mut attempt = old.clone();
                    attempt.id = attempt_id.clone();
                    attempt.attempt_ordinal = ordinal;
                    attempt.root_node_id = root_id.clone();
                    attempt.provider_turn_id = None;
                    attempt.reason = OrchestrationV2RunAttemptReason::SteeringRestart;
                    attempt.status = OrchestrationV2RunAttemptStatus::Pending;
                    attempt.started_at = None;
                    attempt.completed_at = None;
                    let root: OrchestrationV2ExecutionNode = serde_json::from_value(json!({
                        "id":root_id,"threadId":projection.thread.id,"runId":run.id,"rootNodeId":root_id,
                        "parentNodeId":null,"kind":"root_turn","status":"pending","countsForRun":true,
                        "providerThreadId":run.provider_thread_id,"providerTurnId":null,"nativeItemRef":null,
                        "runtimeRequestId":null,"checkpointScopeId":null,"startedAt":null,"completedAt":null}))?;
                    run.root_node_id = Some(root_id);
                    run.active_attempt_id = Some(attempt_id);
                    run.user_message_id = input.message_id.clone();
                    run.status = OrchestrationV2RunStatus::Starting;
                    plan.emit(command, "run.updated", &run, now)?;
                    plan.emit(command, "run-attempt.created", &attempt, now)?;
                    plan.emit(command, "node.updated", &root, now)?;
                    plan.cancel_process_effects = true;
                    plan.effects.push(EffectRequest::ProviderTurnRestart {
                        provider_session_id: ProviderSessionId(session_id.into()),
                        provider_thread_id: run.provider_thread_id.clone().unwrap(),
                        provider_turn_id: ProviderTurnId(turn["id"].as_str().unwrap().into()),
                        interrupted_attempt_id: old.id.clone(),
                        run_id: run.id.clone(),
                    });
                } else {
                    plan.effects.push(EffectRequest::ProviderTurnSteer {
                        provider_session_id: ProviderSessionId(session_id.into()),
                        provider_thread_id: run.provider_thread_id.clone().unwrap(),
                        provider_turn_id: ProviderTurnId(turn["id"].as_str().unwrap().into()),
                        message_id: input.message_id.clone(),
                    });
                }
                let mut message = message(
                    &projection.thread.id,
                    Some(&run.id),
                    run.root_node_id.as_ref(),
                    &input.message_id.0,
                    &input.text,
                    "user",
                    now,
                )?;
                message_metadata(&mut message, input);
                let item = user_item(&projection, &run, &message, "steer", now)?;
                plan.emit(command, "message.updated", &message, now)?;
                plan.emit(command, "turn-item.updated", &item, now)?;
            } else {
                let queued = active_run(&projection).is_some();
                // T3 refuses a pending merge before resolving the queued
                // provider/session. No input or unparking events may publish.
                crate::orchestration::transfer::ensure_start_allowed(
                    &crate::orchestration::transfer::transfers(conn, &command.thread_id)?,
                    &command.thread_id,
                    queued,
                )?;
                if queued {
                    let provider = active_run(&projection)
                        .and_then(|r| r.provider_thread_id.as_ref())
                        .and_then(|id| {
                            records(&projection, "provider-thread")
                                .iter()
                                .find(|p| p["id"] == id.0)
                        })
                        .ok_or_else(|| {
                            unsupported(
                                command,
                                "Active run has no provider thread for queued dispatch",
                            )
                        })?;
                    if let Some(session) = records(&projection, "provider-session")
                        .iter()
                        .find(|s| s["id"] == provider["providerSessionId"])
                        && session["capabilities"]["turns"]["supportsQueuedMessages"] == false
                    {
                        return Err(unsupported(
                            command,
                            "Provider does not support app-owned queued turns",
                        ));
                    }
                }
                let ordinal = projection.runs.iter().map(|r| r.ordinal).max().unwrap_or(0) + 1;
                let mut thread = projection.thread.clone();
                if let Some(selection) = input
                    .metadata
                    .as_ref()
                    .and_then(|m| m.model_selection.as_ref())
                {
                    thread.provider_instance_id = selection.instance_id.clone();
                    thread.model_selection = selection.clone();
                }
                let mut seed = crate::orchestration::task::execution_seed_for(
                    &projection,
                    &thread,
                    ordinal,
                    &input.message_id.0,
                    if queued { "queued" } else { "starting" },
                    &input.driver,
                    now,
                )?;
                if let Some(provider) = records(&projection, "provider-thread")
                    .iter()
                    .find(|p| p["id"] == seed.provider_thread.id.0)
                {
                    seed.provider_thread = serde_json::from_value(provider.clone())?;
                    if !queued {
                        seed.provider_thread.owner_node_id = Some(seed.root.id.clone());
                        seed.provider_thread.last_run_ordinal = Some(ordinal);
                        seed.provider_thread.updated_at = iso(now)?;
                    }
                }
                if queued {
                    seed.run.queue_position = Optional::Present(Some(
                        projection
                            .runs
                            .iter()
                            .filter(|r| r.status == OrchestrationV2RunStatus::Queued)
                            .map(|r| {
                                r.queue_position
                                    .as_ref()
                                    .and_then(|v| *v)
                                    .unwrap_or(r.ordinal)
                            })
                            .max()
                            .unwrap_or(0)
                            + 1,
                    ));
                    if projection.runs.iter().any(|r| {
                        r.status == OrchestrationV2RunStatus::Queued
                            && r.queue_held.as_ref() == Some(&true)
                    }) {
                        seed.run.queue_held = Optional::Present(true);
                    }
                }
                emit_execution(&mut plan, command, &projection.thread.id, &seed, now)?;
                let mut message = message(
                    &projection.thread.id,
                    Some(&seed.run.id),
                    Some(&seed.root.id),
                    &input.message_id.0,
                    &input.text,
                    "user",
                    now,
                )?;
                message_metadata(&mut message, input);
                plan.emit(command, "message.updated", &message, now)?;
                if !queued {
                    plan.emit(
                        command,
                        "turn-item.updated",
                        &user_item(&projection, &seed.run, &message, "turn_start", now)?,
                        now,
                    )?;
                }
            }
        }
        ThreadOperation::Interrupt { run_id, reason } => {
            let run = crate::orchestration::background::interruptible_run(&projection)
                .filter(|r| &r.id == run_id)
                .ok_or_else(|| unsupported(command, "No interruptible run"))?;
            if !projection
                .nodes
                .iter()
                .any(|n| Some(&n.id) == run.root_node_id.as_ref())
                || !records(&projection, "provider-thread").iter().any(|p| {
                    p["id"].as_str() == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
                })
            {
                return Err(unsupported(
                    command,
                    "Run has no interruptible root/provider binding",
                ));
            }
            let cohort = records(&projection, "message")
                .iter()
                .find(|m| m["id"] == run.user_message_id.0)
                .and_then(|m| m["delegatedCompletion"]["parentRunId"].as_str())
                .map(|id| RunId(id.into()))
                .unwrap_or_else(|| run.id.clone());
            crate::orchestration::mailbox::stop(&projection, command, &mut plan, &cohort, now)?;
            let turn = records(&projection, "provider-turn").iter().find(|t| {
                run.active_attempt_id
                    .as_ref()
                    .is_some_and(|id| t["runAttemptId"] == id.0)
                    && (t["status"] == "running" || run_terminal(&run.status))
            });
            let item = json!({"id":format!("turn-item:{}:interrupt-request",encode_component(&run.id.0)),
                "type":"run_interrupt_request","threadId":projection.thread.id,"runId":run.id,"nodeId":run.root_node_id,
                "providerThreadId":run.provider_thread_id,"providerTurnId":turn.map(|t| &t["id"]),"nativeItemRef":null,"parentItemId":null,
                "ordinal":records(&projection,"turn-item").iter().filter_map(|i| i["ordinal"].as_i64()).max().unwrap_or(0)+1,
                "status":"completed","title":"Interrupt requested","startedAt":iso(now)?,"completedAt":iso(now)?,"updatedAt":iso(now)?,
                "message":reason.as_deref().unwrap_or("Interrupt requested")});
            plan.emit(command, "turn-item.updated", &item, now)?;
            plan.cancel_process_effects = true;
            let has_turn = turn.is_some();
            if !has_turn
                && matches!(
                    run.status,
                    OrchestrationV2RunStatus::Preparing
                        | OrchestrationV2RunStatus::Starting
                        | OrchestrationV2RunStatus::Running
                        | OrchestrationV2RunStatus::Waiting
                )
            {
                // No accepted native turn exists, so terminalize synchronously.
                // A process can nevertheless be parked on an early callback;
                // retire that exact admission without inventing a native turn.
                let provider = records(&projection, "provider-thread")
                    .iter()
                    .find(|provider| {
                        Some(provider["id"].as_str().unwrap_or_default())
                            == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
                    })
                    .unwrap();
                crate::orchestration::queue::runtime::observe(
                    &projection,
                    command,
                    &mut plan,
                    &run.id,
                    provider,
                    &zeron_proto::AgentEvent::Done {
                        status: zeron_proto::DoneStatus::Interrupted,
                        result: None,
                        error: None,
                        session_id: None,
                    },
                    now,
                )?;
                plan.effects.push(EffectRequest::ManagedRunInterrupt {
                    run_id: run.id.clone(),
                });
                let mut next = serde_json::to_value(run)?;
                next["status"] = json!("interrupted");
                next["completedAt"] = json!(iso(now)?);
                if let Some(stopped) = plan.events.iter().find_map(|e| {
                    let v = serde_json::to_value(e).ok()?;
                    (v["type"] == "run.updated" && v["payload"]["id"] == run.id.0)
                        .then_some(v["payload"]["delegatedCompletion"].clone())
                }) {
                    next["delegatedCompletion"] = stopped;
                }
                plan.emit(command, "run.updated", &next, now)?;
                if let Some(attempt) = projection
                    .attempts
                    .iter()
                    .find(|a| Some(&a.id) == run.active_attempt_id.as_ref())
                {
                    let mut a = attempt.clone();
                    a.status = OrchestrationV2RunAttemptStatus::Interrupted;
                    a.completed_at = Some(iso(now)?);
                    plan.emit(command, "run-attempt.updated", &a, now)?;
                }
                if let Some(root) = projection
                    .nodes
                    .iter()
                    .find(|n| Some(&n.id) == run.root_node_id.as_ref())
                {
                    let mut root = root.clone();
                    root.status = OrchestrationV2ExecutionNodeStatus::Interrupted;
                    root.completed_at = Some(iso(now)?);
                    plan.emit(command, "node.updated", &root, now)?;
                }
                let mut result = item;
                result["id"] = json!(format!(
                    "turn-item:{}:interrupt-result",
                    encode_component(&run.id.0)
                ));
                result["type"] = json!("run_interrupt_result");
                result["parentItemId"] = json!(format!(
                    "turn-item:{}:interrupt-request",
                    encode_component(&run.id.0)
                ));
                result["ordinal"] = json!(result["ordinal"].as_i64().unwrap() + 1);
                result["status"] = json!("interrupted");
                result["title"] = json!("Interrupted");
                result["message"] = json!("Run interrupted before provider start");
                plan.emit(command, "turn-item.updated", &result, now)?;
            } else if !has_turn && run_terminal(&run.status) {
                // A failed/settled root may have no accepted provider turn.
                // Keep exact attempt/process admission even for local repair.
                plan.effects.push(EffectRequest::ManagedRunInterrupt {
                    run_id: run.id.clone(),
                });
            } else {
                let turn =
                    turn.ok_or_else(|| unsupported(command, "No running turn to interrupt"))?;
                let provider = records(&projection, "provider-thread")
                    .iter()
                    .find(|p| p["id"] == turn["providerThreadId"])
                    .ok_or_else(|| unsupported(command, "Missing provider thread"))?;
                if let Some(session_id) = provider["providerSessionId"].as_str() {
                    let session = records(&projection, "provider-session")
                        .iter()
                        .find(|s| s["id"] == session_id)
                        .ok_or_else(|| unsupported(command, "Inactive provider session"))?;
                    if session["capabilities"]["turns"]["supportsInterrupt"] != true {
                        return Err(unsupported(command, "Provider cannot interrupt"));
                    }
                    plan.effects.push(EffectRequest::ProviderTurnInterrupt {
                        provider_session_id: session_id.into(),
                        provider_thread_id: turn["providerThreadId"].as_str().unwrap().into(),
                        provider_turn_id: turn["id"].as_str().unwrap().into(),
                    });
                } else {
                    // Real output can precede attachment metadata too. The
                    // host's physical runtime fence still permits user Stop.
                    plan.effects.push(EffectRequest::ManagedRunInterrupt {
                        run_id: run.id.clone(),
                    });
                }
            }
        }
    }
    Ok(plan)
}
