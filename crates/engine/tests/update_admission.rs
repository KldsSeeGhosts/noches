use async_trait::async_trait;
use futures::StreamExt;
use std::{sync::Arc, time::Duration};
use zeron_doc::{SessionCommandPayload, SessionCommandStatus};
use zeron_engine::{EngineCore, EngineProfile, HarnessId, HarnessRegistry};
use zeron_harness::{Harness, HarnessError, RunControls, mock::MockHarness};
use zeron_proto::{AgentEvent, DoneStatus, Model, ReasoningLevel, SteeringMode};
use zeron_proto::{RunRequest, SandboxLevel};
use zeron_rpc::{memory_client, methods};

struct WarmHarness(Arc<std::sync::atomic::AtomicBool>);
struct Closed(Arc<std::sync::atomic::AtomicBool>);
impl Drop for Closed {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}
#[async_trait]
impl Harness for WarmHarness {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Warm mock"
    }
    fn supports_steering(&self) -> bool {
        true
    }
    fn steering_mode(&self) -> SteeringMode {
        SteeringMode::TurnBoundary
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[]
    }
    async fn models(&self) -> Result<Vec<Model>, HarnessError> {
        Ok(Vec::new())
    }
    async fn run(
        &self,
        _: RunRequest,
        _: RunControls,
    ) -> Result<futures::stream::BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError>
    {
        let guard = Closed(self.0.clone());
        Ok(futures::stream::iter([
            Ok(AgentEvent::TextDelta {
                text: "completed reply".into(),
            }),
            Ok(AgentEvent::Done {
                status: DoneStatus::Completed,
                result: None,
                error: None,
                session_id: None,
            }),
        ])
        .chain(futures::stream::once(async move {
            let _guard = guard;
            std::future::pending().await
        }))
        .boxed())
    }
}

#[tokio::test]
async fn restart_preparation_retires_completed_warm_wrappers_before_ready() {
    let dir = tempfile::tempdir().unwrap();
    let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let registry = zeron_engine::HarnessRegistry::new();
    registry.register(Arc::new(WarmHarness(closed.clone())));
    let core = EngineCore::assemble_with_profile(
        EngineProfile::local(dir.path()).unwrap(),
        Arc::new(registry),
        HarnessId::Mock,
        None,
    )
    .unwrap();
    core.workspace
        .create_chat(
            "warm",
            None,
            Some(&core.device_id),
            None,
            Some(dir.path().to_string_lossy().into_owned()),
        )
        .unwrap();
    core.sessions
        .dispatch(
            "warm",
            HarnessId::Mock,
            RunRequest {
                instance_id: None,
                prompt: "complete then stay warm".into(),
                harness: None,
                model: None,
                reasoning: None,
                model_options: Default::default(),
                cwd: dir.path().to_string_lossy().into_owned(),
                sandbox: SandboxLevel::WorkspaceWrite,
                runtime_mode: Default::default(),
                interaction_mode: Default::default(),
                auto_approve: true,
                attachments: Vec::new(),
                worktree: None,
                resume: None,
            },
            None,
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while core.sessions.any_active() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!closed.load(std::sync::atomic::Ordering::SeqCst));
    let client = memory_client(core.rpc_service());
    let mut lease = client
        .subscribe_checked(methods::PREPARE_UPDATE_RESTART, serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(lease.recv().await.unwrap()["ready"], true);
    let error = client
        .call(methods::WRITE_WORKSPACE_FILE, serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Update installation is in progress"),
        "writes must be fenced before parsing/executing: {error}"
    );
    assert!(closed.load(std::sync::atomic::Ordering::SeqCst));
    let entries = core
        .doc_host
        .open("warm")
        .unwrap()
        .doc()
        .read_entries()
        .unwrap();
    assert_eq!(
        entries.len(),
        2,
        "retirement must not overwrite the completed transcript"
    );
    drop(lease);
    core.shutdown().await;
}

/// Only the mock harness: the production registry's catalog migration lists
/// Mock solely for a mock-only rig, and its other slots depend on host CLIs.
fn mock_registry() -> HarnessRegistry {
    let registry = HarnessRegistry::new();
    registry.register(Arc::new(MockHarness {
        script: vec![
            AgentEvent::TextDelta {
                text: "resumed reply".into(),
            },
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: None,
                error: None,
                session_id: None,
            },
        ],
    }));
    registry
}

#[tokio::test]
async fn restart_lease_blocks_terminals_preserves_commands_and_resumes_after_cancel() {
    let dir = tempfile::tempdir().unwrap();
    let core = EngineCore::assemble_with_profile(
        EngineProfile::local(dir.path()).unwrap(),
        Arc::new(mock_registry()),
        HarnessId::Mock,
        None,
    )
    .unwrap();
    core.workspace
        .create_chat(
            "update-test",
            None,
            Some(&core.device_id),
            None,
            Some(dir.path().to_string_lossy().into_owned()),
        )
        .unwrap();
    let client = memory_client(core.rpc_service());
    let mut lease = client
        .subscribe_scoped(methods::PREPARE_UPDATE_RESTART, serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(lease.recv().await.unwrap()["ready"], true);
    assert!(
        core.terminals
            .open_with_shell(dir.path().to_str().unwrap(), 80, 24, Some("/bin/sh"))
            .is_err()
    );
    core.doc_host
        .queue_command(
            "update-test",
            SessionCommandPayload::Run {
                request: RunRequest {
                    instance_id: None,
                    prompt: "queued during update".into(),
                    harness: None,
                    model: None,
                    reasoning: None,
                    model_options: Default::default(),
                    cwd: dir.path().to_string_lossy().into_owned(),
                    sandbox: SandboxLevel::WorkspaceWrite,
                    runtime_mode: Default::default(),
                    interaction_mode: Default::default(),
                    auto_approve: true,
                    attachments: Vec::new(),
                    worktree: None,
                    resume: None,
                },
                message_id: "update-prompt".into(),
            },
        )
        .unwrap();
    tokio::time::sleep(Duration::from_millis(30)).await;
    let handle = core.doc_host.open("update-test").unwrap();
    assert_eq!(
        handle.doc().read_commands().unwrap()[0].status,
        SessionCommandStatus::Pending
    );
    drop(lease);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if handle.doc().read_commands().unwrap()[0].status == SessionCommandStatus::Applied {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancelled update must resume the durable command");
    core.shutdown().await;
}
