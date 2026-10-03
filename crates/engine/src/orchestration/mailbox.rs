//! Immutable result delivery, separate from task/run terminality.
use serde_json::{Value, json};
use zeron_proto::orchestration::*;

use super::Result;
use super::command::{Command, Plan};
use super::effects::EffectRequest;
use super::event::{encode_component, iso};
use super::projection::ThreadProjection;
use super::task::{active_run, emit_execution, execution_seed, message, records};

#[derive(Debug, Clone)]
pub enum DeliveryAction {
    BeginSteer,
    Queue,
    Accepted,
    Completed { cancelled: bool },
    Recover,
}

#[derive(Debug, Clone)]
pub struct DeliveryCommand {
    pub parent_run_id: RunId,
    pub generation: i64,
    pub message_id: MessageId,
    pub action: DeliveryAction,
}

pub(crate) fn cohort(run: &OrchestrationV2Run) -> Value {
    serde_json::to_value(run).expect("run codec")["delegatedCompletion"].clone()
}

fn state(task: &Value) -> &str {
    task["completionDelivery"]["state"]
        .as_str()
        .unwrap_or("pending")
}

fn wake_detail(ids: &[Value]) -> String {
    format!(
        "Delegated task completion available. Read task_status for each task to consume its result:\n{}",
        ids.iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// All sibling/result/cohort mutations are planned while holding the parent
/// authority lock. A queued batch can grow; an in-flight batch cannot.
pub(crate) fn reserve(
    projection: &ThreadProjection,
    updated: Vec<Value>,
    command: &Command,
    plan: &mut Plan,
    now: i64,
) -> Result<()> {
    let mut tasks = records(projection, "subagent").to_vec();
    for task in &updated {
        if let Some(old) = tasks.iter_mut().find(|old| old["id"] == task["id"]) {
            *old = task.clone();
        }
    }
    for run in &projection.runs {
        let mut run_value = serde_json::to_value(run)?;
        let mut cohort = cohort(run);
        if cohort.is_null() {
            cohort = json!({"disposition":"open","nextGeneration":1,"delivery":null});
        }
        let disabled = projection.thread.archived_at.is_some()
            || projection.thread.deleted_at.is_some()
            || cohort["disposition"] != "open";
        let mut candidates = vec![];
        for task in &mut tasks {
            if task["origin"] != "app_owned"
                || task["runId"] != run.id.0
                || !task["result"].is_string()
                || matches!(state(task), "acknowledged" | "disposed" | "delivered")
            {
                continue;
            }
            if disabled {
                task["completionDelivery"] = json!({"state":"disposed","observedByRunId":null});
            } else if (task["completionWake"] == "always" || active_run(projection).is_none())
                && state(task) == "pending"
            {
                candidates.push(task["id"].clone());
            }
        }
        if disabled {
            cohort["delivery"] = Value::Null;
        } else if !candidates.is_empty() {
            let delivery = &cohort["delivery"];
            let delivery_message = records(projection, "message")
                .iter()
                .find(|message| message["id"] == delivery["messageId"]);
            let delivery_run = projection
                .runs
                .iter()
                .find(|run| run.user_message_id.0 == delivery["messageId"].as_str().unwrap_or(""));
            let joinable = !delivery.is_null()
                && delivery_run.is_none_or(|run| run.status == OrchestrationV2RunStatus::Queued)
                && delivery_message.is_none_or(|message| message["streaming"] != true);
            if delivery.is_null() || joinable {
                let generation = if joinable {
                    delivery["generation"].as_i64().unwrap()
                } else {
                    cohort["nextGeneration"].as_i64().unwrap_or(1)
                };
                let message_id = if joinable {
                    delivery["messageId"].as_str().unwrap().to_owned()
                } else {
                    format!(
                        "message:delegated-completion:{}:{generation}",
                        encode_component(&run.id.0)
                    )
                };
                let mut ids = if joinable {
                    delivery["taskIds"].as_array().cloned().unwrap_or_default()
                } else {
                    vec![]
                };
                for id in &candidates {
                    if !ids.contains(id) {
                        ids.push(id.clone());
                    }
                }
                for task in &mut tasks {
                    if candidates.contains(&task["id"]) {
                        task["completionDelivery"] =
                            json!({"state":"claimed","observedByRunId":null});
                    }
                }
                cohort["delivery"] =
                    json!({"generation":generation,"messageId":message_id,"taskIds":ids});
                if !joinable {
                    cohort["nextGeneration"] = json!(generation + 1);
                    plan.effects
                        .push(EffectRequest::DelegatedCompletionContinue {
                            parent_run_id: run.id.clone(),
                            generation,
                            message_id: MessageId(message_id.clone()),
                        });
                }
                let mut notification = delivery_message.cloned().unwrap_or(message(
                    &projection.thread.id,
                    None,
                    None,
                    &message_id,
                    &wake_detail(&ids),
                    "user",
                    now,
                )?);
                notification["text"] = json!(wake_detail(&ids));
                notification["delegatedCompletion"] =
                    json!({"parentRunId":run.id,"generation":generation,"taskIds":ids});
                notification["updatedAt"] = json!(iso(now)?);
                plan.emit(command, "message.updated", &notification, now)?;
            }
        }
        if cohort != self::cohort(run) {
            run_value["delegatedCompletion"] = cohort;
            plan.emit(command, "run.updated", &run_value, now)?;
        }
    }
    for task in tasks {
        let old = records(projection, "subagent")
            .iter()
            .find(|old| old["id"] == task["id"]);
        if old.is_none_or(|old| old != &task) {
            plan.emit(command, "subagent.updated", &task, now)?;
        }
    }
    Ok(())
}

pub fn current_delivery(projection: &ThreadProjection, input: &DeliveryCommand) -> Option<Value> {
    let run = projection
        .runs
        .iter()
        .find(|run| run.id == input.parent_run_id)?;
    let cohort = cohort(run);
    let delivery = &cohort["delivery"];
    if projection.thread.archived_at.is_some()
        || projection.thread.deleted_at.is_some()
        || cohort["disposition"] != "open"
        || delivery["generation"].as_i64() != Some(input.generation)
        || delivery["messageId"] != input.message_id.0
        || !(delivery["taskIds"].as_array()?.iter().any(|id| {
            records(projection, "subagent")
                .iter()
                .any(|task| task["id"] == *id && state(task) == "claimed")
        }) || (delivery["taskIds"].as_array()?.is_empty()
            && matches!(
                input.action,
                DeliveryAction::Accepted | DeliveryAction::Completed { .. }
            )))
    {
        return None;
    }
    Some(delivery.clone())
}

/// T3's live-session gate, not a newly resolved catalog/harness capability.
pub(crate) fn can_steer_delivery(projection: &ThreadProjection, input: &DeliveryCommand) -> bool {
    let Some(delivery) = current_delivery(projection, input) else {
        return false;
    };
    if !delivery["taskIds"].as_array().unwrap().iter().all(|id| {
        records(projection, "subagent")
            .iter()
            .any(|task| task["id"] == *id && task["completionWake"] == "always")
    }) {
        return false;
    }
    let Some(run) =
        active_run(projection).filter(|run| run.status == OrchestrationV2RunStatus::Running)
    else {
        return false;
    };
    if records(projection, "message").iter().any(|message| {
        message["id"] == run.user_message_id.0
            && message["attachments"].as_array().is_some_and(Vec::is_empty)
            && matches!(
                message["text"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .to_ascii_lowercase()
                    .as_str(),
                "/compact" | "/logout"
            )
    }) {
        return false;
    }
    let turn_running = records(projection, "provider-turn").iter().any(|turn| {
        turn["runAttemptId"].as_str() == run.active_attempt_id.as_ref().map(|id| id.0.as_str())
            && turn["status"] == "running"
    });
    let session = records(projection, "provider-thread")
        .iter()
        .find(|provider| {
            provider["id"].as_str() == run.provider_thread_id.as_ref().map(|id| id.0.as_str())
        })
        .and_then(|provider| provider["providerSessionId"].as_str())
        .and_then(|id| {
            records(projection, "provider-session")
                .iter()
                .find(|session| session["id"] == id)
        });
    turn_running
        && session.is_some_and(|session| {
            session["capabilities"]["turns"]["supportsActiveSteering"] == true
                && !matches!(session["status"].as_str(), Some("stopped" | "error"))
        })
}

pub(crate) fn plan_delivery(
    projection: &ThreadProjection,
    command: &Command,
    plan: &mut Plan,
    input: &DeliveryCommand,
    now: i64,
) -> Result<()> {
    let Some(delivery) = current_delivery(projection, input) else {
        return Ok(());
    };
    let Some(original) = projection
        .runs
        .iter()
        .find(|run| run.id == input.parent_run_id)
    else {
        return Ok(());
    };
    let Some(saved_message) = records(projection, "message")
        .iter()
        .find(|message| message["id"] == input.message_id.0)
    else {
        return Ok(());
    };
    let existing_run = projection
        .runs
        .iter()
        .find(|run| run.user_message_id == input.message_id);
    match input.action {
        DeliveryAction::BeginSteer => {
            let Some(live) = active_run(projection)
                .filter(|run| run.status == OrchestrationV2RunStatus::Running)
            else {
                return Ok(());
            };
            let mut message = saved_message.clone();
            message["runId"] = json!(live.id);
            message["nodeId"] = json!(live.root_node_id);
            // Durable in-flight fence: siblings must reserve a successor rather
            // than edit a message already offered to the provider.
            message["streaming"] = json!(true);
            message["updatedAt"] = json!(iso(now)?);
            plan.emit(command, "message.updated", &message, now)?;
        }
        DeliveryAction::Queue | DeliveryAction::Recover => {
            if let Some(run) = existing_run {
                if matches!(input.action, DeliveryAction::Recover)
                    && run.status == OrchestrationV2RunStatus::Queued
                {
                    let mut run = serde_json::to_value(run)?;
                    run["queueHeld"] = json!(false);
                    plan.emit(command, "run.updated", &run, now)?;
                }
                return Ok(());
            }
            let mut thread = projection.thread.clone();
            thread.model_selection = original.model_selection.clone();
            thread.provider_instance_id = original.provider_instance_id.clone();
            let driver = records(projection, "provider-thread")
                .first()
                .and_then(|provider| provider["driver"].as_str())
                .unwrap_or("unknown");
            let ordinal = projection
                .runs
                .iter()
                .map(|run| run.ordinal)
                .max()
                .unwrap_or(0)
                + 1;
            let status = if active_run(projection).is_some() {
                "queued"
            } else {
                "starting"
            };
            let mut seed =
                execution_seed(&thread, ordinal, &input.message_id.0, status, driver, now)?;
            if status == "queued" {
                seed.run.queue_position = Optional::Present(Some(ordinal));
                seed.run.queue_held = Optional::Present(false);
                // Queued attempts do not take over the provider-thread binding
                // from the active parent. Bind only when promoted.
                plan.emit(command, "run.created", &seed.run, now)?;
                plan.emit(command, "run-attempt.created", &seed.attempt, now)?;
                plan.emit(command, "node.updated", &seed.root, now)?;
            } else {
                emit_execution(plan, command, &thread.id, &seed, now)?;
            }
            let mut message = saved_message.clone();
            message["runId"] = json!(seed.run.id);
            message["nodeId"] = json!(seed.root.id);
            message["streaming"] = json!(false);
            message["updatedAt"] = json!(iso(now)?);
            plan.emit(command, "message.updated", &message, now)?;
        }
        DeliveryAction::Accepted | DeliveryAction::Completed { .. } => {
            let cancelled = matches!(input.action, DeliveryAction::Completed { cancelled: true });
            let mut tasks = vec![];
            for task in records(projection, "subagent") {
                if delivery["taskIds"]
                    .as_array()
                    .unwrap()
                    .contains(&task["id"])
                    && state(task) == "claimed"
                {
                    let mut task = task.clone();
                    task["completionDelivery"] = json!({"state":if cancelled {"pending"} else {"delivered"},"observedByRunId":null});
                    tasks.push(task);
                }
            }
            let mut next_projection = projection.clone();
            let run = next_projection
                .runs
                .iter_mut()
                .find(|run| run.id == input.parent_run_id)
                .unwrap();
            let mut value = serde_json::to_value(&*run)?;
            value["delegatedCompletion"]["delivery"] = Value::Null;
            *run = serde_json::from_value(value.clone())?;
            plan.emit(command, "run.updated", &value, now)?;
            for task in &tasks {
                plan.emit(command, "subagent.updated", task, now)?;
            }
            // Pass the just-updated tasks, so reserve sees arrivals while the
            // previous batch was active and coalesces them into one successor.
            for task in &tasks {
                if let Some(old) = next_projection
                    .records
                    .get_mut("subagent")
                    .and_then(|tasks| tasks.iter_mut().find(|old| old["id"] == task["id"]))
                {
                    *old = task.clone();
                }
            }
            reserve(&next_projection, vec![], command, plan, now)?;
        }
    }
    Ok(())
}

pub(crate) fn remove_member(
    projection: &ThreadProjection,
    task: &NodeId,
    command: &Command,
    plan: &mut Plan,
    now: i64,
) -> Result<()> {
    for run in &projection.runs {
        let mut value = serde_json::to_value(run)?;
        if let Some(ids) = value["delegatedCompletion"]["delivery"]["taskIds"].as_array_mut()
            && ids.iter().any(|id| id == &task.0)
        {
            ids.retain(|id| id != &task.0);
            let remaining = ids.clone();
            let empty = ids.is_empty();
            let message_id = value["delegatedCompletion"]["delivery"]["messageId"].clone();
            let delivery_run = projection
                .runs
                .iter()
                .find(|run| run.user_message_id.0 == message_id.as_str().unwrap());
            let unstarted =
                delivery_run.is_none_or(|run| run.status == OrchestrationV2RunStatus::Queued);
            // An empty active batch is still an in-flight fence. Its terminal
            // receipt must reserve pending successors, even after all reads.
            if empty && unstarted {
                value["delegatedCompletion"]["delivery"] = Value::Null;
            }
            plan.emit(command, "run.updated", &value, now)?;
            if empty
                && let Some(queued) =
                    delivery_run.filter(|run| run.status == OrchestrationV2RunStatus::Queued)
            {
                let mut row = serde_json::to_value(queued)?;
                row["status"] = json!("cancelled");
                row["completedAt"] = json!(iso(now)?);
                row["queuePosition"] = Value::Null;
                plan.emit(command, "run.updated", &row, now)?;
                for attempt in projection
                    .attempts
                    .iter()
                    .filter(|attempt| attempt.run_id == queued.id)
                {
                    let mut row = serde_json::to_value(attempt)?;
                    row["status"] = json!("cancelled");
                    row["completedAt"] = json!(iso(now)?);
                    plan.emit(command, "run-attempt.updated", &row, now)?;
                }
                for node in projection
                    .nodes
                    .iter()
                    .filter(|node| node.run_id.as_ref() == Some(&queued.id))
                {
                    let mut row = serde_json::to_value(node)?;
                    row["status"] = json!("cancelled");
                    row["completedAt"] = json!(iso(now)?);
                    plan.emit(command, "node.updated", &row, now)?;
                }
            }
            if let Some(message) = records(projection, "message")
                .iter()
                .find(|message| message["id"] == message_id)
                && message["streaming"] != true
                && unstarted
            {
                let mut message = message.clone();
                message["text"] = json!(wake_detail(&remaining));
                message["delegatedCompletion"]["taskIds"] = json!(remaining);
                message["updatedAt"] = json!(iso(now)?);
                plan.emit(command, "message.updated", &message, now)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn stop(
    projection: &ThreadProjection,
    command: &Command,
    plan: &mut Plan,
    run_id: &RunId,
    now: i64,
) -> Result<()> {
    let Some(run) = projection.runs.iter().find(|run| &run.id == run_id) else {
        return Ok(());
    };
    let mut value = serde_json::to_value(run)?;
    let next = value["delegatedCompletion"]["nextGeneration"]
        .as_i64()
        .unwrap_or(1);
    value["delegatedCompletion"] =
        json!({"disposition":"stopped","nextGeneration":next,"delivery":null});
    plan.emit(command, "run.updated", &value, now)?;
    for task in records(projection, "subagent") {
        if task["runId"] == run_id.0 && !matches!(state(task), "acknowledged" | "disposed") {
            let mut task = task.clone();
            task["completionDelivery"] = json!({"state":"disposed","observedByRunId":null});
            plan.emit(command, "subagent.updated", &task, now)?;
        }
    }
    for queued in projection
        .runs
        .iter()
        .filter(|run| run.status == OrchestrationV2RunStatus::Queued)
    {
        let mut value = serde_json::to_value(queued)?;
        value["queueHeld"] = json!(true);
        plan.emit(command, "run.updated", &value, now)?;
    }
    Ok(())
}
