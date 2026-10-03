//! A real session/RunControls injection through the HTTP server; no live CLI,
//! accounts, arbitrary command execution, or simulated "received" bindings.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::{StreamExt, stream::BoxStream};
use serde_json::{Value, json};
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_harness::mcp::McpTransport;
use zeron_harness::{Harness, HarnessError, RunControls};
use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, Model, ReasoningLevel, RunRequest, SteeringMode,
};

struct StubHarness {
    observed: Mutex<Option<tokio::sync::oneshot::Sender<Value>>>,
    credential: Arc<Mutex<Option<String>>>,
}

#[async_trait]
impl Harness for StubHarness {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "MCP fixture"
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
            id: "cpa/exact/custom-model".into(),
            label: "Custom fixture".into(),
            description: None,
            reasoning_levels: vec![],
            options: vec![],
        }])
    }
    async fn run(
        &self,
        _: RunRequest,
        controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        let entry = controls
            .mcp
            .entries()
            .iter()
            .find(|s| s.name == "t3-code")
            .expect("engine injected t3-code");
        assert!(
            controls
                .mcp
                .instructions()
                .contains("same live catalog as the composer")
        );
        let McpTransport::StreamableHttp { url, headers } = &entry.transport else {
            panic!("HTTP binding")
        };
        *self.credential.lock().unwrap() = Some(headers["Authorization"].clone());
        let peer = reqwest::Client::new();
        let mut observed = json!({});
        for (id, method, params) in [
            (
                1,
                "initialize",
                json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}),
            ),
            (2, "tools/list", json!({})),
            (
                3,
                "tools/call",
                json!({"name":"orchestrator_capabilities","arguments":{}}),
            ),
        ] {
            let mut request = peer
                .post(url)
                .json(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
            for (key, value) in headers {
                request = request.header(key, value);
            }
            let response: Value = request.send().await.unwrap().json().await.unwrap();
            observed[id.to_string()] = response["result"].clone();
        }
        self.observed
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .send(observed)
            .unwrap();
        Ok(futures::stream::iter(vec![Ok(AgentEvent::Done {
            status: DoneStatus::Completed,
            result: None,
            error: None,
            session_id: None,
        })])
        .boxed())
    }
}

#[tokio::test]
async fn harness_receives_injected_server_and_calls_live_catalog() {
    let dir = tempfile::tempdir().unwrap();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let registry = Arc::new(HarnessRegistry::new());
    let credential = Arc::new(Mutex::new(None));
    registry.register(Arc::new(StubHarness {
        observed: Mutex::new(Some(sender)),
        credential: credential.clone(),
    }));
    let core = EngineCore::assemble(dir.path(), registry.clone(), HarnessId::Mock, None).unwrap();
    core.workspace
        .create_chat(
            "mcp-e2e",
            None,
            Some(&core.device_id),
            None,
            Some(dir.path().to_string_lossy().into()),
        )
        .unwrap();
    let request: RunRequest = serde_json::from_value(json!({
        "prompt":"fixture","model":"cpa/exact/custom-model","reasoning":null,
        "cwd":dir.path(),"sandbox":"danger-full-access","resume":null
    }))
    .unwrap();
    core.sessions
        .dispatch("mcp-e2e", HarnessId::Mock, request, None)
        .await
        .unwrap();
    let observed = tokio::time::timeout(Duration::from_secs(10), receiver)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observed["1"]["serverInfo"]["name"], "t3-code");
    assert_eq!(observed["2"]["tools"].as_array().unwrap().len(), 52);
    let capabilities = &observed["3"]["structuredContent"];
    assert_eq!(capabilities["parentThreadId"], "mcp-e2e");
    assert_eq!(capabilities["inheritedModel"], "cpa/exact/custom-model");
    assert_eq!(
        capabilities["providers"][0]["models"][0]["id"],
        "cpa/exact/custom-model"
    );
    let composer = registry
        .provider_instances
        .refresh(&registry, HarnessId::Mock)
        .await
        .unwrap();
    assert_eq!(
        composer[0].id,
        capabilities["providers"][0]["models"][0]["id"]
    );
    core.shutdown().await;
    let authorization = credential.lock().unwrap().take().unwrap();
    assert!(
        core.sessions
            .mcp_server()
            .credentials
            .resolve(&authorization)
            .is_none()
    );
    let token = authorization.strip_prefix("Bearer ").unwrap().as_bytes();
    let mut directories = vec![dir.path().to_owned()];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                directories.push(entry.path());
            } else if entry.file_type().unwrap().is_file() {
                let bytes = std::fs::read(entry.path()).unwrap();
                assert!(
                    !bytes.windows(token.len()).any(|window| window == token),
                    "MCP credential persisted in {}",
                    entry.path().display()
                );
            }
        }
    }
}
