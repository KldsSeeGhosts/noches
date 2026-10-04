//! Stable thread boundary shared by MCP, launches, schedules and transfers.
//! CallerScope is authenticated authority; never construct it from tool input.
use async_trait::async_trait;
use zeron_proto::orchestration::{
    CommandId, MessageId, OrchestrationV2Actor, OrchestrationV2CreationSource, ProjectId,
    ScheduledTaskId, ThreadId,
};
use zeron_proto::orchestration_mcp::*;
use zeron_proto::provider_instance::ModelSelection;

use super::service::{CallerScope, ToolError};
use super::threads::timeline::ThreadReadPage;

/// Host-authorized intake, shared by schedules and launches. Not an MCP input:
/// callers must supply their own trusted project scope and durable identities.
pub struct ThreadSendRequest {
    pub project_id: ProjectId,
    pub thread_id: ThreadId,
    pub command_id: CommandId,
    pub message_id: MessageId,
    pub scheduled_task_id: Option<ScheduledTaskId>,
    pub sender_thread_id: Option<ThreadId>,
    pub text: String,
    pub attachments: Vec<serde_json::Value>,
    pub model_selection: Option<ModelSelection>,
    pub mode: T3ThreadSendInputMode,
    pub created_by: OrchestrationV2Actor,
    pub creation_source: OrchestrationV2CreationSource,
}

#[async_trait]
pub trait ThreadService: Send + Sync + 'static {
    async fn send_to_thread(
        &self,
        input: ThreadSendRequest,
    ) -> Result<T3ThreadSendResult, ToolError>;
    async fn list(
        &self,
        caller: CallerScope,
        input: T3ThreadListInput,
    ) -> Result<T3ThreadListResult, ToolError>;
    async fn read(
        &self,
        caller: CallerScope,
        input: T3ThreadReadInput,
    ) -> Result<ThreadReadPage, ToolError>;
    async fn send(
        &self,
        caller: CallerScope,
        input: T3ThreadSendInput,
    ) -> Result<T3ThreadSendResult, ToolError>;
    /// Observational only: a deadline never interrupts or acknowledges work.
    async fn wait(
        &self,
        caller: CallerScope,
        input: T3ThreadWaitInput,
    ) -> Result<T3ThreadWaitResult, ToolError>;
    async fn interrupt(
        &self,
        caller: CallerScope,
        input: T3ThreadInterruptInput,
    ) -> Result<T3ThreadInterruptResult, ToolError>;
    async fn configuration(
        &self,
        caller: CallerScope,
        input: T3ThreadConfigurationInput,
    ) -> Result<T3ThreadConfigurationResult, ToolError>;
    async fn configure(
        &self,
        caller: CallerScope,
        input: T3ThreadConfigureInput,
    ) -> Result<T3ThreadConfigureResult, ToolError>;
    async fn create(
        &self,
        caller: CallerScope,
        input: CreateThreadsInput,
    ) -> Result<CreateThreadsResult, ToolError>;
}
