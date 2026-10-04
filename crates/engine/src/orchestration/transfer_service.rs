//! Domain boundary, deliberately separate from OrchestratorService.
use super::service::{CallerScope, ToolError};
use async_trait::async_trait;
use zeron_proto::orchestration_mcp::*;

#[async_trait]
pub trait TransferService: Send + Sync + 'static {
    async fn fork(
        &self,
        caller: CallerScope,
        input: T3ThreadForkInput,
    ) -> Result<T3ThreadForkResult, ToolError>;
    async fn merge_back(
        &self,
        caller: CallerScope,
        input: T3ThreadMergeBackInput,
    ) -> Result<T3ThreadMergeBackResult, ToolError>;
    async fn transfers(
        &self,
        caller: CallerScope,
        input: T3ThreadTransfersInput,
    ) -> Result<T3ThreadTransfersResult, ToolError>;
}
