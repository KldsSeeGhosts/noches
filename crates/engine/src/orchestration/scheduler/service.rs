//! Domain-owned MCP seam; does not extend the shared OrchestratorService.
use serde_json::Value;
use zeron_proto::orchestration_mcp::OrchestrationToolInput;

use crate::orchestration::service::{CallerScope, ToolError};

#[async_trait::async_trait]
pub trait SchedulerService: Send + Sync + 'static {
    async fn invoke(
        &self,
        caller: CallerScope,
        input: OrchestrationToolInput,
    ) -> Result<Value, ToolError>;
}
