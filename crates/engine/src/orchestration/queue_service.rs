//! Independent P4b service seam; do not grow OrchestratorService.
use async_trait::async_trait;
use serde_json::Value;

use super::service::{CallerScope, ToolError};

#[async_trait]
pub trait QueueService: Send + Sync + 'static {
    async fn call(&self, caller: CallerScope, name: &str, input: Value)
    -> Result<Value, ToolError>;
}

/// TODO(merge-pr-watch): reconcile with the PR-watch slice's link storage.
/// Pure link transformation; the returned metadata is persisted inside the
/// source transaction with thread.metadata-updated.
pub trait QueuePullRequestLinks: Send + Sync {
    fn update(&self, thread: &mut Value, link: Value) -> super::Result<()>;
}

pub struct MetadataPullRequestLinks;
impl QueuePullRequestLinks for MetadataPullRequestLinks {
    fn update(&self, thread: &mut Value, link: Value) -> super::Result<()> {
        thread["linkedPullRequest"] = link;
        Ok(())
    }
}
