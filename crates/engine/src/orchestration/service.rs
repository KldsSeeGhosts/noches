//! Trusted MCP/domain seam. Credentials and transport framing stay outside it.
use zeron_proto::orchestration::{ProjectId, RunId, ThreadId};
use zeron_proto::orchestration_mcp::{
    DelegateTaskInput, DelegateTaskResult, TaskCancelInput, TaskCancelResult, TaskStatusInput,
    TaskStatusResult,
};
use zeron_proto::provider_instance::ProviderInstanceId;
use zeron_proto::{InteractionMode, RuntimeMode};

#[derive(Debug, Clone)]
pub struct CallerScope {
    pub thread_id: ThreadId,
    pub run_id: Option<RunId>,
    pub session_id: String,
    pub project_id: ProjectId,
    pub workspace_root: String,
    pub runtime_mode: RuntimeMode,
    pub interaction_mode: InteractionMode,
    pub provider_instance_id: ProviderInstanceId,
}

/// T3 public failure family, never an arbitrary backend/debug string.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct ToolError {
    pub code: zeron_proto::orchestration_mcp::OrchestratorMcpFailureCode,
    pub message: String,
}

impl ToolError {
    pub fn unavailable() -> Self {
        Self {
            code: zeron_proto::orchestration_mcp::OrchestratorMcpFailureCode::OrchestrationError,
            message: "The operation could not be completed.".into(),
        }
    }

    pub fn wire(&self) -> serde_json::Value {
        serde_json::json!({
            "_tag": "OrchestratorMcpFailure", "code": self.code, "message": self.message
        })
    }
}

#[async_trait::async_trait]
pub trait OrchestratorService: Send + Sync + 'static {
    async fn delegate_task(
        &self,
        caller: CallerScope,
        input: DelegateTaskInput,
    ) -> Result<DelegateTaskResult, ToolError>;
    async fn task_status(
        &self,
        caller: CallerScope,
        input: TaskStatusInput,
    ) -> Result<TaskStatusResult, ToolError>;
    async fn task_cancel(
        &self,
        caller: CallerScope,
        input: TaskCancelInput,
    ) -> Result<TaskCancelResult, ToolError>;
}

pub struct UnavailableOrchestratorService;

#[async_trait::async_trait]
impl OrchestratorService for UnavailableOrchestratorService {
    async fn delegate_task(
        &self,
        _: CallerScope,
        _: DelegateTaskInput,
    ) -> Result<DelegateTaskResult, ToolError> {
        Err(ToolError::unavailable())
    }
    async fn task_status(
        &self,
        _: CallerScope,
        _: TaskStatusInput,
    ) -> Result<TaskStatusResult, ToolError> {
        Err(ToolError::unavailable())
    }
    async fn task_cancel(
        &self,
        _: CallerScope,
        _: TaskCancelInput,
    ) -> Result<TaskCancelResult, ToolError> {
        Err(ToolError::unavailable())
    }
}
