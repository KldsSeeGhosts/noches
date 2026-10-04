//! Optional native lifecycle, not fabricated from resume support. Adapters
//! without this interface use bounded portable context.
use crate::{HarnessError, mcp::SessionMcpContext};
use async_trait::async_trait;
use serde_json::Value;
use zeron_proto::{InteractionMode, RuntimeMode};

pub struct NativeForkRequest {
    pub source_thread_id: String,
    pub source_turn_id: Option<String>,
    /// OpenCode cuts BEFORE the next user turn, unlike Codex's lastTurnId.
    pub source_next_turn_id: Option<String>,
    pub cwd: String,
    pub model: String,
    pub runtime_mode: RuntimeMode,
    pub interaction_mode: InteractionMode,
    pub mcp: SessionMcpContext,
}

#[async_trait]
pub trait SessionLifecycle: Send + Sync {
    fn can_fork_from_turn(&self) -> bool;
    /// A failed/ambiguous response must not be retried against source head.
    async fn fork_thread(&self, request: NativeForkRequest) -> Result<String, HarnessError>;
    /// Explicit unsupported fallback, not inferred success. Native delivery
    /// must happen against the already loaded target process.
    async fn inject_history(
        &self,
        _native_thread_id: &str,
        _messages: &[Value],
        _context: &str,
    ) -> Result<bool, HarnessError> {
        Ok(false)
    }
}
