//! Desktop authority → canonical transfers → lazy runner delivery. No MCP
//! credential or active agent is needed to fork a completed conversation.
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use futures::{stream, stream::BoxStream};
use serde_json::{Value, json};
use zeron_engine::{
    EngineCore, HarnessRegistry,
    orchestration::{WriteBoundary, thread_service::ThreadSendRequest},
};
use zeron_harness::{Harness, HarnessError, RunControls};
use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, Model, ReasoningLevel, RunRequest, SteeringMode,
    orchestration::*,
    orchestration_mcp::T3ThreadSendInputMode,
    transfer::{ForkThreadParams, MergeThreadBackParams, ThreadSourcePoint},
};
use zeron_rpc::{RpcClient, memory_client, methods};

struct RecordingHarness(Arc<Mutex<Vec<RunRequest>>>);

#[async_trait]
impl Harness for RecordingHarness {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Conversation transfer test"
    }
    fn supports_steering(&self) -> bool {
        false
    }
    fn steering_mode(&self) -> SteeringMode {
        SteeringMode::TurnBoundary
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[]
    }
    fn deterministic_turn_end(&self) -> bool {
        true
    }
    async fn models(&self) -> Result<Vec<Model>, HarnessError> {
        Ok(vec![Model {
            id: "mock-1".into(),
            label: "Mock".into(),
            description: None,
            reasoning_levels: vec![],
            options: vec![],
        }])
    }
    async fn run(
        &self,
        request: RunRequest,
        _: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        self.0.lock().unwrap().push(request.clone());
        let native = uuid::Uuid::new_v4().to_string();
        Ok(Box::pin(stream::iter([
            Ok(AgentEvent::SessionStarted {
                instance_id: None,
                session_id: native.clone(),
                harness: HarnessId::Mock,
                model: "mock-1".into(),
                cwd: request.cwd,
                tools: vec![],
                assistant_message_id: uuid::Uuid::new_v4().to_string(),
            }),
            Ok(AgentEvent::TextDelta {
                text: format!("Decision for {}", request.prompt),
            }),
            Ok(AgentEvent::Done {
                status: DoneStatus::Completed,
                result: None,
                error: None,
                session_id: Some(native),
            }),
        ])))
    }
}

async fn setup(root: &Path) -> (EngineCore, RpcClient, Arc<Mutex<Vec<RunRequest>>>) {
    let requests = Arc::new(Mutex::new(vec![]));
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(RecordingHarness(requests.clone())));
    registry.provider_instances.configure(serde_json::from_value(json!([
        {"providerInstanceId":"chat-instance","driverKind":"mock","harnessId":"mock",
         "enabled":true,"installed":true,"adapterRegistered":true,"models":[{"id":"mock-1"}]}
    ])).unwrap()).unwrap();
    let core = EngineCore::assemble(&root.join("data"), registry, HarnessId::Mock, None).unwrap();
    let client = memory_client(core.rpc_service());
    core.workspace
        .create_space(
            "project",
            &core.device_id,
            root.to_str().unwrap(),
            None,
            false,
        )
        .unwrap();
    client
        .call(
            methods::MUTATE,
            json!({
                "op":"createChat","chatId":"source","spaceId":"project","cwd":root,
                "config":{"instanceId":"chat-instance","harness":"mock","model":"mock-1",
                    "reasoning":null,"modelOptions":{},"sandbox":"workspace-write",
                    "runtimeMode":"full-access","interactionMode":"default"}
            }),
        )
        .await
        .unwrap();
    core.workspace
        .set_chat_branch("source", "feature/context")
        .unwrap();
    (core, client, requests)
}

async fn send(core: &EngineCore, chat: &str, key: &str) -> OrchestrationV2Run {
    let result = core
        .orchestration_host
        .as_ref()
        .unwrap()
        .threads
        .send_to_thread(ThreadSendRequest {
            project_id: "project".into(),
            thread_id: chat.into(),
            command_id: key.into(),
            message_id: format!("message:{key}").into(),
            scheduled_task_id: None,
            sender_thread_id: None,
            text: key.into(),
            attachments: vec![],
            model_selection: None,
            mode: T3ThreadSendInputMode::Auto,
            created_by: OrchestrationV2Actor::User,
            creation_source: OrchestrationV2CreationSource::Web,
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(run) = core
                .orchestration
                .store
                .thread(&chat.into())
                .unwrap()
                .unwrap()
                .runs
                .into_iter()
                .find(|r| r.id == result.run_id && r.status == OrchestrationV2RunStatus::Completed)
            {
                break run;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("real runner completes the selected run")
}

fn fork(run: &OrchestrationV2Run, key: &str, target: &str) -> ForkThreadParams {
    ForkThreadParams {
        chat_id: run.thread_id.0.clone(),
        command_id: key.into(),
        target_chat_id: target.into(),
        source_point: ThreadSourcePoint::Run {
            run_id: run.id.0.clone(),
        },
        title: Some("Pinned fork".into()),
    }
}

#[tokio::test]
async fn desktop_fork_is_idle_inherits_selection_and_delivers_pinned_history_then_merges_context_only()
 {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("keep.txt"), "unchanged working files").unwrap();
    let (core, client, requests) = setup(root.path()).await;
    let first = send(&core, "source", "original-goal").await;
    let before = requests.lock().unwrap().len();
    let reply = client
        .fork_thread(fork(&first, "fork-one", "forked"), &core.device_id)
        .await
        .unwrap();
    assert!(reply.refusal.is_none());
    let row = reply.chat.unwrap();
    assert_eq!(row.id, "forked");
    assert_eq!(row.title.as_deref(), Some("Pinned fork"));
    assert_eq!(row.space_id.as_deref(), Some("project"));
    assert_eq!(row.cwd.as_deref(), root.path().to_str());
    assert_eq!(row.branch.as_deref(), Some("feature/context"));
    assert_eq!(
        row.config,
        core.workspace.chat("source").unwrap().unwrap().config
    );
    let projection = core
        .orchestration
        .store
        .thread(&"forked".into())
        .unwrap()
        .unwrap();
    assert!(projection.runs.is_empty());
    assert!(projection.thread.active_provider_thread_id.is_none());
    assert_eq!(projection.thread.created_by, OrchestrationV2Actor::User);
    assert_eq!(
        projection.thread.creation_source,
        OrchestrationV2CreationSource::Web
    );
    assert_eq!(
        core.orchestration
            .store
            .thread_transfers(&"forked".into())
            .unwrap()[0]["createdBy"],
        "user"
    );
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        requests.lock().unwrap().len(),
        before,
        "forking never starts a provider"
    );
    send(&core, "source", "later-parent-secret").await;
    let child = send(&core, "forked", "child-improvement").await;
    let input = requests.lock().unwrap().last().unwrap().clone();
    assert!(input.resume.is_none(), "the mock has no native fork API");
    assert!(input.prompt.contains("original-goal"), "{}", input.prompt);
    assert!(
        !input.prompt.contains("later-parent-secret"),
        "fork history is pinned"
    );
    assert_eq!(input.prompt.matches("\nchild-improvement").count(), 1);
    let source_state = client.thread_transfer_state("source", &core.device_id).await.unwrap();
    assert!(source_state.handoffs.iter().any(|handoff| handoff["deliveryStatus"] == "inline"),
        "source-side Details can see the target's accepted portable-context receipt");
    assert!(source_state.handoffs.iter().all(|handoff| handoff["summaryText"] == ""
        && handoff.get("history").is_none()), "cross-thread delivery visibility remains passive and redacted");
    let before = requests.lock().unwrap().len();
    let merged = client
        .merge_thread_back(
            MergeThreadBackParams {
                chat_id: "forked".into(),
                command_id: "merge-one".into(),
                target_chat_id: "source".into(),
                source_point: ThreadSourcePoint::Run { run_id: child.id.0 },
            },
            &core.device_id,
        )
        .await
        .unwrap();
    assert!(merged.refusal.is_none());
    assert_eq!(merged.target_chat_id, "source");
    assert_eq!(
        requests.lock().unwrap().len(),
        before,
        "merge-back never starts or interrupts a provider"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("keep.txt")).unwrap(),
        "unchanged working files"
    );
    let transfer = core
        .orchestration
        .store
        .thread_transfers(&"source".into())
        .unwrap()
        .into_iter()
        .find(|t| t["type"] == "merge_back")
        .unwrap();
    assert_eq!(transfer["status"], "pending");
    send(&core, "source", "parent-next-message").await;
    let input = requests.lock().unwrap().last().unwrap().clone();
    assert!(
        input.prompt.contains("child-improvement"),
        "{}",
        input.prompt
    );
    assert!(input.prompt.ends_with("parent-next-message"));
    let state = client
        .thread_transfer_state("source", &core.device_id)
        .await
        .unwrap();
    assert!(
        state
            .transfers
            .iter()
            .any(|t| t["type"] == "merge_back" && t["status"] == "consumed")
    );
    core.shutdown().await;
}

#[tokio::test]
async fn lost_response_retry_materializes_exactly_one_fork_and_preserves_config_on_publication_replay()
 {
    let root = tempfile::tempdir().unwrap();
    let (core, client, _) = setup(root.path()).await;
    let first = send(&core, "source", "goal").await;
    core.orchestration_host.as_ref().unwrap().shutdown().await;
    let request = fork(&first, "stable-request", "one-child");
    core.orchestration
        .store
        .inject_failure(WriteBoundary::AfterCommit, 1);
    assert!(
        client
            .fork_thread(request.clone(), &core.device_id)
            .await
            .is_err()
    );
    let retried = client
        .fork_thread(request.clone(), &core.device_id)
        .await
        .unwrap();
    assert!(retried.refusal.is_none());
    let again = client
        .fork_thread(request.clone(), &core.device_id)
        .await
        .unwrap();
    assert_eq!(retried.sequence, again.sequence);
    assert!(
        core.orchestration
            .store
            .thread(&"one-child".into())
            .unwrap()
            .unwrap()
            .runs
            .is_empty()
    );
    assert_eq!(
        core.orchestration
            .store
            .thread_transfers(&"one-child".into())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        core.orchestration
            .store
            .events()
            .unwrap()
            .iter()
            .filter(|e| {
                let event = json!(e.event);
                event["threadId"] == "one-child" && event["type"] == "thread.created"
            })
            .count(),
        1
    );
    let mut changed = request;
    changed.target_chat_id = "different-child".into();
    assert!(client.fork_thread(changed, &core.device_id).await.is_err());
    assert!(core.workspace.chat("different-child").unwrap().is_none());
    core.orchestration.store.rebuild().unwrap();
    let after_rebuild = client
        .fork_thread(fork(&first, "stable-request", "one-child"), &core.device_id)
        .await
        .unwrap();
    assert_eq!(retried.sequence, after_rebuild.sequence);
    assert_eq!(
        retried.chat.unwrap().config,
        after_rebuild.chat.unwrap().config
    );
    core.shutdown().await;
}

#[tokio::test]
async fn desktop_transfer_refuses_foreign_owner_identity_collision_and_non_parent_merge() {
    let root = tempfile::tempdir().unwrap();
    let (core, client, _) = setup(root.path()).await;
    let first = send(&core, "source", "goal").await;
    core.orchestration_host.as_ref().unwrap().shutdown().await;
    core.workspace
        .create_chat("foreign", None, Some("different-device"), None, None)
        .unwrap();
    let mut request = fork(&first, "foreign-source", "never");
    request.chat_id = "foreign".into();
    assert!(client.fork_thread(request, &core.device_id).await.is_err());
    assert!(!core.orchestration.store.is_v2_managed("foreign").unwrap());
    assert!(
        client
            .fork_thread(fork(&first, "collision", "foreign"), &core.device_id)
            .await
            .is_err()
    );
    assert!(
        client
            .fork_thread(fork(&first, "self", "source"), &core.device_id)
            .await
            .is_err()
    );
    let result = client
        .merge_thread_back(
            MergeThreadBackParams {
                chat_id: "source".into(),
                command_id: "non-parent".into(),
                target_chat_id: "source".into(),
                source_point: ThreadSourcePoint::Run {
                    run_id: first.id.0.clone(),
                },
            },
            &core.device_id,
        )
        .await
        .unwrap();
    assert!(result.refusal.unwrap().contains("not a fork"));
    core.workspace
        .create_space("other", &core.device_id, "/different-project", None, false)
        .unwrap();
    core.workspace
        .create_chat("wrong-project", Some("other"), None, None, None)
        .unwrap();
    assert!(
        client
            .merge_thread_back(
                MergeThreadBackParams {
                    chat_id: "source".into(),
                    command_id: "wrong-project".into(),
                    target_chat_id: "wrong-project".into(),
                    source_point: ThreadSourcePoint::Run { run_id: first.id.0 },
                },
                &core.device_id
            )
            .await
            .is_err()
    );
    assert!(
        core.orchestration
            .store
            .thread_transfers(&"source".into())
            .unwrap()
            .is_empty()
    );
    let error: Value = client
        .call(
            methods::FORK_THREAD,
            json!({
                "chatId":"source","commandId":"","targetChatId":"unused",
                "targetDeviceId":core.device_id
            }),
        )
        .await
        .unwrap_err()
        .to_string()
        .into();
    assert!(error.as_str().unwrap().contains("nonempty"));
    core.shutdown().await;
}
