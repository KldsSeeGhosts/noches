//! Queue/question/organization authority. All mutations are planned under the
//! kernel SQL transaction; replicas publish intents, never provider effects.
pub(crate) mod effects;
pub mod host;
pub mod mcp;
mod planner;
pub(crate) mod runtime;
mod search;
#[cfg(test)]
mod tests;
mod title;

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use zeron_proto::orchestration::{CommandId, ThreadId};
use zeron_proto::orchestration_mcp::OrchestratorMcpFailureCode as Code;

use super::command::{Command, Operation};
use super::projection::{self, ThreadProjection};
use super::queue_service::{MetadataPullRequestLinks, QueuePullRequestLinks, QueueService};
use super::service::{CallerScope, ToolError};
use super::{Error, Kernel, ReceiptStatus, Result, task};

pub(crate) use planner::plan;

#[derive(Debug, Clone)]
pub struct QueueCommand {
    pub caller: Option<CallerScope>,
    pub name: String,
    pub input: Value,
}

impl QueueCommand {
    pub(crate) fn command_type(&self) -> &str {
        match self.name.as_str() {
            "t3_queue_edit" => "queued-run.edit",
            "t3_queue_cancel" => "queued-run.cancel",
            "t3_queue_reorder" => "queued-run.reorder",
            "t3_queue_promote_to_steer" => "queued-message.promote-to-steer",
            "t3_pending_request_respond" => "runtime-request.respond",
            "t3_thread_update" => "thread.metadata.update",
            "t3_thread_organize" => match self.input["action"].as_str() {
                Some("pin") => "thread.pin",
                Some("unpin") => "thread.unpin",
                Some("snooze") => "thread.snooze",
                Some("unsnooze") => "thread.unsnooze",
                Some("settle") => "thread.settle",
                Some("unsettle") => "thread.unsettle",
                Some("archive") => "thread.archive",
                Some("unarchive") => "thread.unarchive",
                Some("mark_unread") => "thread.mark-unread",
                _ => "thread.organize",
            },
            _ => &self.name,
        }
    }
}

pub struct QueueDomain {
    pub kernel: Kernel,
    pub links: Arc<dyn QueuePullRequestLinks>,
}

impl QueueDomain {
    pub fn new(kernel: Kernel) -> Self {
        Self {
            kernel,
            links: Arc::new(MetadataPullRequestLinks),
        }
    }

    pub(crate) fn target(
        &self,
        caller: &CallerScope,
        input: &Value,
        writable: bool,
    ) -> std::result::Result<ThreadProjection, ToolError> {
        self.kernel
            .store
            .read(|conn| access(conn, caller, input, writable))
            .map_err(map_access_error)
    }

    pub(crate) async fn mutate(
        &self,
        caller: Option<CallerScope>,
        thread: ThreadId,
        name: &str,
        input: Value,
        id: CommandId,
        now: i64,
    ) -> Result<super::CommandReceipt> {
        let command = Command {
            id,
            thread_id: thread,
            operation: Operation::Queue(Box::new(QueueCommand {
                caller,
                name: name.into(),
                input,
            })),
        };
        self.kernel.dispatch(&command, now).await
    }

    /// Desktop user authority does not require a live agent caller. Target
    /// ownership is checked by the RPC before entering this local seam.
    pub async fn organize_for_user(
        &self,
        workspace: &crate::WorkspaceHost,
        docs: &crate::DocHost,
        registry: &Arc<crate::HarnessRegistry>,
        id: &str,
        mut input: Value,
        acknowledge: bool,
    ) -> std::result::Result<Value, ToolError> {
        if !acknowledge {
            let descriptor = zeron_proto::orchestration_mcp::pinned_tool_inventory()
                .into_iter()
                .find(|t| t.name == "t3_thread_organize")
                .expect("organize contract");
            crate::mcp::codec::decode(&descriptor.decoded_input_schema, &mut input);
            crate::mcp::codec::validate(&descriptor.decoded_input_schema, &input)
                .map_err(|message| ToolError::new(Code::InvalidRequest, message))?;
            if input["action"] == "snooze" && input.get("snoozedUntil").is_none() {
                return Err(ToolError::new(
                    Code::InvalidRequest,
                    "snooze requires snoozedUntil.",
                ));
            }
        }
        let thread = ThreadId(id.into());
        let handle = docs.open(id).map_err(|_| unavailable())?;
        let _guard = handle.orchestration_queue_lock().await;
        if self
            .kernel
            .store
            .thread(&thread)
            .map_err(|_| unavailable())?
            .is_none()
        {
            let chat = workspace
                .chat(id)
                .map_err(|_| unavailable())?
                .ok_or_else(|| {
                    ToolError::new(
                        Code::ThreadNotFound,
                        "The thread was not found in the calling project.",
                    )
                })?;
            let config = chat.config.as_ref();
            // TODO(merge-threads): the shared thread-create path should have
            // admitted every sidebar chat before lifecycle commands arrive.
            let harness = docs.harness_for(id);
            let mut provider = registry
                .provider_instances
                .snapshot(registry)
                .into_iter()
                .find(|p| p.harness_id == Some(harness))
                .ok_or_else(unavailable)?;
            if provider.models.is_empty() && config.and_then(|c| c.model.as_ref()).is_none() {
                registry
                    .provider_instances
                    .refresh(registry, harness)
                    .await
                    .map_err(|_| unavailable())?;
                provider = registry
                    .provider_instances
                    .snapshot(registry)
                    .into_iter()
                    .find(|p| p.provider_instance_id == provider.provider_instance_id)
                    .ok_or_else(unavailable)?;
            }
            let model = config
                .and_then(|c| c.model.clone())
                .or_else(|| provider.models.first().map(|m| m.id.clone()))
                .ok_or_else(unavailable)?;
            let create=Command::wire(serde_json::from_value(json!({
                "type":"thread.create","commandId":format!("lifecycle-adopt:{id}"),
                "threadId":id,"projectId":chat.space_id.as_deref().unwrap_or("scratch"),
                "title":chat.title.as_deref().unwrap_or("Conversation"),"createdBy":"user","creationSource":"web",
                "modelSelection":{"instanceId":provider.provider_instance_id,"model":model},
                "runtimeMode":config.map(|c|c.runtime_mode).unwrap_or_default(),
                "interactionMode":config.map(|c|c.interaction_mode).unwrap_or_default(),
                "branch":chat.branch,"worktreePath":chat.cwd
            })).map_err(|_|unavailable())?).map_err(|_|unavailable())?;
            self.kernel
                .dispatch(&create, crate::now_ms())
                .await
                .map_err(|_| unavailable())?;
        }
        let host = host::HostQueue {
            domain: Arc::new(QueueDomain::new(self.kernel.clone())),
            docs: docs.clone(),
            registry: registry.clone(),
        };
        let current = self
            .kernel
            .store
            .thread(&thread)
            .map_err(|_| unavailable())?
            .ok_or_else(unavailable)?;
        host.synchronize(&current, &handle).await?;
        let name = if acknowledge {
            "host.acknowledge_woke"
        } else {
            "t3_thread_organize"
        };
        let receipt = self
            .mutate(
                None,
                thread.clone(),
                name,
                input,
                CommandId(format!("user:{}", uuid::Uuid::new_v4())),
                crate::now_ms(),
            )
            .await
            .map_err(|_| unavailable())?;
        if receipt.status == ReceiptStatus::Rejected {
            return Err(unavailable());
        }
        host.apply_patches(&thread, &handle)?;
        Ok(json!({"sequence":receipt.result_sequence}))
    }

    /// Host worker, also callable with a fake clock. A wake is lifecycle-only:
    /// it never sends a prompt, restarts, or interrupts a run.
    pub async fn wake_due(&self, now: i64) -> Result<usize> {
        let ids = self.kernel.store.read(|conn| {
            let mut statement = conn.prepare(
                "SELECT id FROM orchestration_projection_threads WHERE
                 json_extract(payload_json,'$.deletedAt') IS NULL AND
                 json_extract(payload_json,'$.archivedAt') IS NULL AND
                 json_extract(payload_json,'$.snoozedUntil') IS NOT NULL AND
                 json_extract(payload_json,'$.snoozedUntil') <= ?1",
            )?;
            let ids = statement
                .query_map([super::event::iso(now)?], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(ids)
        })?;
        let mut count = 0;
        for id in ids {
            let thread = ThreadId(id);
            let receipt = self
                .mutate(
                    None,
                    thread.clone(),
                    "host.unsnooze_due",
                    json!({}),
                    CommandId(format!("snooze-wake:{}:{now}", thread.0)),
                    now,
                )
                .await?;
            if receipt.status == ReceiptStatus::Accepted {
                count += 1;
            }
        }
        Ok(count)
    }

    /// TODO(merge-pr-watch): settlement/watch workers use this host-authority
    /// entry point, sharing the same guard and source transaction as user settle.
    pub async fn settle_for_host(
        &self,
        thread: ThreadId,
        source: zeron_proto::SettleSource,
        now: i64,
    ) -> Result<super::CommandReceipt> {
        self.mutate(
            None,
            thread,
            "t3_thread_organize",
            json!({"action":"settle","settledBy":source}),
            CommandId(format!("host-settle:{}", uuid::Uuid::new_v4())),
            now,
        )
        .await
    }
}

pub(crate) fn unavailable() -> ToolError {
    ToolError::new(
        Code::OrchestrationError,
        "The operation could not be completed.",
    )
}

// Preserve the public refusal family through the private planner without adding
// a global kernel error variant shared by the other wave slices.
fn access_error(code: Code, message: &str) -> Error {
    Error::Invariant(format!(
        "queue-access:{}",
        serde_json::to_string(&ToolError::new(code, message).into_failure()).expect("failure")
    ))
}

pub(crate) fn map_access_error(error: Error) -> ToolError {
    if let Error::Invariant(message) = &error
        && let Some(json) = message.strip_prefix("queue-access:")
        && let Ok(failure) =
            serde_json::from_str::<zeron_proto::orchestration_mcp::OrchestratorMcpFailure>(json)
    {
        return ToolError::new(failure.code, failure.message);
    }
    unavailable()
}

pub(crate) fn access(
    conn: &rusqlite::Connection,
    scope: &CallerScope,
    input: &Value,
    writable: bool,
) -> Result<ThreadProjection> {
    let caller = projection::read_thread(conn, &scope.thread_id)?
        .filter(|p| p.thread.deleted_at.is_none())
        .ok_or_else(|| access_error(Code::ThreadNotFound, "The calling thread was not found."))?;
    let id = input["threadId"].as_str().unwrap_or(&scope.thread_id.0);
    let target = projection::read_thread(conn, &ThreadId(id.into()))?
        .filter(|p| {
            p.thread.deleted_at.is_none() && p.thread.project_id == caller.thread.project_id
        })
        .ok_or_else(|| {
            access_error(
                Code::ThreadNotFound,
                "The thread was not found in the calling project.",
            )
        })?;
    if writable {
        // T3 checks any active run, not equality with the credential's runId.
        if caller.thread.archived_at.is_some()
            || caller.thread.provider_instance_id != scope.provider_instance_id
            || !caller.runs.iter().any(|r| {
                !super::command::run_terminal(&r.status)
                    && r.status != zeron_proto::orchestration::OrchestrationV2RunStatus::Queued
            })
        {
            return Err(access_error(
                Code::ParentNotActive,
                "The calling provider no longer owns an active thread run.",
            ));
        }
        if !caller
            .thread
            .runtime_mode
            .permits(target.thread.runtime_mode)
        {
            return Err(access_error(
                Code::RuntimeModeEscalationDenied,
                &format!(
                    "Child runtime mode {} is broader than parent mode {}.",
                    serde_json::to_value(target.thread.runtime_mode)?
                        .as_str()
                        .unwrap(),
                    serde_json::to_value(caller.thread.runtime_mode)?
                        .as_str()
                        .unwrap()
                ),
            ));
        }
        if caller.thread.interaction_mode == zeron_proto::InteractionMode::Plan
            && target.thread.interaction_mode != zeron_proto::InteractionMode::Plan
        {
            return Err(access_error(
                Code::InteractionModeEscalationDenied,
                "Child interaction mode default is broader than parent mode plan.",
            ));
        }
    }
    Ok(target)
}

pub(crate) fn metadata_access(
    conn: &rusqlite::Connection,
    scope: &CallerScope,
    input: &Value,
) -> Result<ThreadProjection> {
    let parent = projection::read_thread(conn, &scope.thread_id)?.ok_or_else(|| {
        access_error(
            Code::ThreadNotFound,
            &format!("Calling thread {} was not found.", scope.thread_id.0),
        )
    })?;
    let id = input["threadId"].as_str().unwrap_or(&scope.thread_id.0);
    if id == scope.thread_id.0 {
        return Ok(parent);
    }
    projection::read_thread(conn, &ThreadId(id.into()))?
        .filter(|p| {
            p.thread.deleted_at.is_none() && p.thread.project_id == parent.thread.project_id
        })
        .ok_or_else(|| {
            access_error(
                Code::ThreadNotFound,
                &format!(
                    "Thread {id} was not found in project {}.",
                    parent.thread.project_id.0
                ),
            )
        })
}

pub(crate) fn queued(
    projection: &ThreadProjection,
) -> Vec<&zeron_proto::orchestration::OrchestrationV2Run> {
    let mut runs: Vec<_> = projection
        .runs
        .iter()
        .filter(|r| r.status == zeron_proto::orchestration::OrchestrationV2RunStatus::Queued)
        .collect();
    runs.sort_by_key(|r| {
        let automatic = task::records(projection, "message")
            .iter()
            .any(|m| m["id"] == r.user_message_id.0 && m.get("delegatedCompletion").is_some());
        (
            !automatic,
            r.queue_position
                .as_ref()
                .copied()
                .flatten()
                .unwrap_or(r.ordinal),
            r.ordinal,
        )
    });
    runs
}

pub(crate) fn entry(projection: &ThreadProjection, id: &str, limit: usize) -> Option<Value> {
    let run = queued(projection).into_iter().find(|r| r.id.0 == id)?;
    let message = task::records(projection, "message")
        .iter()
        .find(|m| m["id"] == run.user_message_id.0)?;
    let text = message["text"].as_str()?;
    Some(
        json!({"queuedRunId":id,"text":text.chars().take(limit).collect::<String>(),
        "truncated":text.chars().count()>limit}),
    )
}

pub(crate) fn question<'a>(p: &'a ThreadProjection, id: &str) -> Option<(&'a Value, &'a Value)> {
    let request = task::records(p, "runtime-request")
        .iter()
        .find(|r| r["id"] == id && r["kind"] == "user_input" && r["status"] == "pending")?;
    let item = task::records(p, "turn-item")
        .iter()
        .find(|i| i["type"] == "user_input_request" && i["requestId"] == id)?;
    Some((request, item))
}

#[async_trait]
impl QueueService for QueueDomain {
    async fn call(
        &self,
        caller: CallerScope,
        name: &str,
        input: Value,
    ) -> std::result::Result<Value, ToolError> {
        if name == "t3_thread_search" {
            let parent = self.target(&caller, &json!({}), false)?;
            return self
                .kernel
                .store
                .read(|conn| search::search(conn, &parent.thread.project_id, &input))
                .map_err(|_| unavailable());
        }
        let metadata = name == "t3_thread_update";
        let writable = !metadata
            && !matches!(
                name,
                "t3_queue_list"
                    | "t3_queue_read"
                    | "t3_pending_request_list"
                    | "t3_pending_request_read"
            );
        let p = if metadata {
            self.kernel
                .store
                .read(|conn| metadata_access(conn, &caller, &input))
                .map_err(map_access_error)?
        } else {
            self.target(&caller, &input, writable)?
        };
        match name {
            "t3_queue_list" => {
                let cursor = input["cursor"].as_u64().unwrap_or(0) as usize;
                let limit = input["limit"].as_u64().unwrap_or(20) as usize;
                let runs = queued(&p);
                let end = cursor.saturating_add(limit);
                return Ok(json!({"items":runs.iter().skip(cursor).take(limit)
                    .filter_map(|r| entry(&p,&r.id.0,1000)).collect::<Vec<_>>(),
                    "nextCursor":(end<runs.len()).then_some(end)}));
            }
            "t3_queue_read" => {
                return entry(&p, input["queuedRunId"].as_str().unwrap_or(""), 16000).ok_or_else(
                    || ToolError::new(Code::InvalidRequest, "The queued message was not found."),
                );
            }
            "t3_pending_request_list" => {
                return Ok(json!({"requestIds":
                task::records(&p,"runtime-request").iter()
                    .filter(|r|r["kind"]=="user_input" && r["status"]=="pending")
                    .map(|r|r["id"].clone()).collect::<Vec<_>>()}));
            }
            "t3_pending_request_read" | "t3_pending_request_respond" => {
                let id = input["requestId"].as_str().unwrap_or("");
                let (_, item) = question(&p, id).ok_or_else(|| {
                    ToolError::new(
                        Code::InvalidRequest,
                        "The pending user-input request was not found.",
                    )
                })?;
                if name == "t3_pending_request_read" {
                    return Ok(json!({"requestId":id,"questions":item["questions"]}));
                }
            }
            "t3_thread_organize"
                if input["action"] == "snooze" && input.get("snoozedUntil").is_none() =>
            {
                return Err(ToolError::new(
                    Code::InvalidRequest,
                    "snooze requires snoozedUntil.",
                ));
            }
            _ => {}
        }
        let id = if metadata {
            let key = input["clientRequestId"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let action = input["action"].as_str().unwrap();
            CommandId(format!(
                "command:mcp:{}:thread-update:{}:{}:{}",
                super::event::encode_component(&caller.session_id),
                super::event::encode_component(&p.thread.id.0),
                super::event::encode_component(action),
                super::event::encode_component(&key)
            ))
        } else {
            CommandId(format!("mcp:{}", uuid::Uuid::new_v4()))
        };
        let mut input = input;
        // Link transformation is pure; storage stays inside the transaction.
        if metadata
            && matches!(
                input["action"].as_str(),
                Some("link_pull_request" | "unlink_pull_request")
            )
        {
            let mut link = if input["action"] == "link_pull_request" {
                input["pullRequest"].clone()
            } else {
                Value::Null
            };
            if link.is_object() {
                link["projectId"] = json!(p.thread.project_id);
            }
            let mut thread = json!({});
            self.links
                .update(&mut thread, link)
                .map_err(|_| unavailable())?;
            input["resolvedPullRequest"] = thread["linkedPullRequest"].clone();
        }
        let receipt = self
            .mutate(
                Some(caller),
                p.thread.id.clone(),
                name,
                input.clone(),
                id.clone(),
                crate::now_ms(),
            )
            .await
            .map_err(map_access_error)?;
        if receipt.status == ReceiptStatus::Rejected {
            return Err(if metadata {
                ToolError::new(
                    Code::OrchestrationError,
                    format!(
                        "Unable to {} for thread {}: Failed to dispatch orchestration command thread.metadata.update ({}).",
                        input["action"].as_str().unwrap(),
                        p.thread.id.0,
                        id.0
                    ),
                )
            } else {
                map_access_error(Error::Invariant(receipt.error.unwrap_or_default()))
            });
        }
        if metadata {
            // Receipts replay the original event, not today's possibly renamed state.
            let thread = self
                .kernel
                .store
                .read(|conn| {
                    let raw: String = conn.query_row(
                        "SELECT envelope_json FROM orchestration_events WHERE command_id=?1
                     ORDER BY sequence DESC LIMIT 1",
                        [&id.0],
                        |row| row.get(0),
                    )?;
                    Ok(serde_json::from_str::<Value>(&raw)?["event"]["payload"].clone())
                })
                .map_err(|_| unavailable())?;
            return Ok(
                json!({"threadId":p.thread.id,"action":input["action"],"commandId":id,
                "sequence":receipt.result_sequence,"title":thread["title"],
                "titleRegeneration":thread["titleRegeneration"],"linkedPullRequest":thread["linkedPullRequest"],
                "updatedAt":thread["updatedAt"]}),
            );
        }
        Ok(json!({"sequence":receipt.result_sequence}))
    }
}
