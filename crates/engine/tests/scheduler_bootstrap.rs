//! Production host wiring + typed owner-routed Settings RPC. Send/launch is a
//! deliberately injected merge seam here, not a second provider start path.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use zeron_engine::orchestration::scheduler::{ScheduledDispatch, ScheduledTaskDispatch};
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_proto::HarnessId;
use zeron_proto::orchestration::ScheduledTaskRunStatus;
use zeron_proto::scheduler::*;

#[derive(Default)]
struct Dispatch(Mutex<Vec<ScheduledDispatch>>);
#[async_trait]
impl ScheduledTaskDispatch for Dispatch {
    async fn dispatch(&self, run: ScheduledDispatch) -> Result<(), String> {
        self.0.lock().unwrap().push(run);
        Ok(())
    }
}

fn registry() -> Arc<HarnessRegistry> {
    let registry = Arc::new(HarnessRegistry::new());
    registry.register(Arc::new(zeron_harness::mock::MockHarness {
        script: vec![],
    }));
    registry
}

#[tokio::test]
async fn settings_crud_watch_manual_owner_offline_and_restart_use_production_host() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let core = EngineCore::assemble(&data, registry(), HarnessId::Mock, None).unwrap();
    let dispatch = Arc::new(Dispatch::default());
    core.orchestration_host
        .as_ref()
        .unwrap()
        .scheduler
        .set_dispatcher(dispatch.clone());
    let client = zeron_rpc::memory_client(core.rpc_service());
    let owner = core.device_id.clone();
    let list = ScheduledTasksListRequest {
        owner_host_id: owner.clone(),
        ..Default::default()
    };
    let mut watch = client.watch_scheduled_tasks(list.clone()).await.unwrap();
    assert!(
        watch.recv().await.unwrap()["tasks"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let input = serde_json::from_value(json!({
        "title":"Settings task","prompt":"Review","enabled":true,
        "schedule":{"type":"interval","everyMs":60000},"projectId":"project",
        "threadId":"parent","workspaceStrategy":{"type":"root"},
        "modelSelection":{"instanceId":"mock","model":"mock-1"},
        "runtimeMode":"full-access","interactionMode":"default"
    }))
    .unwrap();
    let created = client
        .create_scheduled_task(ScheduledTaskCreateRequest {
            owner_host_id: owner.clone(),
            input,
        })
        .await
        .unwrap();
    let id = created.task.task.id.0;
    assert_eq!(created.task.cadence, "Every minute");
    let updated = tokio::time::timeout(Duration::from_secs(2), watch.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated["tasks"][0]["id"], id);
    let next = created.task.task.next_run_at;
    let renamed = client
        .update_scheduled_task(ScheduledTaskUpdateRequest {
            owner_host_id: owner.clone(),
            id: id.clone(),
            title: Some("Renamed".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(renamed.task.task.next_run_at, next);
    let action = ScheduledTaskActionRequest {
        owner_host_id: owner.clone(),
        id: id.clone(),
    };
    client.run_scheduled_task_now(action.clone()).await.unwrap();
    let result = client.run_scheduled_task_now(action.clone()).await.unwrap();
    assert_eq!(
        result.task.task.last_run_status,
        ScheduledTaskRunStatus::Succeeded
    );
    assert_eq!(result.task.task.run_count, 2);
    assert_eq!(dispatch.0.lock().unwrap().len(), 2);
    let offline = ScheduledTaskActionRequest {
        owner_host_id: "offline-host".into(),
        ..action.clone()
    };
    assert!(client.run_scheduled_task_now(offline).await.is_err());
    assert_eq!(dispatch.0.lock().unwrap().len(), 2);
    assert_eq!(
        client
            .list_scheduled_tasks(list.clone())
            .await
            .unwrap()
            .tasks
            .len(),
        1
    );
    drop(watch);
    drop(client);
    core.shutdown().await;
    drop(core);
    let restarted = EngineCore::assemble(&data, registry(), HarnessId::Mock, None).unwrap();
    assert_eq!(restarted.device_id, owner);
    let client = zeron_rpc::memory_client(restarted.rpc_service());
    let persisted = client.list_scheduled_tasks(list).await.unwrap();
    assert_eq!(persisted.tasks[0].task.run_count, 2);
    assert_eq!(persisted.tasks[0].task.title, "Renamed");
    client.delete_scheduled_task(action).await.unwrap();
    let missing = client
        .update_scheduled_task(ScheduledTaskUpdateRequest {
            owner_host_id: owner,
            id,
            title: Some("Stale form".into()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(missing.to_string().contains("Schedule task not found."));
    drop(client);
    restarted.shutdown().await;
}
