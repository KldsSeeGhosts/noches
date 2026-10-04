//! Additive Settings API; T3's persisted/MCP contracts remain in orchestration.
//!
//! Stable RPC contract (`zeron_rpc::scheduled_tasks::methods`):
//! `ListScheduledTasks` / `WatchScheduledTasks` take
//! `{"ownerHostId":"device","projectId":"project"}` (projectId may be omitted).
//! Both return `ScheduledTasksView`: `{ownerHostId,tasks:[ScheduledTaskView]}`.
//! A view flattens the complete T3 `ScheduledTask`, adding `cadence` and
//! `lastRun:{status,at,error}`. Watch emits a first snapshot then coalesced
//! snapshots, never creates work, and is cancelled on subscription drop.
//!
//! `CreateScheduledTask` takes `{ownerHostId,input:ScheduledTaskUpsertInput}`,
//! with no explicit id; input includes title, prompt, enabled, schedule,
//! projectId, threadId (nullable), workspaceStrategy, modelSelection,
//! runtimeMode and interactionMode. Actor/source are host-owned user/web.
//! `UpdateScheduledTask` takes `{ownerHostId,id,...providedFields}`. Omitted
//! fields preserve the definition; explicit `threadId:null` unbinds it. Supply
//! workspaceStrategy along with a binding edit (bound: root; unbound MCP-style:
//! `{type:"worktree",baseRef:"main",startFromOrigin:true}`).
//! Both mutations and `RunScheduledTaskNow {ownerHostId,id}` return
//! `{task:ScheduledTaskView}`. `DeleteScheduledTask {ownerHostId,id}` returns
//! `{id}`. Run-now is dispatch acceptance, not agent-turn completion; each call
//! has a new claim. Disabled schedules may be run manually.
//!
//! Typed RpcClient methods always add `targetDeviceId=ownerHostId`. Direct
//! callers must do the same. The server checks profile-host ownership; an
//! offline host never falls back to the viewer. No bearer/process data is sent.
use serde::{Deserialize, Serialize};

use crate::orchestration::{
    Optional, OrchestrationV2ThreadLaunchWorkspaceStrategy, ProjectId, ScheduledTask,
    ScheduledTaskId, ScheduledTaskRunStatus, ScheduledTaskUpsertInput, ScheduledTaskUpsertSchedule,
    ThreadId,
};
use crate::provider_instance::ModelSelection;
use crate::{InteractionMode, RuntimeMode};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTasksListRequest {
    #[serde(default)]
    pub owner_host_id: String,
    #[serde(default)]
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTaskCreateRequest {
    #[serde(default)]
    pub owner_host_id: String,
    pub input: ScheduledTaskUpsertInput,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTaskUpdateRequest {
    #[serde(default)]
    pub owner_host_id: String,
    #[serde(default)]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<ScheduledTaskUpsertSchedule>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub thread_id: Optional<Option<ThreadId>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_strategy: Option<OrchestrationV2ThreadLaunchWorkspaceStrategy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_selection: Option<ModelSelection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_mode: Option<RuntimeMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_mode: Option<InteractionMode>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTaskActionRequest {
    #[serde(default)]
    pub owner_host_id: String,
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTaskLastRun {
    pub status: ScheduledTaskRunStatus,
    #[serde(default)]
    pub at: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTaskView {
    /// Complete T3 ScheduledTask, including enabled, nextRunAt and provenance.
    #[serde(flatten)]
    pub task: ScheduledTask,
    #[serde(default)]
    pub cadence: String,
    pub last_run: ScheduledTaskLastRun,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTasksView {
    #[serde(default)]
    pub owner_host_id: String,
    #[serde(default)]
    pub tasks: Vec<ScheduledTaskView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskViewResult {
    pub task: ScheduledTaskView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskDeleted {
    pub id: ScheduledTaskId,
}
