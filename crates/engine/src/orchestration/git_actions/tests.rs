//! Fixture-only tests. No scanner ever receives the user's real HOME.
use super::*;
use serde_json::json;
use zeron_proto::HarnessId;
use zeron_sync::DocsStore;

struct Fixture {
    dir: tempfile::TempDir,
    repo: PathBuf,
    histories: PathBuf,
    service: GitActionsService,
}

impl Fixture {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let histories = dir.path().join("fixture-home");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(&histories).unwrap();
        git(&repo, &["init", "-b", "main"]).await.unwrap();
        git(&repo, &["config", "user.email", "fixture@example.invalid"])
            .await
            .unwrap();
        git(&repo, &["config", "user.name", "Fixture"])
            .await
            .unwrap();
        std::fs::write(repo.join("tracked.txt"), "base\n").unwrap();
        git(&repo, &["add", "tracked.txt"]).await.unwrap();
        git(
            &repo,
            &["-c", "core.hooksPath=/dev/null", "commit", "-m", "base"],
        )
        .await
        .unwrap();
        let repo = std::fs::canonicalize(repo).unwrap();
        let docs = Arc::new(DocsStore::open(dir.path().join("store")).unwrap());
        let store = Store::open(docs.clone(), "fixture-host").unwrap();
        let registry = Arc::new(HarnessRegistry::new());
        registry.register(Arc::new(zeron_harness::mock::MockHarness {
            script: vec![],
        }));
        let sessions = SessionsEngine::new(
            "fixture-host".into(),
            Arc::new(crate::RunJournal::open(dir.path().join("journal")).unwrap()),
            registry.clone(),
        );
        let workspace = WorkspaceHost::open(
            docs.clone(),
            crate::WorkspaceHostConfig {
                device_id: "fixture-host".into(),
                device_name: "Fixture".into(),
                platform: "fixture".into(),
                org_id: "fixture-org".into(),
                user_id: "fixture-user".into(),
                edge: None,
            },
        )
        .unwrap();
        workspace
            .create_space(
                "project",
                "fixture-host",
                repo.to_str().unwrap(),
                None,
                true,
            )
            .unwrap();
        workspace
            .create_chat("thread", Some("project"), None, None, None)
            .unwrap();
        let host = DocHost::new(
            docs,
            crate::DocHostConfig {
                device_id: "fixture-host".into(),
                default_harness: HarnessId::Mock,
                edge: None,
            },
        );
        host.set_workspace(workspace.clone());
        let terminals = Terminals::with_admission(sessions.admission());
        let service = GitActionsService::new(
            store,
            Repos::new(dir.path(), "fixture-host"),
            sessions,
            terminals,
            host,
            workspace,
            registry,
            vec![
                (HistorySource::ClaudeCode, histories.join("claude")),
                (HistorySource::Codex, histories.join("codex")),
            ],
        )
        .unwrap();
        Self {
            dir,
            repo,
            histories,
            service,
        }
    }

    async fn stage(&self) {
        std::fs::write(self.repo.join("tracked.txt"), "approved\n").unwrap();
        git(&self.repo, &["add", "tracked.txt"]).await.unwrap();
    }

    async fn preview(&self) -> GitMessagePreview {
        self.service
            .preview(GitPreviewRequest {
                thread_id: "thread".into(),
                cwd: self.repo.to_string_lossy().into_owned(),
                commit_message: Some("feat: approved scope".into()),
                pr_title: Some("Approved scope".into()),
                pr_body: Some("Fixture body".into()),
                base_branch: "main".into(),
            })
            .await
            .unwrap()
    }

    fn request(&self, preview: &GitMessagePreview) -> GitActionRequest {
        GitActionRequest {
            request_id: crate::new_id(),
            preview_id: preview.preview_id.clone(),
            confirmed_checkout: preview.checkout.clone(),
            authorize_commit: true,
            ..Default::default()
        }
    }

    async fn wait(&self, ns: &str, id: &str) -> Value {
        let mut rx = self.service.watch(ns, id).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let value = rx.borrow_and_update().clone();
                let state = if ns == "scan" {
                    value["state"].clone()
                } else {
                    value
                };
                if state["status"] != "running" {
                    break state;
                }
                rx.changed().await.unwrap();
            }
        })
        .await
        .unwrap()
    }

    fn history(&self, source: HistorySource, native: &str, name: &str) -> PathBuf {
        let root = self.histories.join(if source == HistorySource::Codex {
            "codex"
        } else {
            "claude"
        });
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("{name}.jsonl"));
        let rows = match source {
            HistorySource::Codex => vec![
                json!({"type":"session_meta","payload":{"id":native,"cwd":self.repo}}),
                json!({"type":"event_msg","timestamp":"2026-08-24T10:00:00Z","payload":{"type":"user_message","message":"Fix the fixture"}}),
                json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Fix the fixture"}]}}),
                json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Fixed"}]}}),
            ],
            HistorySource::ClaudeCode => vec![
                json!({"type":"user","sessionId":native,"cwd":self.repo,"timestamp":"2026-08-24T10:00:00Z","message":{"role":"user","content":"Fix the fixture"}}),
                json!({"type":"assistant","sessionId":native,"message":{"role":"assistant","content":[{"type":"text","text":"Fixed"},{"type":"tool_use","name":"Read","input":{}}]}}),
            ],
        };
        std::fs::write(
            &path,
            rows.into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        path
    }

    async fn remote(&self) -> PullPolicy {
        let remote = self.dir.path().join("origin.git");
        std::fs::create_dir_all(&remote).unwrap();
        git(&remote, &["init", "--bare", "-b", "main"])
            .await
            .unwrap();
        git(
            &self.repo,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        )
        .await
        .unwrap();
        git(&self.repo, &["push", "-u", "origin", "main"])
            .await
            .unwrap();
        git(
            &self.repo,
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/main",
            ],
        )
        .await
        .unwrap();
        PullPolicy {
            space_id: "project".into(),
            enabled: true,
            remote: self.service.status(&self.repo).await.unwrap().remotes[0].clone(),
            default_branch: "main".into(),
            cadence_seconds: 1,
        }
    }
}

#[test]
fn refuses_implicit_or_changed_push_destination() {
    let request = GitActionRequest {
        authorize_push: true,
        ..Default::default()
    };
    assert!(validate_destination(&request, &GitCheckout::default()).is_err());
}

#[tokio::test]
async fn commits_exact_staged_tree_preserves_unstaged_and_untracked_and_dedupes() {
    let f = Fixture::new().await;
    f.stage().await;
    std::fs::write(f.repo.join("tracked.txt"), "unstaged after stage\n").unwrap();
    std::fs::write(f.repo.join("private.txt"), "never committed\n").unwrap();
    let preview = f.preview().await;
    let request = f.request(&preview);
    let action = f.service.start(request.clone()).await.unwrap();
    let state = f.wait("action", &action.action_id).await;
    assert_eq!(state["status"], "completed", "{state}");
    assert_eq!(
        git(&f.repo, &["show", "HEAD:tracked.txt"]).await.unwrap(),
        "approved\n"
    );
    assert!(git(&f.repo, &["show", "HEAD:private.txt"]).await.is_err());
    assert_eq!(
        std::fs::read_to_string(f.repo.join("tracked.txt")).unwrap(),
        "unstaged after stage\n"
    );
    assert_eq!(
        git(&f.repo, &["rev-list", "--count", "HEAD"])
            .await
            .unwrap()
            .trim(),
        "2"
    );
    let duplicate = f.service.start(request.clone()).await.unwrap();
    assert_eq!(duplicate.commit.as_deref(), state["commit"].as_str());
    let mut changed = request;
    changed.authorize_push = true;
    assert!(
        f.service
            .start(changed)
            .await
            .unwrap_err()
            .to_string()
            .contains("Idempotency")
    );
    assert!(!f.repo.join(".git/index.lock").exists());
}

#[tokio::test]
async fn stale_staged_content_same_paths_refused_before_receipt() {
    let f = Fixture::new().await;
    f.stage().await;
    let preview = f.preview().await;
    let request = f.request(&preview);
    std::fs::write(f.repo.join("tracked.txt"), "not approved\n").unwrap();
    git(&f.repo, &["add", "tracked.txt"]).await.unwrap();
    assert!(
        f.service
            .start(request.clone())
            .await
            .unwrap_err()
            .to_string()
            .contains("scope changed")
    );
    assert!(
        get::<Value>(&f.service.0.store, "request", &request.request_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        git(&f.repo, &["rev-list", "--count", "HEAD"])
            .await
            .unwrap()
            .trim(),
        "1"
    );
}

#[tokio::test]
async fn authorization_and_checkout_identity_are_not_inferred() {
    let f = Fixture::new().await;
    f.stage().await;
    let preview = f.preview().await;
    let mut request = f.request(&preview);
    request.authorize_commit = false;
    assert!(f.service.start(request).await.is_err());
    let mut request = f.request(&preview);
    request.authorize_create_pr = true;
    assert!(
        f.service
            .start(request)
            .await
            .unwrap_err()
            .to_string()
            .contains("separately authorized push")
    );
    let mut request = f.request(&preview);
    request.confirmed_checkout.checkout_id = "different checkout".into();
    assert!(f.service.start(request).await.is_err());
    git(&f.repo, &["checkout", "-b", "different"])
        .await
        .unwrap();
    assert!(f.service.start(f.request(&preview)).await.is_err());
}

#[tokio::test]
async fn unfinished_operations_detached_head_and_foreign_thread_refuse() {
    let f = Fixture::new().await;
    std::fs::write(f.repo.join(".git/MERGE_HEAD"), "fixture").unwrap();
    assert!(f.service.status(&f.repo).await.is_err());
    std::fs::remove_file(f.repo.join(".git/MERGE_HEAD")).unwrap();
    git(&f.repo, &["checkout", "--detach"]).await.unwrap();
    assert!(
        f.service
            .status(&f.repo)
            .await
            .unwrap_err()
            .to_string()
            .contains("detached")
    );
    assert!(
        f.service
            .thread_checkout("missing", f.repo.to_str().unwrap())
            .is_err()
    );
}

#[tokio::test]
async fn separately_authorized_push_uses_exact_selected_remote_only() {
    let f = Fixture::new().await;
    let policy = f.remote().await;
    let other = f.dir.path().join("upstream.git");
    std::fs::create_dir_all(&other).unwrap();
    git(&other, &["init", "--bare", "-b", "main"])
        .await
        .unwrap();
    git(
        &f.repo,
        &["remote", "add", "upstream", other.to_str().unwrap()],
    )
    .await
    .unwrap();
    git(&f.repo, &["config", "remote.pushDefault", "upstream"])
        .await
        .unwrap();
    f.stage().await;
    let preview = f.preview().await;
    let mut request = f.request(&preview);
    request.authorize_push = true;
    request.push_remote = Some(policy.remote);
    let action = f.service.start(request).await.unwrap();
    let state = f.wait("action", &action.action_id).await;
    assert_eq!(state["status"], "completed", "{state}");
    assert_eq!(
        git(
            Path::new(&preview.checkout.remotes[0].push_url),
            &["rev-parse", "main"]
        )
        .await
        .unwrap()
        .trim(),
        state["commit"].as_str().unwrap()
    );
    assert!(git(&other, &["rev-parse", "main"]).await.is_err());
}

#[test]
fn fork_pr_target_is_explicit_and_remote_mutations_refuse() {
    let origin = GitRemoteChoice {
        name: "origin".into(),
        fetch_url: "git@github.com:fork/repo.git".into(),
        push_url: "git@github.com:fork/repo.git".into(),
    };
    let upstream = GitRemoteChoice {
        name: "upstream".into(),
        fetch_url: "https://github.com/upstream/repo.git".into(),
        push_url: "https://github.com/upstream/repo.git".into(),
    };
    let checkout = GitCheckout {
        remotes: vec![origin.clone(), upstream],
        ..Default::default()
    };
    let mut request = GitActionRequest {
        authorize_push: true,
        authorize_create_pr: true,
        push_remote: Some(origin),
        pr_repository: Some("https://github.com/fork/repo".into()),
        ..Default::default()
    };
    assert!(validate_destination(&request, &checkout).is_ok());
    request.pr_repository = None;
    assert!(validate_destination(&request, &checkout).is_err());
    request.pr_repository = Some("https://github.com/unrelated/repo".into());
    assert!(validate_destination(&request, &checkout).is_err());
    request.pr_repository = Some("https://github.com/fork/repo".into());
    request.push_remote.as_mut().unwrap().push_url = "git@github.com:upstream/repo.git".into();
    assert!(validate_destination(&request, &checkout).is_err());
}

#[tokio::test]
async fn reserved_running_action_recovers_uncertain_without_reexecution() {
    let f = Fixture::new().await;
    let state = GitActionState {
        action_id: "crash".into(),
        status: "running".into(),
        ..Default::default()
    };
    put(&f.service.0.store, "action", "crash", &state).unwrap();
    let reopened = GitActionsService::new(
        f.service.0.store.clone(),
        f.service.0.repos.clone(),
        f.service.0.sessions.clone(),
        f.service.0.terminals.clone(),
        f.service.0.doc_host.clone(),
        f.service.0.workspace.clone(),
        f.service.0.registry.clone(),
        vec![],
    )
    .unwrap();
    assert_eq!(
        reopened
            .state::<GitActionState>("action", "crash")
            .unwrap()
            .status,
        "uncertain"
    );
    assert_eq!(
        git(&f.repo, &["rev-list", "--count", "HEAD"])
            .await
            .unwrap()
            .trim(),
        "1"
    );
}

#[tokio::test]
async fn index_lock_or_active_work_blocks_actions_and_pull() {
    let f = Fixture::new().await;
    let policy = f.remote().await;
    f.service.set_pull_policy(policy).await.unwrap();
    let permit = f.service.0.sessions.admit_work().unwrap();
    assert_eq!(
        f.service
            .retry_pull("project")
            .await
            .unwrap()
            .last_skip_reason
            .as_deref(),
        Some("checkout_not_idle")
    );
    f.stage().await;
    let preview = f.preview().await;
    assert!(f.service.start(f.request(&preview)).await.is_err());
    drop(permit);
    let lock = IndexLock::acquire(&f.repo.join(".git")).unwrap();
    assert_eq!(
        f.service
            .retry_pull("project")
            .await
            .unwrap()
            .last_skip_reason
            .as_deref(),
        Some("checkout_locked")
    );
    drop(lock);
}

#[tokio::test]
async fn opt_in_pull_ff_only_and_skip_reasons_preserve_files() {
    let f = Fixture::new().await;
    assert_eq!(
        f.service
            .retry_pull("project")
            .await
            .unwrap()
            .last_skip_reason
            .as_deref(),
        Some("disabled")
    );
    let policy = f.remote().await;
    let state = f.service.set_pull_policy(policy).await.unwrap();
    assert_eq!(state.policy.cadence_seconds, 60);
    std::fs::write(f.repo.join("untracked"), "private").unwrap();
    let before = git(&f.repo, &["rev-parse", "HEAD"]).await.unwrap();
    assert_eq!(
        f.service
            .retry_pull("project")
            .await
            .unwrap()
            .last_skip_reason
            .as_deref(),
        Some("changed_or_untracked_files")
    );
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]).await.unwrap(), before);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("untracked")).unwrap(),
        "private"
    );
    std::fs::remove_file(f.repo.join("untracked")).unwrap();
    assert_eq!(
        f.service
            .retry_pull("project")
            .await
            .unwrap()
            .last_result
            .as_deref(),
        Some("skipped_up_to_date")
    );
    let producer = f.dir.path().join("producer");
    git(
        f.dir.path(),
        &[
            "clone",
            f.dir.path().join("origin.git").to_str().unwrap(),
            producer.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    git(
        &producer,
        &["config", "user.email", "fixture@example.invalid"],
    )
    .await
    .unwrap();
    git(&producer, &["config", "user.name", "Fixture"])
        .await
        .unwrap();
    std::fs::write(producer.join("remote.txt"), "remote change").unwrap();
    git(&producer, &["add", "remote.txt"]).await.unwrap();
    git(
        &producer,
        &["-c", "core.hooksPath=/dev/null", "commit", "-m", "remote"],
    )
    .await
    .unwrap();
    git(&producer, &["push", "origin", "main"]).await.unwrap();
    let state = f.service.retry_pull("project").await.unwrap();
    assert_eq!(state.last_result.as_deref(), Some("pulled"), "{state:?}");
    assert_eq!(
        std::fs::read_to_string(f.repo.join("remote.txt")).unwrap(),
        "remote change"
    );
    f.stage().await;
    git(
        &f.repo,
        &["-c", "core.hooksPath=/dev/null", "commit", "-m", "local"],
    )
    .await
    .unwrap();
    assert_eq!(
        f.service
            .retry_pull("project")
            .await
            .unwrap()
            .last_skip_reason
            .as_deref(),
        Some("local_commits")
    );
}

#[tokio::test]
async fn pull_wrong_branch_or_upstream_refuses() {
    let f = Fixture::new().await;
    let mut policy = f.remote().await;
    f.service.set_pull_policy(policy.clone()).await.unwrap();
    git(&f.repo, &["branch", "--unset-upstream"]).await.unwrap();
    assert_eq!(
        f.service
            .retry_pull("project")
            .await
            .unwrap()
            .last_skip_reason
            .as_deref(),
        Some("upstream_mismatch")
    );
    git(&f.repo, &["branch", "--set-upstream-to", "origin/main"])
        .await
        .unwrap();
    policy.default_branch = "other".into();
    f.service.set_pull_policy(policy).await.unwrap();
    assert_eq!(
        f.service
            .retry_pull("project")
            .await
            .unwrap()
            .last_skip_reason
            .as_deref(),
        Some("not_default_branch")
    );
}

#[tokio::test]
async fn fixture_scan_preview_import_dedupe_and_no_implicit_continuation() {
    let f = Fixture::new().await;
    f.history(
        HistorySource::Codex,
        "00000000-0000-4000-8000-000000000001",
        "one",
    );
    f.history(
        HistorySource::Codex,
        "00000000-0000-4000-8000-000000000001",
        "duplicate",
    );
    f.history(
        HistorySource::ClaudeCode,
        "00000000-0000-4000-8000-000000000002",
        "claude",
    );
    let scan = f
        .service
        .scan_history(HistoryScanRequest {
            space_id: "project".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let state = f.wait("scan", &scan.scan_id).await;
    assert_eq!(state["status"], "completed", "{state}");
    let ids: Vec<String> = serde_json::from_value(state["candidateIds"].clone()).unwrap();
    assert_eq!(ids.len(), 2);
    let request = HistoryImportRequest {
        space_id: "project".into(),
        candidate_ids: ids.clone(),
    };
    let import = f.service.import_history(request.clone()).await.unwrap();
    let imported = f.wait("import", &import.import_id).await;
    assert_eq!(imported["status"], "completed", "{imported}");
    let chats: Vec<String> = serde_json::from_value(imported["chatIds"].clone()).unwrap();
    assert_eq!(chats.len(), 2);
    for chat_id in &chats {
        let row = f.service.0.workspace.chat(chat_id).unwrap().unwrap();
        assert!(row.harness_session_id.is_none());
        assert!(row.config.is_none());
        assert_eq!(row.room_gen, Some(2));
        let doc = f.service.0.doc_host.open_local(chat_id).unwrap();
        let entries = doc.doc().read_entries().unwrap();
        assert_eq!(
            entries[0].parts[0],
            zeron_doc::MessagePart::Text {
                id: format!("{chat_id}:0:text"),
                text: "Fix the fixture".into()
            }
        );
        assert_eq!(entries[1].role, zeron_doc::MessageRole::Assistant);
    }
    let import = f.service.import_history(request).await.unwrap();
    let duplicate = f.wait("import", &import.import_id).await;
    assert_eq!(duplicate["chatIds"], imported["chatIds"]);
    assert_eq!(f.service.0.workspace.read_chats().unwrap().len(), 3);
}

#[tokio::test]
async fn transcript_changed_and_wrong_project_refuse_import() {
    let f = Fixture::new().await;
    let path = f.history(HistorySource::ClaudeCode, "native", "one");
    let (preview, _) = history::parse(
        HistorySource::ClaudeCode,
        &path,
        &f.repo,
        &CancellationToken::new(),
    )
    .unwrap();
    let preview = preview.unwrap();
    put(
        &f.service.0.store,
        "candidate",
        &preview.candidate_id,
        &preview,
    )
    .unwrap();
    std::fs::write(&path, "changed").unwrap();
    assert!(
        f.service
            .import_one("project", &preview.candidate_id)
            .await
            .is_err()
    );
    assert_eq!(f.service.0.workspace.read_chats().unwrap().len(), 1);
    let other = f.dir.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    f.service
        .0
        .workspace
        .create_space(
            "other",
            "fixture-host",
            other.to_str().unwrap(),
            None,
            false,
        )
        .unwrap();
    assert!(
        f.service
            .import_history(HistoryImportRequest {
                space_id: "other".into(),
                candidate_ids: vec![preview.candidate_id]
            })
            .await
            .is_err()
    );
}

#[tokio::test]
async fn parser_canonical_codex_events_hide_generated_context_not_user_markup() {
    let f = Fixture::new().await;
    let path = f.histories.join("canonical.jsonl");
    let records = vec![
        json!({"type":"session_meta","payload":{"id":"fork","cwd":f.repo}}),
        json!({"type":"session_meta","payload":{"id":"ancestor","cwd":f.repo}}),
        json!({"type":"response_item","payload":{"type":"message","role":"user","internal_chat_message_metadata_passthrough":{"turn_id":"turn"},"content":[{"type":"input_text","text":"<environment_context>hidden</environment_context>"}]}}),
        json!({"type":"event_msg","payload":{"type":"user_message","message":"<environment_context>quoted user text</environment_context>"}}),
        json!({"type":"response_item","payload":{"type":"message","role":"user","internal_chat_message_metadata_passthrough":{"turn_id":"turn"},"content":[{"type":"input_text","text":"<environment_context>quoted user text</environment_context>"}]}}),
        json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Fixed"}]}}),
    ];
    std::fs::write(
        &path,
        records
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    let (preview, _) = history::parse(
        HistorySource::Codex,
        &path,
        &f.repo,
        &CancellationToken::new(),
    )
    .unwrap();
    let preview = preview.unwrap();
    assert_eq!(preview.provenance.native_session_id, "fork");
    assert_eq!(preview.messages.len(), 2);
    assert_eq!(
        preview.messages[0].text,
        "<environment_context>quoted user text</environment_context>"
    );
}

#[tokio::test]
async fn parser_byte_record_and_cancellation_limits_are_fixture_only() {
    let f = Fixture::new().await;
    let path = f.histories.join("huge.jsonl");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(9 * 1024 * 1024)
        .unwrap();
    assert!(
        history::parse(
            HistorySource::Codex,
            &path,
            &f.repo,
            &CancellationToken::new()
        )
        .unwrap()
        .0
        .is_none()
    );
    let path = f.history(HistorySource::ClaudeCode, "native", "normal");
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(history::parse(HistorySource::ClaudeCode, &path, &f.repo, &cancel).is_err());
    let other = f.dir.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    assert!(
        history::parse(
            HistorySource::ClaudeCode,
            &path,
            &other,
            &CancellationToken::new()
        )
        .unwrap()
        .0
        .is_none()
    );
    std::fs::write(
        &path,
        format!(
            "{}\n{}",
            json!({"type":"session_meta","payload":{"cwd":f.repo}}),
            json!({"type":"event_msg","payload":{"type":"user_message","message":"no native id"}})
        ),
    )
    .unwrap();
    assert!(
        history::parse(
            HistorySource::Codex,
            &path,
            &f.repo,
            &CancellationToken::new()
        )
        .unwrap()
        .0
        .is_none()
    );
}

#[tokio::test]
async fn cancelled_scan_resumes_its_discovery_snapshot_and_batch_cursor() {
    let f = Fixture::new().await;
    for index in 0..257 {
        f.history(HistorySource::Codex, "native", &format!("{index:04}"));
    }
    let scan = f
        .service
        .scan_history(HistoryScanRequest {
            space_id: "project".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    f.service.cancel_history(&scan.scan_id).unwrap();
    let cancelled = f.wait("scan", &scan.scan_id).await;
    assert_eq!(cancelled["status"], "cancelled");
    f.service
        .scan_history(HistoryScanRequest {
            space_id: "project".into(),
            scan_id: Some(scan.scan_id.clone()),
        })
        .await
        .unwrap();
    let paused = f.wait("scan", &scan.scan_id).await;
    assert_eq!(paused["status"], "paused", "{paused}");
    assert_eq!(paused["scannedFiles"], 256);
    f.service
        .scan_history(HistoryScanRequest {
            space_id: "project".into(),
            scan_id: Some(scan.scan_id.clone()),
        })
        .await
        .unwrap();
    let completed = f.wait("scan", &scan.scan_id).await;
    assert_eq!(completed["status"], "completed");
    assert_eq!(completed["scannedFiles"], 257);
    assert_eq!(completed["candidateIds"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn missing_native_or_visible_user_never_creates_importable_identity() {
    let f = Fixture::new().await;
    let path = f.histories.join("assistant-only.jsonl");
    std::fs::write(&path,format!("{}\n{}",json!({"type":"session_meta","payload":{"id":"native","cwd":f.repo}}),json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"assistant only"}]}}))).unwrap();
    assert!(
        history::parse(
            HistorySource::Codex,
            &path,
            &f.repo,
            &CancellationToken::new()
        )
        .unwrap()
        .0
        .is_none()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn transcript_symlinks_are_not_followed() {
    let f = Fixture::new().await;
    let path = f.history(HistorySource::ClaudeCode, "native", "one");
    let link = path.with_file_name("symlink.jsonl");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(
        history::parse(
            HistorySource::ClaudeCode,
            &link,
            &f.repo,
            &CancellationToken::new()
        )
        .is_err()
    );
    let scan = f
        .service
        .scan_history(HistoryScanRequest {
            space_id: "project".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let result = f.wait("scan", &scan.scan_id).await;
    assert_eq!(result["totalFiles"], 1);
}

#[tokio::test]
async fn history_rpc_stream_replays_state_and_requires_selection() {
    let f = Fixture::new().await;
    f.history(HistorySource::ClaudeCode, "native", "one");
    struct Rpc(GitActionsService);
    #[async_trait]
    impl zeron_rpc::RpcService for Rpc {
        async fn handle(
            &self,
            method: &str,
            params: Value,
        ) -> Result<zeron_rpc::RpcReply, zeron_rpc::RpcError> {
            rpc::dispatch(&self.0, method, params).await
        }
    }
    let client = zeron_rpc::memory_client(Arc::new(Rpc(f.service.clone())));
    assert!(
        client
            .import_cli_history(&HistoryImportRequest {
                space_id: "project".into(),
                ..Default::default()
            })
            .await
            .is_err()
    );
    let scan = client
        .scan_cli_history(&HistoryScanRequest {
            space_id: "project".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut stream = client.watch_cli_history_scan(&scan.scan_id).await.unwrap();
    let completed = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(value) = stream.recv().await {
            assert!(value.get("files").is_none());
            if value["status"] != "running" {
                return value;
            }
        }
        panic!("missing terminal scan state")
    })
    .await
    .unwrap();
    assert_eq!(completed["status"], "completed");
    let id = completed["candidateIds"][0].as_str().unwrap();
    let preview = client.preview_cli_history(id).await.unwrap();
    assert!(preview.provenance.unsupported_content);
    let state = client.cli_history_scan(&scan.scan_id).await.unwrap();
    assert_eq!(state.candidate_ids, vec![id.to_owned()]);
}

#[derive(Default)]
struct FakeForge {
    pushed: Mutex<Vec<PushCommit>>,
    created: Mutex<Vec<CreatePullRequest>>,
    linked: Mutex<Vec<(String, String)>>,
}
#[async_trait]
impl CommitPublisher for FakeForge {
    async fn push(&self, request: PushCommit) -> anyhow::Result<()> {
        self.pushed.lock().unwrap().push(request);
        Ok(())
    }
}
#[async_trait]
impl PullRequestCreator for FakeForge {
    async fn create(&self, request: CreatePullRequest) -> anyhow::Result<String> {
        assert_eq!(
            self.pushed.lock().unwrap().len(),
            1,
            "PR must follow separately authorized push"
        );
        let url = format!("{}/pull/42", request.repository);
        self.created.lock().unwrap().push(request);
        Ok(url)
    }
}
#[async_trait]
impl PullRequestLinker for FakeForge {
    async fn link(&self, thread_id: &str, url: &str) -> anyhow::Result<()> {
        self.linked
            .lock()
            .unwrap()
            .push((thread_id.into(), url.into()));
        Ok(())
    }
}

#[tokio::test]
async fn chain_creates_explicit_fork_pr_then_links_thread_and_replays_progress() {
    let f = Fixture::new().await;
    git(
        &f.repo,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/fork/fixture.git",
        ],
    )
    .await
    .unwrap();
    git(
        &f.repo,
        &[
            "remote",
            "add",
            "upstream",
            "https://github.com/upstream/fixture.git",
        ],
    )
    .await
    .unwrap();
    f.stage().await;
    let forge = Arc::new(FakeForge::default());
    f.service.set_commit_publisher(forge.clone());
    f.service.set_pr_creator(forge.clone());
    let kernel = crate::orchestration::Kernel::from_store(f.service.0.store.clone());
    let create = crate::orchestration::command::Command::wire(
        serde_json::from_value(json!({
            "type":"thread.create","commandId":"create-thread","threadId":"thread",
            "projectId":"project","title":"Fixture","createdBy":"user","creationSource":"web",
            "modelSelection":{"instanceId":"mock","model":"mock-1"},
            "runtimeMode":"full-access","interactionMode":"default",
            "branch":"main","worktreePath":f.repo
        }))
        .unwrap(),
    )
    .unwrap();
    kernel.dispatch(&create, crate::now_ms()).await.unwrap();
    let links = Arc::new(crate::orchestration::pull_requests::PullRequestService {
        kernel,
        host: Arc::new(crate::orchestration::pull_requests::host::GitHubHost::default()),
    });
    f.service
        .set_pr_linker(Arc::new(UserPullRequestLinker(links.clone())));
    let preview = f.preview().await;
    let mut request = f.request(&preview);
    request.authorize_push = true;
    request.authorize_create_pr = true;
    request.push_remote = Some(
        preview
            .checkout
            .remotes
            .iter()
            .find(|r| r.name == "origin")
            .unwrap()
            .clone(),
    );
    request.pr_repository = Some("https://github.com/fork/fixture".into());
    let state = f.service.start(request).await.unwrap();
    let final_state = f.wait("action", &state.action_id).await;
    assert_eq!(final_state["status"], "completed", "{final_state}");
    assert_eq!(final_state["prLinked"], true);
    let created = forge.created.lock().unwrap();
    assert_eq!(created[0].repository, "https://github.com/fork/fixture");
    assert_eq!(created[0].head, "fork:main");
    assert_eq!(created[0].body, "Fixture body");
    assert_eq!(
        forge.pushed.lock().unwrap()[0].remote_url,
        "https://github.com/fork/fixture.git"
    );
    use crate::orchestration::pull_requests::PullRequestLinks;
    let linked = links
        .links(&zeron_proto::orchestration::ThreadId("thread".into()))
        .unwrap();
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].url, "https://github.com/fork/fixture/pull/42");
    assert_eq!(
        linked[0].source,
        zeron_proto::orchestration::ThreadPullRequestLinkSource::Created
    );
    let progress: Vec<GitProgress> =
        serde_json::from_value(final_state["progress"].clone()).unwrap();
    assert_eq!(
        progress
            .iter()
            .map(|p| p.phase.as_str())
            .collect::<Vec<_>>(),
        vec![
            "commit", "commit", "push", "push", "pr", "pr", "link", "action"
        ]
    );
    assert!(progress.windows(2).all(|p| p[0].sequence < p[1].sequence));
    assert_eq!(
        f.service
            .0
            .store
            .git_action_state(&state.action_id)
            .unwrap()
            .unwrap()
            .pr_url
            .as_deref(),
        Some("https://github.com/fork/fixture/pull/42")
    );
}

#[tokio::test]
async fn no_pr_link_service_reports_pending_not_falsely_linked() {
    let f = Fixture::new().await;
    git(
        &f.repo,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/fork/fixture.git",
        ],
    )
    .await
    .unwrap();
    f.stage().await;
    let forge = Arc::new(FakeForge::default());
    f.service.set_commit_publisher(forge.clone());
    f.service.set_pr_creator(forge);
    let preview = f.preview().await;
    let mut request = f.request(&preview);
    request.authorize_push = true;
    request.authorize_create_pr = true;
    request.push_remote = Some(preview.checkout.remotes[0].clone());
    request.pr_repository = Some("https://github.com/fork/fixture".into());
    let state = f.service.start(request).await.unwrap();
    let final_state = f.wait("action", &state.action_id).await;
    assert_eq!(final_state["status"], "completed");
    assert_eq!(final_state["prLinked"], false);
    assert!(
        final_state["progress"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["kind"] == "link_pending")
    );
}

#[tokio::test]
async fn failed_pr_authority_preserves_created_url_and_uncertain_link() {
    let f = Fixture::new().await;
    git(
        &f.repo,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/fork/fixture.git",
        ],
    )
    .await
    .unwrap();
    f.stage().await;
    let forge = Arc::new(FakeForge::default());
    f.service.set_commit_publisher(forge.clone());
    f.service.set_pr_creator(forge);
    // No canonical thread exists: production authority must refuse the link.
    f.service
        .set_pr_linker(Arc::new(UserPullRequestLinker(Arc::new(
            crate::orchestration::pull_requests::PullRequestService {
                kernel: crate::orchestration::Kernel::from_store(f.service.0.store.clone()),
                host: Arc::new(crate::orchestration::pull_requests::host::GitHubHost::default()),
            },
        ))));
    let preview = f.preview().await;
    let mut request = f.request(&preview);
    request.authorize_push = true;
    request.authorize_create_pr = true;
    request.push_remote = Some(preview.checkout.remotes[0].clone());
    request.pr_repository = Some("https://github.com/fork/fixture".into());
    let state = f.service.start(request).await.unwrap();
    let final_state = f.wait("action", &state.action_id).await;
    assert_eq!(final_state["status"], "uncertain");
    assert_eq!(final_state["prLinked"], false);
    assert_eq!(
        final_state["prUrl"],
        "https://github.com/fork/fixture/pull/42"
    );
    assert!(
        final_state["error"]
            .as_str()
            .unwrap()
            .contains("Thread thread was not found.")
    );
    assert!(
        final_state["progress"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["kind"] == "failed")
    );
}

#[tokio::test]
async fn external_ref_changes_before_commit_cas_cannot_overwrite_head() {
    let f = Fixture::new().await;
    f.stage().await;
    let preview = f.preview().await;
    let request = f.request(&preview);
    // Hold service lane, accept request, then simulate an outside actor.
    let guards = f
        .service
        .0
        .locks
        .acquire([zeron_proto::orchestration::ThreadId(
            preview.checkout.checkout_id.clone(),
        )])
        .await;
    let state = f.service.start(request).await.unwrap();
    git(
        &f.repo,
        &["-c", "core.hooksPath=/dev/null", "commit", "-m", "external"],
    )
    .await
    .unwrap();
    let external = git(&f.repo, &["rev-parse", "HEAD"]).await.unwrap();
    drop(guards);
    let result = f.wait("action", &state.action_id).await;
    assert_eq!(result["status"], "uncertain");
    assert_eq!(
        git(&f.repo, &["rev-parse", "HEAD"]).await.unwrap(),
        external
    );
}

#[tokio::test]
async fn open_terminal_is_not_an_idle_checkout() {
    let f = Fixture::new().await;
    let policy = f.remote().await;
    f.service.set_pull_policy(policy).await.unwrap();
    let shell = if cfg!(windows) { "cmd.exe" } else { "/bin/sh" };
    let terminal = f
        .service
        .0
        .terminals
        .open_with_shell(f.repo.to_str().unwrap(), 80, 24, Some(shell))
        .unwrap();
    assert_eq!(
        f.service
            .retry_pull("project")
            .await
            .unwrap()
            .last_skip_reason
            .as_deref(),
        Some("checkout_not_idle")
    );
    f.service.0.terminals.close(&terminal.id).unwrap();
}

#[tokio::test]
async fn writing_preferences_are_durable_and_do_not_authorize_actions() {
    let f = Fixture::new().await;
    let settings = SourceControlSettings {
        style: WritingStyle::Custom,
        custom_instructions: "Use the user's configured style".into(),
        provider_instance_id: Some("mock".into()),
        model: Some("opaque-model".into()),
    };
    f.service.set_settings(settings.clone()).unwrap();
    assert_eq!(f.service.settings().unwrap(), settings);
    assert_eq!(
        f.service.0.store.source_control_settings().unwrap(),
        settings
    );
    assert!(
        f.service
            .set_settings(SourceControlSettings {
                provider_instance_id: Some("absent".into()),
                ..settings.clone()
            })
            .is_err()
    );
    f.stage().await;
    let preview = f.preview().await; // all explicit messages: no model call
    assert_eq!(preview.settings, settings);
    assert!(
        list::<GitActionState>(&f.service.0.store, "action")
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn restart_pauses_scan_and_import_without_provider_or_git_work() {
    let f = Fixture::new().await;
    let job = history::ScanJob {
        state: HistoryScanState {
            scan_id: "scan".into(),
            space_id: "project".into(),
            status: "running".into(),
            scanned_files: 7,
            ..Default::default()
        },
        cursor: 7,
        project_root: f.repo.to_string_lossy().into_owned(),
        ..Default::default()
    };
    put(&f.service.0.store, "scan", "scan", &job).unwrap();
    put(
        &f.service.0.store,
        "import",
        "import",
        &HistoryImportState {
            import_id: "import".into(),
            status: "running".into(),
            completed_candidate_ids: vec!["done".into()],
            ..Default::default()
        },
    )
    .unwrap();
    let reopened = GitActionsService::new(
        f.service.0.store.clone(),
        f.service.0.repos.clone(),
        f.service.0.sessions.clone(),
        f.service.0.terminals.clone(),
        f.service.0.doc_host.clone(),
        f.service.0.workspace.clone(),
        f.service.0.registry.clone(),
        vec![],
    )
    .unwrap();
    assert_eq!(
        reopened
            .state::<history::ScanJob>("scan", "scan")
            .unwrap()
            .state
            .status,
        "paused"
    );
    assert_eq!(
        reopened
            .state::<history::ScanJob>("scan", "scan")
            .unwrap()
            .cursor,
        7
    );
    let import: HistoryImportState = reopened.state("import", "import").unwrap();
    assert_eq!(import.status, "paused");
    assert_eq!(import.completed_candidate_ids, vec!["done"]);
    assert!(!f.service.0.sessions.any_active());
}

#[tokio::test]
async fn continuation_refuses_nonimported_activity_without_binding_native_resume() {
    let f = Fixture::new().await;
    let file = f.history(
        HistorySource::ClaudeCode,
        "00000000-0000-4000-8000-000000000002",
        "claude",
    );
    let preview = history::parse(
        HistorySource::ClaudeCode,
        &file,
        &f.repo,
        &CancellationToken::new(),
    )
    .unwrap()
    .0
    .unwrap();
    put(
        &f.service.0.store,
        "candidate",
        &preview.candidate_id,
        &preview,
    )
    .unwrap();
    let chat_id = f
        .service
        .import_one("project", &preview.candidate_id)
        .await
        .unwrap();
    let handle = f.service.0.doc_host.open_local(&chat_id).unwrap();
    handle
        .write_user_message("new-user-activity", "Do other work", crate::now_ms())
        .unwrap();
    assert!(
        f.service
            .continue_history("project", &preview.candidate_id, &chat_id)
            .await
            .unwrap_err()
            .to_string()
            .contains("non-imported")
    );
    assert!(
        f.service
            .0
            .workspace
            .chat(&chat_id)
            .unwrap()
            .unwrap()
            .harness_session_id
            .is_none()
    );
}

struct WriterHarness(Arc<Mutex<Vec<zeron_proto::RunRequest>>>);
#[async_trait]
impl zeron_harness::Harness for WriterHarness {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Fixture writer"
    }
    fn supports_steering(&self) -> bool {
        false
    }
    fn steering_mode(&self) -> zeron_proto::SteeringMode {
        zeron_proto::SteeringMode::TurnBoundary
    }
    fn reasoning_levels(&self) -> &[zeron_proto::ReasoningLevel] {
        &[]
    }
    async fn models(&self) -> Result<Vec<zeron_proto::Model>, zeron_harness::HarnessError> {
        Ok(vec![])
    }
    async fn run_source_control(
        &self,
        request: zeron_proto::RunRequest,
        controls: zeron_harness::RunControls,
    ) -> Result<
        futures::stream::BoxStream<
            'static,
            Result<zeron_proto::AgentEvent, zeron_harness::HarnessError>,
        >,
        zeron_harness::HarnessError,
    > {
        use futures::StreamExt;
        assert!(controls.mcp.entries().is_empty());
        self.0.lock().unwrap().push(request);
        Ok(futures::stream::iter([
            Ok(zeron_proto::AgentEvent::TextDelta { text:json!({"commitMessage":"feat: generated fixture","prTitle":"Generated fixture","prBody":"Generated summary"}).to_string() }),
            Ok(zeron_proto::AgentEvent::Done { status:zeron_proto::DoneStatus::Completed,result:None,error:None,session_id:None }),
        ]).boxed())
    }
    async fn run(
        &self,
        _: zeron_proto::RunRequest,
        _: zeron_harness::RunControls,
    ) -> Result<
        futures::stream::BoxStream<
            'static,
            Result<zeron_proto::AgentEvent, zeron_harness::HarnessError>,
        >,
        zeron_harness::HarnessError,
    > {
        panic!("Source-control generation must never use the ordinary coding path")
    }
}

#[tokio::test]
async fn generated_preview_uses_configured_style_model_and_restricted_scratch_run() {
    let f = Fixture::new().await;
    f.stage().await;
    let captured = Arc::new(Mutex::new(Vec::new()));
    f.service
        .0
        .registry
        .register(Arc::new(WriterHarness(captured.clone())));
    f.service
        .set_settings(SourceControlSettings {
            style: WritingStyle::Custom,
            custom_instructions: "Write exactly in my configured style".into(),
            provider_instance_id: Some("mock".into()),
            model: Some("configured-model".into()),
        })
        .unwrap();
    let preview = f
        .service
        .preview(GitPreviewRequest {
            thread_id: "thread".into(),
            cwd: f.repo.to_string_lossy().into_owned(),
            base_branch: "main".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(preview.commit_message, "feat: generated fixture");
    let captured = captured.lock().unwrap();
    assert_eq!(captured.len(), 1);
    let run = &captured[0];
    assert!(run.prompt.contains("Write exactly in my configured style"));
    assert!(run.prompt.contains("approved"));
    assert_eq!(run.model.as_deref(), Some("configured-model"));
    assert_ne!(run.cwd, f.repo.to_string_lossy());
    assert_eq!(run.sandbox, zeron_proto::SandboxLevel::ReadOnly);
    assert_eq!(run.runtime_mode, zeron_proto::RuntimeMode::ApprovalRequired);
    assert!(run.resume.is_none());
    assert!(!run.auto_approve);
    assert!(
        list::<GitActionState>(&f.service.0.store, "action")
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn generated_repo_conventions_include_instructions_and_recent_subjects() {
    let f = Fixture::new().await;
    f.stage().await;
    std::fs::write(f.repo.join("AGENTS.md"), "Fixture repository writing rules").unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    f.service
        .0
        .registry
        .register(Arc::new(WriterHarness(captured.clone())));
    f.service
        .set_settings(SourceControlSettings {
            provider_instance_id: Some("mock".into()),
            ..Default::default()
        })
        .unwrap();
    f.service
        .preview(GitPreviewRequest {
            thread_id: "thread".into(),
            cwd: f.repo.to_string_lossy().into_owned(),
            base_branch: "main".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let captured = captured.lock().unwrap();
    assert!(
        captured[0]
            .prompt
            .contains("Fixture repository writing rules")
    );
    assert!(captured[0].prompt.contains("Recent commit subjects:\nbase"));
}

#[tokio::test]
async fn configured_signing_policy_is_not_silently_bypassed() {
    let f = Fixture::new().await;
    f.stage().await;
    let preview = f.preview().await;
    git(&f.repo, &["config", "commit.gpgsign", "true"])
        .await
        .unwrap();
    let request = f.request(&preview);
    assert!(
        f.service
            .start(request.clone())
            .await
            .unwrap_err()
            .to_string()
            .contains("signing")
    );
    assert!(
        get::<Value>(&f.service.0.store, "request", &request.request_id)
            .unwrap()
            .is_none()
    );
}

struct FixtureClaude;
#[async_trait]
impl zeron_harness::Harness for FixtureClaude {
    fn id(&self) -> HarnessId {
        HarnessId::ClaudeCode
    }
    fn display_name(&self) -> &str {
        "Fixture Claude"
    }
    fn supports_steering(&self) -> bool {
        false
    }
    fn steering_mode(&self) -> zeron_proto::SteeringMode {
        zeron_proto::SteeringMode::TurnBoundary
    }
    fn reasoning_levels(&self) -> &[zeron_proto::ReasoningLevel] {
        &[]
    }
    async fn models(&self) -> Result<Vec<zeron_proto::Model>, zeron_harness::HarnessError> {
        Ok(vec![])
    }
    async fn run(
        &self,
        _: zeron_proto::RunRequest,
        _: zeron_harness::RunControls,
    ) -> Result<
        futures::stream::BoxStream<
            'static,
            Result<zeron_proto::AgentEvent, zeron_harness::HarnessError>,
        >,
        zeron_harness::HarnessError,
    > {
        panic!("Continuation confirmation binds only; it must not execute a provider")
    }
}

#[tokio::test]
async fn explicit_continuation_binds_only_matching_native_identity_and_approval_policy() {
    let f = Fixture::new().await;
    f.service.0.registry.register(Arc::new(FixtureClaude));
    let native = "00000000-0000-4000-8000-000000000002";
    let file = f.history(HistorySource::ClaudeCode, native, "claude");
    let preview = history::parse(
        HistorySource::ClaudeCode,
        &file,
        &f.repo,
        &CancellationToken::new(),
    )
    .unwrap()
    .0
    .unwrap();
    put(
        &f.service.0.store,
        "candidate",
        &preview.candidate_id,
        &preview,
    )
    .unwrap();
    let chat_id = f
        .service
        .import_one("project", &preview.candidate_id)
        .await
        .unwrap();
    f.service
        .continue_history("project", &preview.candidate_id, &chat_id)
        .await
        .unwrap();
    let row = f.service.0.workspace.chat(&chat_id).unwrap().unwrap();
    assert_eq!(row.harness_session_id.as_deref(), Some(native));
    assert_eq!(row.harness_session_cwd.as_deref(), f.repo.to_str());
    assert_eq!(row.config.as_ref().unwrap().harness, HarnessId::ClaudeCode);
    assert_eq!(
        row.config.unwrap().runtime_mode,
        zeron_proto::RuntimeMode::ApprovalRequired
    );
    assert!(!f.service.0.sessions.any_active());
    assert!(
        f.service
            .continue_history("project", &preview.candidate_id, &chat_id)
            .await
            .is_err()
    );
}
