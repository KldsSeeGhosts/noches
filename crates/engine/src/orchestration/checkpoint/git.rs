use super::super::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use zeron_proto::transfer::{FileCheckpoint, RestorePath};

pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
pub fn io(error: std::io::Error) -> Error {
    Error::Invariant(format!("Checkpoint filesystem: {error}"))
}

pub async fn run(root: &Path, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>> {
    use tokio::io::AsyncWriteExt;
    let mut cmd = tokio::process::Command::new("git");
    cmd.arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.fsync=objects,reference",
            "-c",
            "core.fsyncMethod=fsync",
        ])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_AUTHOR_NAME", "Noches")
        .env("GIT_AUTHOR_EMAIL", "noches@localhost")
        .env("GIT_COMMITTER_NAME", "Noches")
        .env("GIT_COMMITTER_EMAIL", "noches@localhost")
        .kill_on_drop(true);
    let mut child = cmd.spawn().map_err(io)?;
    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input)
            .await
            .map_err(io)?;
    } else {
        drop(child.stdin.take());
    }
    let output = child.wait_with_output().await.map_err(io)?;
    if !output.status.success() {
        return Err(Error::Invariant(format!(
            "Checkpoint git {}: {}",
            args.first().unwrap_or(&""),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

async fn text(root: &Path, args: &[&str]) -> Result<String> {
    String::from_utf8(run(root, args, None).await?)
        .map(|s| s.trim().into())
        .map_err(|_| Error::Invariant("Checkpoint Git output is not UTF-8.".into()))
}

pub async fn root(cwd: &Path) -> Result<PathBuf> {
    let root = text(cwd, &["rev-parse", "--show-toplevel"]).await?;
    std::fs::canonicalize(root).map_err(io)
}

/// TODO(parity-checkpoints): port GitVcsDriver's sparse/private-index and
/// submodule policy before offering restore for partially materialized trees.
pub async fn ensure_full_checkout(root: &Path) -> Result<()> {
    if text(root, &["config", "--bool", "core.sparseCheckout"])
        .await
        .ok()
        .as_deref()
        == Some("true")
    {
        return Err(Error::Invariant(
            "File checkpoints for sparse checkouts are not implemented yet.".into(),
        ));
    }
    let index = run(root, &["ls-files", "--stage", "-z"], None).await?;
    if index
        .split(|b| *b == 0)
        .any(|entry| entry.starts_with(b"160000 "))
    {
        return Err(Error::Invariant(
            "File checkpoints for submodule checkouts are not implemented yet.".into(),
        ));
    }
    Ok(())
}

pub async fn is_linked_worktree(root: &Path) -> Result<bool> {
    let git = text(root, &["rev-parse", "--path-format=absolute", "--git-dir"]).await?;
    let common = text(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .await?;
    Ok(std::fs::canonicalize(git).map_err(io)? != std::fs::canonicalize(common).map_err(io)?)
}

/// OS lock on the checkout-local git dir (not the common dir): unrelated
/// worktrees remain independent. Kernel InstanceLock still owns SQLite.
pub struct CheckoutLock {
    _file: std::fs::File,
}
impl CheckoutLock {
    pub async fn acquire(root: &Path) -> Result<Self> {
        let git = text(root, &["rev-parse", "--path-format=absolute", "--git-dir"]).await?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(Path::new(&git).join("noches-checkpoint.lock"))
            .map_err(io)?;
        file.try_lock()
            .map_err(|e| Error::Invariant(format!("Checkpoint checkout is locked: {e}")))?;
        Ok(Self { _file: file })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Captured {
    pub head: Option<String>,
    pub tree: String,
    pub index_tree: String,
}
pub struct Observation {
    pub head: Option<String>,
    pub tree: String,
    pub checksum: String,
}

async fn head(root: &Path) -> Option<String> {
    text(root, &["rev-parse", "--verify", "HEAD"]).await.ok()
}
pub async fn observe(root: &Path) -> Result<Observation> {
    let head = head(root).await;
    let tree = crate::diff_sync::snapshot_tree(root)
        .await
        .map_err(|e| Error::Invariant(e.to_string()))?;
    let index_tree = text(root, &["write-tree"]).await?;
    let status = run(
        root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        None,
    )
    .await?;
    let mut digest = Sha256::new();
    for bytes in [
        head.as_deref().unwrap_or("").as_bytes(),
        tree.as_bytes(),
        index_tree.as_bytes(),
        &status,
    ] {
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    Ok(Observation {
        head,
        tree,
        checksum: format!("{:x}", digest.finalize()),
    })
}

pub async fn capture(root: &Path, reference: &str) -> Result<Captured> {
    if text(root, &["rev-parse", "--verify", reference])
        .await
        .is_ok()
    {
        let message = text(root, &["show", "-s", "--format=%B", reference]).await?;
        return Ok(serde_json::from_str(&message)?); // recovery after ref write before SQL
    }
    let head = head(root).await;
    let tree = crate::diff_sync::snapshot_tree(root)
        .await
        .map_err(|e| Error::Invariant(e.to_string()))?;
    let index_tree = text(root, &["write-tree"]).await?;
    let captured = Captured {
        head,
        tree,
        index_tree,
    };
    // Separate commit keeps the original index tree reachable without storing
    // workspace data in SQLite. Checkpoint ref itself is CAS-create-only.
    let index_commit = String::from_utf8(
        run(
            root,
            &["commit-tree", &captured.index_tree],
            Some(b"Noches checkpoint index\n"),
        )
        .await?,
    )
    .map_err(|e| Error::Invariant(e.to_string()))?;
    let index_commit = index_commit.trim();
    let message = serde_json::to_vec(&captured)?;
    let commit = String::from_utf8(
        run(
            root,
            &["commit-tree", &captured.tree, "-p", index_commit],
            Some(&message),
        )
        .await?,
    )
    .map_err(|e| Error::Invariant(e.to_string()))?;
    let zero = "0".repeat(commit.trim().len());
    run(root, &["update-ref", reference, commit.trim(), &zero], None).await?;
    Ok(captured)
}

pub async fn verify_ref(root: &Path, record: &FileCheckpoint) -> Result<()> {
    if text(
        root,
        &[
            "rev-parse",
            "--verify",
            &format!("{}^{{tree}}", record.checkpoint.r#ref.0),
        ],
    )
    .await?
        != record.tree_sha
    {
        return Err(Error::Invariant(
            "Checkpoint ref is missing or stale.".into(),
        ));
    }
    if std::fs::canonicalize(&record.cwd).map_err(io)? != root {
        return Err(Error::Invariant(
            "Checkpoint workspace identity changed.".into(),
        ));
    }
    Ok(())
}

fn strings(bytes: Vec<u8>) -> Result<Vec<String>> {
    bytes
        .split(|b| *b == 0)
        .filter(|v| !v.is_empty())
        .map(|v| {
            String::from_utf8(v.to_vec())
                .map_err(|_| Error::Invariant("Checkpoint path is not UTF-8.".into()))
        })
        .collect()
}

pub async fn restore_paths(root: &Path, current: &str, target: &str) -> Result<Vec<RestorePath>> {
    let records = strings(
        run(
            root,
            &[
                "diff",
                "--no-renames",
                "--name-status",
                "-z",
                current,
                target,
                "--",
            ],
            None,
        )
        .await?,
    )?;
    let mut paths = Vec::new();
    for pair in records.chunks(2) {
        if pair.len() != 2 {
            return Err(Error::Invariant("Invalid checkpoint path diff.".into()));
        }
        validate_path(&pair[1])?;
        paths.push(RestorePath {
            kind: pair[0].clone(),
            path: pair[1].clone(),
        });
    }
    Ok(paths)
}
fn validate_path(path: &str) -> Result<()> {
    if Path::new(path).is_absolute()
        || path
            .split('/')
            .any(|p| matches!(p, "" | "." | ".." | ".git"))
    {
        return Err(Error::Invariant(
            "Checkpoint contains an unsafe path.".into(),
        ));
    }
    Ok(())
}

pub async fn file_summary(root: &Path, previous: &str, tree: &str) -> Result<Vec<Value>> {
    let paths = restore_paths(root, previous, tree).await?;
    let stats = strings(
        run(
            root,
            &[
                "diff",
                "--no-renames",
                "--numstat",
                "-z",
                previous,
                tree,
                "--",
            ],
            None,
        )
        .await?,
    )?;
    Ok(paths.into_iter().map(|p| {
        let counts = stats.iter().find_map(|s| {
            let mut fields = s.splitn(3,'\t');
            let a = fields.next()?; let d = fields.next()?; let name = fields.next()?;
            (name == p.path).then(|| (a.parse::<i64>().unwrap_or(0), d.parse::<i64>().unwrap_or(0)))
        }).unwrap_or((0,0));
        json!({"path":p.path,"kind":match p.kind.as_str() {"A"=>"added","D"=>"deleted",_=>"modified"},"additions":counts.0,"deletions":counts.1})
    }).collect())
}

pub async fn ignored_collision(root: &Path, paths: &[RestorePath]) -> Result<bool> {
    let ignored = strings(
        run(
            root,
            &[
                "ls-files",
                "--others",
                "--ignored",
                "--exclude-standard",
                "-z",
            ],
            None,
        )
        .await?,
    )?;
    Ok(ignored.iter().any(|i| {
        paths
            .iter()
            .any(|p| Path::new(i).starts_with(&p.path) || Path::new(&p.path).starts_with(i))
    }))
}

pub async fn restore(
    root: &Path,
    checkpoint: &FileCheckpoint,
    paths: &[RestorePath],
) -> Result<()> {
    // Recheck immediately before mutation; ignored data isn't in the checksum.
    if ignored_collision(root, paths).await? {
        return Err(Error::Invariant(
            "Checkpoint restore would overwrite ignored files.".into(),
        ));
    }
    let tracked = run(
        root,
        &[
            "ls-files",
            "--cached",
            &format!("--with-tree={}", checkpoint.tree_sha),
            "-z",
            "--",
            ".",
        ],
        None,
    )
    .await?;
    if !tracked.is_empty() {
        run(
            root,
            &[
                "restore",
                "--source",
                &checkpoint.checkpoint.r#ref.0,
                "--worktree",
                "--staged",
                "--",
                ".",
            ],
            None,
        )
        .await?;
    }
    // Delete only previewed nonignored paths, never a blanket clean. The backup
    // already contains their complete data. Git clean does not follow symlinks
    // and refuses nested repositories without the intentionally absent -ff.
    for path in paths.iter().filter(|p| p.kind == "D") {
        validate_path(&path.path)?;
        run(root, &["clean", "-fd", "--", &path.path], None).await?;
    }
    // T3 restores files then unstages relative to current HEAD. No branch move.
    if head(root).await.is_some() {
        run(root, &["reset", "--quiet", "--", "."], None).await?;
    }
    let observed = observe(root).await?;
    if observed.tree != checkpoint.tree_sha {
        return Err(Error::Invariant(
            "Checkpoint restore was incomplete; inspect retained backup.".into(),
        ));
    }
    Ok(())
}
