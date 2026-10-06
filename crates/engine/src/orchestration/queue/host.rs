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
                    for (index, id) in patch["messageIds"]
                        .as_array()
                        .into_iter()
                        .flatten()
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
