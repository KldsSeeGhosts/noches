use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

fn context() -> SessionMcpContext {
    let mut entry = McpServerEntry::http(
        "scope",
        "http://127.0.0.1:1234/mcp",
        [(
            "Authorization".into(),
            "Bearer private-opaque-test-credential".into(),
        )]
        .into(),
    );
    entry.allowed_tools.push("mcp__scope__echo".into());
    SessionMcpContext::new(vec![entry], "scope instructions".into(), Vec::new()).unwrap()
}

#[test]
fn debug_events_and_diagnostics_do_not_expose_opaque_tokens() {
    let context = context();
    assert!(!format!("{context:?}").contains("private-opaque-test-credential"));
    assert!(
        !format!("{:?}", context.entries()[0].transport).contains("private-opaque-test-credential")
    );
    let raw = "opaque token without a label: private-opaque-test-credential";
    assert!(!crate::redact::redact_output(raw).contains("private-opaque-test-credential"));
    let tail = crate::StderrTail::default();
    tail.push(raw);
    assert!(
        !tail
            .snapshot()
            .unwrap()
            .contains("private-opaque-test-credential")
    );
    let error = crate::HarnessError::Protocol(raw.into());
    assert!(!format!("{error:?} {error}").contains("private-opaque-test-credential"));
    let event = crate::redact::redact_event(zeron_proto::AgentEvent::Subagent {
        parent_tool_use_id: "child".into(),
        event: Box::new(zeron_proto::AgentEvent::Error {
            message: raw.into(),
        }),
    });
    let serialized = serde_json::to_string(&event).unwrap();
    assert!(!serialized.contains("private-opaque-test-credential"));
    assert!(serialized.contains("[redacted]"));
    context.revoke();
    // Revocation must not disable redaction of already queued provider output.
    assert!(!crate::redact::redact_registered(raw).contains("private-opaque-test-credential"));
}

#[test]
fn redacts_case_insensitive_bearer_and_both_query_encodings() {
    let _context = SessionMcpContext::new(
        vec![McpServerEntry::http(
            "scope",
            "http://127.0.0.1:1234/mcp?credential=query%2Bprivate%2Bsecret",
            [(
                "Authorization".into(),
                "bearer lowercase-private-secret".into(),
            )]
            .into(),
        )],
        String::new(),
        Vec::new(),
    )
    .unwrap();
    for value in [
        "lowercase-private-secret",
        "query+private+secret",
        "query%2Bprivate%2Bsecret",
    ] {
        assert_eq!(crate::redact::redact_registered(value), "[redacted]");
    }
}

#[tokio::test]
async fn dropping_the_consumer_closes_even_an_idle_provider_mailbox() {
    let (provider, consumer) = crate::session_event_channel(&context());
    drop(consumer);
    tokio::time::timeout(std::time::Duration::from_secs(1), provider.closed())
        .await
        .expect("redaction relay must propagate consumer shutdown without another provider event");
}

#[test]
fn revocation_is_once_and_racing_registration_runs_immediately() {
    let context = context();
    let count = Arc::new(AtomicUsize::new(0));
    let increments = count.clone();
    context.on_revoke(move || {
        increments.fetch_add(1, Ordering::SeqCst);
    });
    let guard = context.run_guard();
    drop(guard);
    context.revoke();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let increments = count.clone();
    context.on_revoke(move || {
        increments.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(count.load(Ordering::SeqCst), 2);
    assert!(context.entries().is_empty());
    assert!(context.instructions().is_empty());
    assert!(context.allowed_tools().is_empty());
}

#[test]
fn claude_private_config_is_additive_and_deleted_with_its_guard() {
    let context = context();
    let file = context.claude_config().unwrap().unwrap();
    let path = file.path().to_path_buf();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            file.as_file().metadata().unwrap().permissions().mode() & 0o077,
            0
        );
    }
    let config: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        config["mcpServers"]["scope"]["headers"]["Authorization"],
        "Bearer private-opaque-test-credential"
    );
    assert_eq!(config["mcpServers"]["scope"]["timeout"], TOOL_TIMEOUT_MS);
    drop(file);
    assert!(!path.exists());
}

#[test]
fn all_codex_lifecycle_config_shapes_are_additive() {
    let context = context();
    for method in ["thread/start", "thread/resume", "thread/fork"] {
        let config = context.codex_thread_overrides();
        assert_eq!(
            config["mcp_servers.scope"]["http_headers"]["Authorization"],
            "Bearer private-opaque-test-credential",
            "{method}"
        );
        // No whole-table override that erases the user's configured servers.
        assert!(config.get("mcp_servers").is_none());
    }
}

#[test]
fn acp_always_bridges_http_and_cursor_uses_sdk_shape() {
    let context = context().with_executable("/test/noches".into());
    let acp = context.acp_servers();
    assert_eq!(acp[0]["command"], "/test/noches");
    assert_eq!(acp[0]["args"], json!(["acp-mcp-bridge"]));
    assert!(
        acp[0]["env"][0]["value"]
            .as_str()
            .unwrap()
            .contains("private-opaque-test-credential")
    );
    let cursor = context.cursor_servers();
    assert_eq!(cursor["scope"]["type"], "http");
    assert_eq!(
        cursor["scope"]["headers"]["Authorization"],
        "Bearer private-opaque-test-credential"
    );
    assert!(
        context
            .acp_instructions()
            .contains("NOCHES_ACP_MCP_EXECUTABLE")
    );
}

#[test]
fn pi_extension_file_contains_no_bindings_or_tokens() {
    let context = context();
    let dir = tempfile::tempdir().unwrap();
    let file = context.pi_extension(dir.path()).unwrap();
    let source = std::fs::read_to_string(file).unwrap();
    assert!(!source.contains("private-opaque-test-credential"));
    assert!(!source.contains("127.0.0.1:1234"));
    assert!(
        source.contains("registerTool")
            && source.contains("tools/list")
            && source.contains("tools/call")
    );
    assert!(source.contains("before_agent_start") && source.contains("session_shutdown"));
}

#[test]
fn validates_duplicate_names_and_endpoint_without_leaking_it() {
    assert!(
        SessionMcpContext::new(
            vec![
                context().entries()[0].clone(),
                context().entries()[0].clone()
            ],
            String::new(),
            Vec::new()
        )
        .is_err()
    );
    let bad = McpServerEntry::http(
        "scope",
        "https://user:password@example.com/mcp",
        BTreeMap::new(),
    );
    let error = SessionMcpContext::new(vec![bad], String::new(), Vec::new())
        .unwrap_err()
        .to_string();
    assert!(!error.contains("password") && !error.contains("example.com"));
}
