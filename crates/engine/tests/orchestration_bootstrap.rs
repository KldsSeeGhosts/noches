//! Production bootstrap → scoped HTTP tools → real sessions runner → mailbox
//! → steer/queued continuation → explicit result acknowledgement.
use async_trait::async_trait;
use futures::{StreamExt, stream::BoxStream};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_harness::mcp::McpTransport;
use zeron_harness::{Harness, HarnessError, RunControls};
use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, Model, ReasoningLevel, RunRequest, SteeringMode,
};

#[derive(Clone, Default)]
struct Evidence {
    task: Arc<Mutex<Option<String>>>,
    tokens: Arc<Mutex<Vec<String>>>,
    calls: Arc<Mutex<Vec<String>>>,
}
struct Mock {
    steer: bool,
    evidence: Evidence,
}

async fn call(mcp: &zeron_harness::mcp::SessionMcpContext, name: &str, arguments: Value) -> Value {
    let server = mcp.entries().iter().find(|s| s.name == "t3-code").unwrap();
    let McpTransport::StreamableHttp { url, headers } = &server.transport else {
        panic!("HTTP")
    };
    let mut request = reqwest::Client::new().post(url).json(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":arguments}
    }));
    for (key, value) in headers {
        request = request.header(key, value);
    }
    let response = request.send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let result = body["result"]["structuredContent"].clone();
    assert!(result.get("_tag").is_none(), "{name}: {result}");
    result
}

#[async_trait]
impl Harness for Mock {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Production bootstrap mock"
    }
    fn supports_steering(&self) -> bool {
        self.steer
    }
    fn steering_mode(&self) -> SteeringMode {
        if self.steer {
            SteeringMode::StepBoundary
        } else {
            SteeringMode::TurnBoundary
        }
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
            label: "CPA fixture".into(),
            description: None,
            reasoning_levels: vec![],
            options: vec![],
        }])
    }
    async fn run(
        &self,
        request: RunRequest,
        mut controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        let mcp = controls.mcp.clone();
        let server = mcp.entries().iter().find(|s| s.name == "t3-code").unwrap();
        let McpTransport::StreamableHttp { headers, .. } = &server.transport else {
            panic!("HTTP")
        };
        self.evidence
            .tokens
            .lock()
            .unwrap()
            .push(headers["Authorization"].clone());
        let evidence = self.evidence.clone();
        let steer = self.steer;
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        tokio::spawn(async move {
            tx.send(Ok(AgentEvent::SessionStarted {
                session_id: uuid::Uuid::new_v4().to_string(),
                harness: HarnessId::Mock,
                model: request.model.clone().unwrap(),
                cwd: request.cwd.clone(),
                tools: vec![],
                assistant_message_id: uuid::Uuid::new_v4().to_string(),
            }))
            .await
            .unwrap();
            let result = if request.prompt == "PONG child" {
                let caps = call(&mcp, "orchestrator_capabilities", json!({})).await;
                assert!(
                    caps["parentThreadId"]
                        .as_str()
                        .unwrap()
                        .starts_with("thread:delegated-task:")
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
                "PONG".to_string()
            } else {
                if request.prompt == "delegate parent" {
                    evidence
                        .calls
                        .lock()
                        .unwrap()
                        .push("orchestrator_capabilities".into());
                    let caps = call(&mcp, "orchestrator_capabilities", json!({})).await;
                    assert_eq!(caps["inheritedModel"], "cpa/exact/custom-model");
                    evidence.calls.lock().unwrap().push("delegate_task".into());
                    let task = call(&mcp, "delegate_task", json!({"task":"PONG child","mode":"async",
                        "clientRequestId":"bootstrap-round-1","target":{"providerInstanceId":"child",
                            "model":"cpa/exact/custom-model"}})).await;
                    *evidence.task.lock().unwrap() = Some(task["taskId"].as_str().unwrap().into());
                    if !steer {
                        tx.send(Ok(AgentEvent::Done {
                            status: DoneStatus::Completed,
                            result: Some("Waiting for child".into()),
                            error: None,
                            session_id: None,
                        }))
                        .await
                        .unwrap();
                        return;
                    }
                    let mut notification =
                        tokio::time::timeout(Duration::from_secs(10), controls.steering.recv())
                            .await
                            .unwrap()
                            .unwrap();
                    assert!(
                        notification
                            .prompt
                            .contains("Delegated task completion available"),
                        "{}",
                        notification.prompt
                    );
                    notification
                        .notification_acceptance
                        .take()
                        .unwrap()
                        .send(true)
                        .unwrap();
                    evidence.calls.lock().unwrap().push("steer wake".into());
                } else {
                    assert!(
                        request
                            .prompt
                            .contains("Delegated task completion available"),
                        "{}",
                        request.prompt
                    );
                    evidence.calls.lock().unwrap().push("queued wake".into());
                }
                let task_id = evidence.task.lock().unwrap().clone().unwrap();
                let status = call(&mcp, "task_status", json!({"taskId":task_id})).await;
                evidence.calls.lock().unwrap().push("task_status".into());
                assert_eq!(status["status"], "completed");
                assert!(
                    status["summary"].as_str().unwrap().contains("PONG"),
                    "{status}"
                );
                "Parent received PONG".to_string()
            };
            tx.send(Ok(AgentEvent::Done {
                status: DoneStatus::Completed,
                result: Some(result),
                error: None,
                session_id: None,
            }))
            .await
            .unwrap();
            if steer && request.prompt == "delegate parent" {
                let input = tokio::select! {
                    _ = controls.interrupt.cancelled() => return,
                    input = controls.steering.recv() => input.unwrap(),
                };
                assert_eq!(input.prompt, "second parent");
                tx.send(Ok(AgentEvent::Steered {
                    assistant_message_id: None,
                    next_assistant_message_id: Some(uuid::Uuid::new_v4().to_string()),
                }))
                .await
                .unwrap();
                let task = call(
                    &mcp,
                    "delegate_task",
                    json!({"task":"PONG child","mode":"async",
                    "clientRequestId":"bootstrap-round-2","target":{"providerInstanceId":"child",
                        "model":"cpa/exact/custom-model"}}),
                )
                .await;
                let mut notification =
                    tokio::time::timeout(Duration::from_secs(10), controls.steering.recv())
                        .await
                        .unwrap()
                        .unwrap();
                notification
                    .notification_acceptance
                    .take()
                    .unwrap()
                    .send(true)
                    .unwrap();
                let status = call(&mcp, "task_status", json!({"taskId":task["taskId"]})).await;
                assert_eq!(status["status"], "completed");
                tx.send(Ok(AgentEvent::Done {
                    status: DoneStatus::Completed,
                    result: Some("Parent second received PONG".into()),
                    error: None,
                    session_id: None,
                }))
                .await
                .unwrap();
            }
        });
        Ok(futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        })
        .boxed())
    }
}

async fn e2e(steer: bool) {
    let dir = tempfile::tempdir().unwrap();
    let evidence = Evidence::default();
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(Mock {
        steer,
        evidence: evidence.clone(),
    }));
    // Both instances share a mock adapter but keep distinct opaque identities.
    // Real cross-driver evidence is covered by the headless live run.
    registry.provider_instances.configure(serde_json::from_value(json!([
        {"providerInstanceId":"parent","driverKind":"mock","harnessId":"mock","enabled":true,
            "installed":true,"adapterRegistered":true,"models":[{"id":"cpa/exact/custom-model"}]},
        {"providerInstanceId":"child","driverKind":"mock","harnessId":"mock","enabled":true,
            "installed":true,"adapterRegistered":true,"models":[{"id":"cpa/exact/custom-model"}]}
    ])).unwrap()).unwrap();
    let core =
        EngineCore::assemble(&dir.path().join("data"), registry, HarnessId::Mock, None).unwrap();
    assert!(core.orchestration_host.is_some());
    core.workspace
        .create_chat(
            "parent",
            None,
            Some(&core.device_id),
            None,
            Some(dir.path().display().to_string()),
        )
        .unwrap();
    let request = serde_json::from_value(json!({"prompt":"delegate parent","model":"cpa/exact/custom-model",
        "reasoning":null,"cwd":dir.path(),"sandbox":"workspace-write","runtimeMode":"auto","resume":null})).unwrap();
    core.sessions
        .dispatch("parent", HarnessId::Mock, request, None)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let state = core.orchestration.store.ui_state(&"parent".into()).unwrap();
            if state["latestResult"] == "Parent received PONG"
                && state["tasks"][0]["completionDelivery"]["state"] == "acknowledged"
            {
                assert_eq!(state["tasks"][0]["workState"], "result_available");
                let child = state["tasks"][0]["childThreadId"].as_str().unwrap();
                assert!(core.workspace.chat(child).unwrap().is_some());
                let replica = core.doc_host.open("parent").unwrap().doc().orchestration();
                if replica["projection"]["uiState"]["tasks"][0]["completionDelivery"]["state"]
                    == "acknowledged"
                    && core
                        .orchestration
                        .store
                        .pending_publications()
                        .unwrap()
                        .is_empty()
                {
                    assert_eq!(
                        replica["projection"]["uiState"]["tasks"][0]["latestResult"],
                        "PONG"
                    );
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("child completes, wakes parent, acknowledges, publishes");
    let client = zeron_rpc::memory_client(core.rpc_service());
    let state = client
        .call(
            zeron_rpc::methods::GET_ORCHESTRATION_STATE,
            json!({"chatId":"parent"}),
        )
        .await
        .unwrap();
    assert_eq!(state["tasks"][0]["result"], "PONG");
    let discovered = client
        .call(zeron_rpc::methods::LIST_ORCHESTRATION_THREADS, json!({}))
        .await
        .unwrap();
    assert!(
        discovered
            .as_array()
            .unwrap()
            .iter()
            .any(|thread| thread["id"] == state["tasks"][0]["childThreadId"]
                && thread["lineage"]["parentThreadId"] == "parent")
    );
    assert!(
        evidence
            .calls
            .lock()
            .unwrap()
            .contains(&if steer { "steer wake" } else { "queued wake" }.to_string())
    );
    if steer {
        let next: RunRequest = serde_json::from_value(json!({"prompt":"second parent","model":"cpa/exact/custom-model",
            "reasoning":null,"cwd":dir.path(),"sandbox":"workspace-write","runtimeMode":"auto","resume":null})).unwrap();
        core.sessions
            .dispatch("parent", HarnessId::Mock, next, None)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let state = core.orchestration.store.ui_state(&"parent".into()).unwrap();
                if state["latestResult"] == "Parent second received PONG" {
                    assert_eq!(state["tasks"].as_array().unwrap().len(), 2);
                    assert_eq!(
                        state["tasks"][1]["completionDelivery"]["state"],
                        "acknowledged"
                    );
                    assert_eq!(
                        core.orchestration
                            .store
                            .thread(&"parent".into())
                            .unwrap()
                            .unwrap()
                            .runs
                            .len(),
                        2
                    );
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("warm logical run uses updated scoped credential and active steering");
    }
    let tokens = evidence.tokens.lock().unwrap().clone();
    assert!(tokens.len() >= 2);
    assert_ne!(tokens[0], tokens[1], "child must have a fresh scoped token");
    core.shutdown().await;
    for token in tokens {
        assert!(
            core.sessions
                .mcp_server()
                .credentials
                .resolve(&token)
                .is_none()
        );
    }
}

#[tokio::test]
async fn production_bootstrap_child_completes_and_steers_parent_over_http() {
    e2e(true).await;
}
#[tokio::test]
async fn production_bootstrap_child_completes_and_queues_parent_over_http() {
    e2e(false).await;
}
