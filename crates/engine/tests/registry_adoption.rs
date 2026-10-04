//! Registry-created chats use the same V2 intake as orchestration-created ones.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use zeron_doc::{MessagePart, MessageRole, SessionMessageEntry};
use zeron_engine::orchestration::ReceiptStatus;
use zeron_engine::orchestration::scheduler::{ScheduledDispatch, ScheduledTaskDispatch};
use zeron_engine::orchestration::service::CallerScope;
use zeron_engine::orchestration::thread_service::ThreadSendRequest;
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_harness::mock::MockHarness;
use zeron_proto::HarnessId;
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::{OrchestratorMcpFailureCode, T3ThreadSendInputMode};
use zeron_proto::scheduler::*;
use zeron_rpc::{memory_client, methods};

fn registry() -> Arc<HarnessRegistry> {
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(MockHarness {
        script: vec![
            zeron_proto::AgentEvent::TextDelta {
                text: "existing harness reply".into(),
            },
            zeron_proto::AgentEvent::Done {
                status: zeron_proto::DoneStatus::Completed,
                result: None,
                error: None,
                session_id: None,
            },
        ],
    }));
    registry.provider_instances.configure(serde_json::from_value(json!([
        {"providerInstanceId":"chat-instance","driverKind":"mock","harnessId":"mock",
         "enabled":true,"installed":true,"adapterRegistered":true,"models":[{"id":"mock-1"}]}
    ])).unwrap()).unwrap();
    registry
}

async fn chat(core: &EngineCore, id: &str, root: &std::path::Path) {
    core.workspace
        .create_space(
            "project",
            &core.device_id,
            root.to_str().unwrap(),
            None,
            false,
        )
        .unwrap();
    memory_client(core.rpc_service())
        .call(
            methods::MUTATE,
            json!({
                "op":"createChat","chatId":id,"spaceId":"project","cwd":root,
                "config":{"instanceId":"chat-instance","harness":"mock","model":"mock-1",
                  "reasoning":null,"modelOptions":{},"sandbox":"workspace-write",
                  "runtimeMode":"full-access","interactionMode":"default"}
            }),
        )
        .await
        .unwrap();
}

fn caller(id: &str, root: &std::path::Path) -> CallerScope {
    CallerScope {
        thread_id: id.into(),
        run_id: "observational".into(),
        session_id: "adoption-test".into(),
        project_id: "project".into(),
        workspace_root: root.to_str().unwrap().into(),
        runtime_mode: zeron_proto::RuntimeMode::FullAccess,
        interaction_mode: zeron_proto::InteractionMode::Default,
        provider_instance_id: "chat-instance".into(),
    }
}

fn send(id: &str, command: &str, mode: T3ThreadSendInputMode) -> ThreadSendRequest {
    ThreadSendRequest {
        project_id: "project".into(),
        thread_id: id.into(),
        command_id: command.into(),
        message_id: format!("message:{command}").into(),
        scheduled_task_id: None,
        sender_thread_id: None,
        text: command.into(),
        attachments: vec![],
        model_selection: None,
        mode,
        created_by: OrchestrationV2Actor::User,
        creation_source: OrchestrationV2CreationSource::Web,
    }
}

async fn tool(core: &EngineCore, caller: CallerScope, name: &str, arguments: Value) -> Value {
    let server = core.sessions.mcp_server();
    let scope = zeron_engine::mcp::auth::InvocationScope {
        caller,
        environment_id: core.device_id.clone(),
        selection: serde_json::from_value(json!({"instanceId":"chat-instance","model":"mock-1"}))
            .unwrap(),
        capabilities: ["orchestration".into()].into_iter().collect(),
        issued_at: 0,
        task_id: None,
    };
    let credential = server.credentials.issue(scope).unwrap();
    let reply: Value = reqwest::Client::new()
        .post(server.endpoint().await.unwrap())
        .header("authorization", credential.authorization)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":name,"arguments":arguments}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let result = reply["result"]["structuredContent"].clone();
    assert!(
        result.is_object() && result.get("_tag").is_none(),
        "{name}: {reply}"
    );
    result
}

#[tokio::test]
async fn registry_read_import_is_atomic_rebuildable_and_never_executes_history() {
    let dir = tempfile::tempdir().unwrap();
    let core = EngineCore::assemble(dir.path(), registry(), HarnessId::Mock, None).unwrap();
    let host = core.orchestration_host.as_ref().unwrap();
    host.shutdown().await;
    let checkout = dir.path().join("checkout");
    std::fs::create_dir(&checkout).unwrap();
    chat(&core, "ordinary", &checkout).await;
    let handle = core.doc_host.open("ordinary").unwrap();
    for (id, role, text) in [
        ("old-user", MessageRole::User, "Existing question"),
        (
            "old-assistant",
            MessageRole::Assistant,
            "Existing answer 😀",
        ),
    ] {
        handle
            .doc()
            .push_message(&SessionMessageEntry {
                id: id.into(),
                role,
                parts: vec![MessagePart::Text {
                    id: "text".into(),
                    text: text.into(),
                }],
                created_at: 1_800_000_000_000,
                device_id: core.device_id.clone(),
                status: None,
                continuation_of: None,
            })
            .unwrap();
    }
    core.orchestration.store.inject_failure(
        zeron_engine::orchestration::WriteBoundary::AdoptionRecorded,
        1,
    );
    let scope = caller("ordinary", &checkout);
    let input = serde_json::from_value(json!({"threadId":"ordinary"})).unwrap();
    assert!(host.threads.read(scope.clone(), input).await.is_err());
    assert!(!core.orchestration.store.is_v2_managed("ordinary").unwrap());
    assert!(core.orchestration.store.events().unwrap().is_empty());
    let read = host
        .threads
        .read(
            scope.clone(),
            serde_json::from_value(json!({"threadId":"ordinary"})).unwrap(),
        )
        .await
        .unwrap();
    let page = read.wire_value();
    assert_eq!(page["items"][0]["text"], "Existing question");
    assert_eq!(page["items"][1]["text"], "Existing answer 😀");
    assert_eq!(page["thread"]["runCount"], 0);
    let projection = core
        .orchestration
        .store
        .thread(&"ordinary".into())
        .unwrap()
        .unwrap();
    assert_eq!(projection.thread.project_id.0, "project");
    assert_eq!(
        projection.thread.worktree_path.as_deref(),
        checkout.to_str()
    );
    assert_eq!(projection.thread.provider_instance_id.0, "chat-instance");
    assert!(projection.thread.lineage.parent_thread_id.is_none());
    assert!(core.orchestration.store.is_v2_managed("ordinary").unwrap());
    assert!(core.orchestration.store.effects().unwrap().is_empty());
    core.orchestration.store.rebuild().unwrap();
    let timeline = core
        .orchestration
        .store
        .ui_thread_timeline(
            serde_json::from_value(json!({"threadId":"ordinary","view":"activity"})).unwrap(),
        )
        .unwrap();
    assert_eq!(json!(timeline.page)["items"], page["items"]);
    let events = core.orchestration.store.events().unwrap();
    host.threads
        .read(
            scope,
            serde_json::from_value(json!({"threadId":"ordinary"})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(core.orchestration.store.events().unwrap(), events);
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn racing_first_operations_and_acceptance_loss_adopt_once() {
    let dir = tempfile::tempdir().unwrap();
    let core = EngineCore::assemble(dir.path(), registry(), HarnessId::Mock, None).unwrap();
    core.orchestration_host.as_ref().unwrap().shutdown().await;
    chat(&core, "race", dir.path()).await;
    let store = core.orchestration.store.clone();
    store.inject_failure(zeron_engine::orchestration::WriteBoundary::AfterCommit, 1);
    let other = store.clone();
    let first = tokio::task::spawn_blocking(move || other.thread(&"race".into()));
    let second = store.thread(&"race".into());
    let first = first.await.unwrap();
    assert!(first.is_ok() || second.is_ok());
    assert!(store.thread(&"race".into()).unwrap().is_some());
    assert_eq!(
        store
            .events()
            .unwrap()
            .iter()
            .filter(|e| json!(e.event)["type"] == "thread.created")
            .count(),
        1
    );
    assert!(store.is_v2_managed("race").unwrap());
    assert!(store.effects().unwrap().is_empty());
    core.shutdown().await;
}

#[tokio::test]
async fn mcp_send_uses_existing_chat_harness_doc_and_read_and_lifecycle_work() {
    let dir = tempfile::tempdir().unwrap();
    let core = EngineCore::assemble(dir.path(), registry(), HarnessId::Mock, None).unwrap();
    chat(&core, "ordinary", dir.path()).await;
    let scope = caller("ordinary", dir.path());
    let result = tool(
        &core,
        scope.clone(),
        "t3_thread_send",
        json!({
            "threadId":"ordinary","message":"hello existing chat","clientRequestId":"one"
        }),
    )
    .await;
    let message_id = result["messageId"].as_str().unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let p = core
                .orchestration
                .store
                .thread(&"ordinary".into())
                .unwrap()
                .unwrap();
            if p.runs
                .iter()
                .any(|r| r.status == OrchestrationV2RunStatus::Completed)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let entries = core
        .doc_host
        .open("ordinary")
        .unwrap()
        .doc()
        .read_entries()
        .unwrap();
    assert_eq!(entries.iter().filter(|e| e.id == message_id).count(), 1);
    assert!(entries.iter().any(|e| e.role == MessageRole::Assistant));
    assert_eq!(
        core.workspace
            .chat("ordinary")
            .unwrap()
            .unwrap()
            .config
            .unwrap()
            .instance_id
            .unwrap()
            .0,
        "chat-instance"
    );
    let read = tool(
        &core,
        scope,
        "t3_thread_read",
        json!({"threadId":"ordinary"}),
    )
    .await;
    assert!(
        read["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["text"] == "hello existing chat")
    );
    // A second, never-run registry chat must support lifecycle as its first op.
    chat(&core, "parked", dir.path()).await;
    memory_client(core.rpc_service())
        .call(
            methods::ORGANIZE_THREAD,
            json!({
                "chatId":"parked","action":"pin"
            }),
        )
        .await
        .unwrap();
    assert!(core.orchestration.store.is_v2_managed("parked").unwrap());
    let queue = tool(
        &core,
        caller("parked", dir.path()),
        "t3_queue_list",
        json!({}),
    )
    .await;
    assert_eq!(queue["items"], json!([]));
    core.shutdown().await;
}

struct ReplayDispatch {
    inner: zeron_engine::orchestration::scheduler::dispatch::ThreadDispatch,
    accepted: Mutex<Vec<ScheduledDispatch>>,
}
#[async_trait]
impl ScheduledTaskDispatch for ReplayDispatch {
    async fn dispatch(&self, run: ScheduledDispatch) -> Result<(), String> {
        self.inner.dispatch(run.clone()).await?;
        // Acceptance-loss replay while the scheduler claim is still current.
        self.inner.dispatch(run.clone()).await?;
        self.accepted.lock().unwrap().push(run);
        Ok(())
    }
}

#[tokio::test]
async fn run_scheduled_task_now_adopts_registry_chat_and_replay_queues_once() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let core = EngineCore::assemble(&data, registry(), HarnessId::Mock, None).unwrap();
    let host = core.orchestration_host.as_ref().unwrap();
    host.shutdown().await; // Inspect durable intake without racing the start worker.
    let replay = Arc::new(ReplayDispatch {
        inner: zeron_engine::orchestration::scheduler::dispatch::ThreadDispatch {
            store: core.orchestration.store.clone(),
            threads: host.threads.clone(),
            launch: None,
        },
        accepted: Mutex::default(),
    });
    host.scheduler.set_dispatcher(replay.clone());
    let client = memory_client(core.rpc_service());
    for id in ["fresh", "busy"] {
        chat(&core, id, dir.path()).await;
        if id == "busy" {
            host.threads
                .send_to_thread(send(id, "blocker", T3ThreadSendInputMode::Auto))
                .await
                .unwrap();
        }
        let created = client
            .create_scheduled_task(ScheduledTaskCreateRequest {
                owner_host_id: core.device_id.clone(),
                input: serde_json::from_value(json!({
                    "title":"Registry schedule","prompt":"scheduled follow-up","enabled":false,
                    "schedule":{"type":"interval","everyMs":60000},"projectId":"project",
                    "threadId":id,"workspaceStrategy":{"type":"root"},
                    "modelSelection":{"instanceId":"chat-instance","model":"mock-1"},
                    "runtimeMode":"full-access","interactionMode":"default"
                }))
                .unwrap(),
            })
            .await
            .unwrap();
        let outcome = client
            .run_scheduled_task_now(ScheduledTaskActionRequest {
                owner_host_id: core.device_id.clone(),
                id: created.task.task.id.0,
            })
            .await
            .unwrap();
        assert_eq!(
            outcome.task.task.last_run_status,
            ScheduledTaskRunStatus::Succeeded
        );
        let accepted = replay.accepted.lock().unwrap().last().unwrap().clone();
        let p = core
            .orchestration
            .store
            .thread(&id.into())
            .unwrap()
            .unwrap();
        let scheduled: Vec<_> = p
            .runs
            .iter()
            .filter(|r| r.user_message_id == accepted.message_id)
            .collect();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(
            scheduled[0].status,
            if id == "busy" {
                OrchestrationV2RunStatus::Queued
            } else {
                OrchestrationV2RunStatus::Starting
            }
        );
        assert_eq!(p.thread.worktree_path.as_deref(), dir.path().to_str());
        assert_eq!(
            core.orchestration
                .store
                .receipt(&accepted.command_id)
                .unwrap()
                .unwrap()
                .status,
            ReceiptStatus::Accepted
        );
        if id == "busy" {
            let queue = client
                .call(methods::GET_QUEUE_STATE, json!({"chatId":id}))
                .await
                .unwrap();
            assert_eq!(queue["queue"].as_array().unwrap().len(), 1);
            assert_eq!(queue["queue"][0]["messageId"], accepted.message_id.0);
            assert_eq!(queue["queue"][0]["text"], "scheduled follow-up");
        }
    }
    assert_eq!(core.orchestration.store.effects().unwrap().len(), 2);
    core.shutdown().await;
    drop(client);
    drop(core);
    let restarted = EngineCore::assemble(&data, registry(), HarnessId::Mock, None).unwrap();
    restarted
        .orchestration_host
        .as_ref()
        .unwrap()
        .shutdown()
        .await;
    assert_eq!(
        restarted
            .orchestration
            .store
            .thread(&"busy".into())
            .unwrap()
            .unwrap()
            .runs
            .len(),
        2,
        "restart must not re-import or re-accept scheduled history"
    );
    restarted.shutdown().await;
}

#[tokio::test]
async fn foreign_device_and_wrong_project_chats_are_not_adopted() {
    let dir = tempfile::tempdir().unwrap();
    let core = EngineCore::assemble(dir.path(), registry(), HarnessId::Mock, None).unwrap();
    let host = core.orchestration_host.as_ref().unwrap();
    host.shutdown().await;
    chat(&core, "local", dir.path()).await;
    core.workspace
        .create_chat("foreign", None, Some("other-device"), None, None)
        .unwrap();
    core.workspace
        .create_space("other-project", &core.device_id, "/different", None, false)
        .unwrap();
    core.workspace
        .create_chat("different", Some("other-project"), None, None, None)
        .unwrap();
    for id in ["foreign", "different"] {
        let error = host
            .threads
            .send_to_thread(send(id, id, T3ThreadSendInputMode::Auto))
            .await
            .unwrap_err();
        assert_eq!(error.code, OrchestratorMcpFailureCode::ThreadNotFound);
        assert_eq!(
            error.message,
            format!("Thread {id} was not found in project project.")
        );
        assert!(!core.orchestration.store.is_v2_managed(id).unwrap());
    }
    let error = host
        .threads
        .read(
            caller("local", dir.path()),
            serde_json::from_value(json!({"threadId":"foreign"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OrchestratorMcpFailureCode::ThreadNotFound);
    assert!(
        core.orchestration
            .store
            .thread(&"foreign".into())
            .unwrap()
            .is_none()
    );
    core.shutdown().await;
}
