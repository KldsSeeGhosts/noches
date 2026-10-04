//! Opt-in idle main-checkout pull, no stash/reset/force and no mutation retry.
use std::path::Path;

use anyhow::{Context, ensure};
use zeron_proto::git_actions::*;
use zeron_proto::orchestration::ThreadId;

use super::{GitActionsService, git, list, put, required};

impl GitActionsService {
    pub fn pull_state(&self, space_id: &str) -> anyhow::Result<PullState> {
        Ok(
            super::get(&self.0.store, "pull", space_id)?.unwrap_or_else(|| PullState {
                policy: PullPolicy {
                    space_id: space_id.into(),
                    ..Default::default()
                },
                last_skip_reason: Some("disabled".into()),
                ..Default::default()
            }),
        )
    }

    pub async fn set_pull_policy(&self, mut policy: PullPolicy) -> anyhow::Result<PullState> {
        let _policy_lock = self
            .0
            .locks
            .acquire([ThreadId(format!("pull-policy:{}", policy.space_id))])
            .await;
        let space = self.local_space(&policy.space_id)?;
        if policy.enabled {
            required(&policy.default_branch, "default branch")?;
            let current = self.status(Path::new(&space.path)).await?;
            ensure!(
                current.remotes.contains(&policy.remote),
                "Pull remote must match inspected URLs"
            );
        }
        policy.cadence_seconds = policy.cadence_seconds.clamp(60, 3600);
        let state = PullState {
            next_check_at: Some(crate::now_ms() + delay_ms(&policy)),
            policy,
            ..Default::default()
        };
        put(&self.0.store, "pull", &state.policy.space_id, &state)?;
        Ok(state)
    }

    pub async fn retry_pull(&self, space_id: &str) -> anyhow::Result<PullState> {
        let _policy_lock = self
            .0
            .locks
            .acquire([ThreadId(format!("pull-policy:{space_id}"))])
            .await;
        let mut state = self.pull_state(space_id)?;
        let space = self.local_space(space_id)?;
        let now = crate::now_ms();
        state.last_checked_at = Some(now);
        state.next_check_at = Some(now + delay_ms(&state.policy));
        state.last_skip_reason = None;
        state.last_error = None;
        state.last_result = None;
        let result = self.try_pull(Path::new(&space.path), &state.policy).await;
        match result {
            Ok(Ok(result)) => state.last_result = Some(result),
            Ok(Err(skip)) => state.last_skip_reason = Some(skip),
            Err(error) => state.last_error = Some(error.to_string()),
        }
        put(&self.0.store, "pull", space_id, &state)?;
        Ok(state)
    }

    async fn try_pull(
        &self,
        cwd: &Path,
        policy: &PullPolicy,
    ) -> anyhow::Result<Result<String, String>> {
        if !policy.enabled {
            return Ok(Err("disabled".into()));
        }
        // Conservative host-wide admission blocks provider starts, file writes,
        // terminal starts and other Git actions for the entire pull. We never
        // retire a provider, run restart preparation, or stash its files.
        let _idle = match self
            .0
            .sessions
            .admission()
            .begin_update(|| self.checkout_idle())
        {
            Ok(permit) => permit,
            Err(_) => return Ok(Err("checkout_not_idle".into())),
        };
        let identity = self.0.repos.checkout_identity(cwd).await?;
        let _guard = self.0.locks.acquire([ThreadId(identity.id.clone())]).await;
        let common = git(
            cwd,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )
        .await?;
        if std::fs::canonicalize(common.trim())? != identity.git_dir {
            return Ok(Err("not_main_checkout".into()));
        }
        // Detect a cooperating external Git writer. Git itself needs this lock
        // during pull, so release after validation; non-Noches actors must
        // respect normal Git locking (we cannot freeze arbitrary host tools).
        let index_lock = match super::IndexLock::acquire(&identity.git_dir) {
            Ok(lock) => lock,
            Err(_) => return Ok(Err("checkout_locked".into())),
        };
        let current = self.status(cwd).await?;
        if current.branch != policy.default_branch {
            return Ok(Err("not_default_branch".into()));
        }
        if !current.remotes.contains(&policy.remote) {
            return Ok(Err("remote_changed".into()));
        }
        let expected = format!("{}/{}", policy.remote.name, policy.default_branch);
        if current.upstream.as_deref() != Some(expected.as_str()) {
            return Ok(Err("upstream_mismatch".into()));
        }
        let remote_head = format!("refs/remotes/{}/HEAD", policy.remote.name);
        let default = git(cwd, &["symbolic-ref", "--short", &remote_head])
            .await
            .ok();
        if default.as_deref().map(str::trim) != Some(expected.as_str()) {
            return Ok(Err("default_branch_unverified".into()));
        }
        if current.dirty {
            return Ok(Err("changed_or_untracked_files".into()));
        }
        let counts = git(
            cwd,
            &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        )
        .await?;
        let ahead: u64 = counts
            .split_whitespace()
            .next()
            .context("invalid ahead count")?
            .parse()?;
        if ahead != 0 {
            return Ok(Err("local_commits".into()));
        }
        let before_upstream = git(cwd, &["rev-parse", "@{upstream}"]).await?;
        drop(index_lock);
        // Explicit fetch URL defeats changed/default remote selection. No
        // local commits and ff-only refuse divergent remote histories.
        git(
            cwd,
            &[
                "pull",
                "--ff-only",
                "--no-rebase",
                "--",
                &policy.remote.fetch_url,
                &policy.default_branch,
            ],
        )
        .await?;
        let after = git(cwd, &["rev-parse", "HEAD"]).await?;
        let fetched = git(cwd, &["rev-parse", "FETCH_HEAD"]).await?;
        // URL-addressed fetches do not refresh remote-tracking refs. Publish
        // only the fetched SHA with a CAS, so the next idle check does not
        // misclassify a successful pull as a newly-created local commit.
        let tracking_ref = format!(
            "refs/remotes/{}/{}",
            policy.remote.name, policy.default_branch
        );
        git(
            cwd,
            &[
                "update-ref",
                "-m",
                "Noches ff-only pull",
                &tracking_ref,
                fetched.trim(),
                before_upstream.trim(),
            ],
        )
        .await?;
        Ok(Ok(if after.trim() == current.head {
            "skipped_up_to_date"
        } else {
            "pulled"
        }
        .into()))
    }

    pub(crate) async fn pull_tick(&self) {
        if let Ok(states) = list::<PullState>(&self.0.store, "pull") {
            for state in states {
                if state.policy.enabled
                    && state.last_error.is_none()
                    && state.next_check_at.is_none_or(|at| at <= crate::now_ms())
                {
                    let _ = self.retry_pull(&state.policy.space_id).await;
                }
            }
        }
    }

    pub(super) fn local_space(&self, id: &str) -> anyhow::Result<zeron_proto::Space> {
        let space = self.0.workspace.space(id)?.context("Project not found")?;
        ensure!(
            space.device_id == self.0.doc_host.device_id(),
            "Project belongs to another host"
        );
        Ok(space)
    }
}

fn delay_ms(policy: &PullPolicy) -> i64 {
    // Deterministic jitter makes fixtures reproducible and spreads projects.
    let hash = policy
        .space_id
        .bytes()
        .fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)));
    let cadence = policy.cadence_seconds.clamp(60, 3600);
    ((cadence + hash % (cadence / 5 + 1)) * 1000) as i64
}
