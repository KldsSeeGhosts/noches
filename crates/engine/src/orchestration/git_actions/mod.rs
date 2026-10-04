//! Engine-only F1 workflows. Host admission + index locks exclude Noches writers;
//! external non-cooperating processes remain an explicit Git safety limitation.
pub mod history;
pub(super) mod persistence;
mod process;
pub mod pull;
pub mod rpc;
mod writer;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Context, ensure};
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use zeron_proto::git_actions::*;

use super::{Store, ThreadLocks};
use crate::{DocHost, HarnessRegistry, Repos, SessionsEngine, Terminals, WorkspaceHost};
use persistence::{get, list, put};
use process::{git, required, run};

/// TODO(merge-pr-watch): adapt to the PR-watch slice's user-authority link API.
/// A missing implementation is reported as link_pending, never a false success.
#[async_trait]
pub trait PullRequestLinker: Send + Sync {
    async fn link(&self, thread_id: &str, url: &str) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct CreatePullRequest {
    pub cwd: PathBuf,
    pub repository: String,
    pub head: String,
    pub base: String,
    pub title: String,
    pub body: String,
}

#[async_trait]
pub trait PullRequestCreator: Send + Sync {
    async fn create(&self, request: CreatePullRequest) -> anyhow::Result<String>;
}

#[derive(Debug, Clone)]
pub struct PushCommit {
    pub cwd: PathBuf,
    pub remote_url: String,
    pub branch: String,
    pub commit: String,
}
#[async_trait]
pub trait CommitPublisher: Send + Sync {
    async fn push(&self, request: PushCommit) -> anyhow::Result<()>;
}
struct GitCommitPublisher;
#[async_trait]
impl CommitPublisher for GitCommitPublisher {
    async fn push(&self, request: PushCommit) -> anyhow::Result<()> {
        let refspec = format!("{}:refs/heads/{}", request.commit, request.branch);
        git(&request.cwd, &["push", "--", &request.remote_url, &refspec]).await?;
        Ok(())
    }
}

struct GitHubPrCreator;
#[async_trait]
impl PullRequestCreator for GitHubPrCreator {
    async fn create(&self, request: CreatePullRequest) -> anyhow::Result<String> {
        let body = tempfile::NamedTempFile::new()?;
        std::fs::write(body.path(), &request.body)?;
        let output = run(
            &request.cwd,
            "gh",
            &[
                "pr",
                "create",
                "--repo",
                &request.repository,
                "--head",
                &request.head,
                "--base",
                &request.base,
                "--title",
                &request.title,
                "--body-file",
                body.path().to_str().context("invalid body path")?,
            ],
            None,
            None,
        )
        .await?;
        output
            .lines()
            .find(|line| line.starts_with("https://"))
            .map(str::to_owned)
            .context("PR result has no URL")
    }
}

#[derive(Clone)]
pub struct GitActionsService(Arc<Inner>);

struct Inner {
    store: Store,
    repos: Repos,
    sessions: SessionsEngine,
    terminals: Terminals,
    doc_host: DocHost,
    workspace: WorkspaceHost,
    registry: Arc<HarnessRegistry>,
    locks: ThreadLocks,
    watches: Mutex<HashMap<String, watch::Sender<Value>>>,
    cancellations: Mutex<HashMap<String, CancellationToken>>,
    linker: Mutex<Option<Arc<dyn PullRequestLinker>>>,
    pr_creator: Mutex<Arc<dyn PullRequestCreator>>,
    publisher: Mutex<Arc<dyn CommitPublisher>>,
    history_roots: Vec<(HistorySource, PathBuf)>,
    workers: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl GitActionsService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        store: Store,
        repos: Repos,
        sessions: SessionsEngine,
        terminals: Terminals,
        doc_host: DocHost,
        workspace: WorkspaceHost,
        registry: Arc<HarnessRegistry>,
        history_roots: Vec<(HistorySource, PathBuf)>,
    ) -> anyhow::Result<Self> {
        let this = Self(Arc::new(Inner {
            store,
            repos,
            sessions,
            terminals,
            doc_host,
            workspace,
            registry,
            locks: ThreadLocks::default(),
            watches: Mutex::default(),
            cancellations: Mutex::default(),
            linker: Mutex::default(),
            pr_creator: Mutex::new(Arc::new(GitHubPrCreator)),
            publisher: Mutex::new(Arc::new(GitCommitPublisher)),
            history_roots,
            workers: Mutex::default(),
        }));
        // Reserve-before-effect receipts survive response loss. Never replay a
        // Git/forge mutation after process death.
        for mut state in list::<GitActionState>(&this.0.store, "action")? {
            if state.status == "running" {
                state.status = "uncertain".into();
                state.error =
                    Some("Host restarted; inspect Git/forge before another action".into());
                this.publish("action", &state.action_id, &state)?;
            }
        }
        for mut scan in list::<history::ScanJob>(&this.0.store, "scan")? {
            if scan.state.status == "running" {
                scan.state.status = "paused".into();
                this.publish("scan", &scan.state.scan_id, &scan)?;
            }
        }
        for mut import in list::<HistoryImportState>(&this.0.store, "import")? {
            if import.status == "running" {
                import.status = "paused".into();
                import.error =
                    Some("Host restarted; select remaining histories to resume safely".into());
                this.publish("import", &import.import_id, &import)?;
            }
        }
        Ok(this)
    }

    pub fn set_pr_linker(&self, linker: Arc<dyn PullRequestLinker>) {
        *self.0.linker.lock().unwrap_or_else(PoisonError::into_inner) = Some(linker);
    }

    pub fn set_pr_creator(&self, creator: Arc<dyn PullRequestCreator>) {
        *self
            .0
            .pr_creator
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = creator;
    }

    pub fn set_commit_publisher(&self, publisher: Arc<dyn CommitPublisher>) {
        *self
            .0
            .publisher
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = publisher;
    }

    fn publish<T: Serialize>(&self, ns: &str, id: &str, state: &T) -> anyhow::Result<()> {
        put(&self.0.store, ns, id, state)?;
        let value = serde_json::to_value(state)?;
        let mut watches = self
            .0
            .watches
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        watches.retain(|_, sender| sender.receiver_count() != 0);
        if let Some(sender) = watches.get(&format!("{ns}:{id}")) {
            sender.send_replace(value);
        }
        Ok(())
    }

    pub fn watch(&self, ns: &str, id: &str) -> anyhow::Result<watch::Receiver<Value>> {
        let mut watches = self
            .0
            .watches
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(sender) = watches.get(&format!("{ns}:{id}")) {
            return Ok(sender.subscribe());
        }
        let state: Value = get(&self.0.store, ns, id)?.context("unknown operation")?;
        let (tx, rx) = watch::channel(state);
        watches.insert(format!("{ns}:{id}"), tx);
        Ok(rx)
    }

    pub fn state<T: DeserializeOwned>(&self, ns: &str, id: &str) -> anyhow::Result<T> {
        get(&self.0.store, ns, id)?.context("unknown operation")
    }

    pub fn settings(&self) -> anyhow::Result<SourceControlSettings> {
        Ok(get(&self.0.store, "settings", "writer")?.unwrap_or_default())
    }

    pub fn set_settings(&self, settings: SourceControlSettings) -> anyhow::Result<()> {
        ensure!(
            settings.custom_instructions.len() <= 16_000,
            "writing instructions too long"
        );
        if let Some(id) = &settings.provider_instance_id {
            ensure!(
                self.0
                    .registry
                    .provider_instances
                    .snapshot(&self.0.registry)
                    .iter()
                    .any(|p| p.provider_instance_id.0 == *id),
                "Unknown writer provider instance"
            );
        }
        put(&self.0.store, "settings", "writer", &settings)
    }

    pub async fn status(&self, cwd: &Path) -> anyhow::Result<GitCheckout> {
        let identity = self.0.repos.checkout_identity(cwd).await?;
        ensure_checkout_ready(&identity.git_dir)?;
        // `write-tree` can lock/write Git's cache-tree extension even though it
        // is logically a read. Snapshot the index and let it touch ONLY this
        // disposable copy; the real index.lock can remain held throughout CAS.
        let temporary = tempfile::tempdir()?;
        let index = temporary.path().join("index");
        let source_index = identity.git_dir.join("index");
        if source_index.exists() {
            let meta = std::fs::symlink_metadata(&source_index)?;
            ensure!(
                meta.is_file() && meta.len() <= 64 * 1024 * 1024,
                "Invalid or oversized Git index"
            );
            std::fs::copy(source_index, &index)?;
        }
        let cwd = identity.root;
        let branch = git(&cwd, &["branch", "--show-current"])
            .await?
            .trim()
            .to_owned();
        ensure!(!branch.is_empty(), "Cannot act from detached HEAD.");
        let head = git(&cwd, &["rev-parse", "--verify", "HEAD"])
            .await
            .unwrap_or_default()
            .trim()
            .to_owned();
        let staged_tree = run(&cwd, "git", &["write-tree"], None, Some(&index))
            .await?
            .trim()
            .to_owned();
        let names = run(
            &cwd,
            "git",
            &["diff", "--cached", "--name-only", "-z"],
            None,
            Some(&index),
        )
        .await?;
        let staged_paths = names
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        let dirty = !run(
            &cwd,
            "git",
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
            None,
            Some(&index),
        )
        .await?
        .is_empty();
        let upstream = git(&cwd, &["rev-parse", "--abbrev-ref", "@{upstream}"])
            .await
            .ok()
            .map(|s| s.trim().to_owned());
        let mut remotes = Vec::new();
        for remote in git(&cwd, &["remote"]).await?.lines().take(64) {
            required(remote, "remote")?;
            let fetch_url = git(&cwd, &["remote", "get-url", remote])
                .await?
                .trim()
                .to_owned();
            let pushes = git(&cwd, &["remote", "get-url", "--push", "--all", remote]).await?;
            // Multiple push URLs would publish to more than the selected repository.
            if pushes.lines().count() != 1 {
                continue;
            }
            remotes.push(GitRemoteChoice {
                name: remote.into(),
                fetch_url,
                push_url: pushes.trim().into(),
            });
        }
        Ok(GitCheckout {
            checkout_id: identity.id,
            cwd: cwd.to_string_lossy().into_owned(),
            branch,
            head,
            staged_tree,
            staged_paths,
            dirty,
            upstream,
            remotes,
        })
    }

    fn thread_checkout(&self, thread_id: &str, cwd: &str) -> anyhow::Result<()> {
        let chat = self
            .0
            .workspace
            .chat(thread_id)?
            .context("Thread not found")?;
        ensure!(
            chat.device_id == self.0.doc_host.device_id(),
            "Thread belongs to another host"
        );
        ensure!(
            chat.cwd
                .as_deref()
                .and_then(|p| std::fs::canonicalize(p).ok())
                == Some(std::fs::canonicalize(cwd)?),
            "Thread checkout changed"
        );
        Ok(())
    }

    pub async fn preview(&self, request: GitPreviewRequest) -> anyhow::Result<GitMessagePreview> {
        self.thread_checkout(&request.thread_id, &request.cwd)?;
        required(&request.base_branch, "base branch")?;
        let checkout = self.status(Path::new(&request.cwd)).await?;
        ensure!(!checkout.staged_paths.is_empty(), "No staged changes");
        let settings = self.settings()?;
        let (commit_message, pr_title, pr_body) =
            self.generate(&request, &checkout, &settings).await?;
        ensure!(
            self.status(Path::new(&request.cwd)).await? == checkout,
            "Checkout/staged scope changed during message generation"
        );
        self.thread_checkout(&request.thread_id, &request.cwd)?;
        validate_messages(&commit_message, &pr_title, &pr_body)?;
        let preview = GitMessagePreview {
            preview_id: crate::new_id(),
            thread_id: request.thread_id,
            checkout,
            commit_message,
            pr_title,
            pr_body,
            base_branch: request.base_branch,
            settings,
        };
        put(&self.0.store, "preview", &preview.preview_id, &preview)?;
        Ok(preview)
    }

    pub async fn start(&self, request: GitActionRequest) -> anyhow::Result<GitActionState> {
        required(&request.request_id, "request id")?;
        ensure!(request.request_id.len() <= 128, "request id too long");
        let _receipt_lock = self
            .0
            .locks
            .acquire([zeron_proto::orchestration::ThreadId(format!(
                "receipt:{}",
                request.request_id
            ))])
            .await;
        if let Some(prior) = get::<GitActionRequest>(&self.0.store, "request", &request.request_id)?
        {
            ensure!(
                serde_json::to_value(&prior)? == serde_json::to_value(&request)?,
                "Idempotency key reused with different payload"
            );
            return self.state("action", &request.request_id);
        }
        let preview: GitMessagePreview = self.state("preview", &request.preview_id)?;
        self.thread_checkout(&preview.thread_id, &preview.checkout.cwd)?;
        ensure!(request.authorize_commit, "Commit authorization required");
        ensure!(
            !request.authorize_create_pr || request.authorize_push,
            "PR creation requires a separately authorized push"
        );
        ensure!(
            request.confirmed_checkout == preview.checkout,
            "Confirmed scope differs from preview"
        );
        let current = self.status(Path::new(&preview.checkout.cwd)).await?;
        require_unsigned_commit_policy(Path::new(&preview.checkout.cwd)).await?;
        ensure!(
            current == preview.checkout,
            "Checkout/staged scope changed; preview again"
        );
        validate_destination(&request, &current)?;
        let permit = self
            .0
            .sessions
            .admission()
            .begin_update(|| self.checkout_idle())
            .context("Checkout busy: stop active runs and terminals before Git actions")?;
        let state = GitActionState {
            action_id: request.request_id.clone(),
            thread_id: preview.thread_id.clone(),
            status: "running".into(),
            ..Default::default()
        };
        // The receipt is durable before any effect. Missing action state after
        // a storage failure is refused, not automatically replayed.
        put(&self.0.store, "request", &request.request_id, &request)?;
        self.publish("action", &state.action_id, &state)?;
        let initial = state.clone();
        let this = self.clone();
        self.spawn(async move {
            let _permit = permit;
            let mut state = state;
            let result = this.execute(&request, &preview, &mut state).await;
            if let Err(error) = result {
                // A command failure can still leave a commit or remote effect.
                state.status = "uncertain".into();
                state.error = Some(error.to_string());
                let _ = this.event(
                    &mut state,
                    "action",
                    "failed",
                    "Inspect Git/forge before retrying",
                );
            } else {
                state.status = "completed".into();
                let _ = this.event(&mut state, "action", "completed", "Git action completed");
            }
        });
        Ok(initial)
    }

    fn spawn(&self, future: impl std::future::Future<Output = ()> + Send + 'static) {
        let mut workers = self
            .0
            .workers
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        workers.retain(|worker| !worker.is_finished());
        workers.push(tokio::spawn(future));
    }

    fn checkout_idle(&self) -> bool {
        !self.0.sessions.any_active()
            && !self.0.sessions.has_live_harnesses()
            && !self.0.terminals.any_open()
    }

    pub async fn shutdown(&self) {
        for token in self
            .0
            .cancellations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
        {
            token.cancel();
        }
        let workers = std::mem::take(
            &mut *self
                .0
                .workers
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        for worker in workers {
            let _ = worker.await;
        }
    }

    fn event(
        &self,
        state: &mut GitActionState,
        phase: &str,
        kind: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        let sequence = state.progress.last().map_or(1, |p| p.sequence + 1);
        state.progress.push(GitProgress {
            sequence,
            phase: phase.into(),
            kind: kind.into(),
            text: text.into(),
        });
        if state.progress.len() > 256 {
            state.progress.remove(0);
        }
        self.publish("action", &state.action_id, state)
    }

    async fn execute(
        &self,
        request: &GitActionRequest,
        preview: &GitMessagePreview,
        state: &mut GitActionState,
    ) -> anyhow::Result<()> {
        let checkout = &preview.checkout;
        let cwd = Path::new(&checkout.cwd);
        let _guard = self
            .0
            .locks
            .acquire([zeron_proto::orchestration::ThreadId(
                checkout.checkout_id.clone(),
            )])
            .await;
        self.thread_checkout(&preview.thread_id, &checkout.cwd)?;
        let identity = self.0.repos.checkout_identity(cwd).await?;
        let _index_lock = IndexLock::acquire(&identity.git_dir)?;
        require_unsigned_commit_policy(cwd).await?;
        ensure!(
            self.status(cwd).await? == *checkout,
            "Checkout/staged scope changed before commit"
        );
        self.event(
            state,
            "commit",
            "phase_started",
            "Committing confirmed staged tree (hooks disabled)",
        )?;
        // Immutable tree + CAS ref update avoids a hook widening approved scope
        // or an external ref update causing an unintended parent. Never stage.
        let mut args = vec!["commit-tree", checkout.staged_tree.as_str()];
        if !checkout.head.is_empty() {
            args.extend(["-p", checkout.head.as_str()]);
        }
        let commit = run(cwd, "git", &args, Some(&preview.commit_message), None)
            .await?
            .trim()
            .to_owned();
        let reference = format!("refs/heads/{}", checkout.branch);
        let zero = "0".repeat(commit.len());
        let before = if checkout.head.is_empty() {
            &zero
        } else {
            &checkout.head
        };
        git(
            cwd,
            &[
                "update-ref",
                "-m",
                "Noches confirmed commit",
                &reference,
                &commit,
                before,
            ],
        )
        .await?;
        state.commit = Some(commit.clone());
        self.event(state, "commit", "phase_completed", &commit)?;
        if request.authorize_push {
            self.thread_checkout(&preview.thread_id, &checkout.cwd)?;
            let remote = request
                .push_remote
                .as_ref()
                .context("push remote required")?;
            let current = self.status(cwd).await?;
            ensure!(
                current.branch == checkout.branch && current.head == commit,
                "Checkout changed before push"
            );
            validate_destination(request, &current)?;
            self.event(
                state,
                "push",
                "phase_started",
                "Pushing to explicitly selected remote",
            )?;
            // Explicit URL defeats branch.pushRemote/pushDefault; SHA refspec
            // prevents publishing commits that appeared after this transaction.
            let publisher = self
                .0
                .publisher
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            publisher
                .push(PushCommit {
                    cwd: cwd.into(),
                    remote_url: remote.push_url.clone(),
                    branch: checkout.branch.clone(),
                    commit: commit.clone(),
                })
                .await?;
            self.event(state, "push", "phase_completed", "Push completed")?;
        }
        if request.authorize_create_pr {
            self.thread_checkout(&preview.thread_id, &checkout.cwd)?;
            let current = self.status(cwd).await?;
            ensure!(
                current.branch == checkout.branch && current.head == commit,
                "Checkout changed before PR creation"
            );
            validate_destination(request, &current)?;
            let repo = request
                .pr_repository
                .as_deref()
                .context("PR repository required")?;
            let push = request
                .push_remote
                .as_ref()
                .context("push remote required")?;
            let push_repo =
                crate::parse_git_remote(&push.push_url).context("Unsupported PR remote")?;
            let head = format!("{}:{}", push_repo.owner, checkout.branch);
            self.event(
                state,
                "pr",
                "phase_started",
                "Creating PR in explicitly selected repository",
            )?;
            let creator = self
                .0
                .pr_creator
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            let url = creator
                .create(CreatePullRequest {
                    cwd: cwd.into(),
                    repository: repo.into(),
                    head,
                    base: preview.base_branch.clone(),
                    title: preview.pr_title.clone(),
                    body: preview.pr_body.clone(),
                })
                .await?;
            ensure!(
                url.starts_with(&format!("{repo}/pull/")),
                "PR result repository mismatch"
            );
            state.pr_url = Some(url.clone());
            self.event(state, "pr", "phase_completed", &url)?;
            let linker = self
                .0
                .linker
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            if let Some(linker) = linker {
                linker.link(&preview.thread_id, &url).await?;
                state.pr_linked = true;
                self.event(state, "link", "phase_completed", "PR linked to thread")?;
            } else {
                self.event(
                    state,
                    "link",
                    "link_pending",
                    "PR created; PR-watch link service not merged",
                )?;
            }
        }
        Ok(())
    }
}

fn validate_messages(commit: &str, title: &str, body: &str) -> anyhow::Result<()> {
    ensure!(
        !commit.trim().is_empty() && commit.len() <= 32_000 && !commit.contains('\0'),
        "invalid commit message"
    );
    ensure!(
        !title.trim().is_empty() && title.len() <= 256 && !title.contains(['\n', '\r', '\0']),
        "invalid PR title"
    );
    ensure!(
        body.len() <= 64_000 && !body.contains('\0'),
        "invalid PR body"
    );
    Ok(())
}

fn validate_destination(request: &GitActionRequest, current: &GitCheckout) -> anyhow::Result<()> {
    if request.authorize_push {
        let remote = request
            .push_remote
            .as_ref()
            .context("Push authorization needs explicit remote")?;
        required(&remote.name, "remote")?;
        required(&remote.push_url, "push URL")?;
        ensure!(
            current.remotes.contains(remote),
            "Selected remote URLs changed"
        );
    }
    if request.authorize_create_pr {
        let repo = request
            .pr_repository
            .as_deref()
            .context("PR authorization needs explicit repository")?;
        required(repo, "PR repository")?;
        ensure!(
            repo.starts_with("https://"),
            "PR repository must be an explicit HTTPS repository URL"
        );
        let target = crate::parse_git_remote(repo).context("Invalid PR repository")?;
        let push = crate::parse_git_remote(
            &request
                .push_remote
                .as_ref()
                .context("push remote required")?
                .push_url,
        )
        .context("Unsupported PR remote")?;
        ensure!(
            target.host == push.host,
            "Push and PR repository hosts differ"
        );
        let known = current.remotes.iter().any(|remote| {
            [&remote.fetch_url, &remote.push_url]
                .iter()
                .any(|url| crate::parse_git_remote(url).as_ref() == Some(&target))
        });
        ensure!(
            known,
            "PR repository is not among explicitly inspected remotes"
        );
        ensure!(
            repo == format!(
                "https://{}/{}/{}",
                target.host, target.owner, target.repository
            ),
            "Noncanonical PR repository URL"
        );
    }
    Ok(())
}

struct IndexLock {
    path: PathBuf,
}
impl IndexLock {
    fn acquire(git_dir: &Path) -> anyhow::Result<Self> {
        let path = git_dir.join("index.lock");
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .context("Checkout busy: Git index is locked")?;
        Ok(Self { path })
    }
}
impl Drop for IndexLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn ensure_checkout_ready(git_dir: &Path) -> anyhow::Result<()> {
    for marker in [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "rebase-merge",
        "rebase-apply",
        "sequencer",
    ] {
        ensure!(
            !git_dir.join(marker).exists(),
            "Checkout has an unfinished Git operation"
        );
    }
    Ok(())
}

async fn require_unsigned_commit_policy(cwd: &Path) -> anyhow::Result<()> {
    let signing = git(cwd, &["config", "--bool", "--get", "commit.gpgsign"])
        .await
        .ok();
    ensure!(
        signing.as_deref().map(str::trim) != Some("true"),
        "Commit signing is configured; native signing is not implemented. Use the CLI to preserve signing policy."
    );
    Ok(())
}
