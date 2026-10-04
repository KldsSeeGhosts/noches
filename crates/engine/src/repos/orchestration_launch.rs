//! Explicit-name orchestration operations reuse Repos' shell-free Git runner.
//! Unlike legacy auto-naming, caller branch/base/path are preserved verbatim.
use crate::{EngineError, Repos};
use std::path::{Path, PathBuf};

impl Repos {
    pub(crate) async fn orchestration_git(
        &self,
        cwd: &Path,
        args: &[&str],
    ) -> Result<String, EngineError> {
        self.git(args, Some(cwd)).await
    }

    pub(crate) fn orchestration_worktree_path(&self, root: &Path, operation_id: &str) -> PathBuf {
        self.inner
            .worktrees_root
            .join(root.file_name().unwrap_or_default())
            .join(operation_id.replace('/', "-"))
    }
    pub(crate) async fn create_claimed_worktree(
        &self,
        root: &Path,
        path: &Path,
        branch: &str,
        base: &str,
        owner: Option<&str>,
    ) -> Result<(), EngineError> {
        if path.symlink_metadata().is_ok() {
            return Err(EngineError::Other(format!(
                "Worktree destination already exists: {}",
                path.display()
            )));
        }
        self.git(&["check-ref-format", "--branch", branch], Some(root))
            .await?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let path_string = path.to_string_lossy();
        let mut args = vec!["worktree", "add"];
        let reason = owner.map(|o| format!("noches-launch:{o}"));
        if let Some(reason) = &reason {
            args.extend(["--lock", "--reason", reason]);
        }
        args.extend(["-b", branch, &path_string, base]);
        self.git(&args, Some(root)).await?;
        Ok(())
    }
    /// Git records this reason before checkout. A path/branch alone cannot
    /// prove resource ownership after a crash.
    pub(crate) async fn worktree_claim_matches(&self, path: &Path, owner: &str) -> bool {
        let Ok(admin) = self
            .git(&["rev-parse", "--absolute-git-dir"], Some(path))
            .await
        else {
            return false;
        };
        std::fs::read_to_string(Path::new(&admin).join("locked"))
            .is_ok_and(|reason| reason.trim_end() == format!("noches-launch:{owner}"))
    }
    pub(crate) async fn release_worktree_claim(
        &self,
        root: &Path,
        path: &Path,
        owner: &str,
    ) -> Result<(), EngineError> {
        if self.worktree_claim_matches(path, owner).await {
            self.git(&["worktree", "unlock", &path.to_string_lossy()], Some(root))
                .await?;
        }
        Ok(())
    }
    /// Only call after definite CAS refusal for an operation-owned checkout.
    pub(crate) async fn remove_created_worktree(
        &self,
        root: &Path,
        path: &Path,
        branch: &str,
    ) -> Result<(), EngineError> {
        self.git(
            &[
                "worktree",
                "remove",
                "--force",
                "--force",
                &path.to_string_lossy(),
            ],
            Some(root),
        )
        .await?;
        self.git(&["branch", "-D", "--", branch], Some(root))
            .await?;
        Ok(())
    }
    pub(crate) async fn orchestration_refs(
        &self,
        root: &Path,
        include_matching_remote: bool,
    ) -> Result<Vec<serde_json::Value>, EngineError> {
        use serde_json::json;
        let inventory = self
            .orchestration_git(
                root,
                &[
                    "for-each-ref",
                    "--format=%(refname)%09%(committerdate:unix)%09%(symref)",
                    "refs/heads",
                    "refs/remotes",
                ],
            )
            .await?;
        let default = self
            .orchestration_git(root, &["symbolic-ref", "refs/remotes/origin/HEAD"])
            .await
            .ok()
            .map(|s| s.trim_start_matches("refs/remotes/origin/").to_owned());
        let worktrees = self
            .orchestration_git(root, &["worktree", "list", "--porcelain", "-z"])
            .await?;
        let mut paths = std::collections::HashMap::new();
        let mut path = None;
        for field in worktrees.split('\0') {
            if let Some(value) = field.strip_prefix("worktree ") {
                path = Some(value.to_owned());
            }
            if let Some(branch) = field.strip_prefix("branch refs/heads/") {
                if let Some(path) = path.take().filter(|p| Path::new(p).exists()) {
                    paths.insert(branch.to_owned(), path);
                }
            }
        }
        let current = self.current_branch(root).await.ok();
        let mut rows = vec![];
        for line in inventory.lines() {
            let mut fields = line.split('\t');
            let name = fields.next().unwrap_or("");
            let commit = fields
                .next()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            if fields.next().is_some_and(|s| !s.is_empty()) {
                continue;
            }
            if let Some(name) = name.strip_prefix("refs/heads/") {
                rows.push((commit,json!({"name":name,"isRemote":false,"current":current.as_deref()==Some(name),"isDefault":default.as_deref()==Some(name),"worktreePath":paths.get(name)})));
            } else if let Some(name) = name.strip_prefix("refs/remotes/") {
                let remote = name.split('/').next().unwrap_or("");
                let branch = name.strip_prefix(&format!("{remote}/")).unwrap_or(name);
                rows.push((commit,json!({"name":name,"isRemote":true,"remoteName":remote,"current":false,"isDefault":remote=="origin"&&default.as_deref()==Some(branch),"worktreePath":null})));
            }
        }
        let local: std::collections::HashSet<String> = rows
            .iter()
            .filter(|(_, r)| r["isRemote"] == false)
            .filter_map(|(_, r)| r["name"].as_str().map(str::to_owned))
            .collect();
        rows.retain(|(_, r)| {
            include_matching_remote
                || r["isRemote"] == false
                || r["remoteName"] != "origin"
                || !local.contains(
                    r["name"]
                        .as_str()
                        .unwrap()
                        .split_once('/')
                        .map(|(_, n)| n)
                        .unwrap_or(""),
                )
        });
        rows.sort_by(|(at, a), (bt, b)| {
            let priority = |v: &serde_json::Value| {
                if v["current"] == true {
                    0
                } else if v["isDefault"] == true {
                    1
                } else if v["isRemote"] == false {
                    2
                } else {
                    3
                }
            };
            priority(a)
                .cmp(&priority(b))
                .then_with(|| bt.cmp(at))
                .then_with(|| a["name"].as_str().cmp(&b["name"].as_str()))
        });
        Ok(rows.into_iter().map(|(_, r)| r).collect())
    }

    pub(crate) async fn clone_to(&self, url: &str, destination: &Path) -> Result<(), EngineError> {
        if destination.symlink_metadata().is_ok() {
            return Err(EngineError::Other("Destination already exists.".into()));
        }
        self.git(&["clone", "--", url, &destination.to_string_lossy()], None)
            .await?;
        Ok(())
    }
}
