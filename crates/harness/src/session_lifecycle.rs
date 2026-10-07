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
    /// Legacy source turn without a native cursor: fork at head, then revert
    /// this many terminal turns that follow the selected boundary (T3's
    /// fork-then-rollback fallback). Only honored when the adapter reports
    /// [`SessionLifecycle::supports_fork_rollback`]; `source_turn_id` wins.
    pub rollback_turns: Option<usize>,
    pub cwd: String,
    pub model: String,
    pub runtime_mode: RuntimeMode,
    pub interaction_mode: InteractionMode,
    pub mcp: SessionMcpContext,
}

#[async_trait]
pub trait SessionLifecycle: Send + Sync {
    fn can_fork_from_turn(&self) -> bool;
    /// The adapter can trim a head fork with its own paginated revert API when
    /// the selected turn has no native reference.
    fn supports_fork_rollback(&self) -> bool {
        false
    }
    /// Definite pre-flight: the adapter can fork exactly this boundary right
    /// now. False routes to portable context before any fork is attempted
    /// (nothing is recorded as in flight), unlike a failed `fork_thread`,
    /// which must be treated as uncertain.
    async fn can_fork_now(&self, _request: &NativeForkRequest) -> Result<bool, HarnessError> {
        Ok(true)
    }
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
