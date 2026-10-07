//! T3 thread controls. No legacy create_chat/send_message emulation.
pub mod mcp;
pub mod planner;
pub(crate) mod runner;
#[cfg(test)]
mod tests;
pub mod timeline;
pub mod wire;

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use serde_json::json;
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::*;

use super::command::{Command, Operation, run_terminal};
use super::event::{encode_component, mcp_command_id};
use super::projection::ThreadProjection;
use super::service::{CallerScope, ToolError};
use super::task::{DelegationService, active_run, records};
use super::thread_service::{ThreadSendRequest, ThreadService};
use super::{Kernel, ReceiptStatus};
use OrchestratorMcpFailureCode as Code;

#[derive(Clone)]
pub struct KernelThreadService {
    pub kernel: Kernel,
    pub delegation: Arc<DelegationService>,
}

pub(crate) fn failure(error: impl std::fmt::Display) -> ToolError {
    ToolError::new(Code::OrchestrationError, error.to_string())
}

pub(crate) fn request_key(input: &Optional<String>) -> String {
    input
        .as_ref()
        .cloned()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

pub(crate) fn modes(parent: &ThreadProjection, target: &ThreadProjection) -> Result<(), ToolError> {
    if !parent
        .thread
        .runtime_mode
        .permits(target.thread.runtime_mode)
    {
        return Err(ToolError::new(
            Code::RuntimeModeEscalationDenied,
            format!(
                "Child runtime mode {} is broader than parent mode {}.",
                json!(target.thread.runtime_mode).as_str().unwrap(),
                json!(parent.thread.runtime_mode).as_str().unwrap()
            ),
        ));
    }
    if !parent
        .thread
        .interaction_mode
        .permits(target.thread.interaction_mode)
    {
        return Err(ToolError::new(
            Code::InteractionModeEscalationDenied,
            format!(
                "Child interaction mode {} is broader than parent mode {}.",
                json!(target.thread.interaction_mode).as_str().unwrap(),
                json!(parent.thread.interaction_mode).as_str().unwrap()
            ),
        ));
    }
    Ok(())
}

impl KernelThreadService {
    fn caller(
        &self,
        caller: &CallerScope,
        thread_toolkit: bool,
    ) -> Result<ThreadProjection, ToolError> {
        self.kernel
            .store
            .thread_in_project(&caller.thread_id, Some(&caller.project_id))
            .map_err(|_| if thread_toolkit {failure("The operation could not be completed.")} else {
                failure(format!("Unable to read thread {}: Failed to load orchestration projection for thread {}.",caller.thread_id,caller.thread_id))
            })?
            .filter(|p| !thread_toolkit || p.thread.deleted_at.is_none())
            .ok_or_else(|| {
                if self.kernel.store.registry_unavailable(&caller.thread_id, Some(&caller.project_id)) {
                    return ToolError::new(Code::ThreadNotFound,"The calling thread was not found.");
                }
                if thread_toolkit {
                    ToolError::new(Code::ThreadNotFound,"The calling thread was not found.")
                } else {
                    failure(format!("Unable to read thread {}: Failed to load orchestration projection for thread {}.",caller.thread_id,caller.thread_id))
                }
            })
    }

    pub(crate) fn scoped(
        &self,
        caller: &CallerScope,
        target: &str,
        readable: bool,
    ) -> Result<(ThreadProjection, ThreadProjection), ToolError> {
        let parent = self.caller(caller, false)?;
        if target == caller.thread_id.0 {
            if parent.thread.deleted_at.is_some() {
                return Err(ToolError::new(
                    Code::ThreadNotFound,
                    if readable {
                        format!("Thread {target} is no longer available.")
                    } else {
                        format!(
                            "Thread {target} was not found in project {}.",
                            parent.thread.project_id
                        )
                    },
                ));
            }
            return Ok((parent.clone(), parent));
        }
        let load_error = || {
            if self
                .kernel
                .store
                .registry_unavailable(&ThreadId(target.into()), Some(&parent.thread.project_id))
            {
                return ToolError::new(
                    Code::ThreadNotFound,
                    format!(
                        "Thread {target} was not found in project {}.",
                        parent.thread.project_id
                    ),
                );
            }
            failure(format!(
                "Unable to load thread {target} in project {}.",
                parent.thread.project_id
            ))
        };
        let attached = readable
            && records(&parent, "message").iter().any(|m| {
                m["role"] == "user"
                    && m["createdBy"] == "user"
                    && m["context"]["records"].as_array().is_some_and(|rs| {
                        rs.iter()
                            .any(|r| r["kind"] == "thread" && r["threadId"] == target)
                    })
            });
        let projection = self
            .kernel
            .store
            .thread_in_project(
                &ThreadId(target.into()),
                if attached {
                    None
                } else {
                    Some(&parent.thread.project_id)
                },
            )
            .map_err(|_| load_error())?
            .ok_or_else(load_error)?;
        if attached && projection.thread.deleted_at.is_some() {
            return Err(ToolError::new(
                Code::ThreadNotFound,
                format!("Thread {target} is no longer available."),
            ));
        }
        let target = Some(projection)
            .filter(|p| {
                p.thread.deleted_at.is_none()
                    && (p.thread.project_id == parent.thread.project_id || attached)
            })
            .ok_or_else(|| {
                ToolError::new(
                    Code::ThreadNotFound,
                    format!(
                        "Thread {target} was not found in project {}.",
                        parent.thread.project_id
                    ),
                )
            })?;
        Ok((parent, target))
    }

    async fn command(&self, command: Command) -> Result<i64, ToolError> {
        let receipt = self
            .kernel
            .dispatch(&command, crate::now_ms())
            .await
            .map_err(failure)?;
        if receipt.status == ReceiptStatus::Rejected {
            return Err(failure(receipt.error.unwrap_or_default()));
        }
        Ok(receipt.result_sequence)
    }

    fn run<'a>(
        &self,
        projection: &'a ThreadProjection,
        id: Option<&String>,
    ) -> Result<Option<&'a OrchestrationV2Run>, ToolError> {
        match id {
            None => Ok(projection.runs.iter().max_by_key(|r| r.ordinal)),
            Some(id) => projection
                .runs
                .iter()
                .find(|r| r.id.0 == *id)
                .map(Some)
                .ok_or_else(|| {
                    ToolError::new(
                        Code::RunNotFound,
                        format!(
                            "Run {id} does not belong to thread {}.",
                            projection.thread.id
                        ),
                    )
                }),
        }
    }
}

#[async_trait]
impl ThreadService for KernelThreadService {
    async fn list(
        &self,
        caller: CallerScope,
        input: T3ThreadListInput,
    ) -> Result<T3ThreadListResult, ToolError> {
        let parent = self.caller(&caller, false)?;
        let all = self
            .kernel
            .store
            .thread_summaries(&parent.thread.project_id)
            .map_err(|_| {
                failure(format!(
                    "Unable to list threads: Unable to list threads in project {}.",
                    parent.thread.project_id
                ))
            })?;
        let filtered: Vec<_> = all
            .into_iter()
            .filter(|t| {
                (input.include_subagents.as_ref() != Some(&false)
                    || t.relationship_to_parent.as_ref()
                        != Some(&OrchestratorMcpThreadListItemRelationshipToParent::Subagent))
                    && input
                        .statuses
                        .as_ref()
                        .is_none_or(|s| s.contains(&t.status))
                    && input.settled.as_ref().is_none_or(|s| *s == t.settled)
                    && input
                        .title_contains
                        .as_ref()
                        .is_none_or(|s| t.title.to_lowercase().contains(&s.to_lowercase()))
            })
            .collect();
        let start = input.cursor.as_ref().copied().unwrap_or(0) as usize;
        let page: Vec<_> = filtered
            .iter()
            .skip(start)
            .take(input.limit.as_ref().copied().unwrap_or(50) as usize)
            .cloned()
            .collect();
        Ok(T3ThreadListResult {
            project_id: parent.thread.project_id,
            current_thread_id: caller.thread_id,
            next_cursor: (start + page.len() < filtered.len())
                .then_some((start + page.len()) as i64),
            total: filtered.len() as i64,
            threads: page,
        })
    }

    async fn read(
        &self,
        caller: CallerScope,
        input: T3ThreadReadInput,
    ) -> Result<timeline::ThreadReadPage, ToolError> {
        let (parent, target) = self.scoped(&caller, &input.thread_id, true)?;
        let (page, selected) =
            timeline::page(&self.kernel.store, &target, &input).map_err(|_| {
                failure(format!(
                    "Failed to load orchestration projection for thread {}.",
                    target.thread.id
                ))
            })?;
        if input.text_offset.as_ref().copied().unwrap_or(0) == 0
            && target.thread.lineage.parent_thread_id.as_ref() == Some(&parent.thread.id)
            && target.thread.lineage.relationship_to_parent
                == Some(OrchestrationV2AppThreadLineageRelationshipToParent::Subagent)
            && let Some(task) = records(&parent, "subagent").iter().find(|t| {
                t["origin"] == "app_owned"
                    && t["threadId"] == parent.thread.id.0
                    && t["childThreadId"] == target.thread.id.0
            })
            && let Some(transfer) = records(&parent, "context-transfer").iter().find(|t| {
                t["type"] == "subagent_result"
                    && t["sourceThreadId"] == target.thread.id.0
                    && t["targetThreadId"] == parent.thread.id.0
            })
        {
            let result_run = transfer["sourcePoint"]["runId"]
                .as_str()
                .or_else(|| task["childRunId"].as_str())
                .and_then(|id| target.runs.iter().find(|r| r.id.0 == id));
            if let Some(run) = result_run.filter(|r| {
                run_terminal(&r.status) && r.status != OrchestrationV2RunStatus::RolledBack
            }) {
                let result_item = if run.status == OrchestrationV2RunStatus::Failed {
                    records(&target, "turn-item")
                        .iter()
                        .filter(|i| i["runId"] == run.id.0 && i["type"] == "error")
                        .max_by_key(|i| i["ordinal"].as_i64())
                } else {
                    None
                };
                let result_message = if result_item.is_none() {
                    records(&target, "message")
                        .iter()
                        .filter(|m| {
                            m["runId"] == run.id.0
                                && m["role"] == "assistant"
                                && m["text"].as_str().is_some_and(|s| !s.trim().is_empty())
                        })
                        .max_by_key(|m| m["updatedAt"].as_str())
                } else {
                    None
                };
                let fallback_item = if result_item.is_none() {
                    records(&target, "turn-item")
                        .iter()
                        .filter(|i| {
                            i["runId"] == run.id.0
                                && i["type"] == "assistant_message"
                                && i["text"].as_str().is_some_and(|s| !s.trim().is_empty())
                        })
                        .max_by_key(|i| i["ordinal"].as_i64())
                } else {
                    None
                };
                let complete = selected.iter().any(|row| {
                    row.source == target.thread.id
                        && (result_item
                            .or(fallback_item)
                            .is_some_and(|i| i["id"] == row.id)
                            || result_message.is_some_and(|m| {
                                row.item["type"] == "assistant_message"
                                    && row.item["messageId"] == m["id"]
                            }))
                        && timeline::text(&row.item).is_some_and(|s| {
                            s.encode_utf16().count()
                                <= input.max_chars_per_item.as_ref().copied().unwrap_or(20_000)
                                    as usize
                        })
                });
                self.delegation
                    .acknowledge_child_read(&caller, &target.thread.id, 0, false, complete)
                    .await?;
            }
        }
        Ok(page)
    }

    async fn send(
        &self,
        caller: CallerScope,
        input: T3ThreadSendInput,
    ) -> Result<T3ThreadSendResult, ToolError> {
        let (parent, target) = self.scoped(&caller, &input.thread_id, false)?;
        modes(&parent, &target)?;
        let key = request_key(&input.client_request_id);
        self.send_to_thread(ThreadSendRequest {
            project_id: parent.thread.project_id,
            thread_id: target.thread.id,
            command_id: mcp_command_id(&caller.session_id, "thread-send", &key),
            message_id: MessageId(format!(
                "message:mcp:{}:thread-send:{}",
                encode_component(&caller.session_id),
                encode_component(&key)
            )),
            scheduled_task_id: None,
            sender_thread_id: Some(caller.thread_id),
            text: input.message,
            attachments: vec![],
            model_selection: None,
            mode: input
                .mode
                .as_ref()
                .cloned()
                .unwrap_or(T3ThreadSendInputMode::Auto),
            created_by: OrchestrationV2Actor::Agent,
            creation_source: OrchestrationV2CreationSource::Mcp,
        })
        .await
    }

    async fn send_to_thread(
        &self,
        mut input: ThreadSendRequest,
    ) -> Result<T3ThreadSendResult, ToolError> {
        let target = self
            .kernel
            .store
            .thread_in_project(&input.thread_id, Some(&input.project_id))
            .map_err(|_| {
                failure(format!(
                    "Unable to load thread {} in project {}.",
                    input.thread_id, input.project_id
                ))
            })?
            .ok_or_else(|| {
                if self
                    .kernel
                    .store
                    .registry_unavailable(&input.thread_id, Some(&input.project_id))
                {
                    return ToolError::new(
                        Code::ThreadNotFound,
                        format!(
                            "Thread {} was not found in project {}.",
                            input.thread_id, input.project_id
                        ),
                    );
                }
                failure(format!(
                    "Unable to load thread {} in project {}.",
                    input.thread_id, input.project_id
                ))
            })?;
        if target.thread.deleted_at.is_some() || target.thread.project_id != input.project_id {
            return Err(ToolError::new(
                Code::ThreadNotFound,
                format!(
                    "Thread {} was not found in project {}.",
                    input.thread_id, input.project_id
                ),
            ));
        }
        if target.thread.archived_at.is_some() {
            return Err(ToolError::new(
                Code::ThreadNotSendable,
                format!(
                    "Thread {} is archived and cannot receive messages.",
                    input.thread_id
                ),
            ));
        }
        let mode = input.mode.clone();
        let steerable = planner::steerable(&target);
        if matches!(
            mode,
            T3ThreadSendInputMode::Steer | T3ThreadSendInputMode::Restart
        ) && steerable.is_none()
        {
            return Err(ToolError::new(
                Code::ThreadNotSendable,
                format!(
                    "Thread {} has no running turn that can be {}.",
                    input.thread_id,
                    if mode == T3ThreadSendInputMode::Steer {
                        "steered"
                    } else {
                        "restarted"
                    }
                ),
            ));
        }
        // T3 dispatch refuses provider-owned children before resolving any
        // provider session/model or clearing lifecycle parking. Keep the
        // planner's guard too, for ownership changes between these reads.
        if self
            .kernel
            .store
            .read(|conn| planner::native_child(conn, &target))
            .map_err(failure)?
        {
            return Err(failure(format!(
                "Unable to send to thread {}: This subagent is run by its provider and cannot take messages. Message the parent thread instead.",
                input.thread_id
            )));
        }
        let id = input.command_id;
        let message = input.message_id;
        // Driver identity belongs to the saved instance, not the calling provider.
        let driver = records(&target, "provider-thread")
            .iter()
            .find(|p| p["providerInstanceId"] == target.thread.provider_instance_id.0)
            .and_then(|p| p["driver"].as_str())
            .map(str::to_owned);
        let driver = match (driver, input.model_selection.as_ref()) {
            (Some(d), None) => d,
            (None, None) => {
                let resolved = self
                    .delegation
                    .targets
                    .resolve(&target.thread, None)
                    .await?;
                if target.thread.model_selection.model == "default" {
                    input.model_selection = Some(resolved.selection);
                }
                resolved.driver.0
            }
            _ => {
                let selection = input.model_selection.as_ref().unwrap();
                let mut requested =
                    json!({"providerInstanceId":selection.instance_id,"model":selection.model});
                if let Some(options) = selection.options.as_ref() {
                    requested["options"] = json!(options);
                }
                let requested: DelegateTaskInputTarget =
                    serde_json::from_value(requested).map_err(failure)?;
                self.delegation
                    .targets
                    .resolve(&target.thread, Some(&requested))
                    .await?
                    .driver
                    .0
            }
        };
        let requested = planner::Send {
            message_id: message.clone(),
            text: input.text,
            mode: mode.clone(),
            driver,
            sender: input
                .sender_thread_id
                .clone()
                .unwrap_or_else(|| target.thread.id.clone()),
            target_run: if mode == T3ThreadSendInputMode::Queue {
                None
            } else {
                steerable.map(|r| r.id.clone())
            },
            metadata: Some(planner::SendMetadata {
                scheduled_task_id: input.scheduled_task_id,
                sender_thread_id: input.sender_thread_id,
                attachments: input.attachments,
                model_selection: input.model_selection,
                created_by: input.created_by,
                creation_source: input.creation_source,
            }),
        };
        self.command(Command {
            id,
            thread_id: target.thread.id.clone(),
            operation: Operation::Thread(Box::new(planner::ThreadOperation::Send(requested))),
        })
        .await
        .map_err(|e| {
            failure(format!(
                "Unable to send to thread {}: {}",
                input.thread_id, e.message
            ))
        })?;
        let current = self
            .kernel
            .store
            .thread(&target.thread.id)
            .map_err(failure)?
            .unwrap();
        let missing_projection = || {
            failure(format!(
                "Message {message} was accepted on thread {} without a durable run projection.",
                target.thread.id
            ))
        };
        let stored = records(&current, "message")
            .iter()
            .find(|m| m["id"] == message.0)
            .ok_or_else(missing_projection)?;
        let run = current
            .runs
            .iter()
            .find(|r| stored["runId"] == r.id.0)
            .ok_or_else(missing_projection)?;
        let item = records(&current, "turn-item")
            .iter()
            .find(|i| i["type"] == "user_message" && i["messageId"] == message.0);
        let delivery = match item.map(|i| i["inputIntent"].as_str().unwrap_or_default()) {
            None | Some("queued_turn") => "queued",
            Some("turn_start") => "started",
            _ if input.mode == T3ThreadSendInputMode::Restart => "restarted",
            _ => "steered",
        };
        serde_json::from_value(json!({"threadId":current.thread.id,"messageId":message,"runId":run.id,"status":run.status,"delivery":delivery})).map_err(failure)
    }

    async fn wait(
        &self,
        caller: CallerScope,
        input: T3ThreadWaitInput,
    ) -> Result<T3ThreadWaitResult, ToolError> {
        let (_, target) = self.scoped(&caller, &input.thread_id, false)?;
        let selected = self.run(&target, input.run_id.as_ref())?.cloned();
        let timeout = input
            .timeout_ms
            .as_ref()
            .map(JsonNumber::as_f64)
            .unwrap_or(600_000.0)
            .clamp(1.0, 3_600_000.0);
        let deadline = tokio::time::Instant::now()
            + Duration::from_millis(if timeout.is_nan() { 1 } else { timeout as u64 });
        let mut run = selected;
        loop {
            if run.as_ref().is_none_or(|r| run_terminal(&r.status))
                || tokio::time::Instant::now() >= deadline
            {
                break;
            }
            tokio::time::sleep_until(
                (tokio::time::Instant::now() + Duration::from_millis(250)).min(deadline),
            )
            .await;
            let (_, current) = self.scoped(&caller, &input.thread_id, false)?;
            run = self.run(&current, run.as_ref().map(|r| &r.id.0))?.cloned();
        }
        // A terminal event can race the deadline. Like T3, let a final
        // projection read decide whether the selected run actually timed out.
        if run.as_ref().is_some_and(|r| !run_terminal(&r.status)) {
            let (_, current) = self.scoped(&caller, &input.thread_id, false)?;
            run = self.run(&current, run.as_ref().map(|r| &r.id.0))?.cloned();
        }
        serde_json::from_value(
            json!({"threadId":target.thread.id,"runId":run.as_ref().map(|r| &r.id),
            "status":run.as_ref().map(|r| json!(r.status)).unwrap_or(json!("idle")),
            "timedOut":run.as_ref().is_some_and(|r| !run_terminal(&r.status))}),
        )
        .map_err(failure)
    }

    async fn interrupt(
        &self,
        caller: CallerScope,
        input: T3ThreadInterruptInput,
    ) -> Result<T3ThreadInterruptResult, ToolError> {
        let (_, target) = self.scoped(&caller, &input.thread_id, false)?;
        let explicit = if input.run_id.as_ref().is_some() {
            self.run(&target, input.run_id.as_ref())?
        } else {
            None
        };
        let active = super::background::interruptible_run(&target);
        if let Some(run) = explicit
            .filter(|r| run_terminal(&r.status) && active.is_none_or(|active| active.id != r.id))
        {
            return serde_json::from_value(
                json!({"threadId":target.thread.id,"runId":run.id,"status":run.status}),
            )
            .map_err(failure);
        }
        if input.run_id.as_ref().is_none() && active.is_none() {
            return serde_json::from_value(
                json!({"threadId":target.thread.id,"runId":null,"status":"no_active_run"}),
            )
            .map_err(failure);
        }
        if active.is_none()
            || input
                .run_id
                .as_ref()
                .is_some_and(|id| active.unwrap().id.0 != *id)
        {
            return Err(ToolError::new(
                Code::ThreadNotInterruptible,
                format!(
                    "Run {} is not currently interruptible.",
                    input.run_id.as_ref().unwrap()
                ),
            ));
        }
        let run = active.unwrap();
        let key = request_key(&input.client_request_id);
        self.command(Command {
            id: mcp_command_id(&caller.session_id, "thread-interrupt", &key),
            thread_id: target.thread.id.clone(),
            operation: Operation::Thread(Box::new(planner::ThreadOperation::Interrupt {
                run_id: run.id.clone(),
                reason: input.reason.as_ref().cloned(),
            })),
        })
        .await
        .map_err(|e| {
            failure(format!(
                "Unable to interrupt thread {}: {}",
                input.thread_id, e.message
            ))
        })?;
        serde_json::from_value(
            json!({"threadId":target.thread.id,"runId":run.id,"status":"interrupt_requested"}),
        )
        .map_err(failure)
    }

    async fn configuration(
        &self,
        caller: CallerScope,
        input: T3ThreadConfigurationInput,
    ) -> Result<T3ThreadConfigurationResult, ToolError> {
        self.caller(&caller, true)?;
        let target = input.thread_id.as_ref().unwrap_or(&caller.thread_id.0);
        let (_, target) = self.scoped(&caller, target, false).map_err(|e| {
            if e.code == Code::ThreadNotFound {
                ToolError::new(
                    Code::ThreadNotFound,
                    "The thread was not found in the calling project.",
                )
            } else if e.code == Code::OrchestrationError {
                failure("The operation could not be completed.")
            } else {
                e
            }
        })?;
        serde_json::from_value(json!({"threadId":target.thread.id,"modelSelection":target.thread.model_selection,
            "runtimeMode":target.thread.runtime_mode,"interactionMode":target.thread.interaction_mode})).map_err(failure)
    }

    async fn configure(
        &self,
        caller: CallerScope,
        input: T3ThreadConfigureInput,
    ) -> Result<T3ThreadConfigureResult, ToolError> {
        let target = self.caller(&caller, true)?;
        if target.thread.archived_at.is_some()
            || active_run(&target).is_none()
            || target.thread.provider_instance_id != caller.provider_instance_id
        {
            return Err(ToolError::new(
                Code::ParentNotActive,
                "The calling provider no longer owns an active thread run.",
            ));
        }
        let selection = zeron_proto::orchestration::normalize_contract(
            "ModelSelection",
            json!(input.model_selection),
        )
        .map_err(|_| failure("The operation could not be completed."))?;
        let model_selection: zeron_proto::provider_instance::ModelSelection =
            serde_json::from_value(selection.clone()).map_err(failure)?;
        let mut requested =
            json!({"providerInstanceId":model_selection.instance_id,"model":model_selection.model});
        if let Some(options) = model_selection.options.as_ref() {
            requested["options"] = json!(options);
        }
        let requested: DelegateTaskInputTarget =
            serde_json::from_value(requested).map_err(failure)?;
        self.delegation
            .targets
            .resolve(&target.thread, Some(&requested))
            .await
            .map_err(|_| failure("The operation could not be completed."))?;
        let kind = if target.thread.provider_instance_id == model_selection.instance_id {
            "thread.model-selection.set"
        } else {
            "provider.switch"
        };
        let command = Command::wire(
            serde_json::from_value(
                json!({"type":kind,"commandId":format!("mcp:{}",uuid::Uuid::new_v4()),
            "threadId":target.thread.id,"modelSelection":selection}),
            )
            .map_err(failure)?,
        )
        .map_err(failure)?;
        let sequence = self
            .command(command)
            .await
            .map_err(|_| failure("The operation could not be completed."))?;
        serde_json::from_value(json!({"sequence":sequence})).map_err(failure)
    }

    async fn create(
        &self,
        caller: CallerScope,
        input: CreateThreadsInput,
    ) -> Result<CreateThreadsResult, ToolError> {
        let parent = self.caller(&caller, false)?;
        if active_run(&parent).is_none_or(|r| {
            r.root_node_id.is_none() || r.provider_instance_id != caller.provider_instance_id
        }) {
            return Err(ToolError::new(
                Code::ParentNotActive,
                "Thread creation requires an active run owned by this MCP provider session.",
            ));
        }
        self.delegation.create_threads(caller, input).await
    }
}
