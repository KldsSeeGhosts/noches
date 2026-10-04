//! Durable files-only checkpoints. Conversation rollback is deliberately absent.
//! An independent index (diff_sync) snapshots every tracked/nonignored path;
//! immutable hidden refs keep objects alive across GC and engine restarts.
mod git;
#[cfg(test)]
mod tests;

use super::event::{encode_component, iso};
use super::transfer::TransferOperation;
use super::{Error, Kernel, Result};
use rusqlite::{Connection, OptionalExtension};
use serde_json::json;
use std::path::{Path, PathBuf};
use zeron_proto::orchestration::*;
use zeron_proto::transfer::{FileCheckpoint, RestorePreview, RestoreResult};

pub const SHARED_WORKSPACE_RESTORE_MESSAGE: &str = "File restore requires an isolated worktree. This workspace may contain changes from another thread. Rewind the conversation without restoring files instead.";

pub(crate) fn record(conn: &Connection, id: &CheckpointId) -> Result<Option<FileCheckpoint>> {
    let value: Option<String> = conn
        .query_row(
            "SELECT payload_json FROM orchestration_file_checkpoints WHERE id=?1",
            [&id.0],
            |row| row.get(0),
        )
        .optional()?;
    value.map(|v| Ok(serde_json::from_str(&v)?)).transpose()
}

pub(crate) fn timeline(conn: &Connection, thread: &ThreadId) -> Result<Vec<FileCheckpoint>> {
    let mut stmt = conn.prepare("SELECT payload_json FROM orchestration_file_checkpoints WHERE thread_id=?1 ORDER BY captured_at,id")?;
    stmt.query_map([&thread.0], |row| row.get::<_, String>(0))?
        .map(|v| Ok(serde_json::from_str(&v?)?))
        .collect()
}

impl super::Store {
    pub fn checkpoint_timeline(&self, id: &ThreadId) -> Result<Vec<FileCheckpoint>> {
        self.read(|conn| timeline(conn, id))
    }
    pub fn file_checkpoint(&self, id: &CheckpointId) -> Result<Option<FileCheckpoint>> {
        self.read(|conn| record(conn, id))
    }
}

#[derive(Clone)]
pub struct FileCheckpointService {
    pub kernel: Kernel,
}

impl FileCheckpointService {
    /// Called before turn dispatch and after completed provider work. Repeated
    /// observations reuse a deterministic ref and receipted publication.
    pub async fn capture_turn(
        &self,
        thread: &ThreadId,
        run: &OrchestrationV2Run,
        phase: &str,
    ) -> Result<FileCheckpoint> {
        if !matches!(phase, "started" | "completed") {
            return Err(Error::Invariant("Invalid checkpoint phase.".into()));
        }
        let projection = self
            .kernel
            .store
            .thread(thread)?
            .ok_or_else(|| Error::Invariant("Checkpoint thread missing.".into()))?;
        let cwd = projection
            .thread
            .worktree_path
            .as_ref()
            .ok_or_else(|| Error::Invariant("Checkpoint workspace missing.".into()))?;
        let cwd = git::root(Path::new(cwd)).await?;
        let _lock = git::CheckoutLock::acquire(&cwd).await?;
        git::ensure_full_checkout(&cwd).await?;
        let id = CheckpointId(format!(
            "checkpoint:{}:{phase}",
            encode_component(&run.id.0)
        ));
        if let Some(existing) = self.kernel.store.file_checkpoint(&id)? {
            return Ok(existing);
        }
        self.capture_locked(thread, run, phase, id, &cwd).await
    }

    async fn capture_locked(
        &self,
        thread: &ThreadId,
        run: &OrchestrationV2Run,
        phase: &str,
        id: CheckpointId,
        cwd: &Path,
    ) -> Result<FileCheckpoint> {
        let now = crate::now_ms();
        let reference = format!("refs/noches/checkpoints/{}", encode_component(&id.0));
        let captured = git::capture(cwd, &reference).await?;
        let previous = self
            .kernel
            .store
            .checkpoint_timeline(thread)?
            .into_iter()
            .rev()
            .find(|c| c.checkpoint.run_id.as_ref() == Some(&run.id) && c.phase != "backup");
        let files = git::file_summary(
            cwd,
            previous
                .as_ref()
                .map(|p| p.tree_sha.as_str())
                .or(captured.head.as_deref())
                .unwrap_or(git::EMPTY_TREE),
            &captured.tree,
        )
        .await?;
        let record = FileCheckpoint {
            checkpoint: serde_json::from_value(json!({
                "id":id,"threadId":thread,"scopeId":format!("checkpoint-scope:{}",encode_component(&run.id.0)),
                "runId":run.id,"nodeId":run.root_node_id.clone().unwrap_or_else(|| NodeId(format!("checkpoint-node:{}",run.id.0))),
                "parentCheckpointId":previous.as_ref().map(|p| &p.checkpoint.id),
                "ordinalWithinScope":if phase == "started" {0} else {1},
                "appRunOrdinal":run.ordinal,"ref":reference,"status":"ready","files":files,"capturedAt":iso(now)?
            }))?,
            scope: serde_json::from_value(json!({
                "id":format!("checkpoint-scope:{}",encode_component(&run.id.0)),
                "threadId":thread,"runId":run.id,"nodeId":run.root_node_id.clone().unwrap_or_else(|| NodeId(format!("checkpoint-node:{}",run.id.0))),
                "parentScopeId":null,"providerThreadId":run.provider_thread_id,"kind":"root_run",
                "ordinalWithinParent":run.ordinal,"advancesAppRunCount":true,"cwd":cwd,"createdAt":iso(now)?
            }))?,
            cwd: cwd.to_string_lossy().into_owned(),
            head_sha: captured.head,
            tree_sha: captured.tree,
            index_tree_sha: captured.index_tree,
            phase: phase.into(),
        };
        let receipt = self
            .kernel
            .transfer_command(
                thread,
                CommandId(format!("capture:{}", id.0)),
                TransferOperation::Checkpoint {
                    record: Box::new(record.clone()),
                },
            )
            .await?;
        if receipt.status == super::ReceiptStatus::Rejected {
            return Err(Error::Invariant(receipt.error.unwrap_or_default()));
        }
        Ok(record)
    }

    fn owned(&self, thread: &ThreadId, id: &CheckpointId) -> Result<FileCheckpoint> {
        self.kernel
            .store
            .file_checkpoint(id)?
            .filter(|c| &c.checkpoint.thread_id == thread)
            .ok_or_else(|| Error::Invariant("Checkpoint was not found in this thread.".into()))
    }

    /// Mirror T3 realpath containment checks including archived threads,
    /// nested scopes and nonstopped single-thread provider sessions.
    async fn isolated(&self, thread: &ThreadId, cwd: &Path) -> Result<bool> {
        let projection = self
            .kernel
            .store
            .thread(thread)?
            .ok_or_else(|| Error::Invariant("Checkpoint thread missing.".into()))?;
        let Some(worktree) = &projection.thread.worktree_path else {
            return Ok(false);
        };
        if std::fs::canonicalize(worktree).map_err(git::io)? != cwd {
            return Ok(false);
        }
        // Noches adoption historically puts the project root into worktreePath.
        // Detect main checkouts using Git, not that legacy metadata field.
        if !git::is_linked_worktree(cwd).await? {
            return Ok(false);
        }
        let others = self.kernel.store.read(|conn| {
            let mut stmt =
                conn.prepare("SELECT id FROM orchestration_projection_threads WHERE id<>?1")?;
            let ids = stmt
                .query_map([&thread.0], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids.into_iter()
                .map(|id| {
                    super::projection::read_thread(conn, &ThreadId(id)).and_then(|p| {
                        p.ok_or_else(|| Error::Invariant("Thread disappeared.".into()))
                    })
                })
                .collect::<Result<Vec<_>>>()
        })?;
        for other in others.into_iter().filter(|p| p.thread.deleted_at.is_none()) {
            let mut paths: Vec<PathBuf> = other
                .thread
                .worktree_path
                .iter()
                .map(PathBuf::from)
                .collect();
            if other.thread.worktree_path.is_none() {
                // Unknown legacy project-root ownership cannot prove isolation.
                return Ok(false);
            }
            paths.extend(
                self.kernel
                    .store
                    .checkpoint_timeline(&other.thread.id)?
                    .iter()
                    .map(|c| PathBuf::from(&c.cwd)),
            );
            paths.extend(
                super::task::records(&other, "checkpoint-scope")
                    .iter()
                    .filter_map(|scope| scope["cwd"].as_str().map(PathBuf::from)),
            );
            paths.extend(super::task::records(&other, "provider-session").iter()
                .filter(|s| s["status"] != "stopped" && s["capabilities"]["sessions"]["supportsMultipleProviderThreadsPerSession"] != true)
                .filter_map(|s| s["cwd"].as_str().map(PathBuf::from)));
            for path in paths {
                match std::fs::canonicalize(path) {
                    Ok(other) if other.starts_with(cwd) || cwd.starts_with(&other) => {
                        return Ok(false);
                    }
                    Ok(_) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(git::io(e)),
                }
            }
        }
        Ok(true)
    }

    async fn preview_locked(
        &self,
        thread: &ThreadId,
        checkpoint: &FileCheckpoint,
        cwd: &Path,
    ) -> Result<RestorePreview> {
        git::ensure_full_checkout(cwd).await?;
        git::verify_ref(cwd, checkpoint).await?;
        let observation = git::observe(cwd).await?;
        let paths = git::restore_paths(cwd, &observation.tree, &checkpoint.tree_sha).await?;
        let mut refusal = None;
        if !self.isolated(thread, cwd).await? {
            refusal = Some(SHARED_WORKSPACE_RESTORE_MESSAGE.into());
        } else if self.kernel.store.thread(thread)?.is_some_and(|p| {
            p.runs
                .iter()
                .any(|r| !super::command::run_terminal(&r.status))
        }) {
            refusal = Some("File restore requires an idle thread.".into());
        } else if git::ignored_collision(cwd, &paths).await? {
            refusal = Some("Checkpoint restore would overwrite ignored files.".into());
        }
        Ok(RestorePreview {
            checkpoint_id: checkpoint.checkpoint.id.0.clone(),
            cwd: checkpoint.cwd.clone(),
            head_sha: observation.head,
            checksum: observation.checksum,
            paths,
            restores_staging: true,
            allowed: refusal.is_none(),
            refusal,
        })
    }

    pub async fn preview(&self, thread: &ThreadId, id: &CheckpointId) -> Result<RestorePreview> {
        let checkpoint = self.owned(thread, id)?;
        let cwd = git::root(Path::new(&checkpoint.cwd)).await?;
        let _lock = git::CheckoutLock::acquire(&cwd).await?;
        self.preview_locked(thread, &checkpoint, &cwd).await
    }

    /// All refusals precede workspace mutation. A hidden backup is durable before
    /// restore; a failed restore never attempts a broad automatic rollback.
    pub async fn restore(
        &self,
        thread: &ThreadId,
        id: &CheckpointId,
        expected_head: Option<&str>,
        expected_checksum: &str,
    ) -> Result<RestoreResult> {
        let _thread_guards = self.kernel.locks.acquire([thread.clone()]).await;
        let checkpoint = self.owned(thread, id)?;
        let cwd = git::root(Path::new(&checkpoint.cwd)).await?;
        let _lock = git::CheckoutLock::acquire(&cwd).await?;
        let preview = self.preview_locked(thread, &checkpoint, &cwd).await?;
        if let Some(refusal) = preview.refusal {
            return Err(Error::Invariant(refusal));
        }
        if preview.head_sha.as_deref() != expected_head {
            return Err(Error::Invariant(
                "Checkpoint restore refused: HEAD changed since preview.".into(),
            ));
        }
        if preview.checksum != expected_checksum {
            return Err(Error::Invariant(
                "Checkpoint restore refused: workspace changed since preview.".into(),
            ));
        }
        let backup_id = CheckpointId(format!("checkpoint:backup:{}", uuid::Uuid::new_v4()));
        let reference = format!("refs/noches/checkpoints/{}", encode_component(&backup_id.0));
        let captured = git::capture(&cwd, &reference).await?;
        let mut backup = checkpoint.clone();
        backup.checkpoint.id = backup_id.clone();
        backup.checkpoint.r#ref = CheckpointRef(reference);
        backup.checkpoint.parent_checkpoint_id = Some(id.clone());
        backup.checkpoint.captured_at = iso(crate::now_ms())?;
        backup.checkpoint.run_id = None; // distinct backups never dedupe with turn capture
        backup.phase = "backup".into();
        backup.tree_sha = captured.tree;
        backup.index_tree_sha = captured.index_tree;
        backup.head_sha = captured.head;
        backup.checkpoint.files = serde_json::from_value(json!(
            git::file_summary(&cwd, &checkpoint.tree_sha, &backup.tree_sha).await?
        ))?;
        // Already holding this thread's kernel lock: dispatch synchronously to
        // avoid reacquiring it while still publishing the ordinary checkpoint
        // event/receipt/barrier, including a new passive UI version.
        let receipt = self.kernel.store.dispatch(
            &super::Command {
                id: CommandId(format!("backup:{}", backup_id.0)),
                thread_id: thread.clone(),
                operation: super::Operation::Transfer(Box::new(TransferOperation::Checkpoint {
                    record: Box::new(backup),
                })),
            },
            crate::now_ms(),
        )?;
        if receipt.status == super::ReceiptStatus::Rejected {
            return Err(Error::Invariant(receipt.error.unwrap_or_default()));
        }
        // Revalidate after backup creation, which can take seconds on a large repo.
        let observation = git::observe(&cwd).await?;
        if observation.head.as_deref() != expected_head || observation.checksum != expected_checksum
        {
            return Err(Error::Invariant(format!(
                "Checkpoint restore refused: workspace changed while creating backup {}.",
                backup_id.0
            )));
        }
        if !self.isolated(thread, &cwd).await? {
            return Err(Error::Invariant(SHARED_WORKSPACE_RESTORE_MESSAGE.into()));
        }
        git::restore(&cwd, &checkpoint, &preview.paths)
            .await
            .map_err(|e| {
                Error::Invariant(format!(
                    "File restore failed; backup {} is retained: {e}",
                    backup_id.0
                ))
            })?;
        Ok(RestoreResult {
            restored: true,
            backup_checkpoint_id: backup_id.0,
        })
    }
}
