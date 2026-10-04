//! T3 schedule toolkit facade: capability checks stay in the toolkit; all
//! domain validation/authority/error translation stays here.
use std::sync::{Arc, PoisonError, RwLock};

use serde_json::{Value, json};
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::{OrchestrationToolInput, OrchestratorMcpFailureCode as Code};
use zeron_proto::{InteractionMode, RuntimeMode};

use super::service::SchedulerService;
use super::{Scheduler, SchedulerError};
use crate::mcp::codec;
use crate::orchestration::event::mcp_command_id;
use crate::orchestration::service::{CallerScope, ToolError};
use crate::orchestration::{Error, projection::ThreadProjection};

pub async fn dispatch(
    service: &RwLock<Option<Arc<dyn SchedulerService>>>,
    caller: CallerScope,
    input: OrchestrationToolInput,
) -> Value {
    let service = service
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let Some(service) = service else {
        return codec::result(codec::unavailable());
    };
    codec::result(match service.invoke(caller, input).await {
        Ok(value) => value,
        Err(error) => serde_json::to_value(error.into_failure()).expect("failure"),
    })
}

pub fn summary(task: &ScheduledTask) -> Value {
    json!({
        "scheduledTaskId":task.id,"title":task.title,"prompt":task.prompt,"enabled":task.enabled,
        "projectId":task.project_id,"boundThreadId":task.thread_id,"schedule":task.schedule,
        "nextRunAt":task.next_run_at,"lastRunStatus":task.last_run_status
    })
}

fn strategy(bound: bool) -> Value {
    if bound {
        json!({"type":"root"})
    } else {
        json!({"type":"worktree","baseRef":"main","startFromOrigin":true})
    }
}

fn generic() -> ToolError {
    ToolError::new(
        Code::OrchestrationError,
        "The operation could not be completed.",
    )
}

fn mapped(prefix: &str, error: SchedulerError) -> ToolError {
    ToolError::new(
        Code::OrchestrationError,
        format!("{prefix}: {}", error.message),
    )
}

fn decode<T: serde::de::DeserializeOwned>(name: &str, value: Value) -> Result<T, ToolError> {
    let value = normalize_contract(name, value).map_err(|_| generic())?;
    serde_json::from_value(value).map_err(|_| generic())
}

fn schedule_value(value: Value) -> Result<Value, ToolError> {
    let value = match value {
        Value::String(encoded) => serde_json::from_str(&encoded).map_err(|_| generic())?,
        value => value,
    };
    normalize_contract("ScheduledTaskUpsertSchedule", value).map_err(|_| generic())
}

impl Scheduler {
    fn parent(&self, caller: &CallerScope, manual: bool) -> Result<ThreadProjection, ToolError> {
        let parent = self
            .store
            .thread(&caller.thread_id)
            .map_err(|_| generic())?;
        let Some(parent) = parent else {
            return Err(if manual {
                ToolError::new(Code::ThreadNotFound, "The calling thread was not found.")
            } else {
                ToolError::new(
                    Code::OrchestrationError,
                    format!(
                        "Unable to read thread {}: The operation could not be completed.",
                        caller.thread_id
                    ),
                )
            });
        };
        if manual {
            if parent.thread.deleted_at.is_some() {
                return Err(ToolError::new(
                    Code::ThreadNotFound,
                    "The calling thread was not found.",
                ));
            }
            let active = parent
                .runs
                .iter()
                .rev()
                .find(|run| !crate::orchestration::command::run_terminal(&run.status));
            // T3 readMutationCaller checks shell/provider ownership, not the
            // credential's original runId (warm sessions can advance turns).
            if parent.thread.archived_at.is_some()
                || active.is_none()
                || parent.thread.provider_instance_id != caller.provider_instance_id
            {
                return Err(ToolError::new(
                    Code::ParentNotActive,
                    "The calling provider no longer owns an active thread run.",
                ));
            }
            if parent.thread.runtime_mode != RuntimeMode::FullAccess
                || parent.thread.interaction_mode != InteractionMode::Default
            {
                return Err(ToolError::new(
                    Code::CapabilityDenied,
                    "Running a scheduled task requires a live full-access/default thread.",
                ));
            }
        }
        Ok(parent)
    }

    fn scoped(
        &self,
        project: &ProjectId,
        id: &ScheduledTaskId,
    ) -> Result<ScheduledTask, ToolError> {
        let tasks = self
            .list()
            .map_err(|error| mapped("Could not load scheduled task", error))?;
        tasks
            .tasks
            .into_iter()
            .find(|task| &task.id == id && &task.project_id == project)
            .ok_or_else(|| {
                ToolError::new(
                    Code::TaskNotFound,
                    format!("Scheduled task {id} was not found in the calling project."),
                )
            })
    }
}

#[async_trait::async_trait]
impl SchedulerService for Scheduler {
    async fn invoke(
        &self,
        caller: CallerScope,
        input: OrchestrationToolInput,
    ) -> Result<Value, ToolError> {
        let manual = matches!(&input, OrchestrationToolInput::RunScheduledTaskNow(_));
        let parent = self.parent(&caller, manual)?;
        match input {
            OrchestrationToolInput::ScheduleTask(input) => {
                let args = serde_json::to_value(input).map_err(|_| generic())?;
                let prompt = args["prompt"].as_str().ok_or_else(generic)?;
                let bound = args["bindToCurrentThread"].as_bool().unwrap_or(true);
                let first_line = prompt.split('\n').next().unwrap_or("").trim();
                // T3 uses JS slice(0,80); preserve UTF-16 length (Rust's
                // scalar boundary cannot represent an isolated surrogate).
                let title = args["title"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        if first_line.is_empty() {
                            "Scheduled task".into()
                        } else {
                            String::from_utf16_lossy(
                                &first_line.encode_utf16().take(80).collect::<Vec<_>>(),
                            )
                        }
                    });
                let mut value = json!({
                    "title":title,"prompt":prompt,"enabled":args["enabled"].as_bool().unwrap_or(true),
                    "schedule":schedule_value(args["schedule"].clone())?,
                    "projectId":parent.thread.project_id,
                    "threadId":if bound { Some(&caller.thread_id) } else { None },
                    "workspaceStrategy":strategy(bound),"modelSelection":parent.thread.model_selection,
                    "runtimeMode":parent.thread.runtime_mode,"interactionMode":parent.thread.interaction_mode,
                    "createdBy":"agent","creationSource":"mcp"
                });
                if let Some(key) = args["clientRequestId"].as_str() {
                    value["commandId"] =
                        json!(mcp_command_id(&caller.session_id, "schedule-task", key));
                }
                let task = self
                    .upsert(decode("ScheduledTaskUpsertInput", value)?)
                    .map_err(|error| mapped("Could not schedule task", error))?;
                Ok(summary(&task))
            }
            OrchestrationToolInput::ListScheduledTasks(_) => {
                let list = self
                    .list()
                    .map_err(|error| mapped("Could not list scheduled tasks", error))?;
                Ok(json!({"tasks":list.tasks.iter()
                    .filter(|task| task.project_id == parent.thread.project_id)
                    .map(summary).collect::<Vec<_>>()}))
            }
            OrchestrationToolInput::UpdateScheduledTask(input) => {
                let args = serde_json::to_value(&input).map_err(|_| generic())?;
                let id = ScheduledTaskId(input.scheduled_task_id.clone());
                self.scoped(&parent.thread.project_id, &id)?;
                let patch_schedule = args
                    .get("schedule")
                    .cloned()
                    .map(schedule_value)
                    .transpose()?;
                let task = self
                    .mutate(&id, "Could not save schedule task.", |existing, now| {
                        let Some(mut task) = existing else {
                            return Err(Error::Invariant("Schedule task not found.".into()));
                        };
                        // Recheck project ownership in the same transaction as the
                        // edit, so a concurrent owner form cannot move it first.
                        if task.project_id != parent.thread.project_id {
                            return Err(Error::Invariant("Schedule task not found.".into()));
                        }
                        let old = task.clone();
                        if let Some(title) = args["title"].as_str() {
                            task.title = title.into();
                        }
                        if let Some(prompt) = args["prompt"].as_str() {
                            task.prompt = prompt.into();
                        }
                        if let Some(enabled) = args["enabled"].as_bool() {
                            task.enabled = enabled;
                        }
                        if let Some(schedule) = patch_schedule {
                            task.schedule = serde_json::from_value(schedule)?;
                        }
                        if let Some(bound) = args["bindToCurrentThread"].as_bool() {
                            task.thread_id = bound.then(|| caller.thread_id.clone());
                            task.workspace_strategy = serde_json::from_value(strategy(bound))?;
                        }
                        if task.enabled != old.enabled
                            || !super::schedule::same_schedule(&task.schedule, &old.schedule)
                        {
                            task.next_run_at = self.next(&task, now);
                        }
                        task.updated_at = crate::orchestration::event::iso(now)?;
                        Ok(task)
                    })
                    .map_err(|error| mapped("Could not update scheduled task", error))?;
                Ok(summary(&task))
            }
            OrchestrationToolInput::DeleteScheduledTask(input) => {
                let id = ScheduledTaskId(input.scheduled_task_id);
                self.scoped(&parent.thread.project_id, &id)?;
                self.delete(&id)
                    .map_err(|error| mapped("Could not delete scheduled task", error))?;
                Ok(json!({"scheduledTaskId":id,"deleted":true}))
            }
            OrchestrationToolInput::RunScheduledTaskNow(input) => {
                let id = ScheduledTaskId(input.task_id);
                let list = self.list().map_err(|_| generic())?;
                if !list
                    .tasks
                    .iter()
                    .any(|task| task.id == id && task.project_id == parent.thread.project_id)
                {
                    return Err(ToolError::new(
                        Code::InvalidRequest,
                        "The task was not found in the calling project.",
                    ));
                }
                let task = self.run_now(&id).await.map_err(|_| generic())?;
                Ok(
                    json!({"taskId":task.id,"threadId":task.thread_id,"lastRunStatus":task.last_run_status,
                    "runCount":task.run_count,"nextRunAt":task.next_run_at}),
                )
            }
            _ => Err(generic()),
        }
    }
}
