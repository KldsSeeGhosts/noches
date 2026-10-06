use rusqlite::Connection;
use serde_json::{Value, json};
use zeron_proto::orchestration::*;

use super::{QueueCommand, access, question, queued};
use crate::orchestration::command::{Command, Plan};
use crate::orchestration::effects::{EffectRequest, TitleKind};
use crate::orchestration::event::iso;
use crate::orchestration::projection::{self, ThreadProjection};
use crate::orchestration::{Error, Result, task};

fn refuse(message: impl Into<String>) -> Error {
    Error::Invariant(message.into())
}
fn records<'a>(p: &'a ThreadProjection, kind: &str) -> &'a [Value] {
    task::records(p, kind)
}
fn automatic(message: &Value) -> bool {
    message.get("delegatedCompletion").is_some() || message.get("notification").is_some()
}

pub(crate) fn plan(
    conn: &Connection,
    command: &Command,
    op: &QueueCommand,
    now: i64,
) -> Result<Plan> {
    let p = if let Some(caller) = &op.caller {
        if op.name == "t3_thread_update" {
            super::metadata_access(conn, caller, &op.input)?
        } else {
            access(conn, caller, &op.input, true)?
        }
    } else {
        projection::read_thread(conn, &command.thread_id)?
            .ok_or_else(|| refuse("Thread not found."))?
    };
    if p.thread.deleted_at.is_some() {
        return Err(refuse("Thread is deleted."));
    }
    let mut plan = Plan::default();
    match op.name.as_str() {
        "host.sync_loro_queue" => sync_loro(conn, &mut plan, command, &p, &op.input, now)?,
        "host.adopt_loro_delivery" => adopt_delivery(&mut plan, command, &p, &op.input, now)?,
        "host.title_generated" => {
            let mut thread = serde_json::to_value(&p.thread)?;
            if thread["titleRegeneration"]["requestId"] != op.input["requestId"] {
                return Err(refuse("Title regeneration was superseded."));
            }
            if let Some(title) = op.input["title"]
                .as_str()
                .filter(|s| !s.trim_matches(super::search::js_whitespace).is_empty())
            {
                thread["title"] = json!(title);
            }
            thread["titleRegeneration"] = Value::Null;
            thread["updatedAt"] = json!(iso(now)?);
            plan.emit(command, "thread.metadata-updated", &thread, now)?;
        }
        "t3_queue_edit" | "t3_queue_cancel" | "t3_queue_reorder" | "t3_queue_promote_to_steer" => {
            queue_mutation(&mut plan, command, &p, op, now)?
        }
        "t3_pending_request_respond" => respond(&mut plan, command, &p, &op.input, false, now)?,
        "t3_thread_update" => metadata(&mut plan, command, &p, op, now)?,
        "t3_thread_organize" | "host.unsnooze_due" | "host.acknowledge_woke" => {
            organize(conn, &mut plan, command, &p, op, now)?
        }
        _ => return Err(refuse("Unsupported queue operation.")),
    }
    Ok(plan)
}

fn cancel(
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    run: &OrchestrationV2Run,
    now: i64,
) -> Result<()> {
    if let Some(message) = records(p, "message")
        .iter()
        .find(|m| m["id"] == run.user_message_id.0)
        && let Some(parent) = message["delegatedCompletion"]["parentRunId"].as_str()
    {
        // The delegation owner disposes the cohort and all its unacknowledged
        // deliveries together rather than just cancelling this run.
        if let Some(parent_run) = p.runs.iter().find(|r| r.id.0 == parent) {
            let mut value = serde_json::to_value(parent_run)?;
            value["delegatedCompletion"]["disposition"] = json!("disposed");
            value["delegatedCompletion"]["delivery"] = Value::Null;
            plan.emit(command, "run.updated", &value, now)?;
        }
        for task in records(p, "subagent").iter().filter(|t| {
            t["runId"] == parent
                && !matches!(
                    t["completionDelivery"]["state"].as_str(),
                    Some("acknowledged" | "disposed")
                )
        }) {
            let mut value = task.clone();
            value["completionDelivery"] = json!({"state":"disposed","observedByRunId":null});
            plan.emit(command, "subagent.updated", &value, now)?;
        }
        for queued in queued(p).into_iter().filter(|r| {
            records(p, "message").iter().any(|m| {
                m["id"] == r.user_message_id.0 && m["delegatedCompletion"]["parentRunId"] == parent
            })
        }) {
            cancel_graph(plan, command, p, queued, now)?;
        }
        return Ok(());
    }
    cancel_graph(plan, command, p, run, now)
}

fn cancel_graph(
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    run: &OrchestrationV2Run,
    now: i64,
) -> Result<()> {
    let mut value = serde_json::to_value(run)?;
    value["status"] = json!("cancelled");
    value["queuePosition"] = Value::Null;
    value["completedAt"] = json!(iso(now)?);
    plan.emit(command, "run.updated", &value, now)?;
    if let Some(id) = &run.active_attempt_id
        && let Some(attempt) = p.attempts.iter().find(|a| &a.id == id)
        && run.root_node_id.is_some()
    {
        let mut value = serde_json::to_value(attempt)?;
        value["status"] = json!("cancelled");
        value["completedAt"] = json!(iso(now)?);
        plan.emit(command, "run-attempt.updated", &value, now)?;
    }
    if let Some(id) = &run.root_node_id
        && let Some(node) = p.nodes.iter().find(|n| &n.id == id)
    {
        let mut value = serde_json::to_value(node)?;
        value["status"] = json!("cancelled");
        value["completedAt"] = json!(iso(now)?);
        plan.emit(command, "node.updated", &value, now)?;
    }
    Ok(())
}

fn queue_mutation(
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    op: &QueueCommand,
    now: i64,
) -> Result<()> {
    let id = op.input["queuedRunId"].as_str().unwrap_or("");
    let run = queued(p)
        .into_iter()
        .find(|r| r.id.0 == id)
        .ok_or_else(|| refuse(format!("Run {id} is not queued.")))?;
    let message = records(p, "message")
        .iter()
        .find(|m| m["id"] == run.user_message_id.0);
    match op.name.as_str() {
        "t3_queue_cancel" => {
            cancel(plan, command, p, run, now)?;
            plan.queue_patch = Some(json!({"action":"cancel","messageId":run.user_message_id}));
            Ok(())
        }
        "t3_queue_edit" => {
            let text = op.input["text"].as_str().unwrap_or("");
            if text.trim_matches(super::search::js_whitespace).is_empty() {
                return Err(refuse(format!(
                    "Queued run {id} cannot be edited to an empty message."
                )));
            }
            let mut message = message
                .ok_or_else(|| refuse("Queued run has no user message."))?
                .clone();
            if automatic(&message) {
                return Err(refuse("Automatic completion deliveries cannot be edited."));
            }
            message["text"] = json!(text);
            message["updatedAt"] = json!(iso(now)?);
            plan.emit(command, "message.updated", &message, now)?;
            plan.queue_patch =
                Some(json!({"action":"edit","messageId":run.user_message_id,"text":text}));
            if let Some(item) = records(p, "turn-item")
                .iter()
                .find(|i| i["type"] == "user_message" && i["messageId"] == run.user_message_id.0)
            {
                let mut item = item.clone();
                item["text"] = json!(text);
                item["updatedAt"] = json!(iso(now)?);
                plan.emit(command, "turn-item.updated", &item, now)?;
            }
            Ok(())
        }
        "t3_queue_reorder" => {
            if message.is_some_and(automatic) {
                return Err(refuse(
                    "Automatic completion deliveries cannot be reordered.",
                ));
            }
            let runs = queued(p);
            let is_auto = |r: &&OrchestrationV2Run| {
                records(p, "message").iter().any(|m| {
                    m["id"] == r.user_message_id.0 && m.get("delegatedCompletion").is_some()
                })
            };
            let mut automatic_runs: Vec<_> = runs.iter().copied().filter(is_auto).collect();
            let mut ordinary: Vec<_> = runs
                .iter()
                .copied()
                .filter(|r| !is_auto(r) && r.id.0 != id)
                .collect();
            let before = op.input["beforeRunId"].as_str();
            if before.is_some_and(|id| automatic_runs.iter().any(|r| r.id.0 == id)) {
                return Err(refuse(
                    "Queued messages cannot be reordered ahead of automatic completion delivery.",
                ));
            }
            let index = match before {
                None => ordinary.len(),
                Some(id) => ordinary
                    .iter()
                    .position(|r| r.id.0 == id)
                    .ok_or_else(|| refuse(format!("Queue target {id} is not queued.")))?,
            };
            ordinary.insert(index, run);
            automatic_runs.extend(ordinary);
            plan.queue_patch = Some(json!({"action":"reorder","messageId":run.user_message_id,
                "messageIds":automatic_runs.iter().map(|r|r.user_message_id.0.clone()).collect::<Vec<_>>()}));
            for (index, run) in automatic_runs.into_iter().enumerate() {
                let position = index as i64 + 1;
                if run.queue_position.as_ref().copied().flatten() != Some(position) {
                    let mut value = serde_json::to_value(run)?;
                    value["queuePosition"] = json!(position);
                    plan.emit(command, "run.updated", &value, now)?;
                }
            }
            Ok(())
        }
        "t3_queue_promote_to_steer" => {
            if p.thread.archived_at.is_some() {
                return Err(refuse("Thread is not active."));
            }
            let message = message
                .ok_or_else(|| refuse("Queued run is missing message or execution state."))?;
            if automatic(message) {
                return Err(refuse(
                    "Automatic completion deliveries cannot be promoted to Steer.",
                ));
            }
            if !p
                .nodes
                .iter()
                .any(|n| Some(&n.id) == run.root_node_id.as_ref())
                || !p
                    .attempts
                    .iter()
                    .any(|a| Some(&a.id) == run.active_attempt_id.as_ref())
            {
                return Err(refuse("Queued run is missing message or execution state."));
            }
            let target = op.input["targetRunId"].as_str().unwrap_or("");
            steer(&mut Plan::default(), command, p, target, message, now)?;
            cancel(plan, command, p, run, now)?;
            steer(plan, command, p, target, message, now)?;
            plan.queue_patch = Some(json!({"action":"cancel","messageId":run.user_message_id}));
            Ok(())
        }
        _ => unreachable!(),
    }
}

fn steer(
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    target: &str,
    message: &Value,
    now: i64,
) -> Result<()> {
    let run = p
        .runs
        .iter()
        .find(|r| r.id.0 == target)
        .ok_or_else(|| refuse("Target run was not found."))?;
    if run.status != OrchestrationV2RunStatus::Running || run.root_node_id.is_none() {
        return Err(refuse("Target run cannot be steered."));
    }
    // TODO(merge-threads): negotiate provider-owned selection transitions with
    // that slice's steering policy. Never silently steer the old instance
    // when the saved next-turn selection requires a provider handoff.
    if p.thread.provider_instance_id != run.provider_instance_id {
        return Err(refuse("Provider handoff is required before steering."));
    }
    let target_message = records(p, "message")
        .iter()
        .find(|m| m["id"] == run.user_message_id.0);
    let maintenance = |m: &Value| {
        m["text"].as_str().is_some_and(|s| {
            matches!(
                s.trim_matches(super::search::js_whitespace)
                    .to_ascii_lowercase()
                    .as_str(),
                "/compact" | "/logout"
            )
        })
    };
    if maintenance(message) || target_message.is_some_and(maintenance) {
        return Err(refuse("Native maintenance cannot be steered."));
    }
    let provider = records(p, "provider-thread")
        .iter()
        .find(|t| {
            Some(t["id"].as_str().unwrap_or(""))
                == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
        })
        .ok_or_else(|| refuse("Provider thread has no active provider session for steering."))?;
    let session_id = provider["providerSessionId"]
        .as_str()
        .ok_or_else(|| refuse("Provider session is not active."))?;
    let session = records(p, "provider-session")
        .iter()
        .find(|s| s["id"] == session_id)
        .ok_or_else(|| refuse("Provider session is not active."))?;
    if session["capabilities"]["turns"]["supportsActiveSteering"] != true {
        return Err(refuse("Provider does not support active steering."));
    }
    let turn = records(p, "provider-turn")
        .iter()
        .find(|t| {
            t["runAttemptId"].as_str() == run.active_attempt_id.as_ref().map(|a| a.0.as_str())
                && t["status"] == "running"
        })
        .ok_or_else(|| refuse("No running provider turn found for active run."))?;
    let mut message = message.clone();
    message["runId"] = json!(run.id);
    message["nodeId"] = json!(run.root_node_id);
    message["createdAt"] = json!(iso(now)?);
    message["updatedAt"] = json!(iso(now)?);
    plan.emit(command, "message.updated", &message, now)?;
    let mut item = json!({"id":format!("turn-item:message:{}",crate::orchestration::event::encode_component(message["id"].as_str().unwrap())),
        "threadId":p.thread.id,"runId":run.id,"nodeId":run.root_node_id,"providerThreadId":run.provider_thread_id,
        "providerTurnId":turn["id"],"nativeItemRef":null,"parentItemId":null,
        "ordinal":records(p,"turn-item").iter().filter_map(|i|i["ordinal"].as_i64()).max().unwrap_or(0)+1,
        "status":"completed","title":null,"startedAt":iso(now)?,"completedAt":iso(now)?,"updatedAt":iso(now)?,
        "type":"user_message","messageId":message["id"],"inputIntent":if command.command_type()?=="queued-message.promote-to-steer"{"promoted_queued_to_steer"}else{"steer"},
        "createdBy":message["createdBy"],"creationSource":message["creationSource"],
        "text":message["text"],"attachments":message["attachments"]});
    for field in ["context", "scheduledTaskId", "senderThreadId"] {
        if let Some(value) = message.get(field) {
            item[field] = value.clone();
        }
    }
    plan.emit(command, "turn-item.updated", &item, now)?;
    plan.effects.push(EffectRequest::ProviderTurnSteer {
        provider_session_id: ProviderSessionId(session_id.into()),
        provider_thread_id: run.provider_thread_id.clone().unwrap(),
        provider_turn_id: ProviderTurnId(turn["id"].as_str().unwrap().into()),
        message_id: MessageId(message["id"].as_str().unwrap().into()),
    });
    Ok(())
}

fn respond(
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    input: &Value,
    cancelled: bool,
    now: i64,
) -> Result<()> {
    let id = input["requestId"].as_str().unwrap_or("");
    let (request, item) =
        question(p, id).ok_or_else(|| refuse("The pending user-input request was not found."))?;
    let capability = request["responseCapability"]["type"]
        .as_str()
        .unwrap_or("not_resumable");
    if capability == "not_resumable" {
        return Err(refuse(
            request["responseCapability"]["reason"]
                .as_str()
                .unwrap_or("Request is not resumable."),
        ));
    }
    let session_id = request["responseCapability"]["providerSessionId"].as_str();
    if capability == "live"
        && !session_id
            .is_some_and(|id| records(p, "provider-session").iter().any(|s| s["id"] == id))
    {
        return Err(refuse("Provider session was not found."));
    }
    let mut request = request.clone();
    request["status"] = json!("resolved");
    request["resolvedAt"] = json!(iso(now)?);
    if cancelled {
        request["decision"] = json!("cancel");
    } else {
        request["answers"] = input["answers"].clone();
    }
    plan.emit(command, "runtime-request.updated", &request, now)?;
    let status = if cancelled { "cancelled" } else { "completed" };
    if let Some(node) = p.nodes.iter().find(|n| request["nodeId"] == n.id.0) {
        let mut node = serde_json::to_value(node)?;
        node["status"] = json!(status);
        node["completedAt"] = json!(iso(now)?);
        plan.emit(command, "node.updated", &node, now)?;
    }
    let mut item = item.clone();
    item["status"] = json!(status);
    item["completedAt"] = json!(iso(now)?);
    item["updatedAt"] = json!(iso(now)?);
    if !cancelled {
        let questions = item["questions"]
            .as_array()
            .ok_or_else(|| refuse("Question was not found."))?;
        let texts: serde_json::Map<String, Value> = questions
            .iter()
            .map(|q| (q["id"].as_str().unwrap_or("").into(), q["question"].clone()))
            .collect();
        item["questionAnswer"] = json!({"requestId":id,"answers":input["answers"],"attachmentsByQuestionId":{},"questionTextById":texts});
    }
    plan.emit(command, "turn-item.updated", &item, now)?;
    if cancelled {
        return Ok(());
    }
    if capability == "live" {
        plan.effects.push(EffectRequest::RuntimeRequestRespond {
            provider_session_id: ProviderSessionId(session_id.unwrap().into()),
            request_id: RuntimeRequestId(id.into()),
            decision: None,
            answers: Some(serde_json::from_value(input["answers"].clone())?),
        });
    } else if capability == "message" {
        let mut replies = vec![];
        for q in item["questions"].as_array().unwrap() {
            let answer = input["answers"][q["id"].as_str().unwrap_or("")]
                .as_str()
                .unwrap_or("")
                .trim_matches(super::search::js_whitespace);
            if answer.is_empty() {
                if q["required"] == false {
                    continue;
                }
                return Err(refuse("Answer each question before sending."));
            }
            replies.push(format!(
                "{}\n{answer}",
                q["question"].as_str().unwrap_or("")
            ));
        }
        if replies.is_empty() {
            return Err(refuse("Enter an answer before sending."));
        }
        let id = format!("async-answer:{id}");
        let message = json!({"id":id,"threadId":p.thread.id,"runId":null,"nodeId":null,"role":"user","text":replies.join("\n\n"),
            "attachments":[],"streaming":false,"createdBy":"user","creationSource":"server","createdAt":iso(now)?,"updatedAt":iso(now)?});
        if let Some(run) = p
            .runs
            .iter()
            .rev()
            .find(|r| r.status == OrchestrationV2RunStatus::Running)
            && steer(&mut Plan::default(), command, p, &run.id.0, &message, now).is_ok()
        {
            steer(plan, command, p, &run.id.0, &message, now)?;
            return Ok(());
        }
        enqueue_answer(plan, command, p, &id, &message, now)?;
    } else {
        return Err(refuse("Unknown response capability."));
    }
    Ok(())
}

fn enqueue_answer(
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    id: &str,
    message: &Value,
    now: i64,
) -> Result<()> {
    let ordinal = p.runs.iter().map(|r| r.ordinal).max().unwrap_or(0) + 1;
    let busy = p
        .runs
        .iter()
        .any(|r| !crate::orchestration::command::run_terminal(&r.status));
    let driver = records(p, "provider-thread")
        .iter()
        .rev()
        .find(|provider| provider["providerInstanceId"] == p.thread.provider_instance_id.0)
        .and_then(|t| t["driver"].as_str())
        .unwrap_or("unknown");
    let seed = task::execution_seed_for(
        p,
        &p.thread,
        ordinal,
        id,
        if busy { "queued" } else { "starting" },
        driver,
        now,
    )?;
    let mut message = message.clone();
    message["runId"] = json!(seed.run.id);
    plan.emit(command, "message.updated", &message, now)?;
    task::emit_execution(plan, command, &p.thread.id, &seed, now)
}

fn metadata(
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    op: &QueueCommand,
    now: i64,
) -> Result<()> {
    let input = &op.input;
    let mut thread = serde_json::to_value(&p.thread)?;
    match input["action"].as_str() {
        Some("rename") => {
            thread["title"] = input["title"].clone();
            thread["titleRegeneration"] = Value::Null;
        }
        Some("regenerate_title") => {
            thread["titleRegeneration"] = json!({"requestId":command.id,"startedAt":iso(now)?});
            plan.effects.push(EffectRequest::ThreadTitleGenerate {
                kind: TitleKind::Regenerate,
            });
        }
        Some("link_pull_request" | "unlink_pull_request") => {
            let mut authority = p.thread.clone();
            op.links.update_metadata(
                &mut authority,
                serde_json::from_value(input["resolvedPullRequest"].clone())?,
                &iso(now)?,
            )?;
            thread = serde_json::to_value(authority)?;
        }
        _ => return Err(refuse("Invalid metadata action.")),
    }
    thread["updatedAt"] = json!(iso(now)?);
    plan.emit(command, "thread.metadata-updated", &thread, now)
}

fn parse_time(value: &Value) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value.as_str()?)
        .ok()
        .map(|t| t.timestamp_millis())
}

fn organize(
    conn: &Connection,
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    op: &QueueCommand,
    now: i64,
) -> Result<()> {
    let mut thread = serde_json::to_value(&p.thread)?;
    let action = match op.name.as_str() {
        "host.unsnooze_due" => "wake",
        "host.acknowledge_woke" => "ack",
        _ => op.input["action"].as_str().unwrap_or(""),
    };
    if p.thread.archived_at.is_some()
        && !matches!(action, "unarchive" | "archive" | "mark_unread" | "ack")
    {
        return Err(refuse(format!("Thread {} is archived.", p.thread.id.0)));
    }
    let time = json!(iso(now)?);
    let pending: Vec<_> = records(p, "runtime-request")
        .iter()
        .filter(|r| r["status"] == "pending")
        .collect();
    let mut changed = true;
    let event = match action {
        "pin" => {
            changed = thread["pinnedAt"].is_null()
                || thread["settledOverride"] == "settled"
                || !thread["snoozedUntil"].is_null();
            if thread["pinnedAt"].is_null() {
                thread["pinnedAt"] = time.clone();
            }
            if thread["settledOverride"] == "settled" {
                thread["settledOverride"] = json!("active");
                thread["settledAt"] = Value::Null;
            }
            thread["snoozedUntil"] = Value::Null;
            thread["snoozedAt"] = Value::Null;
            "thread.pinned"
        }
        "unpin" => {
            changed = !thread["pinnedAt"].is_null();
            thread["pinnedAt"] = Value::Null;
            thread["pinOrderKey"] = Value::Null;
            "thread.unpinned"
        }
        "snooze" => {
            let wake = parse_time(&op.input["snoozedUntil"])
                .filter(|t| *t > now)
                .ok_or_else(|| refuse("Snooze wake time is not in the future."))?;
            if !pending.is_empty() {
                return Err(refuse(
                    "Thread has a pending approval or user-input request and cannot be snoozed.",
                ));
            }
            if p.runs
                .iter()
                .any(|r| r.status == OrchestrationV2RunStatus::Queued)
            {
                return Err(refuse("Thread has a queued run and cannot be snoozed."));
            }
            changed =
                parse_time(&thread["snoozedUntil"]) != Some(wake) || thread["snoozedAt"].is_null();
            thread["snoozedUntil"] = json!(iso(wake)?);
            if changed {
                thread["snoozedAt"] = time.clone();
            }
            if thread["limitRecovery"].is_object() {
                thread["limitRecovery"]["snooze"] = json!(false);
            }
            "thread.snoozed"
        }
        "unsnooze" | "wake" => {
            if action == "wake" && !parse_time(&thread["snoozedUntil"]).is_some_and(|t| t <= now) {
                return Err(refuse("Snooze is not due."));
            }
            changed = !thread["snoozedUntil"].is_null();
            thread["snoozedUntil"] = Value::Null;
            thread["snoozedAt"] = Value::Null;
            "thread.unsnoozed"
        }
        "settle" => {
            if op.input["settledBy"] == "Auto"
                && (!thread["pinnedAt"].is_null()
                    || !thread["settledOverride"].is_null()
                    || !thread["autoSettleDisabledAt"].is_null())
            {
                return Err(refuse("Thread is excluded from automatic settlement."));
            }
            let automatic_runs: Vec<_> = queued(p)
                .into_iter()
                .filter(|r| {
                    records(p, "message")
                        .iter()
                        .any(|m| m["id"] == r.user_message_id.0 && automatic(m))
                })
                .collect();
            if p.runs.iter().any(|r| {
                !crate::orchestration::command::run_terminal(&r.status)
                    && !automatic_runs.iter().any(|a| a.id == r.id)
            }) || pending
                .iter()
                .any(|r| r["kind"] != "user_input" || r["responseCapability"]["type"] != "message")
            {
                return Err(refuse(
                    "Thread has active or blocked work and cannot be settled.",
                ));
            }
            for request in pending {
                respond(
                    plan,
                    command,
                    p,
                    &json!({"requestId":request["id"]}),
                    true,
                    now,
                )?;
            }
            for run in automatic_runs {
                cancel(plan, command, p, run, now)?;
            }
            changed = thread["settledOverride"] != "settled"
                || thread["settledAt"].is_null()
                || !thread["pinnedAt"].is_null();
            thread["settledOverride"] = json!("settled");
            if changed {
                thread["settledAt"] = time.clone();
            }
            thread["unsettledAt"] = Value::Null;
            thread["pinnedAt"] = Value::Null;
            thread["pinOrderKey"] = Value::Null;
            thread["activeOrderKey"] = Value::Null;
            "thread.settled"
        }
        "unsettle" => {
            changed = thread["settledOverride"] != "active";
            thread["settledOverride"] = json!("active");
            thread["settledAt"] = Value::Null;
            if changed {
                thread["unsettledAt"] = time.clone();
            }
            "thread.unsettled"
        }
        "archive" => {
            if !thread["archivedAt"].is_null() {
                return Err(refuse("Thread is already archived."));
            }
            thread["archivedAt"] = time.clone();
            thread["titleRegeneration"] = Value::Null;
            plan.queue_patch = Some(json!({"action":"cancel_many",
                "messageIds":queued(p).iter().map(|r|r.user_message_id.0.clone()).collect::<Vec<_>>()}));
            "thread.archived"
        }
        "unarchive" => {
            if thread["archivedAt"].is_null() {
                return Err(refuse("Thread is not archived."));
            }
            thread["archivedAt"] = Value::Null;
            "thread.unarchived"
        }
        "mark_unread" => {
            let completed = p
                .runs
                .last()
                .and_then(|r| r.completed_at.as_deref())
                .and_then(|s| parse_time(&json!(s)))
                .ok_or_else(|| refuse("Thread has no completed run to mark unread."))?;
            thread["lastVisitedAt"] = json!(iso(completed - 1)?);
            "thread.marked-unread"
        }
        // wokeAt is deliberately a separate durable record, not an unknown
        // field on the pinned T3 AppThread contract.
        "ack" => "thread.metadata-updated",
        _ => return Err(refuse("Invalid lifecycle action.")),
    };
    if changed {
        thread["updatedAt"] = time.clone();
    }
    plan.emit(command, event, &thread, now)?;
    if action == "archive" {
        // Archive metadata precedes queued-graph disposal in T3's event stream.
        for run in &p.runs {
            if run.status == OrchestrationV2RunStatus::Queued {
                cancel(plan, command, p, run, now)?;
            } else if !crate::orchestration::command::run_terminal(&run.status) {
                plan.effects.push(EffectRequest::ManagedRunInterrupt {
                    run_id: run.id.clone(),
                });
            }
        }
    }
    if matches!(action, "archive" | "settle") {
        for session in records(p, "provider-session")
            .iter()
            .filter(|session| !matches!(session["status"].as_str(), Some("stopped" | "error")))
        {
            let id = session["id"]
                .as_str()
                .ok_or_else(|| refuse("Provider session identity is missing."))?;
            plan.emit(
                command,
                "provider-session.detached",
                &json!({"providerSessionId":id,"detachedAt":iso(now)?,
                    "reason":if action=="archive"{"Thread archived."}else{"Thread settled."}}),
                now,
            )?;
            plan.effects.push(EffectRequest::ProviderSessionDetach {
                provider_session_id: ProviderSessionId(id.into()),
            });
        }
    }
    let mut marker = crate::orchestration::ui_queue::marker(conn, &p.thread.id)?;
    if action == "wake" {
        marker["wokeAt"] = time;
    }
    if action == "ack" {
        marker["wokeAt"] = Value::Null;
    }
    if action == "settle" {
        marker["settledBy"] = if op.input["settledBy"] == "Auto" {
            json!("Auto")
        } else {
            json!("User")
        };
    }
    if action == "unsettle" || action == "pin" {
        marker["settledBy"] = Value::Null;
    }
    // Stored in the domain table by the kernel hook, not as a fabricated T3 event.
    plan.queue_lifecycle = Some(marker);
    Ok(())
}

fn sync_loro(
    conn: &Connection,
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    input: &Value,
    now: i64,
) -> Result<()> {
    let rows: Vec<zeron_doc::QueuedMessage> = serde_json::from_value(input["items"].clone())?;
    let previous = crate::orchestration::ui_queue::loro_intents(conn, &p.thread.id)?;
    for run in queued(p).into_iter().filter(|r| {
        previous.iter().any(|i| i.id == r.user_message_id.0)
            && !rows.iter().any(|i| i.id == r.user_message_id.0)
    }) {
        cancel_graph(plan, command, p, run, now)?;
    }
    let mut ordinal = p.runs.iter().map(|r| r.ordinal).max().unwrap_or(0);
    for (index, row) in rows.iter().enumerate() {
        let known = p.runs.iter().find(|r| r.user_message_id.0 == row.id);
        if let Some(run) = known {
            if run.status != OrchestrationV2RunStatus::Queued {
                continue;
            }
            let message = records(p, "message").iter().find(|m| m["id"] == row.id);
            if let Some(old) = message {
                let mut message = old.clone();
                message["text"] = json!(row.text);
                if message != *old {
                    message["updatedAt"] = json!(iso(now)?);
                    plan.emit(command, "message.updated", &message, now)?;
                }
            }
            let mut value = serde_json::to_value(run)?;
            value["queuePosition"] = json!(index + 1);
            if value != serde_json::to_value(run)? {
                plan.emit(command, "run.updated", &value, now)?;
            }
        } else {
            ordinal += 1;
            let seed = task::execution_seed_for(
                p,
                &p.thread,
                ordinal,
                &row.id,
                "queued",
                input["driver"].as_str().unwrap_or("unknown"),
                row.issued_at,
            )?;
            let mut run = serde_json::to_value(&seed.run)?;
            run["queuePosition"] = json!(index + 1);
            // Do not replace the active provider binding for an intent.
            plan.emit(command, "run.created", &run, now)?;
            plan.emit(command, "run-attempt.created", &seed.attempt, now)?;
            plan.emit(command, "node.updated", &seed.root, now)?;
            let message = json!({"id":row.id,"threadId":p.thread.id,"runId":seed.run.id,"nodeId":seed.root.id,"role":"user",
                "text":row.text,"attachments":[],"streaming":false,"createdBy":"user","creationSource":"web",
                "createdAt":iso(row.issued_at)?,"updatedAt":iso(row.edited_at.unwrap_or(row.issued_at))?});
            plan.emit(command, "message.updated", &message, now)?;
        }
    }
    // Store attachment paths and edit leases losslessly in our own read table.
    plan.queue_intents = Some(rows);
    if plan.events.is_empty() {
        plan.emit(command, "thread.metadata-updated", &p.thread, now)?;
    }
    Ok(())
}

fn adopt_delivery(
    plan: &mut Plan,
    command: &Command,
    p: &ThreadProjection,
    input: &Value,
    now: i64,
) -> Result<()> {
    if p.runs.iter().any(|r| {
        r.status != OrchestrationV2RunStatus::Queued
            && !crate::orchestration::command::run_terminal(&r.status)
    }) {
        return Err(refuse("Thread is not sendable."));
    }
    let run = queued(p)
        .into_iter()
        .find(|r| r.user_message_id.0 == input["messageId"].as_str().unwrap_or(""))
        .ok_or_else(|| refuse("Queued run no longer exists."))?;
    let seed = task::execution_seed_for(
        p,
        &p.thread,
        run.ordinal,
        &run.user_message_id.0,
        "starting",
        input["driver"]
            .as_str()
            .or_else(|| {
                records(p, "provider-thread")
                    .iter()
                    .rev()
                    .find(|provider| {
                        provider["providerInstanceId"] == p.thread.provider_instance_id.0
                    })
                    .and_then(|provider| provider["driver"].as_str())
            })
            .unwrap_or("unknown"),
        now,
    )?;
    // Queue admission only reserves identities. At delivery, bind the existing
    // run/attempt/root together to the selected provider generation, retaining
    // the queue's message identity and the user's current model selection.
    let mut value = serde_json::to_value(run)?;
    value["status"] = json!("starting");
    value["queuePosition"] = Value::Null;
    value["providerInstanceId"] = json!(seed.run.provider_instance_id);
    value["modelSelection"] = json!(seed.run.model_selection);
    value["providerThreadId"] = json!(seed.provider_thread.id);
    plan.emit(command, "run.updated", &value, now)?;
    let mut attempt = p
        .attempts
        .iter()
        .find(|attempt| run.active_attempt_id.as_ref() == Some(&attempt.id))
        .cloned()
        .ok_or_else(|| refuse("Queued run has no active attempt."))?;
    attempt.provider_instance_id = seed.run.provider_instance_id;
    attempt.provider_thread_id = seed.provider_thread.id.clone();
    plan.emit(command, "run-attempt.updated", &attempt, now)?;
    let mut root = p
        .nodes
        .iter()
        .find(|node| run.root_node_id.as_ref() == Some(&node.id))
        .cloned()
        .ok_or_else(|| refuse("Queued run has no root node."))?;
    root.provider_thread_id = Some(seed.provider_thread.id.clone());
    plan.emit(command, "node.updated", &root, now)?;
    let mut provider = seed.provider_thread;
    provider.owner_node_id = run.root_node_id.clone();
    plan.emit(command, "provider-thread.updated", &provider, now)?;
    Ok(())
}
