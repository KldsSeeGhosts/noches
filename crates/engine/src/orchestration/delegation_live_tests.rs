//! Opt-in billable live cross-provider test. The small HTTP fixture is test
//! transport only; production t3-code HTTP/auth belongs to orch/mcp-server.
use super::command::Command;
use super::runner::{RunnerBridge, RunnerInstances, RunnerMcp, RunnerProvider};
use super::service::{CallerScope, OrchestratorService, ToolError};
use super::task::{CatalogTargets, DelegationCatalog, DelegationService, TaskOperation, records};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use zeron_harness::{ClaudeHarness, CodexHarness, Harness};
use zeron_proto::orchestration::*;
use zeron_proto::orchestration_mcp::*;
use zeron_proto::provider_instance::*;
use zeron_proto::{HarnessId, RuntimeMode};

struct LiveInstances {
    codex: Arc<dyn Harness>,
    claude: Arc<dyn Harness>,
    codex_model: String,
    claude_model: String,
}
#[async_trait]
impl DelegationCatalog for LiveInstances {
    async fn providers(&self) -> Result<Vec<OrchestratorMcpProviderCapability>, ToolError> {
        ["codex","claude"].into_iter().map(|instance| serde_json::from_value(json!({
            "providerInstanceId":instance,"driverKind":instance,"displayName":instance,
            "models":[{"id":if instance=="codex" {&self.codex_model} else {&self.claude_model},"label":null}],
            "canRunChildTask":true,"canRunCrossProviderChildTask":true,"constraints":[]
        })).map_err(|error| ToolError::new(OrchestratorMcpFailureCode::OrchestrationError,error.to_string()))).collect()
    }
}
#[async_trait]
impl RunnerInstances for LiveInstances {
    async fn resolve(&self, instance: &ProviderInstanceId) -> Result<RunnerProvider, ToolError> {
        let harness = match instance.0.as_str() {
            "codex" => self.codex.clone(),
            "claude" => self.claude.clone(),
            _ => {
                return Err(ToolError::new(
                    OrchestratorMcpFailureCode::ProviderUnavailable,
                    "Live fixture instance is unavailable.",
                ));
            }
        };
        let samples: BTreeMap<String, Vec<Value>> = serde_json::from_str(include_str!(
            "../../../proto/tests/t3_oracle/fixtures/serde-cases.json"
        ))
        .unwrap();
        let mut caps = samples["OrchestrationV2ProviderCapabilities"][0].clone();
        // Conservative test adapter snapshot: only capabilities exercised here.
        caps["turns"]["supportsActiveSteering"] = json!(true);
        caps["turns"]["supportsInterrupt"] = json!(true);
        caps["turns"]["emitsTurnCompleted"] = json!(true);
        caps["tools"]["supportsMcpTools"] = json!(true);
        caps["streaming"]["streamsAssistantText"] = json!(true);
        caps["identity"]["nativeTurnIds"] = json!("weak");
        caps["identity"]["nativeItemIds"] = json!("weak");
        caps["identity"]["nativeRequestIds"] = json!("weak");
        Ok(RunnerProvider {
            harness,
            capabilities: serde_json::from_value(caps).unwrap(),
        })
    }
}

struct LiveMcp {
    endpoint: String,
    scopes: Arc<Mutex<BTreeMap<String, CallerScope>>>,
}
#[async_trait]
impl RunnerMcp for LiveMcp {
    async fn bind(
        &self,
        scope: CallerScope,
        sessions: &crate::sessions::SessionsEngine,
    ) -> Result<(), ToolError> {
        let token = uuid::Uuid::new_v4().to_string();
        let mut headers = BTreeMap::new();
        headers.insert("Authorization".into(), format!("Bearer {token}"));
        let mut entry =
            zeron_harness::mcp::McpServerEntry::http("t3-code", &self.endpoint, headers);
        entry.allowed_tools = vec![
            "mcp__t3-code__delegate_task".into(),
            "mcp__t3-code__task_status".into(),
            "mcp__t3-code__task_cancel".into(),
        ];
        let context=sessions.register_session_mcp(&scope.thread_id.0,vec![entry],
            "Use the t3-code delegate_task tool for app-owned child tasks. Every round has its own task ID. Read task_status for the result.".into())
            .map_err(|error| ToolError::new(OrchestratorMcpFailureCode::OrchestrationError,error.to_string()))?;
        self.scopes.lock().unwrap().insert(token.clone(), scope);
        let scopes = self.scopes.clone();
        context.on_revoke(move || {
            scopes.lock().unwrap().remove(&token);
        });
        Ok(())
    }
}

async fn serve_connection(
    mut socket: tokio::net::TcpStream,
    scopes: Arc<Mutex<BTreeMap<String, CallerScope>>>,
    service: DelegationService,
) -> anyhow::Result<()> {
    let mut bytes = vec![];
    let header_end = loop {
        let mut chunk = [0; 4096];
        let count = socket.read(&mut chunk).await?;
        if count == 0 {
            return Ok(());
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > 1_048_576 {
            anyhow::bail!("Live fixture request too large");
        }
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let length = headers
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        })
        .unwrap_or(0);
    while bytes.len() < header_end + length {
        let mut chunk = [0; 4096];
        let count = socket.read(&mut chunk).await?;
        if count == 0 {
            anyhow::bail!("Incomplete request");
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    let token = headers.lines().find_map(|line| {
        line.split_once(':')
            .filter(|(key, _)| key.eq_ignore_ascii_case("authorization"))
            .map(|(_, value)| {
                value
                    .trim()
                    .strip_prefix("Bearer ")
                    .unwrap_or("")
                    .to_owned()
            })
    });
    let scope = token.and_then(|token| scopes.lock().unwrap().get(&token).cloned());
    let (status, body) = if !headers.starts_with("POST ") {
        (405, String::new())
    } else if let Some(scope) = scope {
        let request: Value = serde_json::from_slice(&bytes[header_end..header_end + length])?;
        let result = match request["method"].as_str().unwrap_or("") {
            "initialize" => {
                json!({"protocolVersion":request["params"]["protocolVersion"],"capabilities":{"tools":{}},"serverInfo":{"name":"noches-live-delegation-test","version":"1"}})
            }
            "notifications/initialized" => {
                socket
                    .write_all(
                        b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await?;
                return Ok(());
            }
            "tools/list" => {
                json!({"tools":pinned_tool_inventory().into_iter().filter(|tool| matches!(tool.name.as_str(),"delegate_task"|"task_status"|"task_cancel")).map(|tool|
                    json!({"name":tool.name,"description":tool.description,"inputSchema":tool.input_schema,"annotations":tool.annotations})
                ).collect::<Vec<_>>()})
            }
            "tools/call" => {
                let args = request["params"]["arguments"].clone();
                let result: Result<Value, ToolError> =
                    match request["params"]["name"].as_str().unwrap_or("") {
                        "delegate_task" => service
                            .delegate_task(scope, serde_json::from_value(args)?)
                            .await
                            .map(|value| serde_json::to_value(value).unwrap()),
                        "task_status" => service
                            .task_status(scope, serde_json::from_value(args)?)
                            .await
                            .map(|value| serde_json::to_value(value).unwrap()),
                        "task_cancel" => service
                            .task_cancel(scope, serde_json::from_value(args)?)
                            .await
                            .map(|value| serde_json::to_value(value).unwrap()),
                        _ => Err(ToolError::new(
                            OrchestratorMcpFailureCode::CapabilityDenied,
                            "Unknown test tool.",
                        )),
                    };
                match result {
                    Ok(result) => {
                        json!({"content":[{"type":"text","text":result.to_string()}],"structuredContent":result,"isError":false})
                    }
                    Err(error) => {
                        let error = serde_json::to_value(error.into_failure()).unwrap();
                        json!({"content":[{"type":"text","text":error.to_string()}],"structuredContent":error,"isError":false})
                    }
                }
            }
            _ => json!({}),
        };
        (
            200,
            json!({"jsonrpc":"2.0","id":request["id"],"result":result}).to_string(),
        )
    } else {
        (401, String::new())
    };
    let reason = if status == 200 {
        "OK"
    } else if status == 401 {
        "Unauthorized"
    } else {
        "Method Not Allowed"
    };
    socket.write_all(format!("HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "Billable live Codex→Claude MCP test; NOCHES_LIVE_DELEGATION=1 and explicit model environment variables required"]
async fn codex_parent_delegates_to_installed_claude_child_via_injected_mcp() {
    assert_eq!(std::env::var("NOCHES_LIVE_DELEGATION").as_deref(), Ok("1"));
    let codex_model =
        std::env::var("NOCHES_LIVE_CODEX_MODEL").expect("Set installed Codex model explicitly");
    let claude_model =
        std::env::var("NOCHES_LIVE_CLAUDE_MODEL").expect("Set installed Claude model explicitly");
    let codex = Arc::new(CodexHarness::new());
    let claude = Arc::new(ClaudeHarness::new());
    assert!(
        codex.installed() && claude.installed(),
        "Both real CLIs must be installed and authenticated"
    );
    let instances = Arc::new(LiveInstances {
        codex: codex.clone(),
        claude: claude.clone(),
        codex_model: codex_model.clone(),
        claude_model: claude_model.clone(),
    });
    let registry = crate::registry::HarnessRegistry::new();
    registry.register(codex);
    registry.register(claude);
    let dir = tempfile::tempdir().unwrap();
    let core = crate::EngineCore::assemble(
        &dir.path().join("engine"),
        Arc::new(registry),
        HarnessId::Codex,
        None,
    )
    .unwrap();
    let kernel = core.orchestration.clone();
    let service = DelegationService {
        kernel: kernel.clone(),
        targets: Arc::new(CatalogTargets(instances.clone())),
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/mcp", listener.local_addr().unwrap());
    let scopes = Arc::new(Mutex::new(BTreeMap::new()));
    let server_scopes = scopes.clone();
    let server_service = service.clone();
    let stop = CancellationToken::new();
    let server_stop = stop.clone();
    let server = tokio::spawn(async move {
        loop {
            let (socket, _) = tokio::select! {_=server_stop.cancelled()=>break,result=listener.accept()=>result.unwrap()};
            let scopes = server_scopes.clone();
            let service = server_service.clone();
            tokio::spawn(async move {
                if let Err(error) = serve_connection(socket, scopes, service).await {
                    tracing::error!(%error,"live fixture MCP failed");
                }
            });
        }
    });
    let thread = ThreadId("live-codex-parent".into());
    let create=Command::wire(serde_json::from_value(json!({
        "type":"thread.create","commandId":"live-parent-create","createdBy":"user","creationSource":"web",
        "threadId":thread,"projectId":"live-project","title":"Live Codex→Claude delegation",
        "modelSelection":{"instanceId":"codex","model":codex_model},"runtimeMode":"full-access","interactionMode":"default",
        "branch":null,"worktreePath":dir.path()
    })).unwrap()).unwrap();
    assert_eq!(
        kernel
            .dispatch(&create, crate::now_ms())
            .await
            .unwrap()
            .status,
        super::ReceiptStatus::Accepted
    );
    let prompt = format!(
        "Call the injected t3-code delegate_task tool exactly once with target {{\"providerInstanceId\":\"claude\",\"model\":{}}}, mode=\"wait\", clientRequestId=\"live-round-1\", task=\"Do not modify files or run commands. Reply exactly NOCHES_CLAUDE_CHILD_OK.\". Then return the child's summary. Do not use a native spawn tool or create a top-level conversation.",
        json!(claude_model)
    );
    kernel
        .task_command(
            &thread,
            CommandId("live-parent-message".into()),
            TaskOperation::StartMessage {
                prompt,
                driver: ProviderDriverKind("codex".into()),
                message_id: MessageId("live-parent-input".into()),
            },
        )
        .await
        .unwrap();
    let bridge = Arc::new(RunnerBridge {
        kernel: kernel.clone(),
        sessions: core.sessions.clone(),
        doc_host: core.doc_host.clone(),
        workspace: core.workspace.clone(),
        device_id: core.device_id.clone(),
        instances,
        mcp: Arc::new(LiveMcp { endpoint, scopes }),
    });
    bridge.attach_recovery_gate();
    let workers = bridge.spawn_workers(stop.clone());
    let result = tokio::time::timeout(Duration::from_secs(300), async {
        loop {
            let parent = kernel.store.thread(&thread).unwrap().unwrap();
            if let Some(task) = records(&parent, "subagent")
                .iter()
                .find(|task| task["origin"] == "app_owned" && task["result"].is_string())
            {
                assert_eq!(task["providerInstanceId"], "claude");
                assert!(
                    task["result"]
                        .as_str()
                        .unwrap()
                        .contains("NOCHES_CLAUDE_CHILD_OK")
                );
                let child = kernel
                    .store
                    .thread(&ThreadId(task["childThreadId"].as_str().unwrap().into()))
                    .unwrap()
                    .unwrap();
                assert_eq!(child.thread.runtime_mode, RuntimeMode::FullAccess);
                assert_eq!(
                    child.thread.worktree_path.as_ref().unwrap(),
                    &dir.path().display().to_string()
                );
                assert_eq!(child.thread.model_selection.model, claude_model);
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    stop.cancel();
    for worker in workers {
        worker.await.unwrap();
    }
    server.await.unwrap();
    core.sessions.shutdown().await;
    result
        .expect("Real Codex parent must call injected MCP and receive a real Claude child result");
}
