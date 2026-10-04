use crate::orchestration::{
    Error, Result,
    command::{Command, Plan},
    effects::EffectRequest,
    event::iso,
    projection, task,
};
use rusqlite::Connection;
use serde_json::{Value, json};
use zeron_proto::orchestration::*;

#[derive(Debug, Clone)]
pub enum LaunchOperation {
    Create {
        thread: Box<OrchestrationV2AppThread>,
        workflow: Value,
        driver: String,
    },
    Send {
        message_id: String,
        text: String,
        attachments: Vec<Value>,
        queue: bool,
        driver: String,
    },
    Bind {
        expected: Option<String>,
        path: Option<String>,
        branch: Option<String>,
        continuation: Option<String>,
        driver: String,
        /// Binding, readiness hold and workflow checkpoint share one commit.
        workflow: Option<Value>,
    },
    Release,
    Fail {
        detail: String,
    },
    Delete,
}

fn refuse(message: &str) -> Error {
    Error::Invariant(message.into())
}

pub(crate) fn plan(
    conn: &Connection,
    command: &Command,
    op: &LaunchOperation,
    now: i64,
) -> Result<Plan> {
    let mut plan = Plan::default();
    let old = projection::read_thread(conn, &command.thread_id)?;
    if let LaunchOperation::Create {
        thread,
        workflow,
        driver,
    } = op
    {
        if old.is_some() {
            return Err(refuse("Thread already exists."));
        }
        plan.emit(command, "thread.created", thread, now)?;
        if let Some(message) = workflow.get("initialMessage") {
            send(
                conn,
                &mut plan,
                command,
                thread,
                1,
                &command.id.0,
                message["text"].as_str().unwrap_or(""),
                message["attachments"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
                "preparing",
                driver,
                now,
            )?;
            if let Some(sender) = message.get("senderThreadId") {
                if let Some(OrchestrationV2DomainEvent::MessageUpdated(event)) = plan
                    .events
                    .iter_mut()
                    .find(|e| matches!(e, OrchestrationV2DomainEvent::MessageUpdated(_)))
                {
                    event.payload.sender_thread_id =
                        Optional::Present(serde_json::from_value(sender.clone())?);
                }
            }
        }
        conn.execute(
            "INSERT INTO orchestration_launch_workflows VALUES(?1,?2)",
            rusqlite::params![command.thread_id.0, workflow.to_string()],
        )?;
        return Ok(plan);
    }
    let p = old.ok_or_else(|| refuse("The thread was not found."))?;
    if p.thread.deleted_at.is_some() {
        return Err(refuse("The thread was not found."));
    }
    let ordinal = p.runs.iter().map(|r| r.ordinal).max().unwrap_or(0) + 1;
    match op {
        LaunchOperation::Send {
            message_id,
            text,
            attachments,
            queue,
            driver,
        } => {
            if p.thread.archived_at.is_some() {
                return Err(refuse("Thread is archived."));
            }
            let status = if *queue
                || p.runs
                    .iter()
                    .any(|r| !crate::orchestration::command::run_terminal(&r.status))
            {
                "queued"
            } else {
                "starting"
            };
            send(
                conn,
                &mut plan,
                command,
                &p.thread,
                ordinal,
                message_id,
                text,
                attachments.clone(),
                status,
                driver,
                now,
            )?;
        }
        LaunchOperation::Bind {
            expected,
            path,
            branch,
            continuation,
            driver,
            workflow,
        } => {
            if p.thread.worktree_path != *expected {
                return Err(refuse("Workspace binding changed before handoff."));
            }
            if p.thread.archived_at.is_some() {
                return Err(refuse(
                    "Thread was archived while the worktree was being created.",
                ));
            }
            let mut thread = p.thread.clone();
            thread.worktree_path = path.clone();
            thread.branch = branch.clone();
            thread.updated_at = iso(now)?;
            plan.emit(command, "thread.metadata-updated", &thread, now)?;
            if let Some(text) = continuation {
                // This message and the binding commit BEFORE provider detach.
                send(
                    conn,
                    &mut plan,
                    command,
                    &thread,
                    ordinal,
                    &format!("message:{}:continuation", command.id.0),
                    text,
                    vec![],
                    "queued",
                    driver,
                    now,
                )?;
                if workflow.is_some() {
                    for event in &mut plan.events {
                        if let OrchestrationV2DomainEvent::RunCreated(event) = event {
                            event.payload.queue_held = Optional::Present(true);
                        }
                    }
                }
            }
            if let Some(workflow) = workflow {
                conn.execute(
                    "INSERT INTO orchestration_launch_workflows VALUES(?1,?2) ON CONFLICT(thread_id) DO UPDATE SET payload=excluded.payload",
                    rusqlite::params![command.thread_id.0, workflow.to_string()],
                )?;
            }
        }
        LaunchOperation::Release => {
            let mut thread = p.thread.clone();
            thread.updated_at = iso(now)?;
            plan.emit(command, "thread.metadata-updated", &thread, now)?;
            for mut run in p
                .runs
                .iter()
                .cloned()
                .filter(|r| r.status == OrchestrationV2RunStatus::Preparing)
            {
                run.status = OrchestrationV2RunStatus::Starting;
                plan.emit(command, "run.updated", &run, now)?;
                plan.effects
                    .push(EffectRequest::ProviderTurnStart { run_id: run.id });
            }
            let mut released = p.clone();
            for run in &mut released.runs {
                if run.status == OrchestrationV2RunStatus::Queued
                    && run.queue_held.as_ref() == Some(&true)
                {
                    run.queue_held = Optional::Present(false);
                    plan.emit(command, "run.updated", run, now)?;
                }
            }
            crate::orchestration::continuation::drain(&released, command, &mut plan, now)?;
            update_workflow(conn, &command.thread_id, "ready", None)?;
        }
        LaunchOperation::Fail { detail } => {
            let mut thread = p.thread.clone();
            thread.updated_at = iso(now)?;
            plan.emit(command, "thread.metadata-updated", &thread, now)?;
            for mut run in p
                .runs
                .into_iter()
                .filter(|r| r.status == OrchestrationV2RunStatus::Preparing)
            {
                run.status = OrchestrationV2RunStatus::Failed;
                run.completed_at = Some(iso(now)?);
                plan.emit(command, "run.updated", &run, now)?;
            }
            update_workflow(conn, &command.thread_id, "failed", Some(detail))?;
        }
        LaunchOperation::Delete => {
            return super::deletion::plan(&p, command, now);
        }
        LaunchOperation::Create { .. } => unreachable!(),
    }
    Ok(plan)
}

#[allow(clippy::too_many_arguments)]
fn send(
    conn: &Connection,
    plan: &mut Plan,
    command: &Command,
    thread: &OrchestrationV2AppThread,
    ordinal: i64,
    message_id: &str,
    text: &str,
    attachments: Vec<Value>,
    status: &str,
    driver: &str,
    now: i64,
) -> Result<()> {
    let mut seed = task::execution_seed(thread, ordinal, message_id, status, driver, now)?;
    let mut message = task::message(
        &thread.id,
        Some(&seed.run.id),
        Some(&seed.root.id),
        message_id,
        text,
        "user",
        now,
    )?;
    message["attachments"] = json!(attachments);
    plan.emit(command, "message.updated", &message, now)?;
    let previous = projection::read_records(conn, &thread.id.0, "provider-thread")?
        .into_iter()
        .find(|p| p["id"] == seed.provider_thread.id.0);
    if status == "queued" && previous.is_some() {
        // Queuing must not steal a live attempt's provider owner/ordinal.
        plan.emit(command, "run.created", &seed.run, now)?;
        plan.emit(command, "run-attempt.created", &seed.attempt, now)?;
        plan.emit(command, "node.updated", &seed.root, now)?;
        return Ok(());
    }
    if let Some(previous) = previous {
        let previous: OrchestrationV2ProviderThread = serde_json::from_value(previous)?;
        seed.provider_thread.native_thread_ref = previous.native_thread_ref;
        seed.provider_thread.native_conversation_head_ref = previous.native_conversation_head_ref;
        seed.provider_thread.provider_session_id = previous.provider_session_id;
    }
    task::emit_execution(plan, command, &thread.id, &seed, now)
}

fn update_workflow(
    conn: &Connection,
    thread: &ThreadId,
    status: &str,
    detail: Option<&str>,
) -> Result<()> {
    let raw: String = conn.query_row(
        "SELECT payload FROM orchestration_launch_workflows WHERE thread_id=?1",
        [&thread.0],
        |r| r.get(0),
    )?;
    let mut w: Value = serde_json::from_str(&raw)?;
    w["status"] = json!(status);
    w["stage"] = json!(if status == "ready" { "agent" } else { "failed" });
    w["error"] = json!(detail);
    conn.execute(
        "UPDATE orchestration_launch_workflows SET payload=?2 WHERE thread_id=?1",
        rusqlite::params![thread.0, w.to_string()],
    )?;
    Ok(())
}
