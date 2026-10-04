//! Designer contract through the production owner RPC and publication workers.
use std::{sync::Arc, time::Duration};

use chrono::Utc;
use serde_json::{Value, json};
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_harness::mock::MockHarness;
use zeron_proto::{ChatLifecycle, HarnessId, SettleSource};
use zeron_rpc::{memory_client, methods};

async fn published(core: &EngineCore, id: &str, expected: &ChatLifecycle) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if core.workspace.thread_lifecycles().get(id) == Some(expected) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("lifecycle publication must reach the separate registry map");
}

#[tokio::test]
async fn desktop_organization_wake_ack_and_owner_authority() {
    let dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(MockHarness { script: vec![] }));
    let core = EngineCore::assemble(dir.path(), registry, HarnessId::Mock, None).unwrap();
    core.workspace
        .create_chat("local", None, Some(&core.device_id), None, None)
        .unwrap();
    let client = memory_client(core.rpc_service());
    let organize = |action: &str| json!({"chatId":"local","action":action});
    assert!(
        client
            .call(methods::ORGANIZE_THREAD, organize("snooze"))
            .await
            .unwrap_err()
            .to_string()
            .contains("snooze requires snoozedUntil.")
    );
    client
        .call(methods::ORGANIZE_THREAD, organize("pin"))
        .await
        .unwrap();
    let state: Value = client
        .call(methods::GET_QUEUE_STATE, json!({"chatId":"local"}))
        .await
        .unwrap();
    let lifecycle: ChatLifecycle = serde_json::from_value(state["lifecycle"].clone()).unwrap();
    assert!(lifecycle.pinned_at.is_some());
    published(&core, "local", &lifecycle).await;
    assert!(core.workspace.chat("local").unwrap().is_some());

    client
        .call(methods::ORGANIZE_THREAD, organize("settle"))
        .await
        .unwrap();
    let state = client
        .call(methods::GET_QUEUE_STATE, json!({"chatId":"local"}))
        .await
        .unwrap();
    let lifecycle: ChatLifecycle = serde_json::from_value(state["lifecycle"].clone()).unwrap();
    assert_eq!(lifecycle.settled_by, Some(SettleSource::User));
    assert!(lifecycle.settled_at.is_some());
    assert!(lifecycle.pinned_at.is_none());
    published(&core, "local", &lifecycle).await;
    client
        .call(methods::ORGANIZE_THREAD, organize("unsettle"))
        .await
        .unwrap();

    let wake = Utc::now() + chrono::Duration::hours(1);
    client
        .call(
            methods::ORGANIZE_THREAD,
            json!({"chatId":"local","action":"snooze","snoozedUntil":wake}),
        )
        .await
        .unwrap();
    let domain = zeron_engine::orchestration::queue::QueueDomain::new(core.orchestration.clone());
    assert_eq!(
        domain.wake_due(wake.timestamp_millis() + 1).await.unwrap(),
        1
    );
    let state = client
        .call(methods::GET_QUEUE_STATE, json!({"chatId":"local"}))
        .await
        .unwrap();
    let lifecycle: ChatLifecycle = serde_json::from_value(state["lifecycle"].clone()).unwrap();
    assert!(lifecycle.snoozed_until.is_none());
    assert!(lifecycle.woke_at.is_some());
    published(&core, "local", &lifecycle).await;
    client
        .call(methods::ACKNOWLEDGE_THREAD_WOKE, json!({"chatId":"local"}))
        .await
        .unwrap();
    let state = client
        .call(methods::GET_QUEUE_STATE, json!({"chatId":"local"}))
        .await
        .unwrap();
    let lifecycle: ChatLifecycle = serde_json::from_value(state["lifecycle"].clone()).unwrap();
    assert!(lifecycle.woke_at.is_none());
    published(&core, "local", &lifecycle).await;

    core.workspace
        .create_chat("remote", None, Some("other-owner"), None, None)
        .unwrap();
    assert!(
        client
            .call(
                methods::ORGANIZE_THREAD,
                json!({"chatId":"remote","action":"pin"}),
            )
            .await
            .is_err()
    );
    assert!(
        core.orchestration
            .store
            .thread(&"remote".into())
            .unwrap()
            .is_none(),
        "a replica must never apply a lifecycle mutation locally"
    );
}
