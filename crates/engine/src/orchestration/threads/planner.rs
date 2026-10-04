//! Thread-local commands plan against the locked, current SQL projection.
use rusqlite::Connection;
use serde_json::{Value, json};
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::T3ThreadSendInputMode;

use crate::orchestration::command::{Command, Plan, run_terminal};
use crate::orchestration::effects::EffectRequest;
use crate::orchestration::event::{encode_component, iso};
use crate::orchestration::projection::{self, ThreadProjection};
use crate::orchestration::task::{active_run, emit_execution, execution_seed, message, records};
use crate::orchestration::{Error, Result};

#[derive(Debug, Clone)]
pub struct Send {
    pub message_id: MessageId,
    pub text: String,
    pub mode: T3ThreadSendInputMode,
    pub driver: String,
    pub sender: ThreadId,
    pub target_run: Option<RunId>,
}

#[derive(Debug, Clone)]
pub enum ThreadOperation {
    Send(Send),
    Interrupt {
        run_id: RunId,
        reason: Option<String>,
    },
}

impl ThreadOperation {
    pub fn command_type(&self) -> &'static str {
        match self {
            Self::Send(_) => "message.dispatch",
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
    Ok(
        json!({"id":format!("turn-item:user:{}",encode_component(message["id"].as_str().unwrap())),
        "threadId":projection.thread.id,"runId":run.id,"nodeId":run.root_node_id,
        "providerThreadId":run.provider_thread_id,"providerTurnId":projection.attempts.iter().find(|a| Some(&a.id) == run.active_attempt_id.as_ref()).and_then(|a| a.provider_turn_id.as_ref()),
        "nativeItemRef":null,"parentItemId":null,
        "ordinal":records(projection,"turn-item").iter().filter_map(|i| i["ordinal"].as_i64()).max().unwrap_or(0)+1,
        "status":"completed","title":null,"startedAt":iso(now)?,"completedAt":iso(now)?,"updatedAt":iso(now)?,
        "type":"user_message","messageId":message["id"],"inputIntent":intent,"text":message["text"],
        "attachments":message["attachments"],"createdBy":message["createdBy"],"creationSource":message["creationSource"]}),
    )
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
    let item = json!({"id":id,"type":"assistant_message","threadId":projection.thread.id,
        "runId":message["runId"],"nodeId":message["nodeId"],"providerThreadId":attempt["providerThreadId"],
        "providerTurnId":attempt["providerTurnId"],"nativeItemRef":null,"parentItemId":null,
        "ordinal":previous.and_then(|i| i["ordinal"].as_i64()).unwrap_or_else(|| records(projection,"turn-item").iter().filter_map(|i| i["ordinal"].as_i64()).max().unwrap_or(0)+1),
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

fn native_child(conn: &Connection, projection: &ThreadProjection) -> Result<bool> {
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
                    && !(matches!(r.status,OrchestrationV2RunStatus::Running | OrchestrationV2RunStatus::Waiting)
                        && records(&projection,"provider-turn").iter().any(|t| r.active_attempt_id.as_ref().is_some_and(|a| t["runAttemptId"] == a.0)
                            && r.root_node_id.as_ref().is_some_and(|n| t["nodeId"] == n.0) && t["status"] == "completed"))
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
                message["senderThreadId"] = json!(input.sender);
                let item = user_item(&projection, &run, &message, "steer", now)?;
                plan.emit(command, "message.updated", &message, now)?;
                plan.emit(command, "turn-item.updated", &item, now)?;
            } else {
                let queued = active_run(&projection).is_some();
                if queued {
                    let provider = active_run(&projection).and_then(|r| r.provider_thread_id.as_ref())
                        .and_then(|id| records(&projection,"provider-thread").iter().find(|p| p["id"] == id.0))
                        .ok_or_else(|| unsupported(command,"Active run has no provider thread for queued dispatch"))?;
                    if let Some(session) = records(&projection,"provider-session").iter().find(|s| s["id"] == provider["providerSessionId"])
                        && session["capabilities"]["turns"]["supportsQueuedMessages"] == false
                    {return Err(unsupported(command,"Provider does not support app-owned queued turns"));}
                }
                if queued
                    && records(&projection, "context-transfer")
                        .iter()
                        .any(|t| t["type"] == "merge_back" && t["status"] == "pending")
                {
                    return Err(unsupported(command, "Pending merge back"));
                }
                let ordinal = projection.runs.iter().map(|r| r.ordinal).max().unwrap_or(0) + 1;
                let mut seed = execution_seed(
                    &projection.thread,
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
                message["senderThreadId"] = json!(input.sender);
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
            let run = projection
                .runs
                .iter()
                .find(|r| &r.id == run_id && !run_terminal(&r.status))
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
            let item = json!({"id":format!("turn-item:{}:interrupt-request",encode_component(&run.id.0)),
                "type":"run_interrupt_request","threadId":projection.thread.id,"runId":run.id,"nodeId":run.root_node_id,
                "providerThreadId":run.provider_thread_id,"providerTurnId":null,"nativeItemRef":null,"parentItemId":null,
                "ordinal":records(&projection,"turn-item").iter().filter_map(|i| i["ordinal"].as_i64()).max().unwrap_or(0)+1,
                "status":"completed","title":"Interrupt requested","startedAt":iso(now)?,"completedAt":iso(now)?,"updatedAt":iso(now)?,
                "message":reason.as_deref().unwrap_or("Interrupt requested")});
            plan.emit(command, "turn-item.updated", &item, now)?;
            plan.cancel_process_effects = true;
            let has_turn = records(&projection, "provider-turn").iter().any(|t| {
                run.active_attempt_id
                    .as_ref()
                    .is_some_and(|id| t["runAttemptId"] == id.0)
                    && t["status"] == "running"
            });
            if !has_turn
                && matches!(
                    run.status,
                    OrchestrationV2RunStatus::Preparing
                        | OrchestrationV2RunStatus::Starting
                        | OrchestrationV2RunStatus::Running
                )
            {
                // T3 records this terminality synchronously; no provider exists
                // to interrupt, and cancelled starts may never be replayed.
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
            } else {
                plan.effects.push(EffectRequest::ManagedRunInterrupt {
                    run_id: run.id.clone(),
                });
            }
        }
    }
    Ok(plan)
}
