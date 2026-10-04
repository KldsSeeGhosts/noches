//! ScheduledTaskService's host dispatch path, not a provider/MCP caller.
use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::{OptionalExtension, params};
use zeron_proto::orchestration_mcp::T3ThreadSendInputMode;

use super::{ScheduledDispatch, ScheduledTaskDispatch, find};
use crate::orchestration::Store;
use crate::orchestration::thread_service::{ThreadSendRequest, ThreadService};

pub struct ThreadDispatch {
    pub store: Store,
    pub threads: Arc<dyn ThreadService>,
}

#[async_trait]
impl ScheduledTaskDispatch for ThreadDispatch {
    async fn dispatch(&self, run: ScheduledDispatch) -> Result<(), String> {
        // Re-read at the last intake boundary: a claim snapshot is not
        // authority to dispatch a deleted, paused, or replaced definition.
        let task = self
            .store
            .read(|conn| {
                let task = find(conn, &run.task.id)?;
                let owns_claim = conn
                    .query_row(
                        "SELECT active_claim_id=?2 AND last_run_status='running'
                 FROM orchestration_scheduled_tasks WHERE task_id=?1",
                        params![run.task.id.0, run.claim_id],
                        |row| row.get::<_, bool>(0),
                    )
                    .optional()?
                    .unwrap_or(false);
                Ok(task.filter(|t| owns_claim && (run.trigger == "manual" || t.enabled)))
            })
            .map_err(|_| "The operation could not be completed.".to_owned())?
            .ok_or_else(|| "Schedule task not found.".to_owned())?;
        let Some(thread_id) = task.thread_id else {
            // TODO(merge-launch): call launch with the stable command/message
            // IDs, task title/selection/modes/provenance, scheduledTaskId and
            // stored workspaceStrategy. ThreadService::create is MCP-scoped
            // and inherits the caller checkout; it is not a top-level launch.
            return Err("The operation could not be completed.".into());
        };
        self.threads
            .send_to_thread(ThreadSendRequest {
                project_id: task.project_id,
                thread_id,
                command_id: run.command_id,
                message_id: run.message_id,
                scheduled_task_id: Some(task.id),
                sender_thread_id: None,
                text: task.prompt,
                attachments: vec![],
                model_selection: Some(task.model_selection),
                mode: T3ThreadSendInputMode::Auto,
                created_by: task.created_by,
                creation_source: task.creation_source,
            })
            .await
            .map(|_| ())
            .map_err(|e| e.message)
    }
}
