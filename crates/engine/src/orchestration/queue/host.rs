//! Loro admission/lease bridge. The owner drain lane spans SQL acceptance and
//! durable intent application; a SQL outbox repairs a crash between them.
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use zeron_proto::orchestration::{CommandId, ThreadId};

use super::{QueueDomain, unavailable};
use crate::orchestration::queue_service::QueueService;
use crate::orchestration::service::{CallerScope, ToolError};
use crate::{DocHost, HarnessRegistry};

pub struct HostQueue {
    pub domain: Arc<QueueDomain>,
    pub docs: DocHost,
    pub registry: Arc<HarnessRegistry>,
}

impl HostQueue {
    /// The runtime can be briefly idle between admitted restart attempts.
    /// That gap does not authorize draining another document-owned input.
    pub(crate) fn has_starting_turn(&self, chat: &str) -> super::super::Result<bool> {
        self.domain.kernel.store.read(|conn| {
            Ok(conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM orchestration_projection_runs
                 WHERE thread_id=?1 AND status='starting')",
                [chat],
                |row| row.get(0),
            )?)
        })
    }

    pub(crate) async fn prepare_locked(
        &self,
        handle: &Arc<crate::ChatDocHandle>,
    ) -> Result<(), ToolError> {
        let thread = ThreadId(handle.chat_id().into());
        self.apply_patches(&thread, handle)?;
        if let Some(p) = self
            .domain
            .kernel
            .store
            .thread(&thread)
            .map_err(|_| unavailable())?
        {
            if p.thread.deleted_at.is_some() || p.thread.archived_at.is_some() {
                return Err(unavailable());
            }
            self.synchronize(&p, handle).await?;
        }
        Ok(())
    }
    pub async fn repair(&self, thread: &ThreadId) -> Result<(), ToolError> {
        let handle = self.docs.open(&thread.0).map_err(|_| unavailable())?;
        let _guard = handle.orchestration_queue_lock().await;
        self.apply_patches(thread, &handle)?;
        Ok(())
    }

    pub(crate) fn apply_patches(
        &self,
        thread: &ThreadId,
        handle: &Arc<crate::ChatDocHandle>,
    ) -> Result<(), ToolError> {
        let patches=self.domain.kernel.store.read(|conn|{
            let mut statement=conn.prepare("SELECT command_id,payload_json FROM orchestration_queue_patches WHERE thread_id=?1 ORDER BY sequence")?;
            let rows=statement.query_map([&thread.0],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?
                .collect::<std::result::Result<Vec<_>,_>>()?;
            Ok(rows)
        }).map_err(|_|unavailable())?;
        for (id, raw) in patches {
            let patch: Value = serde_json::from_str(&raw).map_err(|_| unavailable())?;
            let message_id = patch["messageId"].as_str().unwrap_or("");
            match patch["action"].as_str() {
                Some("edit") => {
                    handle
                        .doc()
                        .set_queued_text(
                            message_id,
                            patch["text"].as_str().unwrap_or(""),
                            crate::now_ms(),
                        )
                        .map_err(|_| unavailable())?;
                }
                Some("cancel") => {
                    handle
                        .doc()
                        .remove_queued(message_id)
                        .map_err(|_| unavailable())?;
                }
                Some("cancel_many") => {
                    for id in patch["messageIds"].as_array().into_iter().flatten() {
                        handle
                            .doc()
                            .remove_queued(id.as_str().unwrap_or(""))
                            .map_err(|_| unavailable())?;
                    }
                }
                Some("reorder") => {
                    let existing = handle.doc().read_queue().map_err(|_| unavailable())?;
                    for (index, id) in patch["messageIds"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|id| {
                            existing
                                .iter()
                                .any(|row| Some(row.id.as_str()) == id.as_str())
                        })
                        .enumerate()
                    {
                        handle
                            .doc()
                            .move_queued(id.as_str().unwrap_or(""), index)
                            .map_err(|_| unavailable())?;
                    }
                }
                _ => return Err(unavailable()),
            }
            self.docs
                .persist_orchestration_queue(handle)
                .map_err(|_| unavailable())?;
            self.domain
                .kernel
                .store
                .write(|conn| {
                    conn.execute(
                        "DELETE FROM orchestration_queue_patches WHERE command_id=?1",
                        [id],
                    )?;
                    Ok(())
                })
                .map_err(|_| unavailable())?;
        }
        Ok(())
    }

    pub async fn repair_all(&self) -> Result<(), ToolError> {
        let threads = self
            .domain
            .kernel
            .store
            .read(|conn| {
                let mut statement =
                    conn.prepare("SELECT DISTINCT thread_id FROM orchestration_queue_patches")?;
                let ids = statement
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(ids)
            })
            .map_err(|_| unavailable())?;
        for id in threads {
            if self.docs.is_host(&id) {
                self.repair(&ThreadId(id)).await?;
            }
        }
        for handle in self.docs.orchestration_queue_handles() {
            if !self.docs.is_host(handle.chat_id()) {
                continue;
            }
            let thread = ThreadId(handle.chat_id().into());
            let _guard = handle.orchestration_queue_lock().await;
            // The idle repair lane usually sees an unchanged queue. Compare
            // the small intent view before admitting/loading all of the
            // thread's retained runs, messages, and auxiliary records.
            if !self.queue_changed(&thread, &handle)? {
                continue;
            }
            if let Some(p) = self
                .domain
                .kernel
                .store
                .thread(&thread)
                .map_err(|_| unavailable())?
            {
                if p.thread.deleted_at.is_some() || p.thread.archived_at.is_some() {
                    continue;
                }
                self.synchronize(&p, &handle).await?;
            }
        }
        Ok(())
    }

    fn queue_changed(
        &self,
        thread: &ThreadId,
        handle: &Arc<crate::ChatDocHandle>,
    ) -> Result<bool, ToolError> {
        let rows = handle.doc().read_queue().map_err(|_| unavailable())?;
        let previous = self
            .domain
            .kernel
            .store
            .read(|conn| crate::orchestration::ui_queue::loro_intents(conn, thread))
            .map_err(|_| unavailable())?;
        Ok(rows != previous)
    }

    pub(crate) async fn synchronize(
        &self,
        target: &crate::orchestration::projection::ThreadProjection,
        handle: &Arc<crate::ChatDocHandle>,
    ) -> Result<(), ToolError> {
        let rows = handle.doc().read_queue().map_err(|_| unavailable())?;
        let previous = self
            .domain
            .kernel
            .store
            .read(|conn| crate::orchestration::ui_queue::loro_intents(conn, &target.thread.id))
            .map_err(|_| unavailable())?;
        if rows == previous {
            return Ok(());
        }
        let provider = self
            .registry
            .provider_instances
            .snapshot(&self.registry)
            .into_iter()
            .find(|p| p.provider_instance_id == target.thread.provider_instance_id);
        let driver = provider
            .map(|p| p.driver_kind.0)
            .unwrap_or_else(|| "unknown".into());
        let receipt = self
            .domain
            .mutate(
                None,
                target.thread.id.clone(),
                "host.sync_loro_queue",
                json!({"items":rows,"driver":driver}),
                CommandId(format!("queue-sync:{}", uuid::Uuid::new_v4())),
                crate::now_ms(),
            )
            .await
            .map_err(|_| unavailable())?;
        if receipt.status != crate::orchestration::ReceiptStatus::Accepted {
            return Err(unavailable());
        }
        Ok(())
    }

    /// The user controls their queue without impersonating an active agent.
    /// SQL acceptance, legacy drain and durable Loro patches share one lane.
    pub async fn mutate_for_user(
        &self,
        request: zeron_proto::MutateQueuedRunParams,
    ) -> Result<zeron_proto::MutateQueuedRunResult, ToolError> {
        use zeron_proto::QueuedRunAction;
        use zeron_proto::orchestration_mcp::OrchestratorMcpFailureCode as Code;

        for id in [
            &request.chat_id,
            &request.queued_run_id,
            &request.client_request_id,
        ] {
            if id.trim().is_empty() || id.len() > 512 {
                return Err(ToolError::new(
                    Code::InvalidRequest,
                    "Queue identities must be nonempty and at most 512 bytes.",
                ));
            }
        }
        if !self.docs.is_host(&request.chat_id) {
            return Err(ToolError::new(
                Code::OrchestrationError,
                "Queue mutations require the owning host.",
            ));
        }
        let thread = ThreadId(request.chat_id.clone());
        let handle = self
            .docs
            .open(&request.chat_id)
            .map_err(|_| unavailable())?;
        let _guard = handle.orchestration_queue_lock().await;
        self.prepare_locked(&handle).await?;
        let current = self
            .domain
            .kernel
            .store
            .thread(&thread)
            .map_err(|_| unavailable())?
            .ok_or_else(unavailable)?;
        if current.thread.deleted_at.is_some() || current.thread.archived_at.is_some() {
            return Err(unavailable());
        }
        let (name, mut input) = match &request.action {
            QueuedRunAction::Edit {
                text,
                expected_text,
            } => (
                "t3_queue_edit",
                json!({"text":text,"expectedText":expected_text}),
            ),
            QueuedRunAction::Cancel => ("t3_queue_cancel", json!({})),
            QueuedRunAction::Reorder { before_run_id } => {
                ("t3_queue_reorder", json!({"beforeRunId":before_run_id}))
            }
            QueuedRunAction::PromoteToSteer { target_run_id } => (
                "t3_queue_promote_to_steer",
                json!({"targetRunId":target_run_id,"expectedExecution":"active_steering"}),
            ),
            QueuedRunAction::PromoteToRestart { target_run_id } => (
                "t3_queue_promote_to_steer",
                json!({"targetRunId":target_run_id,"expectedExecution":"interrupt_restart"}),
            ),
        };
        input["threadId"] = json!(request.chat_id);
        input["queuedRunId"] = json!(request.queued_run_id);
        let id = CommandId(format!(
            "ui:queue:{}:{}",
            crate::orchestration::event::encode_component(&request.chat_id),
            crate::orchestration::event::encode_component(&request.client_request_id),
        ));
        let payload = serde_json::to_string(&json!({"name":name,"input":input}))
            .map_err(|_| unavailable())?;
        let same = self
            .domain
            .kernel
            .store
            .write(|conn| {
                super::reserve_request(conn, super::QUEUE_USER_REQUESTS, &id.0, &payload)
            })
            .map_err(|_| unavailable())?;
        if !same {
            return Err(ToolError::new(
                Code::InvalidRequest,
                "This queue request identity already belongs to different content or an action.",
            ));
        }
        let replay = self
            .domain
            .kernel
            .store
            .receipt(&id)
            .map_err(|_| unavailable())?
            .is_some();
        if !replay
            && let Some(run) = super::queued(&current)
                .into_iter()
                .find(|run| run.id.0 == request.queued_run_id)
        {
            let rows = handle.doc().read_queue().map_err(|_| unavailable())?;
            if rows
                .iter()
                .any(|row| row.id == run.user_message_id.0 && row.delivery_gate.is_some())
            {
                return Err(ToolError::new(
                    Code::InvalidRequest,
                    "This message is protected by an edit or review gate.",
                ));
            }
            if matches!(request.action, QueuedRunAction::Edit { .. })
                && rows.iter().any(|row| row.id == run.user_message_id.0)
            {
                return Err(ToolError::new(
                    Code::InvalidRequest,
                    "Document-backed messages require an edit lease.",
                ));
            }
        }
        let receipt = self
            .domain
            .mutate(None, thread.clone(), name, input, id, crate::now_ms())
            .await
            .map_err(|_| unavailable())?;
        self.apply_patches(&thread, &handle)?;
        handle.publish_orchestration_queue();
        Ok(zeron_proto::MutateQueuedRunResult {
            sequence: receipt.result_sequence,
            refusal: (receipt.status == crate::orchestration::ReceiptStatus::Rejected).then(|| {
                receipt
                    .error
                    .unwrap_or_else(|| "The queue action was refused.".into())
            }),
        })
    }
}

#[async_trait]
impl QueueService for HostQueue {
    async fn call(
        &self,
        caller: CallerScope,
        name: &str,
        input: Value,
    ) -> Result<Value, ToolError> {
        if !name.starts_with("t3_queue_") && name != "t3_thread_organize" {
            return self.domain.call(caller, name, input).await;
        }
        let writable = !matches!(name, "t3_queue_list" | "t3_queue_read");
        let target = self.domain.target(&caller, &input, writable)?;
        if !self.docs.is_host(&target.thread.id.0) {
            return Err(unavailable());
        }
        let handle = self
            .docs
            .open(&target.thread.id.0)
            .map_err(|_| unavailable())?;
        let _guard = handle.orchestration_queue_lock().await;
        self.apply_patches(&target.thread.id, &handle)?;
        let rows = handle.doc().read_queue().map_err(|_| unavailable())?;
        self.synchronize(&target, &handle).await?;
        if writable {
            let p = self.domain.target(&caller, &input, true)?;
            let message_id = super::queued(&p)
                .into_iter()
                .find(|r| r.id.0 == input["queuedRunId"].as_str().unwrap_or(""))
                .map(|r| r.user_message_id.0.clone());
            if let Some(id) = message_id
                && rows.iter().any(|r| r.id == id && r.delivery_gate.is_some())
            {
                return Err(unavailable());
            }
        }
        let result = self.domain.call(caller, name, input).await?;
        self.apply_patches(&target.thread.id, &handle)?;
        handle.publish_orchestration_queue();
        Ok(result)
    }
}
