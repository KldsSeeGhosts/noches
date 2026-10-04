//! Thread-owned PR links live in the existing event-backed thread projection:
//! receipts, rebuilds, publication barriers and restart persistence stay unified.
pub(crate) mod activity;
pub mod chains;
pub mod host;
pub mod identity;
pub mod mcp;
mod planner;
pub mod reactor;
pub mod settlement;
pub mod watch;
pub(crate) use planner::plan;
#[cfg(test)]
mod tests;

use super::{
    Kernel, Result,
    command::{Command, Operation},
};
use async_trait::async_trait;
pub use identity::Identity;
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::Arc;
use zeron_proto::orchestration::*;

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct PrError {
    #[serde(rename = "_tag")]
    pub tag: String,
    pub message: String,
    #[serde(flatten)]
    pub fields: serde_json::Map<String, Value>,
}
impl PrError {
    pub fn new(tag: &str) -> Self {
        let message = match tag {
            "PullRequestUrlInvalidError" => {
                "This is not a recognised pull request URL. Pass repository and number instead."
            }
            "PullRequestTargetIncompleteError" => "Pass either url, or both repository and number.",
            "PullRequestHostRequiredError" => {
                "This thread's project has no recognised remote. Pass host or url."
            }
            "PullRequestLinkFailedError" => "Could not link the pull request.",
            "PullRequestUnlinkFailedError" => "Could not unlink the pull request.",
            "PullRequestListFailedError" => "Could not list the pull request.",
            _ => "Could not change whether the pull request is watched.",
        };
        Self {
            tag: tag.into(),
            message: message.into(),
            fields: if tag.ends_with("FailedError") {
                json!({"cause":null}).as_object().unwrap().clone()
            } else {
                Default::default()
            },
        }
    }
    pub fn capability(scope: &crate::mcp::auth::InvocationScope) -> Self {
        Self {
            tag: "McpCapabilityUnavailableError".into(),
            message: "MCP credential does not grant the pull-requests capability.".into(),
            fields: json!({
                "capability":"pull-requests","environmentId":scope.environment_id,
                "threadId":scope.caller.thread_id,"providerSessionId":scope.caller.session_id,
                "providerInstanceId":scope.caller.provider_instance_id,
            })
            .as_object()
            .unwrap()
            .clone(),
        }
    }
    pub fn missing(id: &ThreadId) -> Self {
        Self {
            tag: "PullRequestThreadNotFoundError".into(),
            message: format!("Thread {id} was not found."),
            fields: json!({"threadId":id}).as_object().unwrap().clone(),
        }
    }
    pub fn not_open(state: &PullRequestState) -> Self {
        let state = serde_json::to_value(state).unwrap();
        Self {
            tag: "PullRequestNotOpenError".into(),
            message: format!(
                "The pull request is {}, so there is nothing to watch.",
                state.as_str().unwrap()
            ),
            fields: json!({"state":state}).as_object().unwrap().clone(),
        }
    }
}

/// Stable merge seam for queue metadata actions and git-actions auto-link.
/// Methods are host-authority only, not agent-supplied cross-thread targeting.
#[async_trait]
pub trait PullRequestLinks: Send + Sync + 'static {
    /// T3's legacy metadata command updates both link representations inside
    /// its own receipt transaction, not through a second async command.
    fn update_metadata(
        &self,
        thread: &mut OrchestrationV2AppThread,
        linked: Option<ThreadLinkedPullRequest>,
        now: &str,
    ) -> Result<()> {
        let mut links = links_of(thread);
        let previous = thread
            .linked_pull_request
            .as_ref()
            .and_then(Option::as_ref)
            .map(legacy_identity);
        let target = linked.as_ref().map(legacy_identity);
        let watch = target.as_ref().and_then(|target| {
            links
                .iter()
                .find(|link| chains::identity(link).key() == target.key())
                .map(|link| link.watch.clone())
        });
        links.retain(|link| {
            let key = chains::identity(link).key();
            !previous.as_ref().is_some_and(|id| id.key() == key)
                && !target.as_ref().is_some_and(|id| id.key() == key)
        });
        if let Some(target) = target {
            let mut link = new_link(&target, ThreadPullRequestLinkSource::Manual, now);
            link.watch = watch.unwrap_or(Optional::Absent);
            links.push(link);
        }
        thread.linked_pull_request = Optional::Present(linked);
        thread.pull_requests = Optional::Present(links);
        Ok(())
    }
    fn links(&self, thread: &ThreadId) -> Result<Vec<ThreadPullRequestLink>>;
    async fn link(
        &self,
        thread: &ThreadId,
        target: Identity,
        source: ThreadPullRequestLinkSource,
    ) -> Result<bool>;
    async fn unlink(&self, thread: &ThreadId, target: Identity) -> Result<bool>;
    async fn set_watching(
        &self,
        thread: &ThreadId,
        target: Identity,
        watching: bool,
    ) -> Result<(bool, bool)>;
    async fn invoke(
        &self,
        caller: &super::service::CallerScope,
        name: &str,
        args: Value,
    ) -> std::result::Result<Value, PrError>;
}

fn legacy_identity(link: &ThreadLinkedPullRequest) -> Identity {
    let parsed = identity::parse_url(&link.url).filter(|id| id.number == link.number);
    if let Some(id) = parsed.as_ref()
        && (id.host == "dev.azure.com" || id.host.contains(':'))
    {
        return id.clone();
    }
    Identity {
        host: reqwest::Url::parse(&link.url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_lowercase))
            .filter(|host| !host.is_empty())
            .unwrap_or_else(|| "unknown".into()),
        repository: link.repository.trim().to_lowercase(),
        number: link.number,
        url: link.url.clone(),
    }
}

#[derive(Clone)]
pub struct PullRequestService {
    pub kernel: Kernel,
    pub host: Arc<dyn host::PullRequestHost>,
}

#[derive(Debug, Clone)]
pub enum PrOperation {
    Link {
        target: Identity,
        source: ThreadPullRequestLinkSource,
    },
    Unlink {
        target: Identity,
    },
    Watch {
        target: Identity,
        watching: bool,
    },
    Sync {
        target: Identity,
        snapshot: Box<ThreadPullRequestSnapshot>,
        stack: Option<ThreadPullRequestStack>,
        preserve_stack: bool,
    },
    WatchSync {
        target: Identity,
        started_at: String,
        watch: Option<ThreadPullRequestWatch>,
        wake: Option<watch::Wake>,
    },
    Settle {
        expected_sequence: i64,
        settled_at: String,
        auto_merge: bool,
        after_days: Option<i64>,
    },
}
impl PrOperation {
    pub fn command_type(&self) -> &'static str {
        match self {
            Self::Link { .. } => "thread.pull-request.link",
            Self::Unlink { .. } => "thread.pull-request.unlink",
            Self::Watch { .. } => "thread.pull-request.watch",
            Self::Sync { .. } => "thread.pull-request-link.sync",
            Self::WatchSync { .. } => "thread.pull-request-watch.sync",
            Self::Settle { .. } => "thread.auto-settle",
        }
    }
}

pub fn links_of(thread: &OrchestrationV2AppThread) -> Vec<ThreadPullRequestLink> {
    if let Some(links) = thread.pull_requests.as_ref() {
        return links.clone();
    }
    thread
        .linked_pull_request
        .as_ref()
        .and_then(Option::as_ref)
        .map(|legacy| {
            let target = legacy_identity(legacy);
            new_link(
                &target,
                ThreadPullRequestLinkSource::Manual,
                "1970-01-01T00:00:00.000Z",
            )
        })
        .into_iter()
        .collect()
}

fn new_link(
    target: &Identity,
    source: ThreadPullRequestLinkSource,
    now: &str,
) -> ThreadPullRequestLink {
    ThreadPullRequestLink {
        host: target.host.clone(),
        repository: target.repository.clone(),
        number: target.number,
        url: target.url.clone(),
        source,
        linked_at: now.into(),
        snapshot: None,
        stack: None,
        watch: Optional::Absent,
    }
}

impl PullRequestService {
    pub async fn command(&self, thread: &ThreadId, operation: PrOperation) -> Result<()> {
        let command = Command {
            id: CommandId(format!("server:pr:{}:{}", thread.0, uuid::Uuid::new_v4())),
            thread_id: thread.clone(),
            operation: Operation::PullRequest(Box::new(operation)),
        };
        let receipt = self.kernel.dispatch(&command, crate::now_ms()).await?;
        if receipt.status == super::ReceiptStatus::Rejected {
            return Err(super::Error::Invariant(receipt.error.unwrap_or_default()));
        }
        Ok(())
    }
}

#[async_trait]
impl PullRequestLinks for PullRequestService {
    fn links(&self, thread: &ThreadId) -> Result<Vec<ThreadPullRequestLink>> {
        let projection =
            self.kernel.store.thread(thread)?.ok_or_else(|| {
                super::Error::Invariant(format!("Thread {thread} was not found."))
            })?;
        Ok(links_of(&projection.thread))
    }
    async fn link(
        &self,
        thread: &ThreadId,
        target: Identity,
        source: ThreadPullRequestLinkSource,
    ) -> Result<bool> {
        let _guard = self.kernel.locks.acquire([thread.clone()]).await;
        let existing = self
            .links(thread)?
            .into_iter()
            .find(|l| chains::identity(l).key() == target.key());
        if existing.is_some_and(|l| l.source != ThreadPullRequestLinkSource::StackDismissed) {
            return Ok(true);
        }
        // Already holding the thread lock: store dispatch is synchronous.
        let command = Command {
            id: CommandId(format!("server:pr-link:{}", uuid::Uuid::new_v4())),
            thread_id: thread.clone(),
            operation: Operation::PullRequest(Box::new(PrOperation::Link { target, source })),
        };
        let receipt = self.kernel.store.dispatch(&command, crate::now_ms())?;
        if receipt.status == super::ReceiptStatus::Rejected {
            return Err(super::Error::Invariant(receipt.error.unwrap_or_default()));
        }
        Ok(false)
    }
    async fn unlink(&self, thread: &ThreadId, target: Identity) -> Result<bool> {
        let _guard = self.kernel.locks.acquire([thread.clone()]).await;
        if !self
            .links(thread)?
            .iter()
            .any(|l| chains::identity(l).key() == target.key())
        {
            return Ok(false);
        }
        let command = Command {
            id: CommandId(format!("server:pr-unlink:{}", uuid::Uuid::new_v4())),
            thread_id: thread.clone(),
            operation: Operation::PullRequest(Box::new(PrOperation::Unlink { target })),
        };
        let receipt = self.kernel.store.dispatch(&command, crate::now_ms())?;
        if receipt.status == super::ReceiptStatus::Rejected {
            return Err(super::Error::Invariant(receipt.error.unwrap_or_default()));
        }
        Ok(true)
    }
    async fn set_watching(
        &self,
        thread: &ThreadId,
        target: Identity,
        watching: bool,
    ) -> Result<(bool, bool)> {
        let _guard = self.kernel.locks.acquire([thread.clone()]).await;
        let before = self.links(thread)?.into_iter().find(|l| {
            l.source != ThreadPullRequestLinkSource::StackDismissed
                && chains::identity(l).key() == target.key()
        });
        let was = before.as_ref().is_some_and(|l| l.watch.as_ref().is_some());
        let command = Command {
            id: CommandId(format!("server:pr-watch:{}", uuid::Uuid::new_v4())),
            thread_id: thread.clone(),
            operation: Operation::PullRequest(Box::new(PrOperation::Watch {
                target: target.clone(),
                watching,
            })),
        };
        let receipt = self.kernel.store.dispatch(&command, crate::now_ms())?;
        if receipt.status == super::ReceiptStatus::Rejected {
            return Err(super::Error::Invariant(receipt.error.unwrap_or_default()));
        }
        Ok((
            self.links(thread)?
                .iter()
                .any(|l| chains::identity(l).key() == target.key() && l.watch.as_ref().is_some()),
            was,
        ))
    }
    async fn invoke(
        &self,
        caller: &super::service::CallerScope,
        name: &str,
        args: Value,
    ) -> std::result::Result<Value, PrError> {
        mcp::invoke(self, caller, name, args).await
    }
}
