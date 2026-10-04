#[test]
fn pending_ids_are_strict() {
    assert!(super::attachments::pending(
        &serde_json::json!({"id":"pending-123e4567-e89b-42d3-a456-426614174000"})
    ));
    assert!(!super::attachments::pending(
        &serde_json::json!({"id":"pending-../../escape"})
    ));
}

use super::{HostLaunchService, LaunchOperation, ToolError};
#[path = "ui_details_tests.rs"]
mod ui_details_tests;
use crate::mcp::auth::InvocationScope;
use crate::orchestration::launch_service::{LaunchService, LaunchThreadIntake, SendFailure};
use crate::orchestration::service::CallerScope;
use crate::orchestration::{Command, Kernel, Operation, ReceiptStatus, WriteBoundary};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use zeron_proto::orchestration::ThreadId;

#[derive(Default)]
struct Intake {
    trace: Mutex<Vec<Value>>,
    uncertain: bool,
}
#[async_trait]
impl LaunchThreadIntake for Intake {
    async fn send(
        &self,
        input: crate::orchestration::thread_service::ThreadSendRequest,
    ) -> Result<Value, SendFailure> {
        self.trace
            .lock()
            .unwrap()
            .push(json!({"thread":input.thread_id,"text":input.text,"attachments":input.attachments,
                "queue":input.mode == zeron_proto::orchestration_mcp::T3ThreadSendInputMode::Queue,
                "senderThreadId":input.sender_thread_id,"createdBy":input.created_by,"creationSource":input.creation_source}));
        if self.uncertain {
            return Err(SendFailure {
                error: super::unavailable(),
                uncertain: true,
            });
        }
        Ok(
            json!({"threadId":input.thread_id,"messageId":input.message_id,"runId":"run:test","status":"starting"}),
        )
    }
    async fn detach(&self, thread: &str) -> Result<(), ToolError> {
        self.trace.lock().unwrap().push(json!({"detach":thread}));
        Ok(())
    }
}

struct Rig {
    dir: tempfile::TempDir,
    service: HostLaunchService,
    scope: InvocationScope,
    intake: Arc<Intake>,
}
impl Rig {
    async fn new() -> Self {
        Self::with_intake(Arc::new(Intake::default())).await
    }
    async fn with_intake(intake: Arc<Intake>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(zeron_sync::DocsStore::open(dir.path()).unwrap());
        let kernel = Kernel::open(store.clone(), "host").unwrap();
        let root = dir.path().join("repo");
        std::fs::create_dir(&root).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let repos =
            crate::Repos::with_worktrees_root(dir.path(), "host", dir.path().join("worktrees"));
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.name", "Test"],
            vec!["config", "user.email", "test@example.invalid"],
        ] {
            repos.orchestration_git(&root, &args).await.unwrap();
        }
        std::fs::write(root.join("base.txt"), "base\n").unwrap();
        repos
            .orchestration_git(&root, &["add", "base.txt"])
            .await
            .unwrap();
        repos
            .orchestration_git(&root, &["commit", "-m", "Base"])
            .await
            .unwrap();
        let workspace = crate::WorkspaceHost::open(
            store,
            crate::workspace_host::WorkspaceHostConfig {
                device_id: "host".into(),
                device_name: "test".into(),
                platform: "linux".into(),
                org_id: "test".into(),
                user_id: "test".into(),
                edge: None,
            },
        )
        .unwrap();
        workspace
            .create_space(
                "project",
                "host",
                root.to_str().unwrap(),
                Some("Project".into()),
                true,
            )
            .unwrap();
        workspace
            .create_chat(
                "parent",
                Some("project"),
                Some("host"),
                None,
                Some(root.to_string_lossy().into_owned()),
            )
            .unwrap();
        let registry = Arc::new(crate::HarnessRegistry::new());
        registry.register(Arc::new(zeron_harness::mock::MockHarness {
            script: vec![],
        }));
        let service = HostLaunchService::new(
            kernel,
            workspace,
            repos,
            crate::ProjectActionsStore::open(dir.path()).unwrap(),
            crate::Terminals::new(),
            registry,
            dir.path().to_path_buf(),
            intake.clone(),
        )
        .unwrap();
        service.projects().unwrap();
        let create=Command::wire(serde_json::from_value(json!({"type":"thread.create","commandId":"parent-create",
            "threadId":"parent","projectId":"project","title":"Parent","createdBy":"user","creationSource":"web",
            "modelSelection":{"instanceId":"mock","model":"mock-1"},"runtimeMode":"full-access","interactionMode":"default","branch":"main","worktreePath":null})).unwrap()).unwrap();
        assert_eq!(
            service
                .kernel
                .dispatch(&create, crate::now_ms())
                .await
                .unwrap()
                .status,
            ReceiptStatus::Accepted
        );
        let parent = service
            .kernel
            .store
            .thread(&ThreadId("parent".into()))
            .unwrap()
            .unwrap();
        let seed = crate::orchestration::task::execution_seed(
            &parent.thread,
            1,
            "input",
            "starting",
            "mock",
            crate::now_ms(),
        )
        .unwrap();
        let run_id = seed.run.id.clone();
        service
            .kernel
            .dispatch(
                &Command {
                    id: "parent-seed".into(),
                    thread_id: "parent".into(),
                    operation: Operation::CreateExecution(Box::new(seed)),
                },
                crate::now_ms(),
            )
            .await
            .unwrap();
        let scope = InvocationScope {
            environment_id: "host".into(),
            caller: CallerScope {
                thread_id: "parent".into(),
                run_id,
                session_id: "session".into(),
                project_id: "project".into(),
                workspace_root: root,
                runtime_mode: zeron_proto::RuntimeMode::FullAccess,
                interaction_mode: zeron_proto::InteractionMode::Default,
                provider_instance_id: "mock".into(),
            },
            selection: parent.thread.model_selection,
            capabilities: ["orchestration", "worktree"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            issued_at: 0,
            task_id: None,
        };
        Self {
            dir,
            service,
            scope,
            intake,
        }
    }
    async fn call(&self, name: &str, args: Value) -> Value {
        let result = self.service.call(&self.scope, name, args).await;
        let descriptor = zeron_proto::orchestration_mcp::pinned_tool_inventory()
            .into_iter()
            .find(|t| t.name == name)
            .unwrap();
        if result.get("_tag").is_none() {
            crate::mcp::codec::validate(&descriptor.result_schema, &result)
                .unwrap_or_else(|e| panic!("{name} output schema: {e}\n{result}"));
        }
        result
    }
    async fn settled(&self, thread: &str) -> Value {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let w = self.service.workflow(thread).unwrap();
                if w["status"] != "preparing" {
                    return w;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        self.service.terminals.shutdown();
        self.service.workspace.shutdown();
    }
}

struct ReplayScheduledDispatch {
    adapter: crate::orchestration::scheduler::dispatch::ThreadDispatch,
    kernel: Kernel,
    runs: Mutex<Vec<crate::orchestration::scheduler::ScheduledDispatch>>,
}
#[async_trait]
impl crate::orchestration::scheduler::ScheduledTaskDispatch for ReplayScheduledDispatch {
    async fn dispatch(
        &self,
        run: crate::orchestration::scheduler::ScheduledDispatch,
    ) -> Result<(), String> {
        self.runs.lock().unwrap().push(run.clone());
        self.kernel
            .store
            .inject_failure(WriteBoundary::AfterCommit, 1);
        assert!(self.adapter.dispatch(run.clone()).await.is_err());
        // Replay after the committed acceptance response was lost.
        // No new command ID or message ID is allocated on replay.
        self.adapter.dispatch(run).await
    }
}

#[tokio::test]
async fn unbound_schedules_launch_fresh_top_level_threads_with_strategy_and_replay_identity() {
    use crate::orchestration::{scheduler::Scheduler, task::records};
    use zeron_proto::orchestration::{Optional, ScheduledTaskRunStatus};
    for strategy in [
        json!({"type":"root"}),
        json!({"type":"existing_worktree","worktreePath":"ROOT"}),
        json!({"type":"worktree","baseRef":"main","startFromOrigin":false}),
    ] {
        let rig = Rig::new().await;
        let strategy = if strategy["type"] == "existing_worktree" {
            json!({"type":"existing_worktree","worktreePath":rig.scope.caller.workspace_root})
        } else {
            strategy
        };
        let threads = Arc::new(crate::orchestration::threads::KernelThreadService {
            kernel: rig.service.kernel.clone(),
            delegation: Arc::new(crate::orchestration::task::DelegationService {
                kernel: rig.service.kernel.clone(),
                targets: Arc::new(crate::orchestration::assembly::HostCatalog(
                    rig.service.registry.clone(),
                )),
            }),
        });
        let dispatcher = Arc::new(ReplayScheduledDispatch {
            adapter: crate::orchestration::scheduler::dispatch::ThreadDispatch {
                store: rig.service.kernel.store.clone(),
                threads,
                launch: Some(Arc::new(rig.service.clone())),
            },
            kernel: rig.service.kernel.clone(),
            runs: Mutex::new(vec![]),
        });
        let scheduler = Scheduler::new(rig.service.kernel.store.clone(), dispatcher.clone());
        let mut input: zeron_proto::orchestration::ScheduledTaskUpsertInput = serde_json::from_value(json!({
            "id":"fresh","title":"Scheduled review","prompt":"Review changes","enabled":true,
            "schedule":{"type":"interval","everyMs":60000},"projectId":"project","threadId":null,
            "workspaceStrategy":strategy,"modelSelection":{"instanceId":"mock","model":"mock-1"},
            "runtimeMode":"full-access","interactionMode":"default"
        })).unwrap();
        input.created_by =
            Optional::Present(zeron_proto::orchestration::OrchestrationV2Actor::System);
        input.creation_source =
            Optional::Present(zeron_proto::orchestration::OrchestrationV2CreationSource::Server);
        scheduler.upsert(input).unwrap();
        for _ in 0..2 {
            assert_eq!(
                scheduler
                    .run_now(&"fresh".into())
                    .await
                    .unwrap()
                    .last_run_status,
                ScheduledTaskRunStatus::Succeeded
            );
        }
        let runs = dispatcher.runs.lock().unwrap().clone();
        assert_ne!(runs[0].command_id, runs[1].command_id);
        for run in runs {
            let tid = &run.command_id.0;
            let w = rig.settled(tid).await;
            assert_eq!(w["status"], "ready", "{w}");
            assert_eq!(w["strategy"], strategy);
            let p = rig
                .service
                .kernel
                .store
                .thread(&tid.clone().into())
                .unwrap()
                .unwrap();
            assert!(p.thread.lineage.parent_thread_id.is_none());
            assert_eq!(
                p.thread.created_by,
                zeron_proto::orchestration::OrchestrationV2Actor::System
            );
            assert_eq!(p.runs.len(), 1);
            let messages = records(&p, "message");
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0]["id"], run.message_id.0);
            assert_eq!(messages[0]["scheduledTaskId"], "fresh");
            assert_eq!(messages[0]["createdBy"], "system");
            assert_eq!(messages[0]["creationSource"], "server");
            assert!(messages[0].get("senderThreadId").is_none());
            if strategy["type"] == "root" {
                assert!(p.thread.worktree_path.is_none());
            } else {
                assert!(p.thread.worktree_path.is_some());
            }
        }
    }
}

#[tokio::test]
async fn managed_project_title_only_scaffolds_and_soft_commit_failure() {
    let rig = Rig::new().await;
    let refused = rig
        .call("t3_project_create", json!({"title":"Demo","scripts":[]}))
        .await;
    assert_eq!(
        refused["message"],
        "A project started from its title takes only a title; set scripts or defaultModelSelection afterwards with t3_project_update."
    );
    let project = rig
        .call("t3_project_create", json!({"title":"Demo App"}))
        .await;
    assert!(project.get("_tag").is_none(), "{project}");
    let root = std::path::Path::new(project["workspaceRoot"].as_str().unwrap());
    assert!(root.join("README.md").exists());
    assert!(root.join("assets/icon.svg").exists());
    assert!(rig.service.repos.is_repo(root).await);
    let second = rig
        .call("t3_project_create", json!({"title":"Demo App"}))
        .await;
    assert_ne!(project["workspaceRoot"], second["workspaceRoot"]);
    // Either commit or its exact soft-failure field; no missing project.
    assert!(
        project.get("commitError").is_some()
            || rig.service.repos.head_sha(root).await.unwrap().is_some()
    );
}

#[tokio::test]
async fn project_force_delete_never_removes_repository_and_can_register_again() {
    let rig = Rig::new().await;
    let root = rig.scope.caller.workspace_root.clone();
    let refused = rig
        .call("t3_project_delete", json!({"projectId":"project"}))
        .await;
    assert_eq!(
        refused["message"],
        "The project is not empty; force=true is required to delete it."
    );
    let deleted = rig
        .call(
            "t3_project_delete",
            json!({"projectId":"project","force":true}),
        )
        .await;
    assert!(!deleted["deletedAt"].is_null(), "{deleted}");
    assert!(root.join("base.txt").exists());
    assert!(root.join(".git").exists());
}

#[tokio::test]
async fn clone_refuses_existing_destination_without_adopting_or_removing_it() {
    let rig = Rig::new().await;
    let result=rig.call("t3_project_clone",json!({"remoteUrl":rig.scope.caller.workspace_root,"destinationPath":rig.scope.caller.workspace_root})).await;
    assert_eq!(result["code"], "orchestration_error");
    assert!(rig.scope.caller.workspace_root.join("base.txt").exists());
    let destination = rig.dir.path().join("clone");
    let result = rig
        .call(
            "t3_project_clone",
            json!({"remoteUrl":rig.scope.caller.workspace_root,"destinationPath":destination}),
        )
        .await;
    assert_eq!(result["cwd"], destination.to_string_lossy().as_ref());
    assert!(destination.join("base.txt").exists());
}

#[tokio::test]
async fn environment_partial_clear_and_codepoint_read_limit() {
    let rig = Rig::new().await;
    rig.call("t3_environment_read", json!({})).await;
    let text = "😀".repeat(4001);
    let first=rig.call("t3_environment_preferences_update",json!({"newWorktreesStartFromOrigin":true,"sourceControlWritingStyle":{"customInstructions":text}})).await;
    assert_eq!(
        first["sourceControlWritingStyle"]["customInstructions"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        4000
    );
    assert_eq!(first["sourceControlWritingStyle"]["truncated"], true);
    let second = rig
        .call(
            "t3_environment_preferences_update",
            json!({"sourceControlWritingStyle":{"customInstructions":""}}),
        )
        .await;
    assert_eq!(second["newWorktreesStartFromOrigin"], true);
    assert_eq!(
        second["sourceControlWritingStyle"]["customInstructions"],
        ""
    );
    assert_eq!(
        second["sourceControlWritingStyle"]["mode"],
        "repo_conventions"
    );
    let mut wrong = rig.scope.clone();
    wrong.environment_id = "other".into();
    let result = rig
        .service
        .call(&wrong, "t3_environment_read", json!({}))
        .await;
    assert_eq!(
        result["message"],
        "This credential belongs to another environment."
    );
}

async fn upload(rig: &Rig) -> Value {
    let result=rig.call("t3_attachment_prepare_upload",json!({"upload":{"type":"file","name":"note.txt","mimeType":"text/plain","sizeBytes":4}})).await;
    let token = result["relativeUrl"]
        .as_str()
        .unwrap()
        .strip_prefix("/api/attachments/upload/")
        .unwrap();
    assert_eq!(rig.service.upload(token, b"bad").await.0, 400);
    assert_eq!(
        rig.service.upload(&format!("{token}x"), b"note").await.0,
        401
    );
    assert_eq!(rig.service.upload(token, b"note").await.0, 200);
    json!({"id":result["attachmentId"],"type":"file","name":"note.txt","mimeType":"text/plain","sizeBytes":4})
}

#[tokio::test]
async fn signed_upload_claim_copy_and_pending_only_discard() {
    let rig = Rig::new().await;
    let a = upload(&rig).await;
    let (claimed, paths) = rig.service.claim("parent", vec![a.clone()]).unwrap();
    assert!(!super::attachments::pending(&claimed[0]));
    assert_eq!(std::fs::read(&paths[0]).unwrap(), b"note");
    rig.call(
        "t3_attachment_discard",
        json!({"attachmentId":claimed[0]["id"]}),
    )
    .await;
    assert!(paths[0].exists());
    rig.call("t3_attachment_discard", json!({"attachmentId":a["id"]}))
        .await;
    assert!(rig.service.claim("parent", vec![a]).is_err());
    assert!(paths[0].exists());
}

#[tokio::test]
async fn duplicate_claims_rollback_and_uncertain_send_retains_claimed_files() {
    let rig = Rig::with_intake(Arc::new(Intake {
        uncertain: true,
        ..Default::default()
    }))
    .await;
    let a = upload(&rig).await;
    assert_eq!(
        rig.service
            .claim("parent", vec![a.clone(), a.clone()])
            .unwrap_err()
            .message,
        "Duplicate attachment ids are not allowed."
    );
    let result = rig
        .call("t3_thread_send_attachments", json!({"attachments":[a]}))
        .await;
    assert_eq!(result["code"], "orchestration_error");
    let delivered = rig.intake.trace.lock().unwrap()[0]["attachments"][0].clone();
    let path = rig
        .dir
        .path()
        .join("attachments")
        .join(format!("{}.txt", delivered["id"].as_str().unwrap()));
    assert!(path.exists());
}

#[tokio::test]
async fn launch_root_default_is_top_level_and_never_copies_caller_worktree() {
    let rig = Rig::new().await;
    let mut caller = rig
        .service
        .kernel
        .store
        .thread(&ThreadId("parent".into()))
        .unwrap()
        .unwrap()
        .thread;
    caller.worktree_path = Some("/other/worktree".into());
    rig.service
        .kernel
        .dispatch(
            &Command {
                id: "binding".into(),
                thread_id: "parent".into(),
                operation: Operation::SessionBinding(Box::new(caller)),
            },
            crate::now_ms(),
        )
        .await
        .unwrap();
    let result = rig
        .call(
            "t3_thread_launch",
            json!({"title":"Independent","message":"Work"}),
        )
        .await;
    assert_eq!(result["status"], "preparing", "{result}");
    let tid = result["threadId"].as_str().unwrap();
    let w = rig.settled(tid).await;
    assert_eq!(w["status"], "ready", "{w}");
    let p = rig
        .service
        .kernel
        .store
        .thread(&ThreadId(tid.into()))
        .unwrap()
        .unwrap();
    assert!(p.thread.worktree_path.is_none());
    assert!(p.thread.lineage.parent_thread_id.is_none());
    assert_eq!(
        p.runs[0].status,
        zeron_proto::orchestration::OrchestrationV2RunStatus::Starting
    );
    let message = crate::orchestration::task::records(&p, "message");
    assert_eq!(message[0]["senderThreadId"], "parent");
    let idle = rig.call("t3_thread_launch", json!({"title":"Idle"})).await;
    assert!(idle["runId"].is_null());
    assert!(idle["status"].is_null());
}

#[tokio::test]
async fn launch_local_base_has_dirty_isolation_and_existing_checkout_binding() {
    let rig = Rig::new().await;
    std::fs::write(
        rig.scope.caller.workspace_root.join("dirty.txt"),
        "do not copy",
    )
    .unwrap();
    let result=rig.call("t3_thread_launch",json!({"title":"Branch","message":"Work","workspaceStrategy":{"type":"worktree","baseRef":"main","branch":"feature/local","startFromOrigin":false}})).await;
    let w = rig.settled(result["threadId"].as_str().unwrap()).await;
    assert_eq!(w["status"], "ready", "{w}");
    let checkout = std::path::Path::new(w["worktreePath"].as_str().unwrap());
    assert!(checkout.join("base.txt").exists());
    assert!(!checkout.join("dirty.txt").exists());
    let existing=rig.call("t3_thread_launch",json!({"title":"Existing","workspaceStrategy":{"type":"existing_worktree","worktreePath":checkout,"branch":"feature/local"}})).await;
    let w = rig.settled(existing["threadId"].as_str().unwrap()).await;
    assert_eq!(w["status"], "ready", "{w}");
    assert_eq!(
        std::fs::canonicalize(w["worktreePath"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(checkout).unwrap()
    );
}

#[tokio::test]
async fn scratch_exclusivity_and_distinct_nonrepo_directories() {
    let rig = Rig::new().await;
    let invalid = rig
        .call(
            "t3_thread_launch",
            json!({"title":"Scratch","scratch":true,"projectId":"project"}),
        )
        .await;
    assert_eq!(
        invalid["message"],
        "scratch:true picks its own project and folder; omit projectId and workspaceStrategy."
    );
    let a = rig
        .call("t3_thread_launch", json!({"title":"A","scratch":true}))
        .await;
    let b = rig
        .call("t3_thread_launch", json!({"title":"B","scratch":true}))
        .await;
    let wa = rig.settled(a["threadId"].as_str().unwrap()).await;
    let wb = rig.settled(b["threadId"].as_str().unwrap()).await;
    assert_ne!(wa["worktreePath"], wb["worktreePath"]);
    assert!(
        !rig.service
            .repos
            .is_repo(std::path::Path::new(wa["worktreePath"].as_str().unwrap()))
            .await
    );
}

#[tokio::test]
async fn handoff_binding_continuation_precedes_detach_and_repeated_call_refuses() {
    let rig = Rig::new().await;
    let result=rig.call("t3_worktree_handoff",json!({"branch":"feature/handoff","startFromOrigin":false,"continuationPrompt":"Continue work"})).await;
    assert_eq!(result["continuation"]["status"], "scheduled", "{result}");
    let p = rig
        .service
        .kernel
        .store
        .thread(&ThreadId("parent".into()))
        .unwrap()
        .unwrap();
    assert_eq!(
        p.thread.worktree_path.as_deref(),
        result["worktreePath"].as_str()
    );
    assert!(
        crate::orchestration::task::records(&p, "message")
            .iter()
            .any(|m| m["text"] == "Continue work")
    );
    assert_eq!(rig.intake.trace.lock().unwrap()[0]["detach"], "parent");
    let again = rig
        .call("t3_worktree_handoff", json!({"branch":"feature/second"}))
        .await;
    assert_eq!(again["_tag"], "WorktreeMcpFailure");
    assert_eq!(again["code"], "already_in_worktree");
    let state = rig.call("t3_worktree_status", json!({})).await;
    assert_eq!(state["attached"], true);
}

#[tokio::test]
async fn binding_cas_rejects_atomically_without_losing_continuation() {
    let rig = Rig::new().await;
    let result = rig
        .service
        .operation(
            "parent",
            "first-bind",
            LaunchOperation::Bind {
                expected: None,
                path: Some("/one".into()),
                branch: Some("one".into()),
                continuation: Some("First".into()),
                driver: "mock".into(),
                workflow: None,
            },
        )
        .await;
    assert!(result.is_ok());
    let result = rig
        .service
        .operation(
            "parent",
            "stale-bind",
            LaunchOperation::Bind {
                expected: None,
                path: Some("/two".into()),
                branch: Some("two".into()),
                continuation: Some("Second".into()),
                driver: "mock".into(),
                workflow: None,
            },
        )
        .await;
    assert!(result.is_err());
    let p = rig
        .service
        .kernel
        .store
        .thread(&ThreadId("parent".into()))
        .unwrap()
        .unwrap();
    assert_eq!(p.thread.worktree_path.as_deref(), Some("/one"));
    assert!(
        !crate::orchestration::task::records(&p, "message")
            .iter()
            .any(|m| m["text"] == "Second")
    );
}

#[tokio::test]
async fn preparing_identity_survives_kernel_recovery_and_release_is_receipted() {
    let rig = Rig::new().await;
    let mut thread = rig
        .service
        .kernel
        .store
        .thread(&ThreadId("parent".into()))
        .unwrap()
        .unwrap()
        .thread;
    thread.id = ThreadId("prepared".into());
    thread.lineage.root_thread_id = thread.id.clone();
    let w = json!({"threadId":"prepared","projectId":"project","commandId":"prepared-create","status":"preparing","stage":"fetch",
        "strategy":{"type":"root"},"driver":"mock","initialMessage":{"text":"Work","attachments":[]}});
    rig.service
        .operation(
            "prepared",
            "prepared-create",
            LaunchOperation::Create {
                thread: Box::new(thread),
                workflow: w,
                driver: "mock".into(),
            },
        )
        .await
        .unwrap();
    rig.service.kernel.recover(crate::now_ms()).await.unwrap();
    let before = rig
        .service
        .kernel
        .store
        .thread(&ThreadId("prepared".into()))
        .unwrap()
        .unwrap();
    assert_eq!(
        before.runs[0].status,
        zeron_proto::orchestration::OrchestrationV2RunStatus::Preparing
    );
    rig.service
        .operation("prepared", "prepared-release", LaunchOperation::Release)
        .await
        .unwrap();
    rig.service
        .operation("prepared", "prepared-release", LaunchOperation::Release)
        .await
        .unwrap();
    let after = rig
        .service
        .kernel
        .store
        .thread(&ThreadId("prepared".into()))
        .unwrap()
        .unwrap();
    assert_eq!(before.runs[0].id, after.runs[0].id);
    assert_eq!(after.runs.len(), 1);
}

#[tokio::test]
async fn crash_before_and_after_acceptance_preserves_atomic_workflow_and_claims() {
    for boundary in [WriteBoundary::BeforeCommit, WriteBoundary::AfterCommit] {
        let rig = Rig::new().await;
        rig.service.kernel.store.inject_failure(boundary, 1);
        let result = rig
            .call(
                "t3_thread_launch",
                json!({"title":"Crash","message":"Work"}),
            )
            .await;
        assert_eq!(result["code"], "orchestration_error");
        let rows = rig.service.rows("orchestration_launch_workflows").unwrap();
        if boundary == WriteBoundary::BeforeCommit {
            assert!(rows.is_empty());
        } else {
            assert_eq!(rows.len(), 1);
            assert!(
                rig.service
                    .kernel
                    .store
                    .thread(&ThreadId(rows[0]["threadId"].as_str().unwrap().into()))
                    .unwrap()
                    .is_some()
            );
        }
    }
}

#[tokio::test]
async fn setup_opt_in_blocks_on_exit_and_failure_requires_explicit_continue() {
    let rig = Rig::new().await;
    let update=rig.call("t3_project_update",json!({"projectId":"project","scripts":[{"id":"setup","name":"Setup","command":"false","icon":"configure","runOnWorktreeCreate":true,"async":false}]})).await;
    assert!(update.get("_tag").is_none(), "{update}");
    let result = rig
        .call("t3_thread_launch", json!({"title":"Gate","message":"Work"}))
        .await;
    let tid = result["threadId"].as_str().unwrap();
    let w = rig.settled(tid).await;
    assert_eq!(w["status"], "blocked", "{w}");
    assert_eq!(w["setup"]["exitCode"], 1);
    assert!(
        rig.service
            .kernel
            .store
            .thread(&ThreadId(tid.into()))
            .unwrap()
            .unwrap()
            .runs[0]
            .status
            == zeron_proto::orchestration::OrchestrationV2RunStatus::Preparing
    );
    rig.service
        .setup_control(zeron_proto::launch::SetupControlParams {
            chat_id: tid.into(),
            run_id: w["setup"]["runId"].as_str().unwrap().into(),
            action: "continue".into(),
        })
        .await
        .unwrap();
    assert_eq!(rig.settled(tid).await["status"], "ready");
}

#[tokio::test]
async fn worktree_refs_are_branches_not_detached_checkouts_and_keep_remote_identity() {
    let rig = Rig::new().await;
    let root = &rig.scope.caller.workspace_root;
    rig.service
        .repos
        .orchestration_git(root, &["update-ref", "refs/remotes/origin/main", "HEAD"])
        .await
        .unwrap();
    rig.service
        .repos
        .orchestration_git(
            root,
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/main",
            ],
        )
        .await
        .unwrap();
    let detached = rig.dir.path().join("detached");
    rig.service
        .repos
        .orchestration_git(
            root,
            &[
                "worktree",
                "add",
                "--detach",
                detached.to_str().unwrap(),
                "HEAD",
            ],
        )
        .await
        .unwrap();
    let refs = rig.call("t3_worktree_list", json!({})).await;
    assert_eq!(refs["totalCount"], 1);
    assert_eq!(refs["refs"][0]["name"], "main");
    assert_eq!(
        refs["refs"][0]["worktreePath"],
        root.to_string_lossy().as_ref()
    );
    assert!(
        refs["refs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["worktreePath"] != detached.to_string_lossy().as_ref())
    );
    let remote = rig
        .call(
            "t3_worktree_list",
            json!({"includeMatchingRemoteRefs":true,"refKind":"remote"}),
        )
        .await;
    assert_eq!(remote["refs"][0]["name"], "origin/main");
    assert_eq!(remote["refs"][0]["isDefault"], true);
}

#[tokio::test]
async fn origin_launch_uses_upstream_commit_not_local_dirty_or_unpushed_commits() {
    let rig = Rig::new().await;
    let root = &rig.scope.caller.workspace_root;
    let origin = rig.dir.path().join("origin.git");
    rig.service
        .repos
        .orchestration_git(
            root,
            &[
                "clone",
                "--bare",
                root.to_str().unwrap(),
                origin.to_str().unwrap(),
            ],
        )
        .await
        .unwrap();
    rig.service
        .repos
        .orchestration_git(root, &["remote", "add", "origin", origin.to_str().unwrap()])
        .await
        .unwrap();
    std::fs::write(root.join("local-only.txt"), "local").unwrap();
    rig.service
        .repos
        .orchestration_git(root, &["add", "local-only.txt"])
        .await
        .unwrap();
    rig.service
        .repos
        .orchestration_git(root, &["commit", "-m", "Local only"])
        .await
        .unwrap();
    let result=rig.call("t3_thread_launch",json!({"title":"Origin","message":"Work","workspaceStrategy":{"type":"worktree","baseRef":"main","branch":"feature/origin","startFromOrigin":true}})).await;
    let w = rig.settled(result["threadId"].as_str().unwrap()).await;
    assert_eq!(w["status"], "ready", "{w}");
    let path = std::path::Path::new(w["worktreePath"].as_str().unwrap());
    assert!(path.join("base.txt").exists());
    assert!(!path.join("local-only.txt").exists());
    assert_eq!(w["fetchCompleted"], true);
}

#[tokio::test]
async fn handoff_blocking_setup_holds_continuation_until_exit_and_ui_read_is_passive() {
    let rig = Rig::new().await;
    rig.call("t3_project_update",json!({"projectId":"project","scripts":[{"id":"setup","name":"Setup","command":"false","icon":"configure","runOnWorktreeCreate":true,"async":false}]})).await;
    let result=rig.call("t3_worktree_handoff",json!({"branch":"feature/gated","startFromOrigin":false,"continuationPrompt":"Continue"})).await;
    assert_eq!(result["setupScript"]["status"], "started", "{result}");
    let w = rig.settled("parent").await;
    assert_eq!(w["status"], "blocked", "{w}");
    let p = rig
        .service
        .kernel
        .store
        .thread(&ThreadId("parent".into()))
        .unwrap()
        .unwrap();
    assert_eq!(p.runs.last().unwrap().queue_held.as_ref(), Some(&true));
    let frontier = rig.service.kernel.store.projection_frontier().unwrap();
    let ui = rig
        .service
        .kernel
        .store
        .launch_state(&ThreadId("parent".into()))
        .unwrap()
        .unwrap();
    assert_eq!(ui.setup.unwrap().status, "failed");
    assert_eq!(
        frontier,
        rig.service.kernel.store.projection_frontier().unwrap()
    );
    rig.service
        .setup_control(zeron_proto::launch::SetupControlParams {
            chat_id: "parent".into(),
            run_id: w["setup"]["runId"].as_str().unwrap().into(),
            action: "continue".into(),
        })
        .await
        .unwrap();
    assert_eq!(rig.settled("parent").await["status"], "ready");
}

#[tokio::test]
async fn setup_cancel_retry_and_late_run_cas() {
    let rig = Rig::new().await;
    rig.call("t3_project_update",json!({"projectId":"project","scripts":[{"id":"setup","name":"Setup","command":"sleep 2","icon":"configure","runOnWorktreeCreate":true,"async":false}]})).await;
    let result = rig
        .call(
            "t3_thread_launch",
            json!({"title":"Cancel setup","message":"Work"}),
        )
        .await;
    let tid = result["threadId"].as_str().unwrap();
    let w = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let w = rig.service.workflow(tid).unwrap();
            if w["setup"]["status"] == "running" {
                return w;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let run_id = w["setup"]["runId"].as_str().unwrap().to_owned();
    rig.service
        .setup_control(zeron_proto::launch::SetupControlParams {
            chat_id: tid.into(),
            run_id: run_id.clone(),
            action: "cancel".into(),
        })
        .await
        .unwrap();
    assert_eq!(rig.settled(tid).await["setup"]["status"], "cancelled");
    rig.call("t3_project_update",json!({"projectId":"project","scripts":[{"id":"setup","name":"Setup","command":"true","icon":"configure","runOnWorktreeCreate":true,"async":false}]})).await;
    rig.service
        .setup_control(zeron_proto::launch::SetupControlParams {
            chat_id: tid.into(),
            run_id: run_id.clone(),
            action: "retry".into(),
        })
        .await
        .unwrap();
    let w = rig.settled(tid).await;
    assert_eq!(w["status"], "ready");
    assert_ne!(w["setup"]["runId"], run_id);
    assert!(
        rig.service
            .setup_control(zeron_proto::launch::SetupControlParams {
                chat_id: tid.into(),
                run_id,
                action: "continue".into()
            })
            .await
            .is_err()
    );
}

#[tokio::test]
async fn toolkit_launch_slice_preserves_pinned_mcp_framing_and_schemas() {
    let rig = Rig::new().await;
    let toolkit = crate::mcp::toolkit::Toolkit::new(rig.service.registry.clone());
    toolkit.set_launch_service(Arc::new(rig.service.clone()));
    for (name, args) in [
        ("t3_project_list", json!({})),
        ("t3_project_read", json!({"projectId":"project"})),
        ("t3_environment_read", json!({})),
        ("t3_worktree_status", json!({})),
        ("t3_thread_launch", json!({"title":"MCP Idle"})),
    ] {
        let reply=toolkit.request(rig.scope.clone(),json!({"jsonrpc":"2.0","id":name,"method":"tools/call","params":{"name":name,"arguments":args}})).await.unwrap();
        let structured = &reply["result"]["structuredContent"];
        assert!(structured.get("_tag").is_none(), "{reply}");
        let tool = zeron_proto::orchestration_mcp::pinned_tool_inventory()
            .into_iter()
            .find(|t| t.name == name)
            .unwrap();
        crate::mcp::codec::validate(&tool.result_schema, structured)
            .unwrap_or_else(|e| panic!("{name}: {e}\n{reply}"));
        assert_eq!(
            reply["result"],
            crate::mcp::codec::result(structured.clone())
        );
    }
}

#[test]
fn managed_folder_names_match_t3_path_oracle() {
    for (title, expected) in [
        ("Pinball Stats", "pinball-stats"),
        ("  Café & Crème!  ", "cafe-creme"),
        ("../../etc", "etc"),
        ("🎱🎱", "project"),
        ("Con", "con-project"),
        ("LPT1", "lpt1-project"),
        ("console", "console"),
    ] {
        assert_eq!(super::projects::project_folder_name(title), expected);
    }
    assert_eq!(
        super::projects::project_folder_name(&format!("{} b", "a".repeat(63))),
        "a".repeat(63)
    );
    assert_eq!(super::projects::folder_words("🎱"), "");
}

#[tokio::test]
async fn handoff_atomic_binding_hold_and_journal_survive_commit_uncertainty() {
    for boundary in [WriteBoundary::BeforeCommit, WriteBoundary::AfterCommit] {
        let rig = Rig::new().await;
        let w = json!({"threadId":"parent","commandId":"handoff:atomic","kind":"handoff","status":"preparing","bound":true});
        rig.service.kernel.store.inject_failure(boundary, 1);
        assert!(
            rig.service
                .operation(
                    "parent",
                    "handoff:atomic",
                    LaunchOperation::Bind {
                        expected: None,
                        path: Some("/created".into()),
                        branch: Some("feature/atomic".into()),
                        continuation: Some("Resume".into()),
                        driver: "mock".into(),
                        workflow: Some(w),
                    }
                )
                .await
                .is_err()
        );
        let p = rig
            .service
            .kernel
            .store
            .thread(&ThreadId("parent".into()))
            .unwrap()
            .unwrap();
        if boundary == WriteBoundary::BeforeCommit {
            assert_eq!(p.thread.worktree_path, None);
            assert!(rig.service.workflow("parent").is_err());
            assert_eq!(p.runs.len(), 1);
        } else {
            assert_eq!(p.thread.worktree_path.as_deref(), Some("/created"));
            assert_eq!(rig.service.workflow("parent").unwrap()["bound"], true);
            assert_eq!(p.runs.last().unwrap().queue_held.as_ref(), Some(&true));
            assert_eq!(p.runs.len(), 2);
        }
    }
}

#[tokio::test]
async fn handoff_recovers_checkout_before_binding_without_losing_continuation() {
    let rig = Rig::new().await;
    let root = &rig.scope.caller.workspace_root;
    let path = rig.dir.path().join("custom-checkout");
    let w = json!({"threadId":"parent","projectId":"project","commandId":"handoff:recover","kind":"handoff",
        "status":"preparing","stage":"checkout","projectWorkspaceRoot":root,"worktreePath":path,"branch":"feature/recover",
        "strategy":{"type":"worktree","baseRef":"main","branch":"feature/recover","startFromOrigin":false},
        "driver":"mock","checkoutIntent":true,"fetchCompleted":true,"resolvedBaseRef":"main",
        "continuationPrompt":"Resume after crash","runSetupScript":false,"bound":false,"setup":null});
    rig.service
        .save("orchestration_launch_workflows", "parent", &w)
        .unwrap();
    rig.service
        .repos
        .create_claimed_worktree(
            root,
            &path,
            "feature/recover",
            "main",
            Some("handoff:recover"),
        )
        .await
        .unwrap();
    rig.service.recover_preparations().unwrap();
    let w = rig.settled("parent").await;
    assert_eq!(w["status"], "ready", "{w}");
    assert_eq!(w["detachCompleted"], true);
    let p = rig
        .service
        .kernel
        .store
        .thread(&ThreadId("parent".into()))
        .unwrap()
        .unwrap();
    assert_eq!(p.thread.worktree_path.as_deref(), path.to_str());
    assert_eq!(p.runs.len(), 2);
    assert!(
        crate::orchestration::task::records(&p, "message")
            .iter()
            .any(|m| m["text"] == "Resume after crash")
    );
    assert_eq!(rig.intake.trace.lock().unwrap()[0]["detach"], "parent");
    assert_eq!(
        std::fs::read_to_string(path.join("base.txt")).unwrap(),
        "base\n"
    );
}

#[tokio::test]
async fn concurrent_handoff_refuses_before_git_side_effects() {
    let rig = Rig::new().await;
    super::lock(&rig.service.in_flight).insert("parent".into());
    let result = rig
        .call(
            "t3_worktree_handoff",
            json!({"branch":"feature/concurrent"}),
        )
        .await;
    assert_eq!(result["code"], "handoff_in_progress");
    assert!(rig.service.workflow("parent").is_err());
    assert!(
        rig.service
            .repos
            .orchestration_git(
                &rig.scope.caller.workspace_root,
                &["show-ref", "--verify", "refs/heads/feature/concurrent"]
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn signed_upload_expiry_and_limits_reject_before_files() {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as B64};
    let rig = Rig::new().await;
    let result=rig.call("t3_attachment_prepare_upload",json!({"upload":{"type":"file","name":"note.txt","mimeType":"text/plain","sizeBytes":4}})).await;
    let token = result["relativeUrl"]
        .as_str()
        .unwrap()
        .strip_prefix("/api/attachments/upload/")
        .unwrap();
    let (payload, _) = token.split_once('.').unwrap();
    let mut claims: Value = serde_json::from_slice(&B64.decode(payload).unwrap()).unwrap();
    claims["expiresAt"] = json!(crate::now_ms() - 1);
    let payload = B64.encode(claims.to_string());
    let sig = super::attachments::signature(&payload, &rig.service.signing_key);
    assert_eq!(
        rig.service
            .upload(&format!("{payload}.{sig}"), b"note")
            .await
            .0,
        401
    );
    assert!(!rig.dir.path().join("attachments").exists());
    let over=rig.service.prepare_upload(&rig.scope,&json!({"upload":{"type":"file","name":"large.bin","mimeType":"application/octet-stream","sizeBytes":50*1024*1024+1}}));
    assert!(over.is_err());
    let image = rig.service.prepare_upload(
        &rig.scope,
        &json!({"upload":{"name":"large.png","mimeType":"image/png","sizeBytes":10*1024*1024+1}}),
    );
    assert!(image.is_err());
    assert_eq!(
        rig.service
            .claim("parent", vec![json!({"id":"none"}); 9])
            .unwrap_err()
            .message,
        "You can attach up to 8 files per message or question response."
    );
}

#[tokio::test]
async fn setup_timeout_closes_only_owned_terminal_and_preserves_gate() {
    let rig = Rig::new().await;
    let project=rig.call("t3_project_update",json!({"projectId":"project","scripts":[{"id":"setup","name":"Setup","command":"sleep 5","icon":"configure","runOnWorktreeCreate":true,"async":false}]})).await;
    let mut w = json!({"threadId":"parent","commandId":"test:timeout","status":"preparing","setup":null,"setupTimeoutMs":20});
    assert!(
        !rig.service
            .setup(
                "parent",
                &project,
                &rig.scope.caller.workspace_root,
                &mut w,
                true
            )
            .await
            .unwrap()
    );
    let w = rig.service.workflow("parent").unwrap();
    assert_eq!(w["status"], "blocked");
    assert_eq!(w["setup"]["status"], "timed_out");
    assert_eq!(w["setup"]["exitCode"], Value::Null);
}

#[tokio::test]
async fn handoff_invalid_path_branch_and_detached_head_refuse_before_checkout() {
    let rig = Rig::new().await;
    let relative = rig
        .call(
            "t3_worktree_handoff",
            json!({"branch":"feature/relative","path":"relative"}),
        )
        .await;
    assert_eq!(relative["code"], "invalid_request");
    let collision = rig
        .call("t3_worktree_handoff", json!({"branch":"main"}))
        .await;
    assert_eq!(collision["code"], "invalid_request");
    assert!(
        collision["message"]
            .as_str()
            .unwrap()
            .contains("already exists and is checked out")
    );
    rig.service
        .repos
        .orchestration_git(&rig.scope.caller.workspace_root, &["checkout", "--detach"])
        .await
        .unwrap();
    let detached = rig
        .call("t3_worktree_handoff", json!({"branch":"feature/detached"}))
        .await;
    assert_eq!(
        detached["message"],
        "Could not determine the current branch of the project workspace (detached HEAD?). Pass baseRef explicitly."
    );
    assert!(rig.service.workflow("parent").is_err());
}

#[tokio::test]
async fn project_delete_terminalizes_runs_and_journals_owned_cleanup() {
    let rig = Rig::new().await;
    let (claimed, paths) = rig
        .service
        .claim("parent", vec![upload(&rig).await])
        .unwrap();
    rig.service
        .operation(
            "parent",
            "attachment-message",
            LaunchOperation::Send {
                message_id: "attachment-message".into(),
                text: "Attachment".into(),
                attachments: claimed.clone(),
                queue: true,
                driver: "mock".into(),
            },
        )
        .await
        .unwrap();
    let result = rig
        .call(
            "t3_project_delete",
            json!({"projectId":"project","force":true}),
        )
        .await;
    assert!(!result["deletedAt"].is_null());
    let p = rig
        .service
        .kernel
        .store
        .thread(&ThreadId("parent".into()))
        .unwrap()
        .unwrap();
    assert!(
        p.runs
            .iter()
            .all(|r| crate::orchestration::command::run_terminal(&r.status))
    );
    assert!(
        p.attempts
            .iter()
            .all(|a| crate::orchestration::command::attempt_terminal(&a.status))
    );
    assert!(rig.service.kernel.store.effects().unwrap().iter().any(|e|
        matches!(&e.request,crate::orchestration::effects::EffectRequest::AttachmentCleanup{attachment_ids} if attachment_ids.contains(&claimed[0]["id"].as_str().unwrap().to_owned()))));
    rig.service
        .cleanup_owned(
            "parent",
            Some(vec![claimed[0]["id"].as_str().unwrap().into()]),
        )
        .unwrap();
    assert!(!paths[0].exists());
    assert!(rig.scope.caller.workspace_root.join("base.txt").exists());
}

#[tokio::test]
async fn recovery_never_adopts_or_removes_unowned_matching_checkout() {
    let rig = Rig::new().await;
    let root = &rig.scope.caller.workspace_root;
    let path = rig.dir.path().join("foreign");
    let w = json!({"threadId":"parent","projectId":"project","commandId":"handoff:foreign","kind":"handoff",
        "status":"preparing","stage":"checkout","worktreePath":path,"branch":"feature/foreign",
        "strategy":{"type":"worktree","baseRef":"main","branch":"feature/foreign","startFromOrigin":false},
        "driver":"mock","checkoutIntent":true,"fetchCompleted":true,"resolvedBaseRef":"main","bound":false,"setup":null});
    rig.service
        .save("orchestration_launch_workflows", "parent", &w)
        .unwrap();
    rig.service
        .repos
        .create_claimed_worktree(root, &path, "feature/foreign", "main", None)
        .await
        .unwrap();
    rig.service.recover_preparations().unwrap();
    let w = rig.settled("parent").await;
    assert_eq!(w["status"], "failed");
    assert_eq!(
        rig.service
            .kernel
            .store
            .thread(&ThreadId("parent".into()))
            .unwrap()
            .unwrap()
            .thread
            .worktree_path,
        None
    );
    assert!(path.join("base.txt").exists());
}

#[tokio::test]
async fn deleted_launch_is_not_reprepared_or_continued_after_restart() {
    let rig = Rig::new().await;
    let w = json!({"threadId":"parent","projectId":"project","commandId":"prepare:deleted","status":"preparing",
        "strategy":{"type":"root"},"setup":{"runId":"old","status":"running","blocking":true}});
    rig.service
        .save("orchestration_launch_workflows", "parent", &w)
        .unwrap();
    rig.service
        .operation("parent", "delete:parent", LaunchOperation::Delete)
        .await
        .unwrap();
    rig.service.recover_preparations().unwrap();
    assert_eq!(
        rig.service.workflow("parent").unwrap()["status"],
        "cancelled"
    );
    assert!(
        rig.service
            .setup_control(zeron_proto::launch::SetupControlParams {
                chat_id: "parent".into(),
                run_id: "old".into(),
                action: "continue".into(),
            })
            .await
            .is_err()
    );
    assert!(rig.intake.trace.lock().unwrap().is_empty());
    assert!(rig.scope.caller.workspace_root.join("base.txt").exists());
}
