//! Stable thread boundary shared by MCP, launches, schedules and transfers.
//! CallerScope is authenticated authority; never construct it from tool input.
use async_trait::async_trait;
use zeron_proto::orchestration_mcp::*;

use super::service::{CallerScope, ToolError};
use super::threads::timeline::ThreadReadPage;

#[async_trait]
pub trait ThreadService: Send + Sync + 'static {
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
