//! Separate wave-3 domain seam; does not expand OrchestratorService.
use async_trait::async_trait;
use serde_json::Value;

use super::service::ToolError;
use crate::mcp::auth::InvocationScope;
use zeron_proto::orchestration::*;
use zeron_proto::provider_instance::ModelSelection;

/// Trusted host launch input; never decoded from MCP arguments.
pub struct HostThreadLaunchRequest {
    pub command_id: CommandId,
    pub project_id: ProjectId,
    pub title: String,
    pub model_selection: ModelSelection,
    pub runtime_mode: zeron_proto::RuntimeMode,
    pub interaction_mode: zeron_proto::InteractionMode,
    pub workspace_strategy: OrchestrationV2ThreadLaunchWorkspaceStrategy,
    pub message_id: MessageId,
    pub scheduled_task_id: ScheduledTaskId,
    pub text: String,
    pub created_by: OrchestrationV2Actor,
    pub creation_source: OrchestrationV2CreationSource,
}

#[async_trait]
pub trait ScheduledThreadLaunch: Send + Sync + 'static {
    async fn launch_scheduled(&self, input: HostThreadLaunchRequest) -> Result<(), ToolError>;
}

#[async_trait]
pub trait LaunchService: Send + Sync + 'static {
    /// Input has already passed the pinned T3 descriptor validator.
    async fn call(&self, scope: &InvocationScope, name: &str, input: Value) -> Value;
    /// Signed URLs are bearer capabilities independent of the MCP credential.
    async fn upload(&self, token: &str, bytes: &[u8]) -> (u16, Value);
    /// Durable cleanup effects; None means only thread-owned setup terminals.
    async fn cleanup(
        &self,
        thread: &str,
        attachment_ids: Option<Vec<String>>,
    ) -> Result<(), ToolError>;
}

/// An error with uncertain=true MUST retain claimed files.
#[derive(Debug)]
pub struct SendFailure {
    pub error: ToolError,
    pub uncertain: bool,
}

#[async_trait]
pub trait LaunchThreadIntake: Send + Sync + 'static {
    async fn send(
        &self,
        input: super::thread_service::ThreadSendRequest,
    ) -> Result<Value, SendFailure>;
    async fn detach(&self, thread: &str) -> Result<(), ToolError>;
}
