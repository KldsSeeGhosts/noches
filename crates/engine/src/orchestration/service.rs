//! Shared engine/MCP boundary. Scope is minted by the authenticated session,
//! never accepted from a tool's JSON arguments.
use std::path::PathBuf;

use zeron_proto::orchestration::{ProjectId, RunId, ThreadId};
use zeron_proto::orchestration_mcp::{
    DelegateTaskInput, DelegateTaskResult, OrchestratorMcpFailure, OrchestratorMcpFailureCode,
    OrchestratorMcpFailureTag, TaskCancelInput, TaskCancelResult, TaskStatusInput, TaskStatusResult,
};
use zeron_proto::provider_instance::ProviderInstanceId;
use zeron_proto::{InteractionMode, RuntimeMode};

#[derive(Debug, Clone)]
pub struct CallerScope {
    pub thread_id: ThreadId,
    pub run_id: RunId,
    /// Host-local authenticated MCP session identity (not a native thread ID).
    pub session_id: String,
    pub project_id: ProjectId,
    pub workspace_root: PathBuf,
    pub runtime_mode: RuntimeMode,
    pub interaction_mode: InteractionMode,
    pub provider_instance_id: ProviderInstanceId,
}

/// The exact T3 orchestration failure family. Transport/framework validation
/// errors are owned by the MCP layer rather than flattened into this family.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct ToolError {
    pub code: OrchestratorMcpFailureCode,
    pub message: String,
}

impl ToolError {
    pub fn new(code: OrchestratorMcpFailureCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn into_failure(self) -> OrchestratorMcpFailure {
        OrchestratorMcpFailure {
            _tag: OrchestratorMcpFailureTag::OrchestratorMcpFailure,
            code: self.code,
            message: self.message,
        }
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
