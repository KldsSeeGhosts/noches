//! V2 transactional kernel. SQLite owns execution; replicas never do.
//! `assembly` enables the production host by default. Opening only the kernel
//! starts no workers; `runner` bridges its effects to ordinary sessions.

pub(crate) mod adoption;
pub mod assembly;
mod background;
pub mod checkpoint;
pub mod command;
pub mod continuation;
pub(crate) mod controls;
#[cfg(test)]
mod delegation_live_tests;
#[cfg(test)]
mod delegation_tests;
#[cfg(test)]
mod pi_native_tests;
pub mod effects;
pub mod event;
pub mod git_actions;
mod inherited_history;
pub mod launch;
pub mod launch_service;
pub mod mailbox;
pub mod projection;
pub mod pull_requests;
pub mod queue;
pub mod queue_service;
pub mod recovery;
pub mod runner;
pub mod scheduler;
pub(crate) mod selection_sync;
pub mod service;
pub(crate) mod steering;
pub(crate) mod stop_all;
pub mod store;
pub mod sync_publish;
pub mod task;
#[cfg(test)]
mod tests;
pub mod thread_service;
pub mod threads;
pub mod transfer;
pub mod transfer_service;
pub mod ui;
pub(crate) mod ui_details;
pub mod ui_git_actions;
pub mod ui_launch;
pub mod ui_provider_instances;
pub mod ui_pull_requests;
pub mod ui_queue;
pub mod ui_scheduler;
pub mod ui_threads;
pub mod ui_transfer;
mod wake;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError, Weak};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use zeron_proto::orchestration::{OrchestrationV2DomainEvent, ThreadId};
use zeron_sync::DocsStore;

pub use command::{Command, Operation, ProviderGuard};
pub use store::{CommandReceipt, ReceiptStatus, Store, WriteBoundary};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("orchestration codec: {0}")]
    Json(#[from] serde_json::Error),
    #[error("orchestration invariant: {0}")]
    Invariant(String),
    #[error("this profile is owned by a different orchestration host")]
    NotOwner,
    #[error("injected failure at {0:?}")]
    Injected(WriteBoundary),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Weak entries prevent an unbounded lock registry. A caller retains all strong
/// references before waiting; sorting/deduplication prevents AB/BA deadlocks.
#[derive(Default)]
pub struct ThreadLocks {
    entries: Mutex<BTreeMap<String, Weak<AsyncMutex<()>>>>,
}

impl ThreadLocks {
    pub async fn acquire(
        &self,
        threads: impl IntoIterator<Item = ThreadId>,
    ) -> Vec<OwnedMutexGuard<()>> {
        let mut ids: Vec<_> = threads.into_iter().map(|id| id.0).collect();
        ids.sort();
        ids.dedup();
        let locks: Vec<_> = {
            let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
            entries.retain(|_, value| value.strong_count() > 0);
            ids.into_iter()
                .map(|id| {
                    let lock = entries.get(&id).and_then(Weak::upgrade).unwrap_or_default();
                    entries.insert(id, Arc::downgrade(&lock));
                    lock
                })
                .collect()
        };
        let mut guards = Vec::with_capacity(locks.len());
        for lock in locks {
            guards.push(lock.lock_owned().await);
        }
        guards
    }
}

#[derive(Clone)]
pub struct Kernel {
    pub store: Store,
    locks: Arc<ThreadLocks>,
}

impl Kernel {
    /// The caller must be the profile's owning engine, under its InstanceLock.
    /// Opening does not run recovery, publish documents, or execute effects.
    pub fn open(docs: Arc<DocsStore>, host_id: &str) -> Result<Self> {
        Ok(Self::from_store(Store::open(docs, host_id)?))
    }

    pub(crate) fn from_store(store: Store) -> Self {
        Self {
            locks: store.thread_locks.clone(),
            store,
        }
    }

    pub async fn dispatch(&self, command: &Command, now_ms: i64) -> Result<CommandReceipt> {
        let _guards = self.locks.acquire(command.lock_threads()).await;
        self.store.dispatch(command, now_ms)
    }

    /// Guard and projection writes are checked together inside the transaction.
    pub async fn append_provider_events(
        &self,
        thread_id: &ThreadId,
        guard: &ProviderGuard,
        events: &[OrchestrationV2DomainEvent],
        now_ms: i64,
    ) -> Result<bool> {
        let _guards = self.locks.acquire([thread_id.clone()]).await;
        self.store
            .append_provider_events(thread_id, guard, events, now_ms)
    }
}
