//! Native background work is owned by a provider process, not an app task.
//! The waiting selector and Stop use the same bounded ownership rules.
use serde_json::{Value, json};
use zeron_proto::orchestration::*;

use super::{
    Result,
    command::{Command, Plan, run_terminal},
    event::{encode_component, iso},
    projection::ThreadProjection,
    task::{active_run, records},
};

fn within_run(p: &ThreadProjection, value: &Value, through: i64) -> bool {
    value["runId"].as_str().is_none_or(|id| {
        p.runs.iter().find(|r| r.id.0 == id).is_none_or(|r| {
            r.ordinal <= through && r.status != OrchestrationV2RunStatus::RolledBack
        })
    })
}

fn active(value: &Value) -> bool {
    matches!(
        value["status"].as_str(),
        Some("pending" | "running" | "waiting")
    )
}

pub(crate) fn pending_item(p: &ThreadProjection, item: &Value, through: i64) -> bool {
    matches!(
        item["type"].as_str(),
        Some("command_execution" | "dynamic_tool" | "subagent")
    ) && active(item)
        && within_run(p, item, through)
        // Persistent monitors and independently owned delegated tasks are not
        // native background processes. They retain their own control authority.
        && !(item["type"] == "dynamic_tool" && item["input"]["persistent"] == true)
        && item["origin"] != "app_owned"
        && !records(p, "subagent").iter().any(|task| {
            task["origin"] == "app_owned" && task["id"] == item["subagentId"]
        })
}

fn pending_native(p: &ThreadProjection, task: &Value, through: i64) -> bool {
    task["origin"] == "provider_native" && active(task) && within_run(p, task, through)
}

fn provider_within(provider: &Value, through: i64) -> bool {
    provider["lastRunOrdinal"]
        .as_i64()
        .is_some_and(|ordinal| ordinal <= through)
}

pub(crate) fn has_work(p: &ThreadProjection, through: i64) -> bool {
    records(p, "turn-item")
        .iter()
        .any(|item| pending_item(p, item, through))
        || records(p, "subagent")
            .iter()
            .any(|task| pending_native(p, task, through))
        || records(p, "provider-thread").iter().any(|provider| {
            provider_within(provider, through)
                && provider["pendingBackgroundTasks"]
                    .as_array()
                    .is_some_and(|tasks| !tasks.is_empty())
        })
}

/// Queued work cannot steal Stop's target. Rolled-back runs are abandoned,
/// not background waiting. A newer foreground turn always owns its own Stop.
pub(crate) fn settled_run(p: &ThreadProjection) -> Option<&OrchestrationV2Run> {
    if active_run(p).is_some() {
        return None;
    }
    p.runs
        .iter()
        .filter(|run| run.status != OrchestrationV2RunStatus::Queued)
        .max_by_key(|run| run.ordinal)
        .filter(|run| {
            run_terminal(&run.status)
                && run.status != OrchestrationV2RunStatus::RolledBack
                && has_work(p, run.ordinal)
        })
}

pub(crate) fn interruptible_run(p: &ThreadProjection) -> Option<&OrchestrationV2Run> {
    active_run(p).or_else(|| settled_run(p))
}

/// Called only after the exact admitted process has retired. Noches permits
/// one physical provider runtime per app thread; the runtime map is held empty
/// across this transaction. Old provider rosters can therefore be repaired,
/// but newer runs, persistent monitors and app-owned child tasks cannot.
pub(crate) fn settle(
    p: &ThreadProjection,
    command: &Command,
    plan: &mut Plan,
    run: &OrchestrationV2Run,
    effect_id: &str,
    now: i64,
) -> Result<()> {
    if active_run(p).is_some_and(|active| active.id != run.id) {
        return Ok(());
    }
    if !has_work(p, run.ordinal) && run.status != OrchestrationV2RunStatus::Completed {
        return Ok(());
    }
    let time = iso(now)?;
    for (kind, values) in [
        ("turn-item", records(p, "turn-item")),
        ("subagent", records(p, "subagent")),
    ] {
        for value in values.iter().filter(|value| {
            if kind == "turn-item" {
                pending_item(p, value, run.ordinal)
            } else {
                pending_native(p, value, run.ordinal)
            }
        }) {
            let mut next = value.clone();
            next["status"] = json!("interrupted");
            next["completedAt"] = json!(time);
            next["updatedAt"] = json!(time);
            plan.emit(command, &format!("{kind}.updated"), &next, now)?;
        }
    }
    for provider in records(p, "provider-thread").iter().filter(|provider| {
        provider_within(provider, run.ordinal)
            && provider["pendingBackgroundTasks"]
                .as_array()
                .is_some_and(|tasks| !tasks.is_empty())
    }) {
        let mut next = provider.clone();
        next["pendingBackgroundTasks"] = json!([]);
        if next["status"] == "active" {
            next["status"] = json!("idle");
        }
        next["updatedAt"] = json!(time);
        plan.emit(command, "provider-thread.updated", &next, now)?;
    }
    // Preserve the completed root/attempt/reply. This separate result also
    // prevents delayed native child observations from reopening stopped work.
    let item = json!({
        "id":format!("turn-item:background-stop:{}",encode_component(effect_id)),
        "type":"run_interrupt_result","threadId":p.thread.id,"runId":run.id,"nodeId":run.root_node_id,
        "providerThreadId":run.provider_thread_id,"providerTurnId":null,"nativeItemRef":null,
        "parentItemId":format!("turn-item:{}:interrupt-request",encode_component(&run.id.0)),
        "ordinal":super::threads::planner::next_item_ordinal(p,plan)?,
        "status":"interrupted","title":"Background work stopped","message":"Native background work stopped",
        "startedAt":time,"completedAt":time,"updatedAt":time
    });
    plan.emit(command, "turn-item.updated", &item, now)
}

pub(crate) fn was_stopped(p: &ThreadProjection, run: &OrchestrationV2Run) -> bool {
    records(p, "turn-item").iter().any(|item| {
        item["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("turn-item:background-stop:"))
            && p.runs
                .iter()
                .any(|stopped| item["runId"] == stopped.id.0 && stopped.ordinal >= run.ordinal)
    })
}
