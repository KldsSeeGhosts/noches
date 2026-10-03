use super::*;
use crate::orchestration::service::CallerScope;

fn scope() -> InvocationScope {
    InvocationScope {
        environment_id: "host".into(),
        caller: CallerScope {
            thread_id: "parent".into(),
            run_id: "run".into(),
            session_id: String::new(),
            project_id: "project".into(),
            workspace_root: "/repo".into(),
            runtime_mode: zeron_proto::RuntimeMode::FullAccess,
            interaction_mode: zeron_proto::InteractionMode::Default,
            provider_instance_id: "mock".into(),
        },
        selection: serde_json::from_value(json!({"instanceId":"mock","model":"mock-1"})).unwrap(),
        capabilities: ["orchestration", "worktree", "pull-requests"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        task_id: None,
        issued_at: 0,
    }
}

fn harness_registry() -> Arc<HarnessRegistry> {
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(zeron_harness::mock::MockHarness {
        script: vec![],
    }));
    registry
}

#[test]
fn instructions_are_verbatim_pinned_t3_text() {
    use sha2::{Digest, Sha256};
    assert_eq!(INSTRUCTIONS.len(), 5521);
    assert_eq!(
        format!("{:x}", Sha256::digest(INSTRUCTIONS)),
        "23317b4e6443838eb57eae6302ab3808556522838cf5bba4d565b8899e6c3e5b"
    );
}

#[tokio::test]
async fn all_52_core_routes_accept_upstream_inputs_and_refuse_unimplemented_domains() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/t3_mcp_oracle/accepted.json")).unwrap();
    assert_eq!(cases.len(), 52);
    let toolkit = toolkit::Toolkit::new(harness_registry());
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let reply = toolkit
            .request(
                scope(),
                json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                    "params":{"name":name,"arguments":case["arguments"]}
                }),
            )
            .await
            .unwrap();
        assert!(reply.get("error").is_none(), "{name}: {reply}");
        if name == "orchestrator_capabilities" {
            serde_json::from_value::<zeron_proto::orchestration_mcp::OrchestratorCapabilitiesResult>(
                reply["result"]["structuredContent"].clone()).unwrap();
        } else if case["group"] == "pullRequests" {
            assert_eq!(reply["result"]["isError"], true, "{name}");
        } else if matches!(name, "t3_worktree_handoff" | "t3_worktree_status") {
            serde_json::from_value::<zeron_proto::orchestration_mcp::WorktreeMcpFailure>(
                reply["result"]["structuredContent"].clone(),
            )
            .unwrap();
        } else {
            assert_eq!(
                reply["result"],
                codec::result(codec::unavailable()),
                "{name}"
            );
        }
    }
}

#[tokio::test]
async fn parameter_errors_match_executed_pinned_effect_oracle() {
    let oracle: Value =
        serde_json::from_str(include_str!("../../tests/t3_mcp_oracle/validation.json")).unwrap();
    let toolkit = toolkit::Toolkit::new(harness_registry());
    let mut differences = Vec::new();
    for case in oracle["cases"].as_array().unwrap() {
        let reply = toolkit
            .request(
                scope(),
                json!({
                    "jsonrpc":"2.0","id":1,"method":"tools/call",
                    "params":{"name":case["name"],"arguments":case["arguments"]}
                }),
            )
            .await
            .unwrap();
        let field = if case["expected"].get("error").is_some() {
            "error"
        } else {
            "result"
        };
        if reply[field] != case["expected"][field] {
            differences.push(format!(
                "{}: {}\nactual: {}\nexpected: {}",
                case["name"], case["arguments"], reply[field], case["expected"][field]
            ));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n\n"));
}

#[tokio::test]
async fn live_tools_list_is_exact_pinned_core_descriptor_set() {
    let server = Arc::new(McpServer::new(Arc::new(HarnessRegistry::new())));
    let endpoint = server.endpoint().await.unwrap();
    let credential = server.credentials.issue(scope()).unwrap();
    let result: Value = reqwest::Client::new()
        .post(endpoint)
        .header("authorization", credential.authorization)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../../proto/tests/t3_oracle/fixtures/tools.json"
    ))
    .unwrap();
    let expected: Vec<_> = fixtures
        .into_iter()
        .filter(|t| t["phase"] == "core")
        .map(|t| t["descriptor"].clone())
        .collect();
    assert_eq!(expected.len(), 52);
    assert_eq!(result["result"]["tools"], json!(expected));
}

#[test]
fn invalid_expired_revoked_and_refreshed_credentials() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let clock = Arc::new(AtomicU64::new(100));
    let now = clock.clone();
    let registry = CredentialRegistry::new(
        Duration::from_millis(20),
        Arc::new(move || now.load(Ordering::SeqCst)),
    );
    let credential = registry.issue(scope()).unwrap();
    assert!(registry.resolve("Bearer invalid").is_none());
    assert!(registry.resolve("").is_none());
    assert_eq!(credential.authorization.len(), 7 + 43);
    clock.store(120, Ordering::SeqCst);
    assert!(registry.resolve(&credential.authorization).is_some());
    clock.store(140, Ordering::SeqCst);
    registry.touch("parent");
    clock.store(160, Ordering::SeqCst);
    assert!(registry.resolve(&credential.authorization).is_some());
    clock.store(181, Ordering::SeqCst);
    assert!(registry.resolve(&credential.authorization).is_none());
    let credential = registry.issue(scope()).unwrap();
    registry.revoke_session(&credential.session_id);
    assert!(registry.resolve(&credential.authorization).is_none());
    let credential = registry.issue(scope()).unwrap();
    registry.revoke_thread("parent");
    assert!(registry.resolve(&credential.authorization).is_none());
}

async fn post(
    server: &Arc<McpServer>,
    credential: &str,
    method: &str,
    params: Value,
) -> reqwest::Response {
    reqwest::Client::new()
        .post(server.endpoint().await.unwrap())
        .header("authorization", credential)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn http_invalid_revoked_expired_and_wrong_session_are_401() {
    let mut server = McpServer::new(Arc::new(HarnessRegistry::new()));
    let clock = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let now = clock.clone();
    server.credentials = Arc::new(CredentialRegistry::new(
        Duration::from_millis(10),
        Arc::new(move || now.load(std::sync::atomic::Ordering::SeqCst)),
    ));
    let server = Arc::new(server);
    let invalid = post(&server, "Bearer invalid", "tools/list", json!({})).await;
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(invalid.headers()["www-authenticate"], "Bearer");
    assert_eq!(
        invalid.json::<Value>().await.unwrap(),
        json!({
            "error":"invalid_mcp_credential","message":"A valid provider-scoped MCP bearer credential is required."
        })
    );
    let first = server.credentials.issue(scope()).unwrap();
    let second = server.credentials.issue(scope()).unwrap();
    let wrong = reqwest::Client::new()
        .post(server.endpoint().await.unwrap())
        .header("authorization", &first.authorization)
        .header("mcp-session-id", second.session_id)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
    server.credentials.revoke_session(&first.session_id);
    assert_eq!(
        post(&server, &first.authorization, "tools/list", json!({}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    clock.store(11, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        post(&server, &second.authorization, "tools/list", json!({}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn transport_handshake_and_opaque_origin_scope() {
    let server = Arc::new(McpServer::new(Arc::new(HarnessRegistry::new())));
    let credential = server.credentials.issue(scope()).unwrap();
    let initialized = post(
        &server,
        &credential.authorization,
        "initialize",
        json!({"protocolVersion":"older"}),
    )
    .await;
    assert_eq!(
        initialized.headers()["mcp-session-id"],
        credential.session_id
    );
    let initialized: Value = initialized.json().await.unwrap();
    assert_eq!(initialized["result"]["serverInfo"]["name"], "t3-code");
    assert_eq!(initialized["result"]["protocolVersion"], PROTOCOL_VERSION);
    assert_eq!(initialized["result"]["instructions"], INSTRUCTIONS);
    let result: Value = post(&server, &credential.authorization, "tools/call", json!({
        "name":"orchestrator_capabilities","arguments":{"threadId":"foreign","projectId":"foreign","providerInstanceId":"foreign"}
    })).await.json().await.unwrap();
    assert_eq!(
        result["result"]["structuredContent"]["parentThreadId"],
        "parent"
    );
    let notification = reqwest::Client::new()
        .post(server.endpoint().await.unwrap())
        .header("authorization", &credential.authorization)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert_eq!(notification.status(), StatusCode::ACCEPTED);
    assert!(notification.bytes().await.unwrap().is_empty());
}

#[tokio::test]
async fn unavailable_and_validation_families_keep_t3_framing() {
    let server = Arc::new(McpServer::new(harness_registry()));
    let credential = server.credentials.issue(scope()).unwrap();
    for (name, arguments, expected) in [
        (
            "delegate_task",
            json!({"task":"implement"}),
            codec::result(codec::unavailable()),
        ),
        (
            "t3_queue_read",
            json!({"queuedRunId":"run"}),
            codec::result(codec::unavailable()),
        ),
        (
            "t3_worktree_status",
            json!({}),
            codec::result(
                json!({"_tag":"WorktreeMcpFailure","code":"operation_failed","message":"Unable to read thread parent: The operation could not be completed."}),
            ),
        ),
        (
            "link_pull_request",
            json!({"url":"https://github.com/owner/repo/pull/1"}),
            codec::error_text("Could not link the pull request."),
        ),
    ] {
        let result: Value = post(
            &server,
            &credential.authorization,
            "tools/call",
            json!({"name":name,"arguments":arguments}),
        )
        .await
        .json()
        .await
        .unwrap();
        assert_eq!(result["result"], expected, "{name}");
    }
    let invalid: Value = post(
        &server,
        &credential.authorization,
        "tools/call",
        json!({"name":"delegate_task","arguments":{"task":"   "}}),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(invalid["result"]["isError"], false);
    assert_eq!(
        invalid["result"]["structuredContent"]["reason"]["_tag"],
        "ToolParameterValidationError"
    );
    let invalid: Value = post(
        &server,
        &credential.authorization,
        "tools/call",
        json!({"name":"link_pull_request","arguments":{"number":0}}),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(invalid["error"]["code"], -32602);
    let preview: Value = post(
        &server,
        &credential.authorization,
        "tools/call",
        json!({"name":"preview_status"}),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(preview["error"]["code"], -32602);
}

#[tokio::test]
async fn narrowed_capabilities_and_task_scope_refuse_before_service() {
    let server = Arc::new(McpServer::new(Arc::new(HarnessRegistry::new())));
    let mut narrower = scope();
    narrower.capabilities.clear();
    let credential = server.credentials.issue(narrower).unwrap();
    let result: Value = post(
        &server,
        &credential.authorization,
        "tools/call",
        json!({"name":"orchestrator_capabilities","arguments":{}}),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(
        result["result"],
        codec::result(codec::failure(
            "capability_denied",
            "This MCP credential does not grant orchestration capabilities."
        ))
    );
    let mut narrower = scope();
    narrower.task_id = Some("owned".into());
    let credential = server.credentials.issue(narrower).unwrap();
    let result: Value = post(
        &server,
        &credential.authorization,
        "tools/call",
        json!({"name":"task_cancel","arguments":{"taskId":"foreign"}}),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(
        result["result"]["structuredContent"]["code"],
        "task_not_found"
    );
}

#[test]
fn service_seam_and_refusal_serialization_are_generated_contracts() {
    use zeron_proto::orchestration_mcp::{
        OrchestratorMcpFailure, ToolFrameworkAiError, WorktreeMcpFailure,
    };
    let refusal = codec::unavailable();
    assert_eq!(
        serde_json::to_value(
            serde_json::from_value::<OrchestratorMcpFailure>(refusal.clone()).unwrap()
        )
        .unwrap(),
        refusal
    );
    let worktree = json!({"_tag":"WorktreeMcpFailure","code":"operation_failed","message":"The operation could not be completed."});
    assert_eq!(
        serde_json::to_value(
            serde_json::from_value::<WorktreeMcpFailure>(worktree.clone()).unwrap()
        )
        .unwrap(),
        worktree
    );
    let validation = codec::invalid_parameters("delegate_task", "Missing key");
    assert_eq!(
        serde_json::to_value(
            serde_json::from_value::<ToolFrameworkAiError>(validation.clone()).unwrap()
        )
        .unwrap(),
        validation
    );
}

#[tokio::test]
async fn injected_domain_service_receives_only_trusted_caller_and_resolved_target() {
    use crate::orchestration::service::{OrchestratorService, ToolError};
    use zeron_proto::orchestration_mcp::*;
    struct Service(std::sync::Mutex<Vec<Value>>);
    #[async_trait::async_trait]
    impl OrchestratorService for Service {
        async fn delegate_task(
            &self,
            caller: CallerScope,
            input: DelegateTaskInput,
        ) -> Result<DelegateTaskResult, ToolError> {
            self.0.lock().unwrap().push(json!({"thread":caller.thread_id,"project":caller.project_id,"run":caller.run_id,"input":input}));
            Err(ToolError::new(
                OrchestratorMcpFailureCode::OrchestrationError,
                "Fixture service",
            ))
        }
        async fn task_status(
            &self,
            caller: CallerScope,
            input: TaskStatusInput,
        ) -> Result<TaskStatusResult, ToolError> {
            self.0
                .lock()
                .unwrap()
                .push(json!({"thread":caller.thread_id,"task":input.task_id}));
            Err(ToolError::new(
                OrchestratorMcpFailureCode::OrchestrationError,
                "Fixture service",
            ))
        }
        async fn task_cancel(
            &self,
            caller: CallerScope,
            input: TaskCancelInput,
        ) -> Result<TaskCancelResult, ToolError> {
            self.0
                .lock()
                .unwrap()
                .push(json!({"thread":caller.thread_id,"task":input.task_id}));
            Err(ToolError::new(
                OrchestratorMcpFailureCode::OrchestrationError,
                "Fixture service",
            ))
        }
    }
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(zeron_harness::mock::MockHarness {
        script: vec![],
    }));
    let server = Arc::new(McpServer::new(registry));
    let service = Arc::new(Service(std::sync::Mutex::new(Vec::new())));
    server.set_service(service.clone());
    let credential = server.credentials.issue(scope()).unwrap();
    for (name, arguments) in [
        (
            "delegate_task",
            json!({"task":"  implement  ","threadId":"foreign","projectId":"foreign","clientRequestId":"stable"}),
        ),
        ("task_status", json!({"taskId":"child"})),
        ("task_cancel", json!({"taskId":"child"})),
    ] {
        let result: Value = post(
            &server,
            &credential.authorization,
            "tools/call",
            json!({"name":name,"arguments":arguments}),
        )
        .await
        .json()
        .await
        .unwrap();
        assert_eq!(
            result["result"]["structuredContent"]["message"],
            "Fixture service"
        );
    }
    let calls = service.0.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0]["thread"], "parent");
    assert_eq!(calls[0]["project"], "project");
    assert_eq!(calls[0]["run"], "run");
    assert_eq!(calls[0]["input"]["task"], "implement");
    assert_eq!(calls[0]["input"]["target"]["providerInstanceId"], "mock");
    assert_eq!(calls[0]["input"]["clientRequestId"], "stable");
}

#[tokio::test]
async fn delegation_escalation_is_refused_before_domain_dispatch() {
    let server = Arc::new(McpServer::new(harness_registry()));
    let mut narrowed = scope();
    narrowed.caller.runtime_mode = zeron_proto::RuntimeMode::ApprovalRequired;
    narrowed.caller.interaction_mode = zeron_proto::InteractionMode::Plan;
    let credential = server.credentials.issue(narrowed).unwrap();
    for (field, mode, code, message) in [
        (
            "runtimeMode",
            "full-access",
            "runtime_mode_escalation_denied",
            "Child runtime mode full-access is broader than parent mode approval-required.",
        ),
        (
            "interactionMode",
            "default",
            "interaction_mode_escalation_denied",
            "Child interaction mode default is broader than parent mode plan.",
        ),
    ] {
        let mut arguments = json!({"task":"implement"});
        arguments[field] = json!(mode);
        let result: Value = post(
            &server,
            &credential.authorization,
            "tools/call",
            json!({"name":"delegate_task","arguments":arguments}),
        )
        .await
        .json()
        .await
        .unwrap();
        assert_eq!(
            result["result"],
            codec::result(codec::failure(code, message))
        );
    }
}

#[tokio::test]
async fn readonly_preapproval_tracks_annotations_not_names() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = crate::SessionsEngine::new(
        "host".into(),
        Arc::new(crate::RunJournal::open(dir.path()).unwrap()),
        Arc::new(HarnessRegistry::new()),
    );
    let mut scoped = scope();
    scoped.caller.interaction_mode = zeron_proto::InteractionMode::Plan;
    let context = sessions
        .mcp_server()
        .register(&sessions, "parent", scoped)
        .await
        .unwrap();
    assert!(
        context
            .allowed_tools()
            .iter()
            .any(|tool| tool == "mcp__t3-code__orchestrator_capabilities")
    );
    assert!(
        !context
            .allowed_tools()
            .iter()
            .any(|tool| tool.ends_with("task_status")
                || tool.ends_with("t3_thread_read")
                || tool.ends_with("delegate_task"))
    );
    let credential = match &context.entries()[0].transport {
        zeron_harness::mcp::McpTransport::StreamableHttp { headers, .. } => {
            headers["Authorization"].clone()
        }
        _ => panic!("expected HTTP"),
    };
    // Preapproval and registration are independent: all 52 remain discoverable.
    let reply: Value = post(&sessions.mcp_server(), &credential, "tools/list", json!({}))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(reply["result"]["tools"].as_array().unwrap().len(), 52);
}

#[tokio::test]
async fn replacement_revocation_cannot_revoke_successor_and_debug_is_redacted() {
    let dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(HarnessRegistry::new());
    let sessions = crate::SessionsEngine::new(
        "host".into(),
        Arc::new(crate::RunJournal::open(dir.path()).unwrap()),
        registry.clone(),
    );
    let server = sessions.mcp_server();
    let first = server.register(&sessions, "parent", scope()).await.unwrap();
    let authorization = |context: &SessionMcpContext| match &context.entries()[0].transport {
        zeron_harness::mcp::McpTransport::StreamableHttp { headers, .. } => {
            headers["Authorization"].clone()
        }
        _ => panic!("expected HTTP"),
    };
    let first_authorization = authorization(&first);
    let second = server.register(&sessions, "parent", scope()).await.unwrap();
    assert!(server.credentials.resolve(&first_authorization).is_none());
    assert!(
        server
            .credentials
            .resolve(&authorization(&second))
            .is_some()
    );
    first.revoke();
    assert!(
        server
            .credentials
            .resolve(&authorization(&second))
            .is_some()
    );
    let raw = authorization(&second);
    assert!(!format!("{second:?}").contains(raw.strip_prefix("Bearer ").unwrap()));
    sessions.revoke_session_mcp("parent");
    assert!(server.credentials.resolve(&raw).is_none());
}
