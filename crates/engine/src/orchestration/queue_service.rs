//! Independent P4b service seam; do not grow OrchestratorService.
use async_trait::async_trait;
use serde_json::Value;

use super::service::{CallerScope, ToolError};

#[async_trait]
pub trait QueueService: Send + Sync + 'static {
    async fn call(&self, caller: CallerScope, name: &str, input: Value)
    -> Result<Value, ToolError>;
}
