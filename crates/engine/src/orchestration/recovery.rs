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
        if run.is_some_and(|run| run.status != OrchestrationV2RunStatus::Queued)
            && !attempt_terminal(&attempt.status)
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
        if run.is_some_and(|run| run.status != OrchestrationV2RunStatus::Queued)
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
    // Accepted no-op recovery has a receipted event too.
    if plan.events.is_empty() {
        plan.emit(command, "thread.metadata-updated", &projection.thread, now)?;
    }
    Ok(plan)
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
