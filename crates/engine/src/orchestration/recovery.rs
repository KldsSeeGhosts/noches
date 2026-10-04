//! Explicit startup reconciliation. Call under the host's InstanceLock before
//! launching V2 workers. Legacy sessions remain isolated until runner adoption.

use rusqlite::Connection;
use zeron_proto::orchestration::*;

use super::command::{Command, Operation, Plan, attempt_terminal, node_terminal, run_terminal};
use super::event::iso;
use super::projection::{self, ThreadProjection};
use super::store::{Store, WriteBoundary, latest_sequence};
use super::{Kernel, Result};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RecoverySummary {
    pub projection_rebuilt: bool,
    pub retired_process_effects: usize,
    pub uncertain_effects: usize,
    pub requeued_effects: usize,
    pub reconciled_threads: usize,
}

pub(crate) fn plan(
    conn: &Connection,
    command: &Command,
    projection: &ThreadProjection,
    now: i64,
) -> Result<Plan> {
    let mut plan = Plan {
        cancel_process_effects: true,
        ..Plan::default()
    };
    let requests = projection::read_records(conn, &command.thread_id.0, "runtime-request")?;
    let message_nodes: Vec<_> = requests
        .iter()
        .filter(|request| {
            request["status"] == "pending" && request["responseCapability"]["type"] == "message"
        })
        .filter_map(|request| request["nodeId"].as_str())
        .collect();
    for run in &projection.runs {
        if run.status == OrchestrationV2RunStatus::Preparing {
            continue; // launch journal owns recovery; no provider has started
        }
        if run.status == OrchestrationV2RunStatus::Queued {
            if run.queue_held.as_ref() != Some(&true) {
                let mut next = run.clone();
                next.queue_held = Optional::Present(true);
                plan.emit(command, "run.updated", &next, now)?;
            }
        } else if !run_terminal(&run.status) {
            let mut next = run.clone();
            next.status = OrchestrationV2RunStatus::Cancelled;
            next.completed_at = Some(iso(now)?);
            next.queue_position = Optional::Present(None);
            plan.emit(command, "run.updated", &next, now)?;
        }
    }
    for attempt in &projection.attempts {
        // Queued/preparing intents retain execution identity; no live work to retire.
        let run = projection.runs.iter().find(|run| run.id == attempt.run_id);
        if run.is_some_and(|run| {
            !matches!(
                run.status,
                OrchestrationV2RunStatus::Queued | OrchestrationV2RunStatus::Preparing
            )
        }) && !attempt_terminal(&attempt.status)
        {
            let mut next = attempt.clone();
            next.status = OrchestrationV2RunAttemptStatus::Cancelled;
            next.completed_at = Some(iso(now)?);
            plan.emit(command, "run-attempt.updated", &next, now)?;
        }
    }
    for node in &projection.nodes {
        let run = projection
            .runs
            .iter()
            .find(|run| Some(&run.id) == node.run_id.as_ref());
        if (run.is_some_and(|run| {
            !matches!(
                run.status,
                OrchestrationV2RunStatus::Queued | OrchestrationV2RunStatus::Preparing
            )
        }) || (node.run_id.is_none()
            && node.kind == OrchestrationV2ExecutionNodeKind::RootTurn))
            && !node_terminal(&node.status)
            && !message_nodes.contains(&node.id.0.as_str())
        {
            let mut next = node.clone();
            next.status = OrchestrationV2ExecutionNodeStatus::Cancelled;
            next.completed_at = Some(iso(now)?);
            plan.emit(command, "node.updated", &next, now)?;
        }
    }
    for mut request in requests {
        if request["status"] == "pending" && request["responseCapability"]["type"] != "message" {
            request["status"] = serde_json::json!("expired");
            request["resolvedAt"] = serde_json::json!(iso(now)?);
            plan.emit(command, "runtime-request.updated", &request, now)?;
        }
    }
    for mut session in projection::read_records(conn, &command.thread_id.0, "provider-session")? {
        if !matches!(session["status"].as_str(), Some("stopped" | "error")) {
            session["status"] = serde_json::json!("stopped");
            session["updatedAt"] = serde_json::json!(iso(now)?);
            plan.emit(command, "provider-session.updated", &session, now)?;
        }
    }
    for mut turn in projection::read_records(conn, &command.thread_id.0, "provider-turn")? {
        if matches!(turn["status"].as_str(), Some("pending" | "running")) {
            turn["status"] = serde_json::json!("cancelled");
            turn["completedAt"] = serde_json::json!(iso(now)?);
            plan.emit(command, "provider-turn.updated", &turn, now)?;
        }
    }
    let mut cancelled_work = std::collections::BTreeMap::<String, Vec<serde_json::Value>>::new();
    for mut task in projection::read_records(conn, &command.thread_id.0, "subagent")? {
        if task["origin"] == "provider_native"
            && !super::task::terminal(task["status"].as_str().unwrap_or("running"))
        {
            if let Some(provider) = task["providerThreadId"].as_str() {
                let label = task["title"]
                    .as_str()
                    .or(task["prompt"].as_str())
                    .filter(|label| !label.trim().is_empty())
                    .unwrap_or("subagent");
                cancelled_work
                    .entry(provider.into())
                    .or_default()
                    .push(serde_json::json!({
                        "kind":"subagent","label":compact_label(label),"id":task["id"]
                    }));
            }
            task["status"] = serde_json::json!("cancelled");
            task["completedAt"] = serde_json::json!(iso(now)?);
            task["updatedAt"] = serde_json::json!(iso(now)?);
            plan.emit(command, "subagent.updated", &task, now)?;
        }
    }
    for mut provider in projection::read_records(conn, &command.thread_id.0, "provider-thread")? {
        let roster = provider["pendingBackgroundTasks"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for task in &roster {
            let id = task["taskId"].as_str().unwrap_or("unknown");
            let label = task["description"]
                .as_str()
                .filter(|label| !label.trim().is_empty())
                .map(|description| format!("{description} (id {id})"))
                .unwrap_or_else(|| id.into());
            cancelled_work.entry(provider["id"].as_str().unwrap().into()).or_default().push(serde_json::json!({
                "kind":match task["kind"].as_str() { Some("command") => "shell", Some("subagent") => "subagent", Some("monitor") => "monitor", _ => "task" },
                "label":compact_label(&label),"id":id
            }));
        }
        if !roster.is_empty() || provider["status"] == "active" {
            provider["pendingBackgroundTasks"] = serde_json::json!([]);
            if provider["status"] == "active" {
                provider["status"] = serde_json::json!("idle");
            }
            provider["updatedAt"] = serde_json::json!(iso(now)?);
            plan.emit(command, "provider-thread.updated", &provider, now)?;
        }
    }
    for (provider, work) in cancelled_work {
        if let Some(run) = projection
            .runs
            .iter()
            .filter(|run| {
                run.provider_thread_id
                    .as_ref()
                    .is_some_and(|id| id.0 == provider)
                    && run.started_at.is_some()
            })
            .max_by_key(|run| run.ordinal)
        {
            let mut merged = run
                .restart_cancelled_background_work
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?
                .unwrap_or_else(|| serde_json::json!([]));
            let merged = merged.as_array_mut().unwrap();
            for entry in work {
                if !merged
                    .iter()
                    .any(|old| old["kind"] == entry["kind"] && old["id"] == entry["id"])
                {
                    merged.push(entry);
                }
            }
            plan.emit(
                command,
                "run.background-work-cancelled",
                &serde_json::json!({
                    "runId":run.id,"restartCancelledBackgroundWork":merged
                }),
                now,
            )?;
        }
    }
    // Accepted no-op recovery has a receipted event too.
    if plan.events.is_empty() {
        plan.emit(command, "thread.metadata-updated", &projection.thread, now)?;
    }
    Ok(plan)
}

fn compact_label(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() > 160 {
        format!("{}…", text.chars().take(159).collect::<String>())
    } else {
        text
    }
}

impl Store {
    /// Metadata + decoding + missing creation rows are the inexpensive T3-style
    /// startup check. Explicit rebuild always replays the same reducer.
    pub fn verify_projections(&self) -> Result<bool> {
        self.read(|conn| {
            let metadata: (i64, i64) = conn.query_row(
                "SELECT schema_version,last_sequence FROM orchestration_projection_metadata WHERE singleton=1",
                [], |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if metadata != (projection::SCHEMA_VERSION, latest_sequence(conn)?) {
                return Ok(false);
            }
            let missing: bool = conn.query_row(
                "SELECT EXISTS(
                    SELECT stream_id FROM orchestration_events WHERE event_type='thread.created'
                    EXCEPT SELECT id FROM orchestration_projection_threads)",
                [], |row| row.get(0),
            )?;
            if missing { return Ok(false); }
            let unexpected: bool = conn.query_row(
                "SELECT EXISTS(
                    SELECT id FROM orchestration_projection_threads
                    EXCEPT SELECT stream_id FROM orchestration_events WHERE event_type='thread.created')",
                [], |row| row.get(0),
            )?;
            if unexpected { return Ok(false); }
            let mut stmt = conn.prepare("SELECT id FROM orchestration_projection_threads")?;
            for id in stmt.query_map([], |row| row.get::<_, String>(0))? {
                if projection::read_thread(conn, &ThreadId(id?)).is_err() { return Ok(false); }
            }
            let mut stmt = conn.prepare("SELECT kind,payload_json FROM orchestration_projection_records")?;
            for row in stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))? {
                let (kind, payload) = row?;
                macro_rules! decodable {
                    ($ty:ty) => { serde_json::from_str::<$ty>(&payload).is_ok() };
                }
                let valid = match kind.as_str() {
                    "provider-thread" => decodable!(OrchestrationV2ProviderThread),
                    "provider-session" => decodable!(OrchestrationV2ProviderSession),
                    "provider-turn" => decodable!(OrchestrationV2ProviderTurn),
                    "runtime-request" => decodable!(OrchestrationV2RuntimeRequest),
                    "message" => decodable!(OrchestrationV2ConversationMessage),
                    "subagent" => decodable!(OrchestrationV2Subagent),
                    "turn-item" => decodable!(OrchestrationV2TurnItem),
                    "context-transfer" => decodable!(OrchestrationV2ContextTransfer),
                    "context-handoff" => decodable!(OrchestrationV2ContextHandoff),
                    _ => false,
                };
                if !valid { return Ok(false); }
            }
            Ok(true)
        })
    }
}

impl Kernel {
    pub async fn recover(&self, now: i64) -> Result<RecoverySummary> {
        let mut summary = RecoverySummary::default();
        if !self.store.verify_projections()? {
            self.store.rebuild()?;
            summary.projection_rebuilt = true;
        }
        // The host epoch increments atomically with effect retirement. A crash
        // after this transaction leaves accepted source intent intact.
        let (epoch, retired, uncertain, requeued, cancelled_ids) = self.store.write(|tx| {
            let mut cancelled_ids = vec![];
            let mut stmt = tx.prepare(
                "SELECT effect_id FROM orchestration_effect_outbox WHERE process_bound=1 AND status IN ('pending','running')",
            )?;
            for id in stmt.query_map([], |row| row.get::<_, String>(0))? {
                cancelled_ids.push(id?);
            }
            let uncertain = tx.execute(
                "UPDATE orchestration_effect_outbox SET status='uncertain',lease_owner=NULL,
                 lease_expires_at=NULL,last_error='Process lost after dispatch began.'
                 WHERE status='running' AND process_bound=1 AND dispatch_started=1",
                [],
            )?;
            self.store.boundary(WriteBoundary::RecoveryEffects)?;
            let retired = tx.execute(
                "UPDATE orchestration_effect_outbox SET status='cancelled',lease_owner=NULL,
                 lease_expires_at=NULL,completed_at=?1,last_error='Process work retired on startup.'
                 WHERE process_bound=1 AND status IN ('pending','running')",
                [now],
            )?;
            self.store.boundary(WriteBoundary::RecoveryEffects)?;
            let requeued = tx.execute(
                "UPDATE orchestration_effect_outbox SET status='pending',lease_owner=NULL,
                 lease_expires_at=NULL,dispatch_started=0,available_at=?1,
                 last_error='Replay-safe work requeued on startup.'
                 WHERE status='running' AND process_bound=0",
                [now],
            )?;
            self.store.boundary(WriteBoundary::RecoveryEffects)?;
            tx.execute("UPDATE orchestration_host SET epoch=epoch+1 WHERE singleton=1", [])?;
            self.store.boundary(WriteBoundary::EpochAdvanced)?;
            let epoch: i64 = tx.query_row("SELECT epoch FROM orchestration_host WHERE singleton=1", [], |row| row.get(0))?;
            Ok((epoch, retired, uncertain, requeued, cancelled_ids))
        })?;
        self.store.cancellations.cancel(&cancelled_ids);
        summary.retired_process_effects = retired;
        summary.uncertain_effects = uncertain;
        summary.requeued_effects = requeued;
        let thread_ids = self.store.read(|conn| {
            // Recovery resumes only threads adopted into THIS SQL authority.
            let mut stmt =
                conn.prepare("SELECT id FROM orchestration_projection_threads ORDER BY id")?;
            Ok(stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?)
        })?;
        for id in thread_ids {
            let command = Command {
                id: CommandId(format!(
                    "command:runtime-reconcile:startup:{epoch}:{}",
                    super::event::encode_component(&id)
                )),
                thread_id: ThreadId(id),
                operation: Operation::Recover,
            };
            self.dispatch(&command, now).await?;
            summary.reconciled_threads += 1;
        }
        Ok(summary)
    }
}
