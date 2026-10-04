use crate::orchestration::service::{CallerScope, OrchestratorService, ToolError};
use zeron_proto::orchestration_mcp::{
    DelegateTaskInput, DelegateTaskResult, OrchestratorMcpFailureCode, TaskCancelInput,
    TaskCancelResult, TaskStatusInput, TaskStatusResult,
};

pub struct UnavailableOrchestratorService;

fn unavailable() -> ToolError {
    ToolError::new(
        OrchestratorMcpFailureCode::OrchestrationError,
        "The operation could not be completed.",
    )
}

#[async_trait::async_trait]
impl OrchestratorService for UnavailableOrchestratorService {
    async fn delegate_task(
        &self,
        _: CallerScope,
        _: DelegateTaskInput,
    ) -> Result<DelegateTaskResult, ToolError> {
        Err(unavailable())
    }
    async fn task_status(
        &self,
        _: CallerScope,
        _: TaskStatusInput,
    ) -> Result<TaskStatusResult, ToolError> {
        Err(unavailable())
    }
    async fn task_cancel(
        &self,
        _: CallerScope,
        _: TaskCancelInput,
    ) -> Result<TaskCancelResult, ToolError> {
        Err(unavailable())
    }
}
