//! Whole-thread Stop: one durable user request that stops this thread's own
//! work and the app-owned delegated tasks beneath it, and nothing else.
//!
//! The target set is frozen under the request identity, so a retry after an
//! uncertain response repeats exactly the first request and never reaches work
//! started later. Each step is an ordinary kernel command with a derived
//! stable identity and its own exact-run fence: a child `Cancel` pins the
//! child's active run and freezes its physical process in `controls::admit`,
//! an `Interrupt` pins the thread's interruptible run. A target that settled,
//! was replaced or changed owner before its step ran is skipped, never retargeted.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use zeron_proto::orchestration::*;
use zeron_proto::transfer::StopThreadWorkResult;

use super::{
    Command, Error, Kernel, Operation, ReceiptStatus, Result,
    event::encode_component,
    projection::{ThreadProjection, read_thread},
    queue::{SESSION_USER_REQUESTS, reserve_request},
    task::{self, TaskOperation, records},
    threads::planner::ThreadOperation,
};

/// Bounds the walk; a deeper or wider delegation tree is a runaway, not a plan.
const MAX_DEPTH: usize = 8;
const MAX_TARGETS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Step {
    /// The owner's `task_cancel` of one app-owned child under its exact run.
    Cancel {
        parent: ThreadId,
        task_id: NodeId,
        child: ThreadId,
        run_id: RunId,
    },
    /// A thread's own foreground run, or completed-root native background work.
    Interrupt { thread: ThreadId, run_id: RunId },
}

impl Step {
    fn thread(&self) -> &ThreadId {
        match self {
            Self::Cancel { parent, .. } => parent,
            Self::Interrupt { thread, .. } => thread,
        }
    }

    fn operation(&self) -> Operation {
        match self {
            Self::Cancel {
                task_id,
                child,
                run_id,
                ..
            } => Operation::Task(Box::new(TaskOperation::Cancel {
                task_id: task_id.clone(),
                child_thread_id: child.clone(),
                run_id: run_id.clone(),
            })),
            Self::Interrupt { run_id, .. } => {
                Operation::Thread(Box::new(ThreadOperation::Interrupt {
                    run_id: run_id.clone(),
                    reason: None,
                }))
            }
        }
    }
}

/// Descendants first, so a child's completion mail is disposed in its parent
/// before that parent's own run is interrupted; the thread's own work is last.
fn collect(
    read: &mut dyn FnMut(&ThreadId) -> Result<Option<ThreadProjection>>,
    root: &ThreadId,
) -> Result<Vec<Step>> {
    let mut steps = vec![];
    let mut seen = BTreeSet::new();
    visit(read, root, 0, &mut seen, &mut steps)?;
    if let Some(p) = read(root)?
        && let Some(run) = super::background::interruptible_run(&p)
    {
        steps.push(Step::Interrupt {
            thread: root.clone(),
            run_id: run.id.clone(),
        });
    }
    Ok(steps)
}

fn visit(
    read: &mut dyn FnMut(&ThreadId) -> Result<Option<ThreadProjection>>,
    thread: &ThreadId,
    depth: usize,
    seen: &mut BTreeSet<ThreadId>,
    steps: &mut Vec<Step>,
) -> Result<()> {
    if depth > MAX_DEPTH || !seen.insert(thread.clone()) {
        return Ok(());
    }
    let Some(p) = read(thread)? else {
        return Ok(());
    };
    for task in records(&p, "subagent") {
        // Ownership is the parent's own task record (`find_task`'s rule), never
        // a lineage guess: forks, native children and other threads' tasks
        // never appear here.
        let (Some(id), Some(child)) = (task["id"].as_str(), task["childThreadId"].as_str()) else {
            continue;
        };
        if task["origin"] != "app_owned"
            || task["threadId"] != thread.0
            || !task["result"].is_null()
            || task::terminal(task["status"].as_str().unwrap_or("running"))
        {
            continue;
        }
        let child = ThreadId(child.into());
        visit(read, &child, depth + 1, seen, steps)?;
        let Some(child_projection) = read(&child)? else {
            continue;
        };
        if steps.len() >= MAX_TARGETS {
            return Err(Error::Invariant(
                "Too many delegated tasks to stop in one request.".into(),
            ));
        }
        if let Some(run) = task::active_run(&child_projection) {
            steps.push(Step::Cancel {
                parent: thread.clone(),
                task_id: NodeId(id.into()),
                child,
                run_id: run.id.clone(),
            });
        } else if let Some(run) = super::background::settled_run(&child_projection) {
            // The child's reply is complete; only its native background work
            // stops, and the completed result still reaches its owner.
            steps.push(Step::Interrupt {
                thread: child,
                run_id: run.id.clone(),
            });
        }
    }
    Ok(())
}

fn request_key(thread: &ThreadId, request: &str) -> String {
    format!(
        "ui:stop-all:{}:{}",
        encode_component(&thread.0),
        encode_component(request)
    )
}

pub(crate) fn step_id(thread: &ThreadId, request: &str, index: usize) -> CommandId {
    CommandId(format!(
        "command:user-stop-all:{}:{}:{index}",
        encode_component(&thread.0),
        encode_component(request)
    ))
}

/// The frozen plan: computed once inside the reservation transaction, then
/// replayed verbatim.
fn frozen_plan(kernel: &Kernel, thread: &ThreadId, request: &str) -> Result<Vec<Step>> {
    let key = request_key(thread, request);
    kernel.store.write(|conn| {
        use rusqlite::OptionalExtension;
        let stored: Option<String> = conn
            .query_row(
                &format!("SELECT payload_json FROM {SESSION_USER_REQUESTS} WHERE command_id=?1"),
                [&key],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(stored) = stored {
            return Ok(serde_json::from_str(&stored)?);
        }
        let p = read_thread(conn, thread)?
            .filter(|p| p.thread.deleted_at.is_none())
            .ok_or_else(|| Error::Invariant("The thread was not found.".into()))?;
        if p.thread.archived_at.is_some() {
            return Err(Error::Invariant("Thread is not interruptible.".into()));
        }
        if serde_json::to_value(&p.thread.lineage)?["relationshipToParent"] == "subagent" {
            return Err(Error::Invariant(
                "Stop a delegated task from the thread that owns it.".into(),
            ));
        }
        let steps = collect(&mut |id| read_thread(conn, id), thread)?;
        let payload = serde_json::to_string(&steps)?;
        if !reserve_request(conn, SESSION_USER_REQUESTS, &key, &payload)? {
            return Err(Error::Invariant(
                "This stop request already belongs to different work.".into(),
            ));
        }
        Ok(steps)
    })
}

pub(crate) async fn stop_thread_work(
    kernel: &Kernel,
    thread: &ThreadId,
    request: &str,
) -> Result<StopThreadWorkResult> {
    let steps = frozen_plan(kernel, thread, request)?;
    let mut result = StopThreadWorkResult::default();
    for (index, step) in steps.iter().enumerate() {
        let receipt = kernel
            .dispatch(
                &Command {
                    id: step_id(thread, request, index),
                    thread_id: step.thread().clone(),
                    operation: step.operation(),
                },
                crate::now_ms(),
            )
            .await?;
        result.sequence = result.sequence.max(receipt.result_sequence);
        if receipt.status == ReceiptStatus::Rejected {
            tracing::info!(
                thread = %step.thread().0, error = ?receipt.error,
                "whole-thread stop skipped a target that changed"
            );
            result.skipped += 1;
        } else {
            result.stopped_runs += 1;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_keys_are_stable_and_component_encoded() {
        let thread = ThreadId("a:b".into());
        assert_eq!(request_key(&thread, "r"), request_key(&thread, "r"));
        assert_ne!(request_key(&thread, "r"), request_key(&thread, "s"));
        assert_ne!(step_id(&thread, "r", 0), step_id(&thread, "r", 1));
    }
}
