use super::super::transfer::tests::fixture;
use super::*;

async fn repository() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("main");
    tokio::fs::create_dir(&main).await.unwrap();
    git::run(&main, &["init", "-b", "main"], None)
        .await
        .unwrap();
    // These fixtures assert exact restored bytes, not the runner's checkout EOL.
    git::run(&main, &["config", "core.autocrlf", "false"], None)
        .await
        .unwrap();
    tokio::fs::write(main.join("file.txt"), "initial\n")
        .await
        .unwrap();
    git::run(&main, &["add", "file.txt"], None).await.unwrap();
    git::run(
        &main,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@localhost",
            "commit",
            "-m",
            "initial",
        ],
        None,
    )
    .await
    .unwrap();
    let linked = dir.path().join("isolated");
    git::run(
        &main,
        &[
            "worktree",
            "add",
            "-b",
            "isolated",
            linked.to_str().unwrap(),
        ],
        None,
    )
    .await
    .unwrap();
    (dir, main, linked)
}

#[tokio::test]
async fn timeline_immutable_refs_reopen_preview_restore_backup_no_rewind() {
    let (_repo, _main, cwd) = repository().await;
    let (db, kernel, run) = fixture(&cwd);
    let service = FileCheckpointService {
        kernel: kernel.clone(),
    };
    let first = service
        .capture_turn(&"source".into(), &run, "started")
        .await
        .unwrap();
    tokio::fs::write(cwd.join("file.txt"), "agent change\n")
        .await
        .unwrap();
    tokio::fs::write(cwd.join("new.txt"), "new\n")
        .await
        .unwrap();
    let completed = service
        .capture_turn(&"source".into(), &run, "completed")
        .await
        .unwrap();
    assert_ne!(first.tree_sha, completed.tree_sha);
    assert_eq!(
        service
            .capture_turn(&"source".into(), &run, "completed")
            .await
            .unwrap()
            .tree_sha,
        completed.tree_sha
    );
    let reopened = Kernel::open(
        std::sync::Arc::new(zeron_sync::DocsStore::open(db.path()).unwrap()),
        "host",
    )
    .unwrap();
    assert_eq!(
        reopened
            .store
            .checkpoint_timeline(&"source".into())
            .unwrap()
            .len(),
        2
    );
    let fork = kernel
        .transfer_command(
            &"source".into(),
            "fork:checkpoint".into(),
            TransferOperation::Fork {
                target: "checkpoint-child".into(),
                source: super::super::transfer::SourcePoint::Checkpoint {
                    checkpoint_id: first.checkpoint.id.clone(),
                },
                title: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(fork.status, super::super::ReceiptStatus::Accepted);
    let state = kernel
        .store
        .transfer_ui_state(&"checkpoint-child".into())
        .unwrap();
    assert_eq!(state.transfers[0]["sourcePoint"]["runId"], run.id.0);
    // T3 canonicalizes checkpoint selection back to its run's current point.
    assert_eq!(
        state.transfers[0]["sourcePoint"]["checkpointId"],
        completed.checkpoint.id.0
    );
    // Remove the test fork's ownership claim before checking isolated restore.
    kernel.store.write(|tx| {
        let value: String = tx.query_row("SELECT payload_json FROM orchestration_projection_threads WHERE id='checkpoint-child'",[],|r| r.get(0))?;
        let mut child: serde_json::Value = serde_json::from_str(&value)?;
        child["deletedAt"] = json!("2026-10-04T00:00:00Z");
        tx.execute("UPDATE orchestration_projection_threads SET payload_json=?1 WHERE id='checkpoint-child'",[child.to_string()])?;Ok(())
    }).unwrap();
    let preview = service
        .preview(&"source".into(), &first.checkpoint.id)
        .await
        .unwrap();
    assert!(preview.allowed);
    assert_eq!(preview.paths.len(), 2);
    assert!(
        preview
            .paths
            .iter()
            .any(|p| p.path == "new.txt" && p.kind == "D")
    );
    let before_run = kernel.store.thread(&"source".into()).unwrap().unwrap().runs;
    let result = service
        .restore(
            &"source".into(),
            &first.checkpoint.id,
            preview.head_sha.as_deref(),
            &preview.checksum,
        )
        .await
        .unwrap();
    assert!(result.restored);
    assert_eq!(
        tokio::fs::read_to_string(cwd.join("file.txt"))
            .await
            .unwrap(),
        "initial\n"
    );
    assert!(!cwd.join("new.txt").exists());
    assert!(
        kernel
            .store
            .file_checkpoint(&result.backup_checkpoint_id.into())
            .unwrap()
            .is_some()
    );
    assert_eq!(
        kernel.store.thread(&"source".into()).unwrap().unwrap().runs,
        before_run
    );
}

#[tokio::test]
async fn refuses_main_checksum_head_and_checkout_lock() {
    let (_repo, main, cwd) = repository().await;
    let (_db, kernel, run) = fixture(&main);
    let service = FileCheckpointService { kernel };
    let checkpoint = service
        .capture_turn(&"source".into(), &run, "completed")
        .await
        .unwrap();
    let preview = service
        .preview(&"source".into(), &checkpoint.checkpoint.id)
        .await
        .unwrap();
    assert!(!preview.allowed);
    assert_eq!(
        preview.refusal.as_deref(),
        Some(SHARED_WORKSPACE_RESTORE_MESSAGE)
    );
    assert!(
        service
            .restore(
                &"source".into(),
                &checkpoint.checkpoint.id,
                preview.head_sha.as_deref(),
                &preview.checksum
            )
            .await
            .is_err()
    );
    let (_db, kernel, run) = fixture(&cwd);
    let service = FileCheckpointService { kernel };
    let checkpoint = service
        .capture_turn(&"source".into(), &run, "completed")
        .await
        .unwrap();
    let preview = service
        .preview(&"source".into(), &checkpoint.checkpoint.id)
        .await
        .unwrap();
    tokio::fs::write(cwd.join("file.txt"), "user edit\n")
        .await
        .unwrap();
    assert!(
        service
            .restore(
                &"source".into(),
                &checkpoint.checkpoint.id,
                preview.head_sha.as_deref(),
                &preview.checksum
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("workspace changed since preview")
    );
    assert!(
        service
            .restore(
                &"source".into(),
                &checkpoint.checkpoint.id,
                Some("wrong-head"),
                &preview.checksum
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("HEAD changed since preview")
    );
    let _lock = git::CheckoutLock::acquire(&cwd).await.unwrap();
    assert!(
        service
            .preview(&"source".into(), &checkpoint.checkpoint.id)
            .await
            .unwrap_err()
            .to_string()
            .contains("locked")
    );
}

#[tokio::test]
async fn refuses_archived_shared_nested_scope_and_symlink_alias() {
    let (_repo, _main, cwd) = repository().await;
    let (_db, kernel, run) = fixture(&cwd);
    let service = FileCheckpointService {
        kernel: kernel.clone(),
    };
    let checkpoint = service
        .capture_turn(&"source".into(), &run, "completed")
        .await
        .unwrap();
    let mut other = super::super::transfer::tests::sample("OrchestrationV2AppThread");
    other["id"] = json!("archived-other");
    other["projectId"] = json!("project");
    other["worktreePath"] = json!(cwd);
    other["archivedAt"] = json!("2026-10-04T00:00:00Z");
    kernel.store.write(|tx| {
        tx.execute("INSERT INTO orchestration_projection_threads(id,thread_id,last_sequence,payload_json) VALUES('archived-other','archived-other',1,?1)",[other.to_string()])?;
        Ok(())
    }).unwrap();
    assert_eq!(
        service
            .preview(&"source".into(), &checkpoint.checkpoint.id)
            .await
            .unwrap()
            .refusal
            .as_deref(),
        Some(SHARED_WORKSPACE_RESTORE_MESSAGE)
    );
    let nested = cwd.join("nested");
    tokio::fs::create_dir(&nested).await.unwrap();
    other["worktreePath"] = json!(nested);
    kernel.store.write(|tx| {
        tx.execute("UPDATE orchestration_projection_threads SET payload_json=?1 WHERE id='archived-other'",[other.to_string()])?; Ok(())
    }).unwrap();
    assert!(
        !service
            .preview(&"source".into(), &checkpoint.checkpoint.id)
            .await
            .unwrap()
            .allowed
    );
    #[cfg(unix)]
    {
        let alias = _repo.path().join("alias");
        std::os::unix::fs::symlink(&cwd, &alias).unwrap();
        other["worktreePath"] = json!(alias);
        kernel.store.write(|tx| {
            tx.execute("UPDATE orchestration_projection_threads SET payload_json=?1 WHERE id='archived-other'",[other.to_string()])?; Ok(())
        }).unwrap();
        assert!(
            !service
                .preview(&"source".into(), &checkpoint.checkpoint.id)
                .await
                .unwrap()
                .allowed
        );
    }
    // An unrelated thread's nested checkpoint scope still owns this checkout.
    other["worktreePath"] = json!(_main);
    let mut scope = json!(checkpoint.scope);
    scope["id"] = json!("other-scope");
    scope["threadId"] = json!("archived-other");
    scope["cwd"] = json!(nested);
    kernel.store.write(|tx| {
        tx.execute("UPDATE orchestration_projection_threads SET payload_json=?1 WHERE id='archived-other'",[other.to_string()])?;
        tx.execute("INSERT INTO orchestration_projection_records(thread_id,kind,id,payload_json,last_sequence) VALUES('archived-other','checkpoint-scope','other-scope',?1,1)",[scope.to_string()])?; Ok(())
    }).unwrap();
    assert!(
        !service
            .preview(&"source".into(), &checkpoint.checkpoint.id)
            .await
            .unwrap()
            .allowed
    );
    kernel.store.write(|tx| {
        other["deletedAt"] = json!("2026-10-04T00:01:00Z");
        tx.execute("UPDATE orchestration_projection_threads SET payload_json=?1 WHERE id='archived-other'",[other.to_string()])?; Ok(())
    }).unwrap();
    assert!(
        service
            .preview(&"source".into(), &checkpoint.checkpoint.id)
            .await
            .unwrap()
            .allowed
    );
}

#[tokio::test]
async fn captures_tracked_ignored_files_and_refuse_ignored_collision() {
    let (_repo, _main, cwd) = repository().await;
    let (_db, kernel, run) = fixture(&cwd);
    let service = FileCheckpointService { kernel };
    tokio::fs::write(cwd.join(".gitignore"), "file.txt\nignored.txt\n")
        .await
        .unwrap();
    tokio::fs::write(cwd.join("file.txt"), "tracked despite ignore\n")
        .await
        .unwrap();
    let first = service
        .capture_turn(&"source".into(), &run, "started")
        .await
        .unwrap();
    tokio::fs::write(cwd.join("file.txt"), "later\n")
        .await
        .unwrap();
    let preview = service
        .preview(&"source".into(), &first.checkpoint.id)
        .await
        .unwrap();
    assert!(preview.paths.iter().any(|p| p.path == "file.txt"));
    service
        .restore(
            &"source".into(),
            &first.checkpoint.id,
            preview.head_sha.as_deref(),
            &preview.checksum,
        )
        .await
        .unwrap();
    assert_eq!(
        tokio::fs::read_to_string(cwd.join("file.txt"))
            .await
            .unwrap(),
        "tracked despite ignore\n"
    );
    // Immutable target contains an untracked file that is later ignored.
    tokio::fs::write(cwd.join("added.txt"), "checkpoint data\n")
        .await
        .unwrap();
    let target = service
        .capture_turn(&"source".into(), &run, "completed")
        .await
        .unwrap();
    tokio::fs::remove_file(cwd.join("added.txt")).await.unwrap();
    tokio::fs::write(cwd.join(".gitignore"), "added.txt\n")
        .await
        .unwrap();
    tokio::fs::write(cwd.join("added.txt"), "uncheckpointed ignored user data\n")
        .await
        .unwrap();
    let preview = service
        .preview(&"source".into(), &target.checkpoint.id)
        .await
        .unwrap();
    assert!(!preview.allowed);
    assert_eq!(
        preview.refusal.as_deref(),
        Some("Checkpoint restore would overwrite ignored files.")
    );
    let error = service
        .restore(
            &"source".into(),
            &target.checkpoint.id,
            preview.head_sha.as_deref(),
            &preview.checksum,
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("overwrite ignored files"));
    assert_eq!(
        tokio::fs::read_to_string(cwd.join("added.txt"))
            .await
            .unwrap(),
        "uncheckpointed ignored user data\n"
    );
}

#[tokio::test]
async fn ref_before_sql_crash_recovers_original_tree_and_restore_keeps_staged_backup() {
    let (_repo, _main, cwd) = repository().await;
    let (_db, kernel, run) = fixture(&cwd);
    let service = FileCheckpointService {
        kernel: kernel.clone(),
    };
    kernel
        .store
        .inject_failure(super::super::WriteBoundary::BeforeCommit, 1);
    assert!(
        service
            .capture_turn(&"source".into(), &run, "started")
            .await
            .is_err()
    );
    assert!(
        kernel
            .store
            .checkpoint_timeline(&"source".into())
            .unwrap()
            .is_empty()
    );
    tokio::fs::write(cwd.join("file.txt"), "staged user change\n")
        .await
        .unwrap();
    git::run(&cwd, &["add", "file.txt"], None).await.unwrap();
    let index_before = git::run(&cwd, &["write-tree"], None).await.unwrap();
    let target = service
        .capture_turn(&"source".into(), &run, "started")
        .await
        .unwrap();
    let stored = git::run(
        &cwd,
        &["show", &format!("{}:file.txt", target.checkpoint.r#ref.0)],
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        stored, b"initial\n",
        "recover existing immutable ref, never recapture edited tree"
    );
    let preview = service
        .preview(&"source".into(), &target.checkpoint.id)
        .await
        .unwrap();
    let head_before = preview.head_sha.clone();
    let restored = service
        .restore(
            &"source".into(),
            &target.checkpoint.id,
            preview.head_sha.as_deref(),
            &preview.checksum,
        )
        .await
        .unwrap();
    git::run(&cwd, &["diff", "--cached", "--quiet"], None)
        .await
        .unwrap();
    let backup = kernel
        .store
        .file_checkpoint(&restored.backup_checkpoint_id.into())
        .unwrap()
        .unwrap();
    assert_eq!(
        backup.index_tree_sha,
        String::from_utf8(index_before).unwrap().trim()
    );
    let retained_index = git::run(
        &cwd,
        &[
            "rev-parse",
            &format!("{}^1^{{tree}}", backup.checkpoint.r#ref.0),
        ],
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        String::from_utf8(retained_index).unwrap().trim(),
        backup.index_tree_sha
    );
    let after = service
        .preview(&"source".into(), &target.checkpoint.id)
        .await
        .unwrap();
    assert_eq!(after.head_sha, head_before);
    assert!(after.paths.is_empty());
}

#[tokio::test]
async fn sparse_capture_refuses_before_publishing_a_partial_tree() {
    let (_repo, _main, cwd) = repository().await;
    let (_db, kernel, run) = fixture(&cwd);
    git::run(&cwd, &["sparse-checkout", "init", "--cone"], None)
        .await
        .unwrap();
    let error = FileCheckpointService {
        kernel: kernel.clone(),
    }
    .capture_turn(&"source".into(), &run, "completed")
    .await
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("sparse checkouts are not implemented")
    );
    assert!(
        kernel
            .store
            .checkpoint_timeline(&"source".into())
            .unwrap()
            .is_empty()
    );
    assert!(
        git::run(&cwd, &["for-each-ref", "refs/noches/checkpoints"], None)
            .await
            .unwrap()
            .is_empty()
    );
}
