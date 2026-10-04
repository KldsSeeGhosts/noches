//! Settings read model and host-authorized owner actions. No UI execution path.
use std::sync::Arc;

use futures::StreamExt;
use serde_json::Value;
use zeron_proto::orchestration::*;
use zeron_proto::scheduler::*;
use zeron_rpc::scheduled_tasks::methods;
use zeron_rpc::{RpcError, RpcReply, parse_params};

use super::scheduler::{Scheduler, SchedulerError, schedule};
use super::{Error, event};

pub fn is_method(method: &str) -> bool {
    matches!(
        method,
        methods::LIST
            | methods::WATCH
            | methods::CREATE
            | methods::UPDATE
            | methods::DELETE
            | methods::RUN_NOW
    )
}

pub fn view(task: ScheduledTask) -> ScheduledTaskView {
    ScheduledTaskView {
        cadence: schedule::cadence(&task.schedule),
        last_run: ScheduledTaskLastRun {
            status: task.last_run_status.clone(),
            at: task.last_run_at.clone(),
            error: task.last_run_error.clone(),
        },
        task,
    }
}

fn failure(error: SchedulerError) -> RpcError {
    RpcError::Failed(error.message)
}

fn owner(service: &Scheduler, id: &str) -> Result<(), RpcError> {
    if id != service.store.host_id.as_ref() {
        return Err(RpcError::Failed(
            "Scheduled task actions require the owning host.".into(),
        ));
    }
    Ok(())
}

fn list(
    service: &Scheduler,
    request: &ScheduledTasksListRequest,
) -> Result<ScheduledTasksView, RpcError> {
    owner(service, &request.owner_host_id)?;
    let list = service.list().map_err(failure)?;
    Ok(ScheduledTasksView {
        owner_host_id: request.owner_host_id.clone(),
        tasks: list
            .tasks
            .into_iter()
            .filter(|task| {
                request
                    .project_id
                    .as_ref()
                    .is_none_or(|id| &task.project_id == id)
            })
            .map(view)
            .collect(),
    })
}

impl Scheduler {
    pub fn update_for_user(
        &self,
        request: ScheduledTaskUpdateRequest,
    ) -> Result<ScheduledTask, RpcError> {
        owner(self, &request.owner_host_id)?;
        let id = ScheduledTaskId(request.id.clone());
        self.mutate(&id, "Could not update schedule task.", |old, now| {
            let Some(mut task) = old else {
                return Err(Error::Invariant("Schedule task not found.".into()));
            };
            let old_schedule = task.schedule.clone();
            let old_enabled = task.enabled;
            if let Some(title) = request.title {
                task.title = title;
            }
            if let Some(prompt) = request.prompt {
                task.prompt = prompt;
            }
            if let Some(enabled) = request.enabled {
                task.enabled = enabled;
            }
            if let Some(schedule) = request.schedule {
                normalize_contract(
                    "ScheduledTaskUpsertSchedule",
                    serde_json::to_value(&schedule)?,
                )
                .map_err(Error::Invariant)?;
                task.schedule = serde_json::from_value(serde_json::to_value(schedule)?)?;
            }
            if let Optional::Present(thread) = request.thread_id {
                task.thread_id = thread;
            }
            if let Some(strategy) = request.workspace_strategy {
                task.workspace_strategy = strategy;
            }
            if let Some(model) = request.model_selection {
                task.model_selection = model;
            }
            if let Some(mode) = request.runtime_mode {
                task.runtime_mode = mode;
            }
            if let Some(mode) = request.interaction_mode {
                task.interaction_mode = mode;
            }
            // Explicit schedule edits use the writable floor; otherwise a
            // legacy short interval may still be paused or renamed.
            if !schedule::same_schedule(&task.schedule, &old_schedule) {
                normalize_contract(
                    "ScheduledTaskUpsertSchedule",
                    serde_json::to_value(&task.schedule)?,
                )
                .map_err(Error::Invariant)?;
            }
            let value = normalize_contract("ScheduledTask", serde_json::to_value(&task)?)
                .map_err(Error::Invariant)?;
            task = serde_json::from_value(value)?;
            if task.enabled != old_enabled
                || !schedule::same_schedule(&task.schedule, &old_schedule)
            {
                task.next_run_at = self.next(&task, now);
            }
            task.updated_at = event::iso(now)?;
            Ok(task)
        })
        .map_err(failure)
    }
}

pub async fn dispatch(
    service: &Arc<Scheduler>,
    method: &str,
    params: Value,
) -> Result<RpcReply, RpcError> {
    match method {
        methods::LIST => {
            let input = parse_params(params)?;
            RpcReply::value(&list(service, &input)?)
        }
        methods::WATCH => {
            let input: ScheduledTasksListRequest = parse_params(params)?;
            // Subscribe before validating/taking the initial snapshot.
            let rx = service.subscribe();
            let first = serde_json::to_value(list(service, &input)?)
                .map_err(|e| RpcError::Failed(e.to_string()))?;
            let service = service.clone();
            let stream = futures::stream::unfold(
                (rx, service, input, Some(first)),
                |(mut rx, service, input, first)| async move {
                    let value = if let Some(value) = first {
                        value
                    } else {
                        rx.changed().await.ok()?;
                        rx.borrow_and_update();
                        match list(&service, &input) {
                            Ok(value) => serde_json::to_value(value).ok()?,
                            Err(error) => {
                                tracing::warn!(%error, "Could not refresh scheduled task settings");
                                return None;
                            }
                        }
                    };
                    Some((value, (rx, service, input, None)))
                },
            )
            .boxed();
            Ok(RpcReply::Stream(stream))
        }
        methods::CREATE => {
            let request: ScheduledTaskCreateRequest = parse_params(params)?;
            owner(service, &request.owner_host_id)?;
            let mut input = request.input;
            if input.id.as_ref().is_some() || input.require_existing.as_ref() == Some(&true) {
                return Err(RpcError::BadParams(
                    "CreateScheduledTask must not edit an existing task.".into(),
                ));
            }
            input.created_by = Optional::Present(OrchestrationV2Actor::User);
            input.creation_source = Optional::Present(OrchestrationV2CreationSource::Web);
            RpcReply::value(&ScheduledTaskViewResult {
                task: view(service.upsert(input).map_err(failure)?),
            })
        }
        methods::UPDATE => {
            let request = parse_params(params)?;
            RpcReply::value(&ScheduledTaskViewResult {
                task: view(service.update_for_user(request)?),
            })
        }
        methods::DELETE | methods::RUN_NOW => {
            let request: ScheduledTaskActionRequest = parse_params(params)?;
            owner(service, &request.owner_host_id)?;
            let id = ScheduledTaskId(request.id);
            if method == methods::DELETE {
                RpcReply::value(&service.delete(&id).map_err(failure)?)
            } else {
                RpcReply::value(&ScheduledTaskViewResult {
                    task: view(service.run_now(&id).await.map_err(failure)?),
                })
            }
        }
        _ => Err(RpcError::Failed("Unknown scheduled task operation.".into())),
    }
}
