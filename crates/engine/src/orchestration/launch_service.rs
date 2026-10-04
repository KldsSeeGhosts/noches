//! Separate wave-3 domain seam; does not expand OrchestratorService.
use async_trait::async_trait;
use serde_json::Value;

use super::service::ToolError;
use crate::mcp::auth::InvocationScope;

#[async_trait]
pub trait LaunchService: Send + Sync + 'static {
    /// Input has already passed the pinned T3 descriptor validator.
    async fn call(&self, scope: &InvocationScope, name: &str, input: Value) -> Value;
    /// Signed URLs are bearer capabilities independent of the MCP credential.
    async fn upload(&self, token: &str, bytes: &[u8]) -> (u16, Value);
}

/// TODO(merge-threads): replace with the threads slice's canonical intake.
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
        thread: &str,
        message_id: &str,
        text: &str,
        attachments: Vec<Value>,
        queue: bool,
    ) -> Result<Value, SendFailure>;
    async fn detach(&self, thread: &str) -> Result<(), ToolError>;
}
