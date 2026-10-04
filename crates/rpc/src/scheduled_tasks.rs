//! Typed owner-routed Settings client API. An unavailable owner fails at the
//! transport, never falls back to scheduling on the viewer.
use zeron_proto::scheduler::*;

use crate::{RpcClient, RpcError, RpcSubscription};

pub mod methods {
    pub const LIST: &str = "ListScheduledTasks";
    pub const WATCH: &str = "WatchScheduledTasks";
    pub const CREATE: &str = "CreateScheduledTask";
    pub const UPDATE: &str = "UpdateScheduledTask";
    pub const DELETE: &str = "DeleteScheduledTask";
    pub const RUN_NOW: &str = "RunScheduledTaskNow";
}

fn routed<T: serde::Serialize>(input: &T, owner: &str) -> Result<serde_json::Value, RpcError> {
    if owner.trim().is_empty() {
        return Err(RpcError::BadParams(
            "Scheduled task ownerHostId is required.".into(),
        ));
    }
    let mut value = serde_json::to_value(input).map_err(|e| RpcError::BadParams(e.to_string()))?;
    value["targetDeviceId"] = serde_json::json!(owner);
    Ok(value)
}

impl RpcClient {
    pub async fn list_scheduled_tasks(
        &self,
        input: ScheduledTasksListRequest,
    ) -> Result<ScheduledTasksView, RpcError> {
        self.call_as(methods::LIST, routed(&input, &input.owner_host_id)?)
            .await
    }

    /// First item is a full snapshot, then coalesced snapshots after changes.
    pub async fn watch_scheduled_tasks(
        &self,
        input: ScheduledTasksListRequest,
    ) -> Result<RpcSubscription, RpcError> {
        self.subscribe_scoped(methods::WATCH, routed(&input, &input.owner_host_id)?)
            .await
    }

    pub async fn create_scheduled_task(
        &self,
        input: ScheduledTaskCreateRequest,
    ) -> Result<ScheduledTaskViewResult, RpcError> {
        self.call_as(methods::CREATE, routed(&input, &input.owner_host_id)?)
            .await
    }

    pub async fn update_scheduled_task(
        &self,
        input: ScheduledTaskUpdateRequest,
    ) -> Result<ScheduledTaskViewResult, RpcError> {
        self.call_as(methods::UPDATE, routed(&input, &input.owner_host_id)?)
            .await
    }

    pub async fn delete_scheduled_task(
        &self,
        input: ScheduledTaskActionRequest,
    ) -> Result<ScheduledTaskDeleted, RpcError> {
        self.call_as(methods::DELETE, routed(&input, &input.owner_host_id)?)
            .await
    }

    pub async fn run_scheduled_task_now(
        &self,
        input: ScheduledTaskActionRequest,
    ) -> Result<ScheduledTaskViewResult, RpcError> {
        self.call_as(methods::RUN_NOW, routed(&input, &input.owner_host_id)?)
            .await
    }
}
