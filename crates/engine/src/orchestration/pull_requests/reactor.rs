use super::{PrOperation, PullRequestService, chains, links_of, settlement, watch};
use crate::orchestration::{Result, projection::ThreadProjection};
use futures::StreamExt;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zeron_proto::orchestration::*;

/// TODO(merge-threads/queue): thread/project settings authority plugs in here.
/// The host default matches T3 (merge on, inactivity after three days); the selector accepts
/// full lifecycle/settings inputs without adding them to OrchestratorService.
pub trait PrThreadContext: Send + Sync {
    fn cwd(&self, thread: &OrchestrationV2AppThread) -> Option<PathBuf>;
    fn settlement_settings(&self, _thread: &OrchestrationV2AppThread) -> (bool, Option<i64>) {
        (true, Some(3))
    }
}

pub struct PullRequestReactor {
    pub service: Arc<PullRequestService>,
    pub context: Arc<dyn PrThreadContext>,
    failures: HashMap<String, usize>,
    last_sync: HashMap<String, i64>,
    retry_stacks: HashSet<String>,
}
impl PullRequestReactor {
    pub fn new(service: Arc<PullRequestService>, context: Arc<dyn PrThreadContext>) -> Self {
        Self {
            service,
            context,
            failures: HashMap::new(),
            last_sync: HashMap::new(),
            retry_stacks: HashSet::new(),
        }
    }
    pub fn spawn(mut self, stop: CancellationToken) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop.cancelled() => break,
                    result = self.sweep() => { if let Err(error) = result { tracing::warn!(%error,"PR watch sweep failed"); } }
                }
                tokio::select! { _ = stop.cancelled() => break, _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {} }
            }
        })
    }
    fn threads(&self) -> Result<Vec<ThreadProjection>> {
        self.service.kernel.store.read(|conn| {
            let mut stmt =
                conn.prepare("SELECT id FROM orchestration_projection_threads ORDER BY id")?;
            let ids = stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids.into_iter()
                .filter_map(|id| {
                    crate::orchestration::projection::read_thread(conn, &ThreadId(id)).transpose()
                })
                .collect()
        })
    }
    pub async fn sweep(&mut self) -> Result<()> {
        let mut groups: Vec<(String, Vec<(ThreadProjection, ThreadPullRequestLink)>)> = vec![];
        let mut active_failures = HashSet::new();
        for thread in self.threads()? {
            if thread.thread.archived_at.is_some() || thread.thread.deleted_at.is_some() {
                continue;
            }
            for link in links_of(&thread.thread)
                .into_iter()
                .filter(|l| l.source != ThreadPullRequestLinkSource::StackDismissed)
            {
                if let Some(w) = link.watch.as_ref() {
                    active_failures.insert(format!(
                        "{} {} {}",
                        thread.thread.id,
                        chains::identity(&link).key(),
                        w.started_at
                    ));
                }
                let key = chains::identity(&link).key();
                if let Some((_, entries)) = groups.iter_mut().find(|(k, _)| k == &key) {
                    entries.push((thread.clone(), link));
                } else {
                    groups.push((key, vec![(thread.clone(), link)]));
                }
            }
        }
        self.failures.retain(|k, _| active_failures.contains(k));
        let present: HashSet<_> = groups.iter().map(|(k, _)| k.clone()).collect();
        self.last_sync.retain(|k, _| present.contains(k));
        self.retry_stacks.retain(|k| present.contains(k));
        let mut pending = vec![];
        for (key, entries) in groups {
            let now = crate::now_ms();
            let watched = entries
                .iter()
                .any(|(t, l)| l.watch.as_ref().is_some() && !settled(&t.thread));
            let merged = entries.iter().all(|(_, l)| {
                l.snapshot
                    .as_ref()
                    .is_some_and(|s| s.state == PullRequestState::Merged)
            });
            if merged && !self.retry_stacks.contains(&key) {
                for (thread, link) in &entries {
                    if let Some(watch) = link.watch.as_ref() {
                        let _ = self
                            .service
                            .command(
                                &thread.thread.id,
                                PrOperation::WatchSync {
                                    target: chains::identity(link),
                                    started_at: watch.started_at.clone(),
                                    watch: None,
                                    wake: None,
                                },
                            )
                            .await;
                    }
                }
                self.settle_entries(&entries).await?;
                continue;
            }
            let due = watched
                || self.retry_stacks.contains(&key)
                || entries.iter().any(|(t, l)| {
                    l.snapshot.is_none()
                        || (l
                            .snapshot
                            .as_ref()
                            .is_some_and(|s| s.state == PullRequestState::Open)
                            && !settled(&t.thread))
                })
                || self
                    .last_sync
                    .get(&key)
                    .is_none_or(|last| now - last >= 900_000);
            if !due {
                self.settle_entries(&entries).await?;
                continue;
            }
            let cwd = self.context.cwd(&entries[0].0.thread);
            pending.push((key, entries, watched, cwd));
        }
        let host = self.service.host.clone();
        let mut reads = futures::stream::iter(pending)
            .map(move |(key, entries, watched, cwd)| {
                let host = host.clone();
                async move {
                    let read = match cwd {
                        Some(cwd) => {
                            host.read(&cwd, &chains::identity(&entries[0].1), watched)
                                .await
                        }
                        None => {
                            Err(crate::source_control::ChangeRequestError::RepositoryUnavailable)
                        }
                    };
                    (key, entries, read)
                }
            })
            .buffer_unordered(4);
        while let Some((key, entries, read)) = reads.next().await {
            let now = crate::now_ms();
            match read {
                Err(error) => {
                    tracing::debug!(%error, pull_request=%key,"PR host read failed");
                    for (thread, link) in &entries {
                        let Some(watch) = link.watch.as_ref().filter(|_| !settled(&thread.thread))
                        else {
                            continue;
                        };
                        let failure_key =
                            format!("{} {} {}", thread.thread.id, key, watch.started_at);
                        let count = self.failures.entry(failure_key.clone()).or_default();
                        *count += 1;
                        if *count >= 15 {
                            let wake = watch::Wake {
                                text: format!(
                                    "T3 Code stopped watching pull request #{} ({}) because it could not read it from the host for 15 minutes. Check it yourself, and call watch_pull_request to watch it again.",
                                    link.number, link.url
                                ),
                                notification: json!({"source":{"kind":"monitor"},"outcome":"failed","summary":format!("#{}: stopped watching, could not read it",link.number)}),
                            };
                            let stopped = self
                                .service
                                .command(
                                    &thread.thread.id,
                                    PrOperation::WatchSync {
                                        target: chains::identity(link),
                                        started_at: watch.started_at.clone(),
                                        watch: None,
                                        wake: Some(wake),
                                    },
                                )
                                .await
                                .is_ok();
                            let stopped = if stopped {
                                true
                            } else {
                                // A refused wake is not a host failure. Stop
                                // without news if the thread settled mid-read.
                                self.service
                                    .command(
                                        &thread.thread.id,
                                        PrOperation::WatchSync {
                                            target: chains::identity(link),
                                            started_at: watch.started_at.clone(),
                                            watch: None,
                                            wake: None,
                                        },
                                    )
                                    .await
                                    .is_ok()
                            };
                            if stopped {
                                self.failures.remove(&failure_key);
                            }
                        }
                    }
                }
                Ok(read) => {
                    self.last_sync.insert(key.clone(), now);
                    if read.stack_read_failed {
                        self.retry_stacks.insert(key.clone());
                    } else {
                        self.retry_stacks.remove(&key);
                    }
                    for (thread, link) in &entries {
                        // Stack discovery + snapshots precede settlement, atomically.
                        let changed = snapshot_changed(link.snapshot.as_ref(), &read.snapshot)
                            || link.stack != read.stack
                            || read.stack.as_ref().is_some_and(|stack| {
                                stack.layers.iter().any(|layer| {
                                    !links_of(&thread.thread).iter().any(|existing| {
                                        let existing_key = chains::identity(existing);
                                        let link_key = chains::identity(link);
                                        existing.number == layer.number
                                            && existing_key.host == link_key.host
                                            && existing_key.repository == link_key.repository
                                    })
                                })
                            });
                        let synced = if read.stack_read_failed || !changed {
                            Ok(())
                        } else {
                            self.service
                                .command(
                                    &thread.thread.id,
                                    PrOperation::Sync {
                                        target: chains::identity(link),
                                        snapshot: Box::new(read.snapshot.clone()),
                                        stack: read.stack.clone(),
                                        preserve_stack: read.stack_read_failed,
                                    },
                                )
                                .await
                        };
                        if let Err(error) = synced {
                            tracing::debug!(%error,"PR snapshot sync refused");
                            continue;
                        }
                        let Some(baseline) = link.watch.as_ref() else {
                            continue;
                        };
                        if settled(&thread.thread)
                            && read.snapshot.state != PullRequestState::Merged
                        {
                            continue;
                        }
                        self.failures.remove(&format!(
                            "{} {} {}",
                            thread.thread.id, key, baseline.started_at
                        ));
                        let report =
                            watch::evaluate(baseline, &read.detail, read.remarks.as_deref());
                        let open = read.snapshot.state == PullRequestState::Open;
                        let wake = (open && !report.changes.is_empty()).then(|| {
                            watch::message(
                                link.number,
                                &link.url,
                                &read.snapshot.base_branch,
                                &report,
                            )
                        });
                        if !open || report.next != *baseline || wake.is_some() {
                            // CAS rejects reads overtaken by unwatch/restart/settle.
                            let result = self
                                .service
                                .command(
                                    &thread.thread.id,
                                    PrOperation::WatchSync {
                                        target: chains::identity(link),
                                        started_at: baseline.started_at.clone(),
                                        watch: (open && !report.exhausted).then_some(report.next),
                                        wake,
                                    },
                                )
                                .await;
                            if let Err(error) = result {
                                tracing::debug!(%error,"PR watch sync refused");
                            }
                        }
                    }
                    if !read.stack_read_failed {
                        self.settle_entries(&entries).await?;
                    }
                }
            }
        }
        Ok(())
    }
    async fn settle_entries(
        &self,
        entries: &[(ThreadProjection, ThreadPullRequestLink)],
    ) -> Result<()> {
        let mut visited = HashSet::new();
        for (thread, _) in entries {
            if !visited.insert(thread.thread.id.clone()) {
                continue;
            }
            let Some(current) = self.service.kernel.store.thread(&thread.thread.id)? else {
                continue;
            };
            let (auto_merge, after_days) = self.context.settlement_settings(&current.thread);
            let links = links_of(&current.thread);
            let Some(at) = settlement::resolve(
                &settlement::candidate(&current),
                &links,
                crate::now_ms(),
                auto_merge,
                after_days,
            ) else {
                continue;
            };
            // A branch reused for fresh open work must not settle on its old PR.
            if let Some(branch) = &current.thread.branch
                && let Some(cwd) = self.context.cwd(&current.thread)
            {
                match self.service.host.branch_is_open(&cwd, branch).await {
                    Ok(false) => {}
                    Ok(true) | Err(_) => continue,
                }
            }
            let _ = self
                .service
                .command(
                    &current.thread.id,
                    PrOperation::Settle {
                        expected_sequence: current.through_sequence,
                        settled_at: crate::orchestration::event::iso(at)?,
                        auto_merge,
                        after_days,
                    },
                )
                .await;
        }
        Ok(())
    }
}
fn settled(thread: &OrchestrationV2AppThread) -> bool {
    thread.settled_at.is_some()
        || thread.settled_override == Some(OrchestrationV2AppThreadSettledOverride::Settled)
}

fn snapshot_changed(
    previous: Option<&ThreadPullRequestSnapshot>,
    next: &ThreadPullRequestSnapshot,
) -> bool {
    let Some(previous) = previous else {
        return true;
    };
    let mut previous = serde_json::to_value(previous).expect("snapshot");
    let mut next = serde_json::to_value(next).expect("snapshot");
    previous.as_object_mut().unwrap().remove("syncedAt");
    next.as_object_mut().unwrap().remove("syncedAt");
    previous != next
}
