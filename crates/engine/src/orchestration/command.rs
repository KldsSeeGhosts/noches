//! Pure planning under the SQL authority lock. Policy/catalog/tool preconditions
//! belong to callers; receipts do not bypass those gates.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use zeron_proto::orchestration::*;

use super::effects::EffectRequest;
use super::event::{encode_component, iso, make};
use super::projection::{self, ThreadProjection};
use super::{Error, Result};

#[derive(Debug, Clone)]
pub struct Command {
    pub id: CommandId,
    pub thread_id: ThreadId,
    pub operation: Operation,
}

#[derive(Debug, Clone)]
pub enum Operation {
    Wire(Box<OrchestrationV2Command>),
    /// Primitive for later message/queue planners, not an MCP command. All rows
    /// commit together; ordinal and graph relationships are guarded here.
    CreateExecution(Box<ExecutionSeed>),
    ProviderEvents {
        guard: ProviderGuard,
        events: Vec<OrchestrationV2DomainEvent>,
    },
    ReplaceAttempt {
        guard: ProviderGuard,
        attempt: Box<OrchestrationV2RunAttempt>,
        root: Box<OrchestrationV2ExecutionNode>,
    },
    /// Adoption records do not grant task ownership or enqueue provider work.
    Adopt {
        legacy_chat_id: String,
        thread: Box<OrchestrationV2AppThread>,
    },
    /// Trusted host startup only. Does not schedule restart continuations yet.
    Recover,
    /// Trusted ordinary-session admission updates the next turn's binding.
    SessionBinding(Box<OrchestrationV2AppThread>),
    Task(Box<super::task::TaskOperation>),
}

#[derive(Debug, Clone)]
pub struct ExecutionSeed {
    pub run: OrchestrationV2Run,
    pub attempt: OrchestrationV2RunAttempt,
    pub root: OrchestrationV2ExecutionNode,
    pub provider_thread: OrchestrationV2ProviderThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderGuard {
    pub run_id: RunId,
    pub active_attempt_id: RunAttemptId,
    pub provider_thread_id: ProviderThreadId,
    pub provider_session_id: Option<ProviderSessionId>,
    pub provider_turn_id: Option<ProviderTurnId>,
    pub expected_last_run_ordinal: i64,
}

impl Command {
    pub fn wire(command: OrchestrationV2Command) -> Result<Self> {
        let value = serde_json::to_value(&command)?;
        Ok(Self {
            id: serde_json::from_value(value["commandId"].clone())?,
            thread_id: serde_json::from_value(value["threadId"].clone())?,
            operation: Operation::Wire(Box::new(command)),
        })
    }

    pub fn lock_threads(&self) -> Vec<ThreadId> {
        // Future transfer planners must include all target threads here. Provider
        // batches cannot cross threads, so callers cannot hide lock participants.
        let mut threads = vec![self.thread_id.clone()];
        if let Operation::Task(operation) = &self.operation {
            threads.extend(operation.lock_threads(&self.id));
        }
        threads
    }

    pub fn command_type(&self) -> Result<String> {
        Ok(match &self.operation {
            Operation::Wire(command) => serde_json::to_value(command)?["type"]
                .as_str()
                .expect("wire discriminator")
                .to_string(),
            Operation::CreateExecution(_) => "kernel.execution.create".into(),
            Operation::ProviderEvents { .. } => "kernel.provider.events".into(),
            Operation::ReplaceAttempt { .. } => "kernel.attempt.replace".into(),
            Operation::Adopt { .. } => "kernel.thread.adopt".into(),
            Operation::Recover => "kernel.runtime.recover".into(),
            Operation::SessionBinding(_) => "kernel.session.binding".into(),
            Operation::Task(operation) => operation.command_type().into(),
        })
    }
}

#[derive(Default)]
pub(crate) struct Plan {
    pub events: Vec<OrchestrationV2DomainEvent>,
    pub effects: Vec<EffectRequest>,
    pub cancel_process_effects: bool,
    pub adoption: Option<String>,
    pub routed_effects: Vec<(ThreadId, EffectRequest)>,
    pub cancel_threads: Vec<ThreadId>,
}

impl Plan {
    pub(crate) fn emit_on<T: Serialize>(
        &mut self,
        command: &Command,
        thread: &ThreadId,
        event_type: &str,
        payload: &T,
        now: i64,
    ) -> Result<()> {
        self.events.push(make(
            EventId(format!(
                "event:{}:{}",
                encode_component(&command.id.0),
                self.events.len()
            )),
            thread,
            event_type,
            payload,
            now,
        )?);
        Ok(())
    }
    pub(crate) fn emit<T: Serialize>(
        &mut self,
        command: &Command,
        event_type: &str,
        payload: &T,
        now: i64,
    ) -> Result<()> {
        self.events.push(make(
            EventId(format!(
                "event:{}:{}",
                encode_component(&command.id.0),
                self.events.len()
            )),
            &command.thread_id,
            event_type,
            payload,
            now,
        )?);
        Ok(())
    }
}

fn refuse(message: impl Into<String>) -> Error {
    Error::Invariant(message.into())
}

fn unused_entity(conn: &Connection, table: &str, id: &str) -> Result<()> {
    let exists: bool = conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
        [id],
        |row| row.get(0),
    )?;
    if exists {
        return Err(refuse("Execution identity is already in use."));
    }
    Ok(())
}

fn provider_owner(conn: &Connection, id: &str, thread: &ThreadId) -> Result<()> {
    let owner: Option<String> = conn.query_row(
        "SELECT thread_id FROM orchestration_projection_records WHERE kind='provider-thread' AND id=?1 LIMIT 1",
        [id], |row| row.get(0),
    ).optional()?;
    if owner.as_deref().is_some_and(|owner| owner != thread.0) {
        return Err(refuse("Provider thread is owned by another app thread."));
    }
    Ok(())
}

pub fn run_terminal(status: &OrchestrationV2RunStatus) -> bool {
    matches!(
        status,
        OrchestrationV2RunStatus::Completed
            | OrchestrationV2RunStatus::Interrupted
            | OrchestrationV2RunStatus::Failed
            | OrchestrationV2RunStatus::Cancelled
            | OrchestrationV2RunStatus::RolledBack
    )
}

pub fn attempt_terminal(status: &OrchestrationV2RunAttemptStatus) -> bool {
    !matches!(
        status,
        OrchestrationV2RunAttemptStatus::Pending | OrchestrationV2RunAttemptStatus::Running
    )
}

pub fn node_terminal(status: &OrchestrationV2ExecutionNodeStatus) -> bool {
    matches!(
        status,
        OrchestrationV2ExecutionNodeStatus::Completed
            | OrchestrationV2ExecutionNodeStatus::Interrupted
            | OrchestrationV2ExecutionNodeStatus::Failed
            | OrchestrationV2ExecutionNodeStatus::Cancelled
            | OrchestrationV2ExecutionNodeStatus::RolledBack
    )
}

fn run_transition(old: &OrchestrationV2RunStatus, next: &OrchestrationV2RunStatus) -> bool {
    use OrchestrationV2RunStatus::*;
    old == next
        || (!run_terminal(old) && run_terminal(next))
        || matches!(
            (old, next),
            (Preparing, Queued | Starting)
                | (Queued, Starting)
                | (Starting, Running | Waiting)
                | (Running, Waiting)
                | (Waiting, Running)
        )
}

fn node_transition(
    old: &OrchestrationV2ExecutionNodeStatus,
    next: &OrchestrationV2ExecutionNodeStatus,
) -> bool {
    use OrchestrationV2ExecutionNodeStatus::*;
    old == next
        || (!node_terminal(old) && node_terminal(next))
        || matches!(
            (old, next),
            (Idle, Pending | Running | Waiting)
                | (Pending, Running | Waiting)
                | (Running, Waiting)
                | (Waiting, Running)
        )
}

pub(crate) fn check_guard(
    conn: &Connection,
    projection: &ThreadProjection,
    guard: &ProviderGuard,
) -> Result<()> {
    let run = projection.runs.iter().find(|run| run.id == guard.run_id);
    let attempt = projection
        .attempts
        .iter()
        .find(|attempt| attempt.id == guard.active_attempt_id);
    let provider_threads =
        projection::read_records(conn, &projection.thread.id.0, "provider-thread")?;
    let provider = provider_threads
        .iter()
        .find(|provider| provider["id"] == guard.provider_thread_id.0);
    let valid = run
        .zip(attempt)
        .zip(provider)
        .is_some_and(|((run, attempt), provider)| {
            !run_terminal(&run.status)
                && !matches!(
                    run.status,
                    OrchestrationV2RunStatus::Preparing | OrchestrationV2RunStatus::Queued
                )
                && !attempt_terminal(&attempt.status)
                && run.active_attempt_id.as_ref() == Some(&guard.active_attempt_id)
                && run.provider_thread_id.as_ref() == Some(&guard.provider_thread_id)
                && attempt.run_id == run.id
                && attempt.provider_thread_id == guard.provider_thread_id
                && attempt.provider_turn_id == guard.provider_turn_id
                && provider["appThreadId"] == projection.thread.id.0
                && provider["providerInstanceId"] == run.provider_instance_id.0
                && provider["lastRunOrdinal"].as_i64() == Some(guard.expected_last_run_ordinal)
                && run.ordinal == guard.expected_last_run_ordinal
                && provider["providerSessionId"].as_str()
                    == guard.provider_session_id.as_ref().map(|id| id.0.as_str())
        });
    if !valid {
        return Err(refuse("Stale provider ownership."));
    }
    Ok(())
}

/// Validate every row in the proposed batch against the current owner and
/// transition, including earlier rows in that same batch.
fn provider_batch(
    projection: &ThreadProjection,
    guard: &ProviderGuard,
    events: &[OrchestrationV2DomainEvent],
) -> Result<()> {
    let mut state = projection.clone();
    for event in events {
        let value = serde_json::to_value(event)?;
        if value["threadId"] != projection.thread.id.0
            || value
                .get("runId")
                .is_some_and(|id| id != &serde_json::json!(guard.run_id))
        {
            return Err(refuse("Provider batch crosses its owner."));
        }
        match event {
            OrchestrationV2DomainEvent::RunUpdated(event) => {
                let next = &event.payload;
                let old = state
                    .runs
                    .iter_mut()
                    .find(|run| run.id == next.id)
                    .ok_or_else(|| refuse("Provider cannot create a run."))?;
                if next.id != guard.run_id
                    || next.thread_id != old.thread_id
                    || next.ordinal != old.ordinal
                    || next.active_attempt_id != old.active_attempt_id
                    || next.provider_thread_id != old.provider_thread_id
                    || next.provider_instance_id != old.provider_instance_id
                    || next.model_selection != old.model_selection
                    || next.root_node_id != old.root_node_id
                    || next.requested_at != old.requested_at
                    || next.user_message_id != old.user_message_id
                    || (!next.delegated_completion.is_absent()
                        && next.delegated_completion != old.delegated_completion)
                    || !run_transition(&old.status, &next.status)
                {
                    return Err(refuse("Invalid run transition or ownership."));
                }
                *old = next.clone();
            }
            OrchestrationV2DomainEvent::RunAttemptUpdated(event) => {
                let next = &event.payload;
                let old = state
                    .attempts
                    .iter_mut()
                    .find(|attempt| attempt.id == next.id)
                    .ok_or_else(|| refuse("Provider cannot create an attempt."))?;
                if next.id != guard.active_attempt_id
                    || next.run_id != old.run_id
                    || next.attempt_ordinal != old.attempt_ordinal
                    || next.root_node_id != old.root_node_id
                    || next.provider_thread_id != old.provider_thread_id
                    || next.provider_instance_id != old.provider_instance_id
                    || (old.provider_turn_id.is_some()
                        && old.provider_turn_id != next.provider_turn_id)
                    || (attempt_terminal(&old.status) && old.status != next.status)
                    || (old.status == OrchestrationV2RunAttemptStatus::Running
                        && next.status == OrchestrationV2RunAttemptStatus::Pending)
                {
                    return Err(refuse("Invalid attempt transition or ownership."));
                }
                *old = next.clone();
            }
            OrchestrationV2DomainEvent::NodeUpdated(event) => {
                let next = &event.payload;
                if next.run_id.as_ref() != Some(&guard.run_id)
                    || next.thread_id != state.thread.id
                    || next.provider_thread_id.as_ref() != Some(&guard.provider_thread_id)
                    || state
                        .runs
                        .iter()
                        .find(|run| run.id == guard.run_id)
                        .and_then(|run| run.root_node_id.as_ref())
                        != Some(&next.root_node_id)
                    || state
                        .attempts
                        .iter()
                        .find(|attempt| attempt.id == guard.active_attempt_id)
                        .and_then(|attempt| attempt.provider_turn_id.as_ref())
                        != next.provider_turn_id.as_ref()
                {
                    return Err(refuse("Invalid node ownership."));
                }
                if let Some(old) = state.nodes.iter_mut().find(|node| node.id == next.id) {
                    if old.root_node_id != next.root_node_id
                        || old.parent_node_id != next.parent_node_id
                        || old.kind != next.kind
                        || old.counts_for_run != next.counts_for_run
                        || !node_transition(&old.status, &next.status)
                    {
                        return Err(refuse("Invalid node transition."));
                    }
                    *old = next.clone();
                } else {
                    let root = state
                        .runs
                        .iter()
                        .find(|run| run.id == guard.run_id)
                        .and_then(|run| run.root_node_id.as_ref());
                    if Some(&next.root_node_id) != root
                        || !next
                            .parent_node_id
                            .as_ref()
                            .is_some_and(|id| state.nodes.iter().any(|node| &node.id == id))
                    {
                        return Err(refuse("Node graph is not rooted in this attempt."));
                    }
                    state.nodes.push(next.clone());
                }
            }
            // Provider/session binding changes are host commands in later runner
            // integration, not unguarded raw provider mutations.
            _ => return Err(refuse("Provider event outside kernel execution subset.")),
        }
    }
    if events.is_empty() {
        return Err(refuse("Provider batch is empty."));
    }
    Ok(())
}

pub(crate) fn plan(conn: &Connection, command: &Command, now: i64) -> Result<Plan> {
    if let Operation::Task(operation) = &command.operation {
        return super::task::plan(conn, command, operation, now);
    }
    let projection = projection::read_thread(conn, &command.thread_id)?;
    let mut plan = Plan::default();
    if let Operation::Wire(wire) = &command.operation
        && let OrchestrationV2Command::ThreadCreate(create) = wire.as_ref()
    {
        if projection.is_some() {
            return Err(refuse(format!(
                "Thread {} already exists.",
                command.thread_id.0
            )));
        }
        if create.command_id != command.id
            || create.thread_id != command.thread_id
            || create.title.trim().is_empty()
        {
            return Err(refuse("Invalid thread command identity or title."));
        }
        let thread: OrchestrationV2AppThread = serde_json::from_value(serde_json::json!({
            "id": create.thread_id, "projectId": create.project_id, "title": create.title,
            "createdBy": create.created_by, "creationSource": create.creation_source,
            "providerInstanceId": create.model_selection.instance_id, "modelSelection": create.model_selection,
            "runtimeMode": create.runtime_mode, "interactionMode": create.interaction_mode,
            "branch": create.branch, "worktreePath": create.worktree_path,
            "activeProviderThreadId": null,
            "lineage": {"parentThreadId":null,"relationshipToParent":null,"rootThreadId":create.thread_id},
            "forkedFrom":null,"createdAt":iso(now)?,"updatedAt":iso(now)?,
            "archivedAt":null,"settledOverride":null,"settledAt":null,
            "snoozedUntil":null,"snoozedAt":null,"lastVisitedAt":null,"deletedAt":null
        }))?;
        plan.emit(command, "thread.created", &thread, now)?;
        if let Some(imported) = create.imported_native_thread.as_ref() {
            let provider_id = ProviderThreadId(format!(
                "provider-thread:provider:{}:provider-instance:{}:native-thread:{}",
                encode_component(&imported.r#ref.driver.0),
                encode_component(&create.model_selection.instance_id.0),
                encode_component(&imported.r#ref.native_id),
            ));
            provider_owner(conn, &provider_id.0, &command.thread_id)?;
            let provider: OrchestrationV2ProviderThread = serde_json::from_value(
                serde_json::json!({
                    "id":provider_id,"driver":imported.r#ref.driver,
                    "providerInstanceId":create.model_selection.instance_id,"providerSessionId":null,
                    "appThreadId":create.thread_id,"ownerNodeId":null,"nativeThreadRef":imported.r#ref,
                    "nativeConversationHeadRef":null,"status":"not_loaded","firstRunOrdinal":null,
                    "lastRunOrdinal":null,"handoffIds":[],"forkedFrom":null,"contextUsage":null,
                    "nativeMetadata":imported.metadata.as_ref(),"createdAt":iso(now)?,"updatedAt":iso(now)?
                }),
            )?;
            plan.emit(command, "provider-thread.updated", &provider, now)?;
        }
        return Ok(plan);
    }
    if let Operation::Adopt {
        legacy_chat_id,
        thread,
    } = &command.operation
    {
        if projection.is_some() || thread.id != command.thread_id || legacy_chat_id.is_empty() {
            return Err(refuse("Invalid legacy adoption."));
        }
        if thread.lineage.parent_thread_id.is_some() {
            return Err(refuse("Legacy adoption cannot fabricate task lineage."));
        }
        let already_adopted: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM orchestration_adoptions WHERE legacy_chat_id=?1 OR thread_id=?2)",
            params![legacy_chat_id, command.thread_id.0], |row| row.get(0),
        )?;
        if already_adopted {
            return Err(refuse("Legacy chat is already adopted."));
        }
        plan.emit(command, "thread.created", thread, now)?;
        plan.adoption = Some(legacy_chat_id.clone());
        return Ok(plan);
    }
    let projection = projection
        .ok_or_else(|| refuse(format!("Thread {} was not found.", command.thread_id.0)))?;
    if projection.thread.deleted_at.is_some() && !matches!(command.operation, Operation::Recover) {
        return Err(refuse(format!(
            "Thread {} is deleted.",
            command.thread_id.0
        )));
    }
    match &command.operation {
        Operation::Wire(wire) => {
            let value = serde_json::to_value(wire)?;
            if value["commandId"] != command.id.0 || value["threadId"] != command.thread_id.0 {
                return Err(refuse("Wire and envelope identities differ."));
            }
            let mut thread = projection.thread.clone();
            let event_type = match wire.as_ref() {
                OrchestrationV2Command::ThreadRuntimeModeSet(set) => {
                    thread.runtime_mode = set.runtime_mode.clone();
                    "thread.runtime-mode-updated"
                }
                OrchestrationV2Command::ThreadInteractionModeSet(set) => {
                    thread.interaction_mode = set.interaction_mode.clone();
                    "thread.interaction-mode-updated"
                }
                OrchestrationV2Command::ThreadModelSelectionSet(set) => {
                    let switched = thread.provider_instance_id != set.model_selection.instance_id;
                    thread.provider_instance_id = set.model_selection.instance_id.clone();
                    thread.model_selection = set.model_selection.clone();
                    if switched {
                        "thread.provider-switched"
                    } else {
                        "thread.model-selection-updated"
                    }
                }
                OrchestrationV2Command::ThreadUnarchive(_) => {
                    thread.archived_at = None;
                    "thread.unarchived"
                }
                // Archive/delete need cohort/task/runtime policy (P3/P4). Do
                // not silently implement a weaker version in the kernel.
                _ => {
                    return Err(refuse(format!(
                        "Command outside kernel slice: {}.",
                        command.command_type()?
                    )));
                }
            };
            thread.updated_at = iso(now)?;
            plan.emit(command, event_type, &thread, now)?;
        }
        Operation::CreateExecution(seed) => {
            if projection.thread.archived_at.is_some() {
                return Err(refuse("Thread is archived."));
            }
            let next_ordinal = projection
                .runs
                .iter()
                .map(|run| run.ordinal)
                .max()
                .unwrap_or(0)
                + 1;
            let run = &seed.run;
            let attempt = &seed.attempt;
            let root = &seed.root;
            let provider = &seed.provider_thread;
            unused_entity(conn, projection::TABLES[1], &run.id.0)?;
            unused_entity(conn, projection::TABLES[2], &attempt.id.0)?;
            unused_entity(conn, projection::TABLES[3], &root.id.0)?;
            provider_owner(conn, &provider.id.0, &command.thread_id)?;
            if run.thread_id != command.thread_id
                || run.ordinal != next_ordinal
                || run.active_attempt_id.as_ref() != Some(&attempt.id)
                || run.root_node_id.as_ref() != Some(&root.id)
                || run.provider_thread_id.as_ref() != Some(&provider.id)
                || run.provider_instance_id != attempt.provider_instance_id
                || run.provider_instance_id != provider.provider_instance_id
                || attempt.run_id != run.id
                || attempt.attempt_ordinal != 1
                || attempt.provider_thread_id != provider.id
                || attempt.root_node_id != root.id
                || root.thread_id != command.thread_id
                || root.run_id.as_ref() != Some(&run.id)
                || root.root_node_id != root.id
                || root.parent_node_id.is_some()
                || root.provider_thread_id.as_ref() != Some(&provider.id)
                || root.kind != OrchestrationV2ExecutionNodeKind::RootTurn
                || !root.counts_for_run
                || provider.app_thread_id.as_ref() != Some(&command.thread_id)
                || provider.last_run_ordinal != Some(run.ordinal)
                || !matches!(
                    run.status,
                    OrchestrationV2RunStatus::Preparing
                        | OrchestrationV2RunStatus::Queued
                        | OrchestrationV2RunStatus::Starting
                )
                || attempt.status != OrchestrationV2RunAttemptStatus::Pending
                || root.status != OrchestrationV2ExecutionNodeStatus::Pending
                || projection.runs.iter().any(|old| old.id == run.id)
                || projection.attempts.iter().any(|old| old.id == attempt.id)
                || projection.nodes.iter().any(|old| old.id == root.id)
            {
                return Err(refuse("Invalid execution graph, ordinal or initial state."));
            }
            if run.status == OrchestrationV2RunStatus::Starting
                && projection.runs.iter().any(|run| {
                    !run_terminal(&run.status) && run.status != OrchestrationV2RunStatus::Queued
                })
            {
                return Err(refuse("Thread already has blocking work."));
            }
            plan.emit(command, "provider-thread.updated", provider, now)?;
            plan.emit(command, "run.created", run, now)?;
            plan.emit(command, "run-attempt.created", attempt, now)?;
            plan.emit(command, "node.updated", root, now)?;
            if run.status == OrchestrationV2RunStatus::Starting {
                plan.effects.push(EffectRequest::ProviderTurnStart {
                    run_id: run.id.clone(),
                });
            }
        }
        Operation::ProviderEvents { guard, events } => {
            check_guard(conn, &projection, guard)?;
            for event in events {
                if let OrchestrationV2DomainEvent::NodeUpdated(event) = event {
                    let owner: Option<String> = conn
                        .query_row(
                            "SELECT thread_id FROM orchestration_projection_nodes WHERE id=?1",
                            [&event.payload.id.0],
                            |row| row.get(0),
                        )
                        .optional()?;
                    if owner
                        .as_deref()
                        .is_some_and(|owner| owner != command.thread_id.0)
                    {
                        return Err(refuse("Node identity belongs to another app thread."));
                    }
                }
            }
            provider_batch(&projection, guard, events)?;
            plan.events = events.clone();
        }
        Operation::ReplaceAttempt {
            guard,
            attempt,
            root,
        } => {
            check_guard(conn, &projection, guard)?;
            unused_entity(conn, projection::TABLES[2], &attempt.id.0)?;
            unused_entity(conn, projection::TABLES[3], &root.id.0)?;
            let mut run = projection
                .runs
                .iter()
                .find(|run| run.id == guard.run_id)
                .unwrap()
                .clone();
            let mut old = projection
                .attempts
                .iter()
                .find(|attempt| attempt.id == guard.active_attempt_id)
                .unwrap()
                .clone();
            if attempt.run_id != run.id
                || attempt.attempt_ordinal != old.attempt_ordinal + 1
                || attempt.id == old.id
                || attempt.status != OrchestrationV2RunAttemptStatus::Pending
                || attempt.provider_thread_id != guard.provider_thread_id
                || attempt.provider_instance_id != run.provider_instance_id
                || attempt.root_node_id != root.id
                || root.thread_id != command.thread_id
                || root.run_id.as_ref() != Some(&run.id)
                || root.root_node_id != root.id
                || root.parent_node_id.is_some()
                || root.kind != OrchestrationV2ExecutionNodeKind::RootTurn
                || root.status != OrchestrationV2ExecutionNodeStatus::Pending
                || root.provider_thread_id.as_ref() != Some(&guard.provider_thread_id)
                || projection.attempts.iter().any(|old| old.id == attempt.id)
                || projection.nodes.iter().any(|old| old.id == root.id)
            {
                return Err(refuse("Invalid replacement attempt."));
            }
            old.status = OrchestrationV2RunAttemptStatus::Superseded;
            old.completed_at = Some(iso(now)?);
            if let Some(mut old_root) = projection
                .nodes
                .iter()
                .find(|node| node.id == old.root_node_id)
                .cloned()
            {
                old_root.status = OrchestrationV2ExecutionNodeStatus::Interrupted;
                old_root.completed_at = Some(iso(now)?);
                plan.emit(command, "node.updated", &old_root, now)?;
            }
            run.active_attempt_id = Some(attempt.id.clone());
            run.root_node_id = Some(root.id.clone());
            run.status = OrchestrationV2RunStatus::Starting;
            plan.emit(command, "run-attempt.updated", &old, now)?;
            plan.emit(command, "run-attempt.created", attempt, now)?;
            plan.emit(command, "node.updated", root, now)?;
            plan.emit(command, "run.updated", &run, now)?;
            plan.cancel_process_effects = true;
            plan.effects
                .push(EffectRequest::ProviderTurnStart { run_id: run.id });
        }
        Operation::Recover => return super::recovery::plan(conn, command, &projection, now),
        Operation::SessionBinding(thread) => {
            if thread.id != projection.thread.id || thread.lineage != projection.thread.lineage {
                return Err(refuse("Session binding cannot change ownership."));
            }
            plan.emit(command, "thread.metadata-updated", thread, now)?;
        }
        Operation::Adopt { .. } => unreachable!(),
        Operation::Task(_) => unreachable!("routed before the kernel subset"),
    }
    Ok(plan)
}
