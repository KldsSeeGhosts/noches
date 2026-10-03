//! Isolated MCP stdio fixture. No real credentials or user projects.
use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::BoxStream;
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_harness::{Harness, HarnessError, RunControls};
use zeron_mcp::{Origin, Tools, Zeron};
use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, Model, ReasoningLevel, RunRequest, SteeringMode,
};

struct SmokeHarness;
#[async_trait]
impl Harness for SmokeHarness {
    fn id(&self) -> HarnessId {
        HarnessId::Codex
    }
    fn display_name(&self) -> &str {
        "Scripted Codex"
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
    async fn models(&self) -> Result<Vec<Model>, HarnessError> {
        Ok(vec![Model {
            id: "smoke-1".into(),
            label: "Smoke 1".into(),
            description: None,
            reasoning_levels: vec![],
            options: vec![],
        }])
    }
    async fn run(
        &self,
        request: RunRequest,
        _controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        use futures::StreamExt;
        let session_id = uuid::Uuid::new_v4().to_string();
        Ok(futures::stream::iter([
            Ok(AgentEvent::SessionStarted {
                harness: HarnessId::Codex,
                model: "smoke-1".into(),
                tools: vec![],
                cwd: request.cwd,
                session_id: session_id.clone(),
                assistant_message_id: uuid::Uuid::new_v4().to_string(),
            }),
            Ok(AgentEvent::TextDelta {
                text: "pong".into(),
            }),
            Ok(AgentEvent::Done {
                status: DoneStatus::Completed,
                result: None,
                error: None,
                session_id: Some(session_id),
            }),
        ])
        .boxed())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("device-id"), "smoke-device")?;
    let registry = HarnessRegistry::new();
    registry.register(Arc::new(SmokeHarness));
    let core = EngineCore::assemble(dir.path(), Arc::new(registry), HarnessId::Codex, None)?;
    core.workspace.create_space(
        "smoke-project",
        "smoke-device",
        dir.path().to_str().unwrap(),
        Some("Smoke project".into()),
        false,
    )?;
    // Workspace watch publication follows the registry mutation asynchronously.
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let mut spaces = core.workspace.watch_spaces();
        while !spaces
            .borrow_and_update()
            .iter()
            .any(|space| space.id == "smoke-project")
        {
            spaces.changed().await?;
        }
        Ok::<_, tokio::sync::watch::error::RecvError>(())
    })
    .await??;
    let tools = Tools::new(Arc::new(Zeron::with_client(
        zeron_rpc::memory_client(core.rpc_service()),
        Origin::default(),
    )));
    zeron_mcp::serve_stdio(Arc::new(tools)).await?;
    core.shutdown().await;
    Ok(())
}
