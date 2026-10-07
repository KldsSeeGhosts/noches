//! C05: one driver, two real subprocess launch contexts, one composer/MCP catalog.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_harness::CodexHarness;
use zeron_proto::provider_settings::{
    EnableProviderInstance, ProviderInstanceKey, WriteProviderInstance,
};
use zeron_proto::{AgentEvent, DoneStatus, HarnessId, RunRequest};
use zeron_rpc::methods;

fn fixture() -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../harness/tests/fixtures/fake-codex.sh");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

async fn settle(core: &EngineCore, chat: &str, after: u64) {
    tokio::time::timeout(Duration::from_secs(12), async {
        loop {
            if core
                .sessions
                .subscribe(chat, after)
                .unwrap()
                .0
                .iter()
                .any(|e| {
                    matches!(
                        e.event,
                        AgentEvent::Done {
                            status: DoneStatus::Completed,
                            ..
                        }
                    )
                })
                && !core.sessions.turn_in_flight(chat)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
    })
    .await
    .unwrap();
}

fn request(root: &std::path::Path, instance: &str, model: &str) -> RunRequest {
    serde_json::from_value(
        json!({"prompt":"scenario:instance","harness":"codex","instanceId":instance,
        "model":model,"reasoning":null,"cwd":root,"sandbox":"workspace-write",
        "runtimeMode":"full-access","resume":null}),
    )
    .unwrap()
}

#[tokio::test]
async fn two_codex_profiles_do_not_share_homes_auth_models_or_native_resume() {
    let root = tempfile::tempdir().unwrap();
    let binary = fixture();
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(CodexHarness::new().with_executable(&binary)));
    let core = EngineCore::assemble(
        &root.path().join("data"),
        registry.clone(),
        HarnessId::Codex,
        None,
    )
    .unwrap();
    let client = zeron_rpc::memory_client(core.rpc_service());
    for (instance, model) in [("direct", "cpa/direct-model"), ("proxy", "cpa/proxy-model")] {
        let input: WriteProviderInstance = serde_json::from_value(json!({
            "instanceId":instance,"instance":{"driver":"codex","displayName":instance,
                "enabled":true,"config":{"binaryPath":binary},
                "environment":[{"name":"NOCHES_TEST_INSTANCE_MODEL","value":model}]}
        }))
        .unwrap();
        client.create_provider_instance(&input, None).await.unwrap();
    }
    let (one, two) = tokio::join!(
        client.call(methods::LIST_MODELS, json!({"instanceId":"direct"})),
        client.call(methods::LIST_MODELS, json!({"instanceId":"proxy"}))
    );
    assert_eq!(one.unwrap()[0]["id"], "cpa/direct-model");
    assert_eq!(two.unwrap()[0]["id"], "cpa/proxy-model");
    let rows = registry.provider_instances.snapshot(&registry);
    for (id, model) in [("direct", "cpa/direct-model"), ("proxy", "cpa/proxy-model")] {
        let row = rows
            .iter()
            .find(|p| p.provider_instance_id.as_ref() == id)
            .unwrap();
        assert!(row.capability().can_run_child_task);
        assert_eq!(row.capability().models[0].id, model);
        assert_eq!(
            row.authentication,
            zeron_engine::provider_instances::Authentication::Authenticated
        );
    }
    core.workspace
        .create_chat(
            "chat",
            None,
            Some(&core.device_id),
            None,
            Some(root.path().display().to_string()),
        )
        .unwrap();
    core.sessions
        .dispatch(
            "chat",
            HarnessId::Codex,
            request(root.path(), "direct", "cpa/direct-model"),
            None,
        )
        .await
        .unwrap();
    settle(&core, "chat", 0).await;
    let events = core.sessions.subscribe("chat", 0).unwrap().0;
    assert!(
        events
            .iter()
            .any(|e| matches!(&e.event, AgentEvent::TextDelta { text }
        if text.contains("provider-instances/direct/codex") && text.contains("cpa/direct-model")))
    );
    assert!(events.iter().any(|e| matches!(&e.event, AgentEvent::SessionStarted { instance_id: Some(id), .. } if id.as_ref() == "direct")));
    assert_eq!(
        core.workspace
            .chat("chat")
            .unwrap()
            .unwrap()
            .harness_session_instance_id
            .unwrap()
            .as_ref(),
        "direct"
    );
    let after = events.last().unwrap().seq;
    core.sessions
        .dispatch(
            "chat",
            HarnessId::Codex,
            request(root.path(), "proxy", "cpa/proxy-model"),
            None,
        )
        .await
        .unwrap();
    settle(&core, "chat", after).await;
    let events = core.sessions.subscribe("chat", after).unwrap().0;
    assert!(events.iter().any(|e| matches!(&e.event, AgentEvent::SessionStarted { session_id, .. } if session_id == "th-1")),
        "a foreign instance's native ID must not ride thread/resume");
    assert!(
        events
            .iter()
            .any(|e| matches!(&e.event, AgentEvent::TextDelta { text }
        if text.contains("provider-instances/proxy/codex") && text.contains("cpa/proxy-model")))
    );
    let settings = client.provider_instance_settings(None).await.unwrap();
    assert_eq!(
        settings
            .iter()
            .find(|r| r.instance_id.as_ref() == "proxy")
            .unwrap()
            .model_count,
        1
    );
    core.shutdown().await;
}

#[tokio::test]
async fn settings_rpc_reconciles_enable_delete_auth_and_validation_without_resurrection() {
    let root = tempfile::tempdir().unwrap();
    let binary = fixture();
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(CodexHarness::new().with_executable(&binary)));
    let core = EngineCore::assemble(
        &root.path().join("data"),
        registry.clone(),
        HarnessId::Codex,
        None,
    )
    .unwrap();
    let client = zeron_rpc::memory_client(core.rpc_service());
    let input: WriteProviderInstance = serde_json::from_value(json!({
        "instanceId":"signed_out","instance":{"driver":"codex","config":{"binaryPath":binary},
            "environment":[{"name":"NOCHES_TEST_INSTANCE_MODEL","value":"exact/model"},
                {"name":"NOCHES_TEST_INSTANCE_SIGNED_OUT","value":"true"},
                {"name":"OPAQUE_KEY","value":"not-for-rpc-output","sensitive":true}]}
    }))
    .unwrap();
    client.create_provider_instance(&input, None).await.unwrap();
    assert!(client.create_provider_instance(&input, None).await.is_err());
    client
        .call(methods::LIST_MODELS, json!({"instanceId":"signed_out"}))
        .await
        .unwrap();
    let settings = client.provider_instance_settings(None).await.unwrap();
    let row = settings
        .iter()
        .find(|r| r.instance_id.as_ref() == "signed_out")
        .unwrap();
    assert_eq!(row.authentication, "unauthenticated");
    assert_eq!(row.constraints, ["Provider is not authenticated."]);
    assert!(
        !serde_json::to_string(row)
            .unwrap()
            .contains("not-for-rpc-output")
    );
    assert!(
        core.sessions
            .dispatch(
                "refused",
                HarnessId::Codex,
                request(root.path(), "signed_out", "exact/model"),
                None
            )
            .await
            .is_err()
    );
    assert!(core.sessions.subscribe("refused", 0).unwrap().0.is_empty());
    client
        .set_provider_instance_enabled(
            &EnableProviderInstance {
                instance_id: "signed_out".into(),
                enabled: false,
            },
            None,
        )
        .await
        .unwrap();
    assert!(
        client
            .call(methods::LIST_MODELS, json!({"instanceId":"signed_out"}))
            .await
            .is_err()
    );
    client
        .delete_provider_instance(
            &ProviderInstanceKey {
                instance_id: "signed_out".into(),
            },
            None,
        )
        .await
        .unwrap();
    let inventory: Value = client
        .call(methods::LIST_PROVIDER_INSTANCES, json!({}))
        .await
        .unwrap();
    assert!(
        !inventory
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["providerInstanceId"] == "signed_out")
    );
    core.shutdown().await;
    registry
        .provider_instances
        .load(&registry, &root.path().join("data"))
        .unwrap();
    assert!(
        registry
            .provider_instances
            .config(&"signed_out".into())
            .is_none()
    );
}

#[tokio::test]
async fn legacy_harness_toggle_only_changes_the_canonical_instance() {
    let root = tempfile::tempdir().unwrap();
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(CodexHarness::new().with_executable(fixture())));
    registry.register(Arc::new(
        zeron_harness::ClaudeHarness::new().with_executable(fixture()),
    ));
    let core = EngineCore::assemble(
        &root.path().join("data"),
        registry.clone(),
        HarnessId::Codex,
        None,
    )
    .unwrap();
    // Startup discovery applies this host's account state last; let it finish,
    // then sign the fixture in so the toggle is the only constraint under test.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while !core
            .registry
            .provider_instances
            .snapshot(&core.registry)
            .iter()
            .any(|p| p.harness_id == Some(HarnessId::Codex) && p.model_count > 0)
        {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("startup provider discovery");
    core.registry.provider_instances.set_authentication(
        HarnessId::Codex,
        zeron_engine::provider_instances::Authentication::Authenticated,
    );
    let client = zeron_rpc::memory_client(core.rpc_service());
    let input: WriteProviderInstance = serde_json::from_value(json!({
        "instanceId":"proxy","instance":{"driver":"codex","config":{"binaryPath":fixture()}}
    }))
    .unwrap();
    client.create_provider_instance(&input, None).await.unwrap();
    client
        .call(
            methods::SET_HARNESS_ENABLED,
            json!({"harness":"codex","enabled":false}),
        )
        .await
        .unwrap();
    let rows = client.provider_instance_settings(None).await.unwrap();
    assert!(
        rows.iter()
            .find(|r| r.instance_id.as_ref() == "codex")
            .unwrap()
            .constraints
            .contains(&"Provider instance is disabled.".into())
    );
    assert!(
        rows.iter()
            .find(|r| r.instance_id.as_ref() == "proxy")
            .unwrap()
            .constraints
            .is_empty()
    );
    client
        .call(
            methods::SET_HARNESS_ENABLED,
            json!({"harness":"codex","enabled":true}),
        )
        .await
        .unwrap();
    assert!(
        client
            .provider_instance_settings(None)
            .await
            .unwrap()
            .iter()
            .find(|r| r.instance_id.as_ref() == "codex")
            .unwrap()
            .constraints
            .is_empty()
    );
    core.shutdown().await;
}

#[tokio::test]
async fn claude_accounts_probe_auth_with_instance_config_and_custom_model_overlays() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let binary = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../harness/tests/fixtures/fake-claude.sh");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(
        zeron_harness::ClaudeHarness::new().with_executable(&binary),
    ));
    let core = EngineCore::assemble(
        &root.path().join("data"),
        registry.clone(),
        HarnessId::ClaudeCode,
        None,
    )
    .unwrap();
    let client = zeron_rpc::memory_client(core.rpc_service());
    for (instance, signed_out, model) in [
        ("personal", false, "cpa/personal"),
        ("work", true, "cpa/work"),
    ] {
        let input: WriteProviderInstance = serde_json::from_value(json!({
            "instanceId":instance,"instance":{"driver":"claudeAgent",
                "config":{"binaryPath":binary,"customModels":[{"slug":model,"name":instance,
                    "capabilities":{"optionDescriptors":[{"type":"boolean","id":"thinking","label":"Thinking","currentValue":true}]}}]},
                "environment":[{"name":"NOCHES_TEST_INSTANCE_SIGNED_OUT","value":signed_out.to_string()}]}
        })).unwrap();
        client.create_provider_instance(&input, None).await.unwrap();
        let models = client
            .call(methods::LIST_MODELS, json!({"instanceId":instance}))
            .await
            .unwrap();
        assert!(models.as_array().unwrap().iter().any(|m| m["id"] == model));
        assert!(!models.as_array().unwrap().iter().any(|m| m["id"]
            == if instance == "work" {
                "cpa/personal"
            } else {
                "cpa/work"
            }));
    }
    let rows = registry.provider_instances.snapshot(&registry);
    let personal = rows
        .iter()
        .find(|p| p.provider_instance_id.as_ref() == "personal")
        .unwrap();
    let work = rows
        .iter()
        .find(|p| p.provider_instance_id.as_ref() == "work")
        .unwrap();
    assert_eq!(
        personal.authentication,
        zeron_engine::provider_instances::Authentication::Authenticated
    );
    assert_eq!(work.constraints(), ["Provider is not authenticated."]);
    assert_eq!(
        serde_json::to_value(personal.capability()).unwrap()["models"][0]["options"][0]["currentValue"],
        true
    );
    core.shutdown().await;
}
