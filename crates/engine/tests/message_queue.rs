//! The pending-message queue: what happens to a message typed while the agent
//! is busy, and how it eventually reaches the agent.
//!
//! The policy under test (`DocHost::drain_queue`):
//! - idle agent → the queue drains immediately, in order;
//! - busy agent that only takes input at a turn boundary → the queue HOLDS,
//!   and flushes when the turn ends;
//! - busy agent, including one that takes input mid-turn → held until turn end;
//! - "send now" → interrupts whatever is running.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;

use zeron_doc::{
    MessagePart, MessageRole, QueueDeliveryGate, SessionCommandPayload, SessionMessageEntry,
};
use zeron_engine::doc_host::{
    BeginQueueEditOutcome, FinishQueueEditAction, FinishQueueEditOutcome,
};
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_harness::{Harness, HarnessError, RunControls};
use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, Model, ReasoningLevel, RunRequest, SteeringMode,
    UserInputQuestion,
};

const CHAT: &str = "chat-queue";

async fn canonical_input(core: &EngineCore, key: &str) -> String {
    canonical_input_with_attachments(core, key, vec![]).await
}

async fn canonical_input_with_attachments(
    core: &EngineCore,
    key: &str,
    attachments: Vec<serde_json::Value>,
) -> String {
    canonical_message(
        core,
        key,
        attachments,
        zeron_proto::orchestration_mcp::T3ThreadSendInputMode::Queue,
    )
    .await
}

async fn canonical_message(
    core: &EngineCore,
    key: &str,
    attachments: Vec<serde_json::Value>,
    mode: zeron_proto::orchestration_mcp::T3ThreadSendInputMode,
) -> String {
    use zeron_engine::orchestration::thread_service::ThreadSendRequest;
    use zeron_proto::orchestration::{OrchestrationV2Actor, OrchestrationV2CreationSource};
    let thread = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    core.orchestration_host
        .as_ref()
        .unwrap()
        .threads
        .send_to_thread(ThreadSendRequest {
            project_id: thread.thread.project_id,
            thread_id: CHAT.into(),
            command_id: format!("canonical:{key}").into(),
            message_id: format!("message:{key}").into(),
            scheduled_task_id: None,
            sender_thread_id: None,
            text: key.into(),
            attachments,
            model_selection: None,
            mode,
            created_by: OrchestrationV2Actor::Agent,
            creation_source: OrchestrationV2CreationSource::Mcp,
        })
        .await
        .unwrap()
        .run_id
        .0
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn canonical_steering_waits_for_adapter_confirmation_and_recovers_only_unconfirmed_input() {
    use zeron_engine::orchestration::effects::{EffectRequest, EffectStatus};
    for response in [Some(true), Some(false), None] {
        let (core, harness, prompts) = setup(SteeringMode::StepBoundary).await;
        *harness.steering_receipt.lock().unwrap() = response;
        core.doc_host
            .queue_message(CHAT, "opening", vec![])
            .unwrap();
        // Attachment is projected at SessionStarted, before InputAccepted
        // binds the running provider turn. Wait for the actual admission
        // predicate rather than racing that later provider observation.
        wait_for(
            || {
                core.orchestration
                    .store
                    .queue_ui_state(&CHAT.into())
                    .is_ok_and(|state| state.can_promote_to_steer)
            },
            "running, steerable canonical turn",
        )
        .await;
        let original_run = core
            .orchestration
            .store
            .queue_ui_state(&CHAT.into())
            .unwrap()
            .active_run_id
            .unwrap();
        let key = format!("canonical-steer-{response:?}");
        canonical_message(
            &core,
            &key,
            vec![],
            zeron_proto::orchestration_mcp::T3ThreadSendInputMode::Steer,
        )
        .await;
        let effect = core.orchestration.store.effects().unwrap().into_iter().find(|effect| matches!(&effect.request,
            EffectRequest::ProviderTurnSteer { message_id, .. } if message_id.0 == format!("message:{key}"))).unwrap();
        wait_for(
            || {
                core.orchestration
                    .store
                    .effect(&effect.id)
                    .unwrap()
                    .is_some_and(|effect| {
                        matches!(
                            effect.status,
                            EffectStatus::Succeeded
                                | EffectStatus::Failed
                                | EffectStatus::Uncertain
                        )
                    })
            },
            "canonical steering receipt outcome",
        )
        .await;
        assert_eq!(
            core.orchestration
                .store
                .effect(&effect.id)
                .unwrap()
                .unwrap()
                .status,
            match response {
                Some(true) => EffectStatus::Succeeded,
                Some(false) => EffectStatus::Failed,
                None => EffectStatus::Uncertain,
            }
        );
        assert_eq!(
            prompts.lock().unwrap().len(),
            1,
            "canonical steering must not start a second provider"
        );
        let _ = harness.finish.send(());
        wait_for(
            || {
                !core.sessions.turn_in_flight(CHAT)
                    && core
                        .orchestration
                        .store
                        .thread(&CHAT.into())
                        .unwrap()
                        .unwrap()
                        .runs
                        .iter()
                        .any(|run| {
                            run.id.0 == original_run
                                && run.status
                                    == zeron_proto::orchestration::OrchestrationV2RunStatus::Completed
                        })
            },
            "original turn runtime and canonical completion",
        )
        .await;
        assert_eq!(
            prompts.lock().unwrap().len(),
            1,
            "failed or uncertain input cannot auto-redispatch through the legacy ledger"
        );
        core.doc_host
            .queue_message(CHAT, "explicit next turn", vec![])
            .unwrap();
        wait_for(
            || prompts.lock().unwrap().len() == 2,
            "explicit next turn after steering",
        )
        .await;
        let prompt = prompts.lock().unwrap()[1].clone();
        assert_eq!(prompt.contains(&key), response != Some(true), "{prompt}");
        let _ = harness.finish.send(());
        core.shutdown().await;
    }
}

async fn canonical_frame(
    rx: &mut zeron_rpc::RpcSubscription,
    predicate: impl Fn(&zeron_proto::QueueUiState) -> bool,
) -> zeron_proto::QueueUiState {
    let mut last = None;
    let found = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let value = rx.recv().await.expect("queue watch remains open");
            if let Ok(queue) =
                serde_json::from_value::<zeron_proto::QueueUiState>(value["canonical"].clone())
            {
                if predicate(&queue) {
                    return queue;
                }
                last = Some(queue);
            }
        }
    })
    .await;
    found.unwrap_or_else(|_| {
        panic!("canonical queue snapshot must arrive without polling; last frame: {last:?}")
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_canonical_queue_is_live_editable_idempotent_and_owner_authoritative() {
    use serde_json::json;
    use zeron_rpc::methods;
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    let mut legacy = client
        .subscribe_checked(methods::WATCH_QUEUE, json!({"chatId":CHAT}))
        .await
        .unwrap();
    let mut watch = client
        .subscribe_checked(
            methods::WATCH_QUEUE,
            json!({"chatId":CHAT,"includeCanonical":true}),
        )
        .await
        .unwrap();
    assert!(
        legacy.recv().await.unwrap().get("canonical").is_none(),
        "old watch contracts are unchanged"
    );
    core.doc_host
        .queue_message(CHAT, "opening", vec![])
        .unwrap();
    wait_for(
        || prompts.lock().unwrap().iter().any(|text| text == "opening"),
        "active run",
    )
    .await;
    let attachments = vec![
        json!({"type":"image","id":"upload","name":"diagram.png","mimeType":"image/png","sizeBytes":12}),
    ];
    let run =
        canonical_input_with_attachments(&core, "agent queued work", attachments.clone()).await;
    let state = canonical_frame(&mut watch, |queue| {
        queue.queue.iter().any(|entry| entry.queued_run_id == run)
    })
    .await;
    assert!(
        !state
            .queue
            .iter()
            .find(|entry| entry.queued_run_id == run)
            .unwrap()
            .document_backed
    );
    assert!(
        core.doc_host
            .open(CHAT)
            .unwrap()
            .doc()
            .read_queue()
            .unwrap()
            .is_empty(),
        "SQL-only work is not inserted into Loro"
    );
    let edit = json!({"chatId":CHAT,"queuedRunId":run,"clientRequestId":"edit-identity",
        "action":{"type":"edit","text":"edited agent work","expectedText":"agent queued work"}});
    let first = client
        .call(methods::MUTATE_QUEUED_RUN, edit.clone())
        .await
        .unwrap();
    assert!(first["refusal"].is_null(), "{first}");
    let sequence = core.orchestration.store.projection_frontier().unwrap();
    assert_eq!(
        client
            .call(methods::MUTATE_QUEUED_RUN, edit.clone())
            .await
            .unwrap(),
        first
    );
    assert_eq!(
        core.orchestration.store.projection_frontier().unwrap(),
        sequence,
        "response-loss retry emits no second edit"
    );
    let mut collision = edit.clone();
    collision["action"]["text"] = json!("different payload");
    assert!(
        client
            .call(methods::MUTATE_QUEUED_RUN, collision)
            .await
            .unwrap_err()
            .to_string()
            .contains("identity")
    );
    let mut stale = edit;
    stale["clientRequestId"] = json!("stale-editor");
    stale["action"]["text"] = json!("overwrite");
    let refusal = client
        .call(methods::MUTATE_QUEUED_RUN, stale)
        .await
        .unwrap();
    assert!(refusal["refusal"].as_str().unwrap().contains("changed"));
    let updated = canonical_frame(&mut watch, |queue| {
        queue
            .queue
            .iter()
            .any(|entry| entry.text == "edited agent work")
    })
    .await;
    assert_eq!(
        updated
            .queue
            .iter()
            .find(|entry| entry.queued_run_id == run)
            .unwrap()
            .attachments,
        attachments
    );
    let projection = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    let message = projection.records["message"]
        .iter()
        .find(|message| message["id"] == "message:agent queued work")
        .unwrap();
    assert_eq!(
        message["createdBy"], "agent",
        "user edits must preserve original provenance"
    );
    assert_eq!(message["creationSource"], "mcp");
    let cancel = json!({"chatId":CHAT,"queuedRunId":run,"clientRequestId":"cancel-identity","action":{"type":"cancel"}});
    let first = client
        .call(methods::MUTATE_QUEUED_RUN, cancel.clone())
        .await
        .unwrap();
    assert!(first["refusal"].is_null());
    assert_eq!(
        client
            .call(methods::MUTATE_QUEUED_RUN, cancel)
            .await
            .unwrap(),
        first
    );
    canonical_frame(&mut watch, |queue| {
        !queue.queue.iter().any(|entry| entry.queued_run_id == run)
    })
    .await;
    core.workspace
        .create_chat("replica", None, Some("foreign-owner"), None, None)
        .unwrap();
    assert!(client.call(methods::MUTATE_QUEUED_RUN,
        json!({"chatId":"replica","queuedRunId":run,"clientRequestId":"foreign","action":{"type":"cancel"}})).await.is_err());
    assert!(
        core.orchestration
            .store
            .thread(&"replica".into())
            .unwrap()
            .is_none()
    );
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_queue_promotion_steers_host_owned_images_natively() {
    use serde_json::json;
    use zeron_rpc::methods;
    let (core, harness, prompts) = setup(SteeringMode::StepBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.doc_host
        .queue_message(CHAT, "opening", vec![])
        .unwrap();
    wait_for(
        || prompts.lock().unwrap().iter().any(|text| text == "opening"),
        "active run",
    )
    .await;
    let id = core
        .doc_host
        .queue_message(
            CHAT,
            "look at these",
            vec!["/uploads/shot.png".into(), "/uploads/notes.txt".into()],
        )
        .unwrap();
    let mut watch = client
        .subscribe_checked(
            methods::WATCH_QUEUE,
            json!({"chatId":CHAT,"includeCanonical":true}),
        )
        .await
        .unwrap();
    let snapshot = canonical_frame(&mut watch, |queue| {
        queue.can_promote_to_steer && queue.queue.iter().any(|entry| entry.message_id == id)
    })
    .await;
    let queued = snapshot
        .queue
        .iter()
        .find(|entry| entry.message_id == id)
        .unwrap()
        .queued_run_id
        .clone();
    let reply = client
        .call(
            methods::MUTATE_QUEUED_RUN,
            json!({"chatId":CHAT,"queuedRunId":queued,"clientRequestId":"promote-images",
                "action":{"type":"promoteToSteer","targetRunId":snapshot.active_run_id.unwrap()}}),
        )
        .await
        .unwrap();
    assert!(reply["refusal"].is_null(), "{reply}");
    wait_for(
        || !harness.steered_images.lock().unwrap().is_empty(),
        "native steer",
    )
    .await;
    assert_eq!(
        *harness.steered_images.lock().unwrap(),
        vec![vec!["/uploads/shot.png".to_string()]],
        "only the image is inlined; every path stays a text reference"
    );
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_canonical_edit_replaces_attachments_and_cleans_dropped_files_once() {
    use serde_json::json;
    use zeron_rpc::methods;
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.doc_host
        .queue_message(CHAT, "opening", vec![])
        .unwrap();
    wait_for(
        || prompts.lock().unwrap().iter().any(|text| text == "opening"),
        "active run",
    )
    .await;
    let uploads = core.uploads.dir().to_path_buf();
    std::fs::create_dir_all(&uploads).unwrap();
    let file = |name: &str| {
        let path = uploads.join(name);
        std::fs::write(&path, b"png").unwrap();
        path.to_string_lossy().into_owned()
    };
    let (first, second) = (file("v2-first-a.png"), file("v2-second-b.png"));
    let claim = json!({"type":"image","id":"claimed","name":"agent.png","mimeType":"image/png","sizeBytes":3});
    let run = canonical_input_with_attachments(&core, "agent work", vec![claim.clone()]).await;
    let mut watch = client
        .subscribe_checked(
            methods::WATCH_QUEUE,
            json!({"chatId":CHAT,"includeCanonical":true}),
        )
        .await
        .unwrap();
    let state = canonical_frame(&mut watch, |queue| {
        queue.queue.iter().any(|entry| entry.queued_run_id == run)
    })
    .await;
    let entry = state.queue.iter().find(|e| e.queued_run_id == run).unwrap();
    let edit = |id: &str, expected: String, paths: Vec<String>, remove: Vec<&str>| {
        json!({"chatId":CHAT,"queuedRunId":run,"clientRequestId":id,
            "action":{"type":"edit","text":"agent work","expectedText":"agent work",
                "attachments":{"expected":expected,"paths":paths,"removeIds":remove}}})
    };
    // A file the host never committed is refused before anything changes.
    let refused = client
        .call(
            methods::MUTATE_QUEUED_RUN,
            edit(
                "outside",
                entry.attachment_fingerprint(),
                vec!["/etc/hosts".into()],
                vec![],
            ),
        )
        .await
        .unwrap();
    assert!(
        refused["refusal"].as_str().unwrap().contains("not found"),
        "{refused}"
    );
    // Add one upload, keep the claimed attachment.
    let add = edit(
        "add",
        entry.attachment_fingerprint(),
        vec![first.clone()],
        vec![],
    );
    let reply = client
        .call(methods::MUTATE_QUEUED_RUN, add.clone())
        .await
        .unwrap();
    assert!(reply["refusal"].is_null(), "{reply}");
    let frontier = core.orchestration.store.projection_frontier().unwrap();
    assert_eq!(
        client.call(methods::MUTATE_QUEUED_RUN, add).await.unwrap(),
        reply,
        "response-loss replay"
    );
    assert_eq!(
        core.orchestration.store.projection_frontier().unwrap(),
        frontier
    );
    let state = canonical_frame(&mut watch, |queue| {
        queue
            .queue
            .iter()
            .any(|e| e.attachment_paths == vec![first.clone()])
    })
    .await;
    let entry = state
        .queue
        .iter()
        .find(|e| e.queued_run_id == run)
        .unwrap()
        .clone();
    assert_eq!(entry.attachments, vec![claim]);
    // The first fingerprint is stale now.
    let stale = client
        .call(
            methods::MUTATE_QUEUED_RUN,
            edit(
                "stale",
                "claims=7:claimed|paths=".into(),
                vec![second.clone()],
                vec![],
            ),
        )
        .await
        .unwrap();
    assert!(
        stale["refusal"].as_str().unwrap().contains("changed"),
        "{stale}"
    );
    assert!(std::path::Path::new(&first).exists());
    // Replace the upload and drop the claimed attachment: the old file goes.
    let replace = edit(
        "replace",
        entry.attachment_fingerprint(),
        vec![second.clone()],
        vec!["claimed"],
    );
    let reply = client
        .call(methods::MUTATE_QUEUED_RUN, replace)
        .await
        .unwrap();
    assert!(reply["refusal"].is_null(), "{reply}");
    wait_for(
        || !std::path::Path::new(&first).exists(),
        "dropped upload cleanup",
    )
    .await;
    assert!(std::path::Path::new(&second).exists());
    let state = canonical_frame(&mut watch, |queue| {
        queue
            .queue
            .iter()
            .any(|e| e.attachment_paths == vec![second.clone()] && e.attachments.is_empty())
    })
    .await;
    assert_eq!(state.queue.len(), 1);
    // Cancelling the queued message releases what it still owned.
    let cancel = json!({"chatId":CHAT,"queuedRunId":run,"clientRequestId":"cancel","action":{"type":"cancel"}});
    let reply = client
        .call(methods::MUTATE_QUEUED_RUN, cancel)
        .await
        .unwrap();
    assert!(reply["refusal"].is_null(), "{reply}");
    wait_for(
        || !std::path::Path::new(&second).exists(),
        "cancelled upload cleanup",
    )
    .await;
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_mixed_queue_reorder_preserves_intents_and_edit_leases() {
    use serde_json::json;
    use zeron_rpc::methods;
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.doc_host
        .queue_message(CHAT, "opening", vec![])
        .unwrap();
    wait_for(
        || prompts.lock().unwrap().iter().any(|text| text == "opening"),
        "active run",
    )
    .await;
    let sql = canonical_input(&core, "SQL work").await;
    let a = core
        .doc_host
        .queue_message(CHAT, "typed a", vec![])
        .unwrap();
    let b = core
        .doc_host
        .queue_message(CHAT, "typed b", vec![])
        .unwrap();
    let mut watch = client
        .subscribe_checked(
            methods::WATCH_QUEUE,
            json!({"chatId":CHAT,"includeCanonical":true}),
        )
        .await
        .unwrap();
    let snapshot = canonical_frame(&mut watch, |queue| queue.queue.len() == 3).await;
    let ra = snapshot
        .queue
        .iter()
        .find(|entry| entry.message_id == a)
        .unwrap()
        .queued_run_id
        .clone();
    let rb = snapshot
        .queue
        .iter()
        .find(|entry| entry.message_id == b)
        .unwrap()
        .queued_run_id
        .clone();
    for (key, run, before) in [("b-before-sql", &rb, &sql), ("sql-before-a", &sql, &ra)] {
        let result = client
            .call(
                methods::MUTATE_QUEUED_RUN,
                json!({"chatId":CHAT,"queuedRunId":run,
            "clientRequestId":key,"action":{"type":"reorder","beforeRunId":before}}),
            )
            .await
            .unwrap();
        assert!(result["refusal"].is_null(), "{result}");
    }
    assert_eq!(
        queue_texts(&core),
        ["typed b", "typed a"],
        "SQL-only positions never corrupt Loro order"
    );
    let lease = client.call(methods::BEGIN_QUEUED_MESSAGE_EDIT,
        json!({"chatId":CHAT,"id":a,"editorDeviceId":core.device_id,"editorInstanceId":"lease-test"})).await.unwrap();
    assert_eq!(lease["outcome"], "acquired");
    assert!(
        client
            .call(
                methods::MUTATE_QUEUED_RUN,
                json!({"chatId":CHAT,"queuedRunId":ra,"clientRequestId":"bypass-lease",
        "action":{"type":"edit","text":"unsafe","expectedText":"typed a"}})
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("protected")
    );
    assert_eq!(queue_texts(&core), ["typed b", "typed a"]);
    let stale = client
        .call(
            methods::MUTATE_QUEUED_RUN,
            json!({"chatId":CHAT,"queuedRunId":sql,"clientRequestId":"wrong-active",
        "action":{"type":"promoteToSteer","targetRunId":"stale-run"}}),
        )
        .await
        .unwrap();
    assert!(stale["refusal"].is_string());
    assert_eq!(
        prompts.lock().unwrap().len(),
        1,
        "refused steering never starts or interrupts a provider"
    );
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_canonical_promotion_targets_the_running_attempt_without_restart() {
    use serde_json::json;
    use zeron_rpc::methods;
    let (core, harness, prompts) = setup(SteeringMode::StepBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.doc_host
        .queue_message(CHAT, "opening", vec![])
        .unwrap();
    wait_for(
        || prompts.lock().unwrap().iter().any(|text| text == "opening"),
        "active run",
    )
    .await;
    let queued = canonical_input(&core, "queued steering").await;
    let mut watch = client
        .subscribe_checked(
            methods::WATCH_QUEUE,
            json!({"chatId":CHAT,"includeCanonical":true}),
        )
        .await
        .unwrap();
    let snapshot = canonical_frame(&mut watch, |queue| queue.can_promote_to_steer).await;
    let active = snapshot.active_run_id.unwrap();
    let request = json!({"chatId":CHAT,"queuedRunId":queued,"clientRequestId":"promote-identity",
        "action":{"type":"promoteToSteer","targetRunId":active}});
    let reply = client
        .call(methods::MUTATE_QUEUED_RUN, request.clone())
        .await
        .unwrap();
    assert!(reply["refusal"].is_null(), "{reply}");
    assert_eq!(
        client
            .call(methods::MUTATE_QUEUED_RUN, request)
            .await
            .unwrap(),
        reply
    );
    canonical_frame(&mut watch, |queue| queue.queue.is_empty()).await;
    wait_for(
        || {
            user_messages(&core)
                .iter()
                .any(|text| text == "queued steering")
        },
        "steering effect delivery",
    )
    .await;
    assert_eq!(
        prompts.lock().unwrap().len(),
        1,
        "promotion never starts a second provider"
    );
    let projection = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    assert_eq!(
        projection
            .runs
            .iter()
            .find(|run| run.id.0 == queued)
            .unwrap()
            .status,
        zeron_proto::orchestration::OrchestrationV2RunStatus::Cancelled
    );
    assert_eq!(
        projection
            .runs
            .iter()
            .find(|run| run.id.0 == active)
            .unwrap()
            .status,
        zeron_proto::orchestration::OrchestrationV2RunStatus::Running
    );
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_restart_promotion_preserves_identity_native_history_attachments_and_other_queue_rows()
 {
    use serde_json::json;
    use zeron_proto::{QueuePromotionMode, orchestration::*};
    use zeron_rpc::methods;
    for document_backed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let profile =
            zeron_engine::profile::EngineProfile::development(dir.path(), "test-org", "test-user");
        let store_root = profile.store_root().to_path_buf();
        let (harness, prompts) = HeldHarness::new(SteeringMode::TurnBoundary);
        let registry = HarnessRegistry::new();
        registry.register(harness.clone());
        let core =
            EngineCore::assemble_with_profile(profile, Arc::new(registry), HarnessId::Mock, None)
                .unwrap();
        create_chat(&core).await;
        let client = zeron_rpc::memory_client(core.rpc_service());
        core.doc_host
            .queue_message(CHAT, "original direction", vec![])
            .unwrap();
        let original = running_canonical(&core).await;
        let active = original.runs[0].id.clone();
        let image = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            image.path(),
            include_bytes!("../../ui/assets/file-icons/files/nuxt.png"),
        )
        .unwrap();
        let path = image.path().to_string_lossy().into_owned();
        let attachments = vec![
            json!({"type":"image","id":"restart-upload","name":"diagram.png",
            "mimeType":"image/png","sizeBytes":image.as_file().metadata().unwrap().len()}),
        ];
        let (selected, message_id) = if document_backed {
            let id = core
                .doc_host
                .queue_message(CHAT, "new direction", vec![path.clone()])
                .unwrap();
            let mut watch = client
                .subscribe_checked(
                    methods::WATCH_QUEUE,
                    json!({"chatId":CHAT,"includeCanonical":true}),
                )
                .await
                .unwrap();
            let state = canonical_frame(&mut watch, |state| {
                state.queue.iter().any(|row| row.message_id == id)
            })
            .await;
            let queued = state
                .queue
                .iter()
                .find(|row| row.message_id == id)
                .unwrap()
                .queued_run_id
                .clone();
            (queued, id)
        } else {
            let fixture_store = zeron_sync::DocsStore::open(&store_root).unwrap();
            fixture_store
                .with_connection(|conn| {
                    conn.execute(
                        "INSERT INTO orchestration_launch_claims VALUES(?1,?2)",
                        rusqlite::params![
                            "restart-upload",
                            json!({"threadId":CHAT,"path":path,"attachment":attachments[0]})
                                .to_string()
                        ],
                    )?;
                    Ok::<_, rusqlite::Error>(())
                })
                .unwrap();
            (
                canonical_input_with_attachments(&core, "new direction", attachments.clone()).await,
                "message:new direction".to_string(),
            )
        };
        let other = canonical_input(&core, "later SQL work").await;
        let typed = core
            .doc_host
            .queue_message(CHAT, "later typed work", vec![])
            .unwrap();
        let mut watch = client
            .subscribe_checked(
                methods::WATCH_QUEUE,
                json!({"chatId":CHAT,"includeCanonical":true}),
            )
            .await
            .unwrap();
        let snapshot = canonical_frame(&mut watch, |state| {
            state.queue.len() == 3
                && state.promotion_mode == Some(QueuePromotionMode::InterruptRestart)
        })
        .await;
        assert!(!snapshot.can_promote_to_steer);
        // The host refuses a stale non-interrupting action before consuming input.
        let stale = client
            .call(
                methods::MUTATE_QUEUED_RUN,
                json!({"chatId":CHAT,
            "queuedRunId":selected,"clientRequestId":"stale-steer",
            "action":{"type":"promoteToSteer","targetRunId":active}}),
            )
            .await
            .unwrap();
        assert!(stale["refusal"].is_string());
        assert_eq!(harness.requests.lock().unwrap().len(), 1);
        core.sessions.mcp_server().credentials.revoke_thread(CHAT);
        let request = json!({"chatId":CHAT,"queuedRunId":selected,"clientRequestId":"restart-once",
            "action":{"type":"promoteToRestart","targetRunId":active}});
        let reply = client
            .call(methods::MUTATE_QUEUED_RUN, request.clone())
            .await
            .unwrap();
        assert!(reply["refusal"].is_null(), "{reply}");
        wait_for(
            || {
                core.orchestration
                    .store
                    .thread(&CHAT.into())
                    .is_ok_and(|p| {
                        p.is_some_and(|p| {
                            p.runs.iter().any(|r| {
                                r.id == active
                                    && r.status == OrchestrationV2RunStatus::Running
                                    && r.active_attempt_id != original.runs[0].active_attempt_id
                            })
                        })
                    })
                    && harness.requests.lock().unwrap().len() == 2
                    && user_message_id(&core, "new direction").as_deref()
                        == Some(message_id.as_str())
            },
            "replacement acceptance and original-message transcript materialization",
        )
        .await;
        assert_eq!(
            client
                .call(methods::MUTATE_QUEUED_RUN, request)
                .await
                .unwrap(),
            reply,
            "response-loss retry must not restart the replacement"
        );
        let remaining = canonical_frame(&mut watch, |state| state.queue.len() == 2).await;
        assert!(remaining.queue.iter().any(|row| row.queued_run_id == other));
        assert!(remaining.queue.iter().any(|row| row.message_id == typed));
        assert!(
            !remaining
                .queue
                .iter()
                .any(|row| row.message_id == message_id)
        );
        assert_eq!(queue_texts(&core), vec!["later typed work"]);
        assert!(
            user_message_id(&core, "later typed work").is_none(),
            "retiring the old process cannot present another queued input as sent"
        );
        let after = core
            .orchestration
            .store
            .thread(&CHAT.into())
            .unwrap()
            .unwrap();
        assert_eq!(
            after.runs.len(),
            4,
            "one existing run and three originally queued runs only"
        );
        let restarted = after.runs.iter().find(|r| r.id == active).unwrap();
        assert_eq!(restarted.user_message_id.0, message_id);
        assert_eq!(
            after
                .runs
                .iter()
                .find(|r| r.id.0 == selected)
                .unwrap()
                .status,
            OrchestrationV2RunStatus::Cancelled
        );
        assert_eq!(
            after
                .attempts
                .iter()
                .find(|a| Some(&a.id) == original.runs[0].active_attempt_id.as_ref())
                .unwrap()
                .status,
            OrchestrationV2RunAttemptStatus::Superseded
        );
        let promoted = after.records["message"]
            .iter()
            .find(|m| m["id"] == message_id)
            .unwrap();
        assert_eq!(promoted["runId"], active.0);
        if !document_backed {
            assert_eq!(promoted["attachments"], json!(attachments));
            assert_eq!(promoted["createdBy"], "agent");
            assert_eq!(promoted["creationSource"], "mcp");
        }
        let requests = harness.requests.lock().unwrap().clone();
        assert_eq!(
            requests[1].resume.as_deref(),
            Some("sess-queue"),
            "restart must resume the exact accepted predecessor conversation"
        );
        assert_eq!(requests[1].attachments, vec![path.clone()]);
        assert!(requests[1].prompt.starts_with("new direction"));
        assert!(
            requests[1].prompt.contains(&path),
            "promoted attachments must reach native input"
        );
        assert_eq!(
            user_message_id(&core, "new direction").as_deref(),
            Some(message_id.as_str())
        );
        assert_eq!(prompts.lock().unwrap().len(), 2);
        assert!(core.orchestration.store.verify_projections().unwrap());
        core.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_model_change_promotion_restarts_on_the_native_session_fences_stale_clicks_and_replays_once()
 {
    use serde_json::json;
    use zeron_proto::{QueuePromotionMode, orchestration::*};
    use zeron_rpc::methods;
    let tmp = tempfile::tempdir().unwrap();
    // A codex-driver adapter absorbs a model change on its live native session.
    let (mut harness, _) = HeldHarness::new(SteeringMode::TurnBoundary);
    Arc::get_mut(&mut harness).unwrap().id = HarnessId::Codex;
    let registry = HarnessRegistry::new();
    registry.register(harness.clone());
    let core = EngineCore::assemble(
        &tmp.path().join("data"),
        Arc::new(registry),
        HarnessId::Codex,
        None,
    )
    .unwrap();
    create_chat(&core).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.doc_host
        .queue_message(CHAT, "original direction", vec![])
        .unwrap();
    let original = running_canonical(&core).await;
    let active = original.runs[0].id.clone();
    let selected = canonical_input(&core, "new direction").await;
    canonical_input(&core, "later SQL work").await;
    let saved = original.thread.model_selection.clone();
    let mut watch = client
        .subscribe_checked(
            methods::WATCH_QUEUE,
            json!({"chatId":CHAT,"includeCanonical":true}),
        )
        .await
        .unwrap();
    let before = canonical_frame(&mut watch, |state| {
        state.queue.len() == 2 && state.promotion_mode.is_some()
    })
    .await;
    assert_eq!(before.promotion_selection, None, "nothing changed yet");
    // The thread's saved next-turn selection moves to another model the live
    // catalog advertises (exact ids are validated on the host).
    let advertised = core
        .registry
        .provider_instances
        .snapshot(&core.registry)
        .into_iter()
        .find(|p| p.provider_instance_id == saved.instance_id)
        .and_then(|p| p.models.into_iter().map(|m| m.id).find(|id| *id != *saved.model))
        .expect("the catalog advertises a second model");
    let next: zeron_proto::provider_instance::ModelSelection =
        serde_json::from_value(json!({"instanceId":saved.instance_id,"model":advertised}))
            .unwrap();
    let receipt = core
        .orchestration
        .dispatch(
            &zeron_engine::orchestration::Command::wire(
                serde_json::from_value(json!({"type":"thread.model-selection.set",
                    "commandId":"select-next-model","threadId":CHAT,"modelSelection":next}))
                .unwrap(),
            )
            .unwrap(),
            1_800_000_000_000,
        )
        .await
        .unwrap();
    assert_eq!(
        receipt.status,
        zeron_engine::orchestration::ReceiptStatus::Accepted
    );
    let hint = canonical_frame(&mut watch, |state| {
        state.promotion_mode == Some(QueuePromotionMode::InterruptRestart)
            && state.promotion_selection.is_some()
    })
    .await;
    assert_eq!(hint.promotion_selection.as_ref(), Some(&next));
    assert!(!hint.can_promote_to_steer);
    // The host refuses clicks reviewed against another mode or selection.
    for (id, action) in [
        (
            "stale-steer",
            json!({"type":"promoteToSteer","targetRunId":active,"expectedSelection":next}),
        ),
        (
            "handoff-click",
            json!({"type":"promoteToRestart","targetRunId":active,"handoff":true,
                "expectedSelection":next}),
        ),
        (
            "stale-selection",
            json!({"type":"promoteToRestart","targetRunId":active,"handoff":false,
                "expectedSelection":{"instanceId":saved.instance_id,"model":"not-the-reviewed-model"}}),
        ),
        (
            "unreviewed-selection",
            json!({"type":"promoteToRestart","targetRunId":active,"handoff":false}),
        ),
    ] {
        let reply = client
            .call(
                methods::MUTATE_QUEUED_RUN,
                json!({"chatId":CHAT,"queuedRunId":selected,"clientRequestId":id,"action":action}),
            )
            .await
            .unwrap();
        assert!(reply["refusal"].is_string(), "{id}: {reply}");
    }
    assert_eq!(harness.requests.lock().unwrap().len(), 1);
    core.sessions.mcp_server().credentials.revoke_thread(CHAT);
    let request = json!({"chatId":CHAT,"queuedRunId":selected,"clientRequestId":"model-once",
        "action":{"type":"promoteToRestart","targetRunId":active,"handoff":false,"expectedSelection":next}});
    let reply = client
        .call(methods::MUTATE_QUEUED_RUN, request.clone())
        .await
        .unwrap();
    assert!(reply["refusal"].is_null(), "{reply}");
    wait_for(
        || {
            core.orchestration
                .store
                .thread(&CHAT.into())
                .is_ok_and(|p| {
                    p.is_some_and(|p| {
                        p.runs.iter().any(|r| {
                            r.id == active
                                && r.status == OrchestrationV2RunStatus::Running
                                && r.active_attempt_id != original.runs[0].active_attempt_id
                        })
                    })
                })
                && harness.requests.lock().unwrap().len() == 2
        },
        "replacement acceptance on the new model",
    )
    .await;
    assert_eq!(
        client
            .call(methods::MUTATE_QUEUED_RUN, request)
            .await
            .unwrap(),
        reply,
        "response-loss retry must not restart the replacement"
    );
    let after = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    let run = after.runs.iter().find(|r| r.id == active).unwrap();
    assert_eq!(run.model_selection, next);
    assert_eq!(
        run.provider_thread_id, original.runs[0].provider_thread_id,
        "an absorbed model change keeps the native generation"
    );
    assert_eq!(
        after
            .runs
            .iter()
            .find(|r| r.user_message_id.0 == "message:later SQL work")
            .unwrap()
            .status,
        OrchestrationV2RunStatus::Queued,
        "other queued rows are untouched"
    );
    let requests = harness.requests.lock().unwrap().clone();
    assert_eq!(requests[1].model.as_deref(), Some(&*next.model));
    assert_eq!(
        requests[1].resume.as_deref(),
        Some("sess-queue"),
        "the accepted predecessor conversation continues natively"
    );
    assert!(requests[1].prompt.starts_with("new direction"));
    assert!(
        core.orchestration
            .store
            .thread_transfers(&CHAT.into())
            .unwrap()
            .iter()
            .all(|t| t["type"] != "provider_handoff"),
        "an absorbed selection change needs no handoff"
    );
    assert!(core.orchestration.store.verify_projections().unwrap());
    core.shutdown().await;
}

struct ReleaseCount(Arc<std::sync::atomic::AtomicUsize>);

impl Drop for ReleaseCount {
    fn drop(&mut self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// A turn that does not end until the test says so, so "the agent is busy" is
/// a state the test controls rather than races.
struct HeldHarness {
    id: HarnessId,
    steering: SteeringMode,
    finish: tokio::sync::broadcast::Sender<()>,
    prompts: Arc<Mutex<Vec<String>>>,
    requests: Mutex<Vec<RunRequest>>,
    /// Park the first turn on a question instead of just hanging, so the chat
    /// sits in `AwaitingInput` rather than `Working`.
    asks: bool,
    park_after_done: bool,
    native_background: bool,
    /// Native session id this adapter reports (distinct per harness in the
    /// two-instance fixtures).
    session: String,
    /// Assistant text streamed before the turn parks, so a restart has partial
    /// same-run history to hand off.
    partial_text: Option<String>,
    /// Provider turn streams dropped so far: an interrupted or finished
    /// process releases its stream exactly once.
    released: Arc<std::sync::atomic::AtomicUsize>,
    steering_receipt: Arc<Mutex<Option<bool>>>,
    /// Keep each steer's ACK sender alive and unanswered (a hung adapter),
    /// instead of dropping it, which reads as an immediate uncertain result.
    held_receipts: Arc<Mutex<Option<Vec<tokio::sync::oneshot::Sender<bool>>>>>,
    /// Image files that rode each steer into the live turn.
    steered_images: Arc<Mutex<Vec<Vec<String>>>>,
}

impl HeldHarness {
    fn new(steering: SteeringMode) -> (Arc<Self>, Arc<Mutex<Vec<String>>>) {
        Self::build(steering, false)
    }

    fn asking(steering: SteeringMode) -> (Arc<Self>, Arc<Mutex<Vec<String>>>) {
        Self::build(steering, true)
    }

    fn build(steering: SteeringMode, asks: bool) -> (Arc<Self>, Arc<Mutex<Vec<String>>>) {
        let (finish, _) = tokio::sync::broadcast::channel(16);
        let prompts = Arc::new(Mutex::new(Vec::new()));
        (
            Arc::new(Self {
                id: HarnessId::Mock,
                steering,
                finish,
                prompts: prompts.clone(),
                requests: Mutex::new(Vec::new()),
                asks,
                park_after_done: false,
                native_background: false,
                session: "sess-queue".into(),
                partial_text: None,
                released: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                steering_receipt: Arc::new(Mutex::new(Some(true))),
                held_receipts: Arc::new(Mutex::new(None)),
                steered_images: Arc::new(Mutex::new(Vec::new())),
            }),
            prompts,
        )
    }
}

#[async_trait]
impl Harness for HeldHarness {
    fn id(&self) -> HarnessId {
        self.id
    }
    fn display_name(&self) -> &str {
        "Held"
    }
    fn supports_steering(&self) -> bool {
        true
    }
    fn steering_mode(&self) -> SteeringMode {
        self.steering
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[ReasoningLevel::Medium]
    }
    async fn models(&self) -> Result<Vec<Model>, HarnessError> {
        Ok(vec![])
    }
    async fn run(
        &self,
        request: RunRequest,
        controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        // Publishing the prompt is the fixture's readiness signal. Install the
        // finish receiver first so the test cannot send into an empty channel.
        let mut finish = self.finish.subscribe();
        self.prompts.lock().unwrap().push(request.prompt.clone());
        self.requests.lock().unwrap().push(request.clone());
        if self.asks {
            // Only the engine can mint a request id it will honour, so the
            // question has to go through controls rather than the stream.
            let _answer = (controls.request_input)(vec![UserInputQuestion {
                id: "q1".into(),
                header: "Choose".into(),
                question: "which one?".into(),
                options: vec!["a".into(), "b".into()],
                multi_select: false,
            }]);
        }
        let mut steering = controls.steering;
        let steering_receipt = self.steering_receipt.clone();
        let held_receipts = self.held_receipts.clone();
        let session = self.session.clone();
        let steered_images = self.steered_images.clone();
        let mut opening = vec![
            Ok(AgentEvent::SessionStarted {
                instance_id: None,
                harness: self.id,
                model: "mock-1".into(),
                tools: vec![],
                cwd: request.cwd.clone(),
                session_id: session.clone(),
                assistant_message_id: format!("a-{}", request.prompt),
            }),
            Ok(AgentEvent::InputAccepted),
        ];
        if let Some(text) = &self.partial_text {
            opening.push(Ok(AgentEvent::TextDelta { text: text.clone() }));
        }
        let released = ReleaseCount(self.released.clone());
        if self.native_background {
            opening.push(Ok(AgentEvent::Subagent {
                parent_tool_use_id: "background-child".into(),
                event: Box::new(AgentEvent::TextDelta {
                    text: "Working in background".into(),
                }),
            }));
        }
        let started = futures::stream::iter(opening);
        let result = self
            .native_background
            .then(|| "Finished main reply".to_owned());
        let done = futures::stream::once(async move {
            let _released = released;
            loop {
                tokio::select! {
                    _ = finish.recv() => {
                        return Ok(AgentEvent::Done {
                            status: DoneStatus::Completed,
                            result: result.clone(),
                            error: None,
                            session_id: Some(session.clone()),
                        });
                    }
                    steer = steering.recv() => {
                        if let Some(mut steer) = steer {
                            steered_images.lock().unwrap().push(steer.attachments.clone());
                            if let Some(receipt) = steer.notification_acceptance.take() {
                                if let Some(held) = held_receipts.lock().unwrap().as_mut() {
                                    held.push(receipt);
                                } else if let Some(accepted) = *steering_receipt.lock().unwrap() {
                                    let _ = receipt.send(accepted);
                                }
                            }
                        } else {
                            return Ok(AgentEvent::Done {
                                status: DoneStatus::Completed,
                                result: result.clone(),
                                error: None,
                                session_id: Some(session.clone()),
                            });
                        }
                    }
                }
            }
        });
        if self.park_after_done {
            Ok(started
                .chain(done)
                .chain(futures::stream::pending())
                .boxed())
        } else {
            Ok(started.chain(done).boxed())
        }
    }
}

async fn wait_for<F>(mut predicate: F, what: &str)
where
    F: FnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !predicate() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for {what}"
        );
        tokio::time::sleep(Duration::from_millis(15)).await;
    }
}

/// Occurrence-counted write injection needs a quiet store: the first turn's
/// settlement commits can otherwise land after injection on a slow runner.
async fn settled_store(core: &EngineCore) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut last = core.orchestration.store.projection_frontier().unwrap();
    let mut quiet = 0;
    while quiet < 10 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for a quiet orchestration store"
        );
        tokio::time::sleep(Duration::from_millis(30)).await;
        let frontier = core.orchestration.store.projection_frontier().unwrap();
        quiet = if frontier == last { quiet + 1 } else { 0 };
        last = frontier;
    }
}

fn queue_texts(core: &EngineCore) -> Vec<String> {
    core.doc_host
        .open(CHAT)
        .ok()
        .and_then(|h| h.doc().read_queue().ok())
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.text)
        .collect()
}

fn user_messages(core: &EngineCore) -> Vec<String> {
    let entries: Vec<SessionMessageEntry> = core
        .doc_host
        .open(CHAT)
        .ok()
        .and_then(|h| h.doc().read_entries().ok())
        .unwrap_or_default();
    entries
        .iter()
        .filter(|e| e.role == MessageRole::User)
        .map(|e| {
            e.parts
                .iter()
                .filter_map(|p| match p {
                    MessagePart::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .collect()
}

fn user_message_id(core: &EngineCore, text: &str) -> Option<String> {
    core.doc_host
        .open(CHAT)
        .ok()?
        .doc()
        .read_entries()
        .ok()?
        .into_iter()
        .find(|entry| {
            entry.role == MessageRole::User
                && entry.parts.iter().any(
                    |part| matches!(part, MessagePart::Text { text: body, .. } if body == text),
                )
        })
        .map(|entry| entry.id)
}

async fn setup(steering: SteeringMode) -> (EngineCore, Arc<HeldHarness>, Arc<Mutex<Vec<String>>>) {
    setup_with(HeldHarness::new(steering)).await
}

/// [`setup`] with a harness whose turn parks on a question.
async fn setup_asking(
    steering: SteeringMode,
) -> (EngineCore, Arc<HeldHarness>, Arc<Mutex<Vec<String>>>) {
    setup_with(HeldHarness::asking(steering)).await
}

async fn setup_with(
    built: (Arc<HeldHarness>, Arc<Mutex<Vec<String>>>),
) -> (EngineCore, Arc<HeldHarness>, Arc<Mutex<Vec<String>>>) {
    let tmp = tempfile::tempdir().unwrap();
    // Leak the tempdir guard: the engine outlives this helper and the test only
    // cares that the path is unique per run.
    let path = tmp.keep();
    let (harness, prompts) = built;
    let core = assemble_at(&path.join("data"), harness.clone());
    create_chat(&core).await;
    (core, harness, prompts)
}

fn assemble_at(path: &std::path::Path, harness: Arc<HeldHarness>) -> EngineCore {
    let registry = HarnessRegistry::new();
    registry.register(harness);
    EngineCore::assemble(path, Arc::new(registry), HarnessId::Mock, None)
        .expect("engine core assembles")
}

async fn create_chat(core: &EngineCore) {
    let client = zeron_rpc::memory_client(core.rpc_service());
    client
        .call(
            zeron_rpc::methods::MUTATE,
            serde_json::json!({
                "op": "createChat",
                "chatId": CHAT,
                "deviceId": core.device_id,
            }),
        )
        .await
        .expect("createChat");
    // Pre-title so the auto-titler never dispatches a harness request of its own.
    core.workspace
        .rename_chat(CHAT, "Pre-titled")
        .expect("rename chat");
}

async fn attached_sessions(core: &EngineCore) -> Vec<zeron_proto::transfer::ProviderSessionRef> {
    wait_for(
        || {
            core.orchestration
                .store
                .transfer_ui_state(&CHAT.into())
                .is_ok_and(|state| !state.attached_provider_sessions.is_empty())
        },
        "accepted session attachment",
    )
    .await;
    core.orchestration
        .store
        .transfer_ui_state(&CHAT.into())
        .unwrap()
        .attached_provider_sessions
}

async fn running_canonical(
    core: &EngineCore,
) -> zeron_engine::orchestration::projection::ThreadProjection {
    wait_for(
        || {
            core.orchestration
                .store
                .thread(&CHAT.into())
                .is_ok_and(|p| {
                    p.is_some_and(|p| p.attempts.iter().any(|a| a.provider_turn_id.is_some()))
                })
        },
        "canonical provider turn",
    )
    .await;
    core.orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn canonical_stop_releases_ingestion_lane_and_terminalizes_the_exact_run() {
    use zeron_engine::orchestration::threads::planner::ThreadOperation;
    use zeron_engine::orchestration::{Command, Operation, ReceiptStatus};
    use zeron_proto::orchestration::OrchestrationV2RunStatus;
    let (core, _, _) = setup(SteeringMode::TurnBoundary).await;
    core.doc_host
        .queue_message(CHAT, "stop this", vec![])
        .unwrap();
    let p = running_canonical(&core).await;
    let run = p.runs[0].clone();
    let result = core
        .orchestration
        .dispatch(
            &Command {
                id: "exact-stop".into(),
                thread_id: CHAT.into(),
                operation: Operation::Thread(Box::new(ThreadOperation::Interrupt {
                    run_id: run.id.clone(),
                    reason: None,
                })),
            },
            chrono::Utc::now().timestamp_millis(),
        )
        .await
        .unwrap();
    assert_eq!(result.status, ReceiptStatus::Accepted);
    wait_for(
        || {
            core.orchestration
                .store
                .thread(&CHAT.into())
                .unwrap()
                .unwrap()
                .runs
                .iter()
                .any(|r| r.id == run.id && r.status == OrchestrationV2RunStatus::Interrupted)
        },
        "canonical stop ingestion",
    )
    .await;
    assert!(!core.sessions.has_live_runtime(CHAT));
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unanswered_steer_ack_does_not_hold_the_thread_lane_against_stop() {
    use zeron_engine::orchestration::effects::{EffectRequest, EffectStatus};
    use zeron_engine::orchestration::threads::planner::ThreadOperation;
    use zeron_engine::orchestration::{Command, Operation, ReceiptStatus};
    let (core, harness, _) = setup(SteeringMode::StepBoundary).await;
    *harness.held_receipts.lock().unwrap() = Some(vec![]);
    core.doc_host
        .queue_message(CHAT, "opening", vec![])
        .unwrap();
    wait_for(
        || {
            core.orchestration
                .store
                .queue_ui_state(&CHAT.into())
                .is_ok_and(|state| state.can_promote_to_steer)
        },
        "running, steerable canonical turn",
    )
    .await;
    let run_id = core
        .orchestration
        .store
        .queue_ui_state(&CHAT.into())
        .unwrap()
        .active_run_id
        .unwrap();
    canonical_message(
        &core,
        "unanswered-steer",
        vec![],
        zeron_proto::orchestration_mcp::T3ThreadSendInputMode::Steer,
    )
    .await;
    let effect = core.orchestration.store.effects().unwrap().into_iter().find(|effect| matches!(&effect.request,
        EffectRequest::ProviderTurnSteer { message_id, .. } if message_id.0 == "message:unanswered-steer")).unwrap();
    wait_for(
        || {
            core.orchestration
                .store
                .effect(&effect.id)
                .unwrap()
                .is_some_and(|effect| effect.status == EffectStatus::Running)
        },
        "steer effect waiting on its adapter ACK",
    )
    .await;
    wait_for(
        || harness.held_receipts.lock().unwrap().as_ref().is_some_and(|held| !held.is_empty()),
        "adapter holding the steer ACK",
    )
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    // The steer ACK wait is 10s; Stop must not queue behind it.
    let stop = tokio::time::timeout(
        std::time::Duration::from_secs(4),
        core.orchestration.dispatch(
            &Command {
                id: "stop-during-steer".into(),
                thread_id: CHAT.into(),
                operation: Operation::Thread(Box::new(ThreadOperation::Interrupt {
                    run_id: run_id.into(),
                    reason: None,
                })),
            },
            chrono::Utc::now().timestamp_millis(),
        ),
    )
    .await
    .expect("Stop waited behind the steer ACK")
    .unwrap();
    assert_eq!(stop.status, ReceiptStatus::Accepted);
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delayed_stop_with_stale_sql_cannot_cancel_an_active_or_idle_replacement() {
    use zeron_engine::orchestration::effects::{EffectExecutor, EffectOutcome, EffectRequest};
    use zeron_engine::orchestration::threads::planner::ThreadOperation;
    use zeron_engine::orchestration::{Command, Operation, ReceiptStatus};
    for idle in [false, true] {
        let (mut harness, _) = HeldHarness::new(SteeringMode::TurnBoundary);
        Arc::get_mut(&mut harness).unwrap().park_after_done = true;
        let dir = tempfile::tempdir().unwrap();
        let profile =
            zeron_engine::profile::EngineProfile::development(dir.path(), "test-org", "test-user");
        let store_root = profile.store_root().to_path_buf();
        let registry = HarnessRegistry::new();
        registry.register(harness.clone());
        let core =
            EngineCore::assemble_with_profile(profile, Arc::new(registry), HarnessId::Mock, None)
                .unwrap();
        create_chat(&core).await;
        core.doc_host
            .queue_message(CHAT, "original", vec![])
            .unwrap();
        let original = running_canonical(&core).await;
        core.orchestration_host.as_ref().unwrap().shutdown().await;
        let receipt = core
            .orchestration
            .dispatch(
                &Command {
                    id: "delayed-stop".into(),
                    thread_id: CHAT.into(),
                    operation: Operation::Thread(Box::new(ThreadOperation::Interrupt {
                        run_id: original.runs[0].id.clone(),
                        reason: None,
                    })),
                },
                chrono::Utc::now().timestamp_millis(),
            )
            .await
            .unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Accepted);
        let effect = core
            .orchestration
            .store
            .effects()
            .unwrap()
            .into_iter()
            .find(|e| matches!(e.request, EffectRequest::ProviderTurnInterrupt { .. }))
            .unwrap();
        core.sessions.interrupt(CHAT).await.unwrap();
        let mut next = harness.requests.lock().unwrap()[0].clone();
        next.prompt = "replacement".into();
        next.resume = None;
        core.sessions
            .dispatch(
                CHAT,
                HarnessId::Mock,
                next,
                Some("replacement-input".into()),
            )
            .await
            .unwrap();
        wait_for(
            || harness.requests.lock().unwrap().len() == 2,
            "replacement started",
        )
        .await;
        if idle {
            let _ = harness.finish.send(());
            wait_for(
                || {
                    core.sessions
                        .session_status(CHAT)
                        .is_some_and(|s| s.status == zeron_proto::SessionStatus::Idle)
                },
                "replacement idle",
            )
            .await;
        }
        // Simulate delayed canonical observation/reconstruction while the
        // runtime map already contains a replacement. Logical SQL alone must
        // not authorize cancelling whichever process now occupies this chat.
        core.orchestration.store.rebuild().unwrap();
        let diagnostic_store = zeron_sync::DocsStore::open(&store_root).unwrap();
        for (table, payload) in [
            (
                "orchestration_projection_runs",
                serde_json::to_value(&original.runs[0]).unwrap(),
            ),
            (
                "orchestration_projection_attempts",
                serde_json::to_value(&original.attempts[0]).unwrap(),
            ),
            (
                "orchestration_projection_nodes",
                serde_json::to_value(&original.nodes[0]).unwrap(),
            ),
            (
                "orchestration_projection_records",
                original.records["provider-turn"][0].clone(),
            ),
        ] {
            diagnostic_store.with_connection(|conn| {
                assert_eq!(conn.execute(
                    &format!("UPDATE {table} SET status=?1,payload_json=?2 WHERE id=?3 AND thread_id=?4"),
                    rusqlite::params![payload["status"].as_str(), payload.to_string(),
                        payload["id"].as_str(), CHAT],
                ).unwrap(), 1);
            });
        }
        let bridge = core.orchestration_host.as_ref().unwrap().bridge.clone();
        assert_eq!(
            bridge
                .execute(&effect, tokio_util::sync::CancellationToken::new())
                .await,
            EffectOutcome::Succeeded
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            core.sessions.has_live_runtime(CHAT),
            "stale Stop must preserve replacement (idle={idle})"
        );
        assert_eq!(harness.requests.lock().unwrap().len(), 2);
        core.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_admitted_without_a_frozen_target_fails_but_a_legacy_row_still_succeeds() {
    use zeron_engine::orchestration::effects::{EffectExecutor, EffectOutcome, EffectRequest};
    use zeron_engine::orchestration::threads::planner::ThreadOperation;
    use zeron_engine::orchestration::{Command, Operation, ReceiptStatus};
    for legacy in [false, true] {
        let (harness, _) = HeldHarness::new(SteeringMode::TurnBoundary);
        let dir = tempfile::tempdir().unwrap();
        let profile =
            zeron_engine::profile::EngineProfile::development(dir.path(), "test-org", "test-user");
        let store_root = profile.store_root().to_path_buf();
        let registry = HarnessRegistry::new();
        registry.register(harness.clone());
        let core =
            EngineCore::assemble_with_profile(profile, Arc::new(registry), HarnessId::Mock, None)
                .unwrap();
        create_chat(&core).await;
        core.doc_host
            .queue_message(CHAT, "original", vec![])
            .unwrap();
        let original = running_canonical(&core).await;
        core.orchestration_host.as_ref().unwrap().shutdown().await;
        let receipt = core
            .orchestration
            .dispatch(
                &Command {
                    id: "untargeted-stop".into(),
                    thread_id: CHAT.into(),
                    operation: Operation::Thread(Box::new(ThreadOperation::Interrupt {
                        run_id: original.runs[0].id.clone(),
                        reason: None,
                    })),
                },
                chrono::Utc::now().timestamp_millis(),
            )
            .await
            .unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Accepted);
        let effect = core
            .orchestration
            .store
            .effects()
            .unwrap()
            .into_iter()
            .find(|e| matches!(e.request, EffectRequest::ProviderTurnInterrupt { .. }))
            .unwrap();
        let diagnostic_store = zeron_sync::DocsStore::open(&store_root).unwrap();
        diagnostic_store.with_connection(|conn| {
            let sql = if legacy {
                "DELETE FROM orchestration_control_targets WHERE effect_id=?1"
            } else {
                "UPDATE orchestration_control_targets SET target_json=NULL WHERE effect_id=?1"
            };
            assert_eq!(conn.execute(sql, [&effect.id]).unwrap(), 1);
        });
        let bridge = core.orchestration_host.as_ref().unwrap().bridge.clone();
        assert_eq!(
            bridge
                .execute(&effect, tokio_util::sync::CancellationToken::new())
                .await,
            if legacy {
                EffectOutcome::Succeeded
            } else {
                EffectOutcome::Failed
            },
            "legacy={legacy}"
        );
        core.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn canonical_restart_replaces_only_the_interrupted_attempt_and_keeps_one_run() {
    use zeron_proto::orchestration::{OrchestrationV2RunAttemptStatus, OrchestrationV2RunStatus};
    let (core, harness, _) = setup(SteeringMode::TurnBoundary).await;
    core.doc_host
        .queue_message(CHAT, "original direction", vec![])
        .unwrap();
    let original = running_canonical(&core).await;
    let run = canonical_message(
        &core,
        "new direction",
        vec![],
        zeron_proto::orchestration_mcp::T3ThreadSendInputMode::Restart,
    )
    .await;
    assert_eq!(run, original.runs[0].id.0);
    wait_for(
        || harness.requests.lock().unwrap().len() == 2,
        "replacement attempt start",
    )
    .await;
    wait_for(
        || {
            core.orchestration
                .store
                .thread(&CHAT.into())
                .unwrap()
                .unwrap()
                .runs
                .iter()
                .any(|r| r.id.0 == run && r.status == OrchestrationV2RunStatus::Running)
        },
        "replacement canonical running",
    )
    .await;
    let p = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    assert_eq!(p.runs.len(), 1);
    assert_ne!(
        p.runs[0].active_attempt_id,
        original.runs[0].active_attempt_id
    );
    assert_eq!(
        p.attempts
            .iter()
            .find(|a| Some(&a.id) == original.runs[0].active_attempt_id.as_ref())
            .unwrap()
            .status,
        OrchestrationV2RunAttemptStatus::Superseded
    );
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_disconnect_stops_the_observed_session_preserves_history_and_cannot_retarget_a_retry()
{
    use zeron_proto::transfer::DisconnectThreadSessionParams;
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.doc_host
        .queue_message(CHAT, "opening", vec![])
        .unwrap();
    let sessions = attached_sessions(&core).await;
    assert_eq!(sessions.len(), 1);
    let request = DisconnectThreadSessionParams {
        chat_id: CHAT.into(),
        client_request_id: "disconnect-once".into(),
        provider_sessions: sessions.clone(),
    };
    let mut stale_revision = request.clone();
    stale_revision.client_request_id = "stale-attachment-revision".into();
    stale_revision.provider_sessions[0].attachment_sequence += 1;
    assert!(
        client
            .disconnect_thread_session(stale_revision, &core.device_id)
            .await
            .unwrap()
            .refusal
            .as_deref()
            .unwrap()
            .contains("changed")
    );
    assert!(core.sessions.has_live_runtime(CHAT));
    let first = client
        .disconnect_thread_session(request.clone(), &core.device_id)
        .await
        .unwrap();
    assert!(first.refusal.is_none(), "{:?}", first.refusal);
    wait_for(
        || !core.sessions.has_live_runtime(CHAT),
        "disconnected runtime teardown",
    )
    .await;
    assert_eq!(
        prompts.lock().unwrap().len(),
        1,
        "disconnect must not start another provider"
    );
    assert!(
        core.orchestration
            .store
            .transfer_ui_state(&CHAT.into())
            .unwrap()
            .attached_provider_sessions
            .is_empty()
    );
    let p = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    assert!(
        p.records["provider-thread"]
            .iter()
            .any(|provider| provider["nativeThreadRef"]["nativeId"] == "sess-queue"),
        "disconnect must keep the accepted native conversation"
    );

    core.doc_host
        .queue_message(CHAT, "continue after disconnect", vec![])
        .unwrap();
    let replacement = attached_sessions(&core).await;
    wait_for(|| prompts.lock().unwrap().len() == 2, "replacement input").await;
    assert_eq!(
        harness.requests.lock().unwrap()[1].resume.as_deref(),
        Some("sess-queue")
    );
    assert_ne!(sessions, replacement);
    let replay = client
        .disconnect_thread_session(request.clone(), &core.device_id)
        .await
        .unwrap();
    assert_eq!(first.sequence, replay.sequence);
    assert!(replay.refusal.is_none());
    assert!(
        core.sessions.has_live_runtime(CHAT),
        "replay cannot disconnect a replacement"
    );
    let mut stale = request.clone();
    stale.client_request_id = "stale-panel".into();
    assert!(
        client
            .disconnect_thread_session(stale, &core.device_id)
            .await
            .unwrap()
            .refusal
            .as_deref()
            .unwrap()
            .contains("changed")
    );
    let mut collision = request;
    collision.provider_sessions = replacement;
    assert!(
        client
            .disconnect_thread_session(collision, &core.device_id)
            .await
            .expect_err("a request identity cannot target different sessions")
            .to_string()
            .contains("different sessions")
    );
    core.workspace
        .create_chat("replica", None, Some("foreign-owner"), None, None)
        .unwrap();
    assert!(
        client
            .disconnect_thread_session(
                DisconnectThreadSessionParams {
                    chat_id: "replica".into(),
                    client_request_id: "foreign".into(),
                    provider_sessions: sessions,
                },
                &core.device_id
            )
            .await
            .is_err()
    );
    assert!(core.sessions.has_live_runtime(CHAT));
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_disconnect_does_not_require_a_live_agent_credential() {
    use zeron_proto::transfer::DisconnectThreadSessionParams;
    let (core, harness, _) = setup(SteeringMode::TurnBoundary).await;
    core.doc_host
        .queue_message(CHAT, "long-running session", vec![])
        .unwrap();
    let provider_sessions = attached_sessions(&core).await;
    // Credential liveness and the user's authority to stop their runtime are
    // different boundaries. Never fabricate an agent scope for this action.
    core.sessions.mcp_server().credentials.revoke_thread(CHAT);
    let client = zeron_rpc::memory_client(core.rpc_service());
    let reply = client
        .disconnect_thread_session(
            DisconnectThreadSessionParams {
                chat_id: CHAT.into(),
                client_request_id: "user-not-agent".into(),
                provider_sessions,
            },
            &core.device_id,
        )
        .await
        .unwrap();
    assert!(reply.refusal.is_none(), "{:?}", reply.refusal);
    wait_for(
        || !core.sessions.has_live_runtime(CHAT),
        "user-authorized teardown",
    )
    .await;
    assert_eq!(harness.requests.lock().unwrap().len(), 1);
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delayed_disconnect_survives_response_loss_rebuild_and_does_not_stop_an_idle_replacement() {
    use zeron_engine::orchestration::{
        WriteBoundary,
        effects::{EffectExecutor, EffectRequest},
    };
    use zeron_proto::transfer::DisconnectThreadSessionParams;
    let (mut harness, prompts) = HeldHarness::new(SteeringMode::TurnBoundary);
    Arc::get_mut(&mut harness).unwrap().park_after_done = true;
    let (core, harness, _) = setup_with((harness, prompts)).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.doc_host
        .queue_message(CHAT, "first runtime", vec![])
        .unwrap();
    let sessions = attached_sessions(&core).await;
    core.orchestration_host.as_ref().unwrap().shutdown().await;
    settled_store(&core).await;
    let request = DisconnectThreadSessionParams {
        chat_id: CHAT.into(),
        client_request_id: "after-commit-loss".into(),
        provider_sessions: sessions,
    };
    // First commit reserves the payload; second accepts detach plus its outbox.
    core.orchestration
        .store
        .inject_failure(WriteBoundary::AfterCommit, 2);
    assert!(
        client
            .disconnect_thread_session(request.clone(), &core.device_id)
            .await
            .is_err()
    );
    let accepted = client
        .disconnect_thread_session(request.clone(), &core.device_id)
        .await
        .unwrap();
    assert!(accepted.refusal.is_none());
    core.orchestration.store.rebuild().unwrap();
    let replay = client
        .disconnect_thread_session(request, &core.device_id)
        .await
        .unwrap();
    assert_eq!(accepted.sequence, replay.sequence);
    let effect = core
        .orchestration
        .store
        .effects()
        .unwrap()
        .into_iter()
        .find(|effect| {
            matches!(
                effect.request,
                EffectRequest::ProviderSessionDisconnect { .. }
            )
        })
        .unwrap();

    core.sessions.interrupt(CHAT).await.unwrap();
    let mut next = harness.requests.lock().unwrap()[0].clone();
    next.prompt = "replacement runtime".into();
    next.resume = None;
    core.sessions
        .dispatch(
            CHAT,
            HarnessId::Mock,
            next,
            Some("replacement-input".into()),
        )
        .await
        .unwrap();
    attached_sessions(&core).await;
    let _ = harness.finish.send(());
    wait_for(
        || {
            core.sessions
                .session_status(CHAT)
                .is_some_and(|status| status.status == zeron_proto::SessionStatus::Idle)
        },
        "idle replacement",
    )
    .await;
    assert!(
        core.sessions.has_live_runtime(CHAT),
        "fixture must retain a parked provider process"
    );
    let bridge = core.orchestration_host.as_ref().unwrap().bridge.clone();
    assert_eq!(
        bridge
            .execute(&effect, tokio_util::sync::CancellationToken::new())
            .await,
        zeron_engine::orchestration::effects::EffectOutcome::Succeeded
    );
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        core.sessions.has_live_runtime(CHAT),
        "a delayed disconnect must not cancel an idle replacement"
    );
    assert_eq!(harness.requests.lock().unwrap().len(), 2);
    core.shutdown().await;
}

/// Nothing is running, so a queued message is just a message: it goes out at
/// once, and the queue is empty again.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queue_drains_immediately_when_the_agent_is_idle() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;

    let queued_id = core
        .doc_host
        .queue_message(CHAT, "first", Vec::new())
        .expect("queue message");

    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "first"),
        "the queued message to dispatch",
    )
    .await;
    wait_for(
        || user_message_id(&core, "first").as_deref() == Some(queued_id.as_str()),
        "the queue id to become the transcript message id",
    )
    .await;
    wait_for(|| queue_texts(&core).is_empty(), "the queue to empty").await;

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// A turn-boundary agent cannot take a message mid-turn, so the queue holds it —
/// visible, editable — and sends it when the turn ends. This is the case the
/// composer's steering warning is about.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queue_holds_during_a_turn_and_flushes_in_order_at_its_end() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;

    // Start a turn and let it hang.
    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;

    let second_id = core
        .doc_host
        .queue_message(CHAT, "second", Vec::new())
        .expect("queue second");
    let third_id = core
        .doc_host
        .queue_message(CHAT, "third", Vec::new())
        .expect("queue third");

    // Both must still be sitting there: the agent is busy and cannot be steered.
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(queue_texts(&core), vec!["second", "third"]);
    assert!(!prompts.lock().unwrap().iter().any(|p| p == "second"));

    // Turn ends → the queue flushes, head first.
    let _ = harness.finish.send(());
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "second"),
        "the queue to flush at turn end",
    )
    .await;
    wait_for(
        || user_message_id(&core, "second").as_deref() == Some(second_id.as_str()),
        "the held queue id to become the next-turn message id",
    )
    .await;
    assert_eq!(
        queue_texts(&core),
        vec!["third"],
        "only the head goes: the next turn is now running"
    );

    let _ = harness.finish.send(());
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "third"),
        "the rest of the queue to flush",
    )
    .await;
    wait_for(
        || {
            core.orchestration
                .store
                .thread(&CHAT.into())
                .unwrap()
                .is_some_and(|p| {
                    p.runs.iter().any(|run| {
                        run.user_message_id.0 == third_id
                            && run.status
                                == zeron_proto::orchestration::OrchestrationV2RunStatus::Running
                    })
                })
        },
        "the third queued run to accept its provider turn",
    )
    .await;
    let projection = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    assert_eq!(
        projection.runs.len(),
        3,
        "delivery never creates another logical run"
    );
    let provider = projection.runs[0].provider_thread_id.as_ref().unwrap();
    for run in &projection.runs {
        assert_eq!(run.provider_thread_id.as_ref(), Some(provider));
        let attempt = projection
            .attempts
            .iter()
            .find(|attempt| run.active_attempt_id.as_ref() == Some(&attempt.id))
            .unwrap();
        assert_eq!(&attempt.provider_thread_id, provider);
        assert_eq!(
            attempt.native_thread_id.as_ref().map(String::as_str),
            Some("sess-queue")
        );
        let root = projection
            .nodes
            .iter()
            .find(|node| run.root_node_id.as_ref() == Some(&node.id))
            .unwrap();
        assert_eq!(root.provider_thread_id.as_ref(), Some(provider));
    }
    wait_for(
        || user_message_id(&core, "third").as_deref() == Some(third_id.as_str()),
        "the next held queue id to remain stable too",
    )
    .await;
    let order = prompts.lock().unwrap().clone();
    let sent: Vec<&String> = order
        .iter()
        .filter(|p| ["opening", "second", "third"].contains(&p.as_str()))
        .collect();
    assert_eq!(
        sent,
        vec!["opening", "second", "third"],
        "queued messages keep their order"
    );

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// Quitting the host must neither release the queue through the interrupt's
/// Idle transition nor release it when the persisted document is reopened.
/// An explicit queue action remains the thaw gesture.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restart_restores_the_queue_frozen_until_an_explicit_send() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let (before_harness, before_prompts) = HeldHarness::new(SteeringMode::TurnBoundary);
    let before = assemble_at(&data_dir, before_harness);
    create_chat(&before).await;

    before
        .doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || {
            before_prompts
                .lock()
                .unwrap()
                .iter()
                .any(|p| p == "opening")
        },
        "the opening turn to start",
    )
    .await;
    let queued_id = before
        .doc_host
        .queue_message(CHAT, "after restart", Vec::new())
        .expect("queue recovered message");
    wait_for(
        || queue_texts(&before) == vec!["after restart"],
        "the follow-up to remain queued",
    )
    .await;

    before.shutdown().await;
    drop(before);

    let (after_harness, after_prompts) = HeldHarness::new(SteeringMode::TurnBoundary);
    let after = assemble_at(&data_dir, after_harness);
    // Materializing the persisted chat starts its background drain task. Give
    // that task time to prove the restored row stays frozen.
    assert_eq!(queue_texts(&after), vec!["after restart"]);
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(
        after_prompts.lock().unwrap().is_empty(),
        "startup must not dispatch a recovered queue"
    );
    assert_eq!(queue_texts(&after), vec!["after restart"]);

    assert!(
        after
            .doc_host
            .send_queued_now(CHAT, &queued_id)
            .await
            .expect("explicit send succeeds")
    );
    wait_for(
        || {
            after_prompts
                .lock()
                .unwrap()
                .iter()
                .any(|p| p == "after restart")
        },
        "the explicit send to thaw the recovered queue",
    )
    .await;

    after.shutdown().await;
}

/// Cancel is not a normal turn boundary: it freezes the visible queue instead
/// of immediately promoting its head into a new inference. The freeze survives
/// incidental queue edits and is lifted by an explicit send action.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_a_turn_freezes_the_queue_until_an_explicit_send() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;

    for text in ["second", "third"] {
        core.doc_host
            .queue_message(CHAT, text, Vec::new())
            .expect("queue follow-up");
    }
    let second_id = core
        .doc_host
        .open(CHAT)
        .unwrap()
        .doc()
        .read_queue()
        .unwrap()[0]
        .id
        .clone();

    core.doc_host
        .queue_command(CHAT, SessionCommandPayload::Interrupt {})
        .expect("queue interrupt");
    wait_for(
        || !core.sessions.turn_in_flight(CHAT),
        "the interrupted turn to settle",
    )
    .await;

    core.doc_host
        .update_queued_message(CHAT, &second_id, "second edited")
        .expect("edit frozen row");
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(queue_texts(&core), vec!["second edited", "third"]);
    assert!(
        !prompts.lock().unwrap().iter().any(|p| p == "second edited"),
        "Cancel must not turn a queued row into a new inference"
    );

    core.doc_host
        .send_queued_now(CHAT, &second_id)
        .await
        .expect("explicit queue send");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "second edited"),
        "the explicitly selected row to start",
    )
    .await;
    assert_eq!(queue_texts(&core), vec!["third"]);

    let _ = harness.finish.send(());
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "third"),
        "normal draining to resume after the explicit send",
    )
    .await;
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_all_freezes_the_queue_like_ordinary_stop_including_the_replay() {
    use zeron_proto::orchestration::OrchestrationV2RunStatus;
    use zeron_proto::transfer::StopThreadWorkParams;
    let (core, _harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.doc_host
        .queue_message(CHAT, "opening", vec![])
        .unwrap();
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;
    core.doc_host
        .queue_message(CHAT, "loro follow-up", vec![])
        .unwrap();
    let sql_run = canonical_input(&core, "sql follow-up").await;
    let request = StopThreadWorkParams {
        chat_id: CHAT.into(),
        client_request_id: "stop-all-freeze".into(),
    };
    let result = client
        .stop_thread_work(request.clone(), &core.device_id)
        .await
        .unwrap();
    assert!(result.stopped_runs >= 1, "{result:?}");
    wait_for(
        || !core.sessions.turn_in_flight(CHAT),
        "the stopped turn to settle",
    )
    .await;
    // A replay after an uncertain response must not release the queue either.
    let replay = client
        .stop_thread_work(request, &core.device_id)
        .await
        .unwrap();
    assert_eq!(replay.stopped_runs, result.stopped_runs);
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(queue_texts(&core), vec!["loro follow-up"]);
    let prompts = prompts.lock().unwrap().clone();
    assert!(
        !prompts
            .iter()
            .any(|p| p == "loro follow-up" || p == "sql follow-up"),
        "Stop all must not turn a queued row into a new inference: {prompts:?}"
    );
    let projection = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    let sql = projection
        .runs
        .iter()
        .find(|run| run.id.0 == sql_run)
        .expect("sql-only run");
    assert_eq!(sql.status, OrchestrationV2RunStatus::Queued);
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_stop_reaches_completed_native_background_and_holds_queue_without_agent_credentials() {
    use zeron_proto::orchestration::OrchestrationV2RunStatus;
    fn records<'a>(
        p: &'a zeron_engine::orchestration::projection::ThreadProjection,
        kind: &str,
    ) -> &'a [serde_json::Value] {
        p.records.get(kind).map(Vec::as_slice).unwrap_or_default()
    }
    for parked in [false, true] {
        let (mut harness, prompts) = HeldHarness::new(SteeringMode::TurnBoundary);
        let fixture = Arc::get_mut(&mut harness).unwrap();
        fixture.park_after_done = parked;
        fixture.native_background = true;
        let (core, harness, prompts) = setup_with((harness, prompts)).await;
        core.doc_host
            .queue_message(CHAT, "opening with background", vec![])
            .unwrap();
        let original = running_canonical(&core).await;
        let run_id = original.runs[0].id.clone();
        let queued_id = core
            .doc_host
            .queue_message(CHAT, "held follow-up", vec![])
            .unwrap();
        let BeginQueueEditOutcome::Acquired {
            lease_id,
            base_text_hash,
            ..
        } = core
            .doc_host
            .begin_queued_message_edit(CHAT, &queued_id, "desktop", "editor")
            .await
            .unwrap()
        else {
            panic!("fixture queue edit must be acquired");
        };
        let _ = harness.finish.send(());
        wait_for(
            || {
                core.orchestration
                    .store
                    .queue_ui_state(&CHAT.into())
                    .is_ok_and(|s| {
                        s.background_run_id.as_ref() == Some(&run_id.0) && s.active_run_id.is_none()
                    })
                    && !core.sessions.turn_in_flight(CHAT)
            },
            "completed root with pending native work",
        )
        .await;
        if parked {
            assert!(
                core.sessions.has_live_runtime(CHAT),
                "must exercise a settled live process"
            );
        } else {
            wait_for(
                || !core.sessions.has_live_runtime(CHAT),
                "already dead native process",
            )
            .await;
        }
        let before = core
            .orchestration
            .store
            .thread(&CHAT.into())
            .unwrap()
            .unwrap();
        core.sessions.mcp_server().credentials.revoke_thread(CHAT);
        let transcript_before = core
            .doc_host
            .open(CHAT)
            .unwrap()
            .doc()
            .read_entries()
            .unwrap();
        let client = zeron_rpc::memory_client(core.rpc_service());
        client
            .call(
                zeron_rpc::methods::QUEUE_COMMAND,
                serde_json::json!({
                    "chatId":CHAT,"command":{"kind":"interrupt"}
                }),
            )
            .await
            .unwrap();
        wait_for(
            || {
                !core.sessions.has_live_runtime(CHAT)
                    && core
                        .orchestration
                        .store
                        .thread(&CHAT.into())
                        .unwrap()
                        .is_some_and(|p| {
                            records(&p, "subagent").iter().any(|task| {
                                task["origin"] == "provider_native"
                                    && task["status"] == "interrupted"
                            })
                        })
            },
            "exact background Stop and durable settlement",
        )
        .await;
        let after = core
            .orchestration
            .store
            .thread(&CHAT.into())
            .unwrap()
            .unwrap();
        assert_eq!(after.runs[0].status, OrchestrationV2RunStatus::Completed);
        assert_eq!(after.runs[0].completed_at, before.runs[0].completed_at);
        assert_eq!(after.attempts[0], before.attempts[0]);
        assert_eq!(
            core.doc_host
                .open(CHAT)
                .unwrap()
                .doc()
                .read_entries()
                .unwrap(),
            transcript_before,
            "background Stop must preserve the ordinary completed transcript too"
        );
        assert_eq!(
            records(&after, "message")
                .iter()
                .find(|m| m["role"] == "assistant"),
            records(&before, "message")
                .iter()
                .find(|m| m["role"] == "assistant")
        );
        assert!(
            core.orchestration
                .store
                .queue_ui_state(&CHAT.into())
                .unwrap()
                .background_run_id
                .is_none()
        );
        assert!(matches!(
            core.doc_host
                .finish_queued_message_edit(
                    CHAT,
                    &queued_id,
                    &lease_id,
                    FinishQueueEditAction::Commit,
                    Some("edited held follow-up"),
                    Some(&base_text_hash),
                )
                .await
                .unwrap(),
            FinishQueueEditOutcome::Committed
        ));
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(queue_texts(&core), vec!["edited held follow-up"]);
        assert_eq!(
            prompts.lock().unwrap().len(),
            1,
            "Stop cannot release a queued inference"
        );
        assert!(core.orchestration.store.verify_projections().unwrap());
        core.doc_host
            .send_queued_now(CHAT, &queued_id)
            .await
            .unwrap();
        wait_for(
            || {
                prompts
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|p| p == "edited held follow-up")
            },
            "explicit next send after background Stop",
        )
        .await;
        let _ = harness.finish.send(());
        core.shutdown().await;
    }
}

/// A `Steer` command asks for the running turn directly — a client that decided
/// for itself, and the path a question's follow-up prompt takes. It obeys the
/// same rule as a typed message: a turn-boundary agent's mailbox is not read
/// mid-turn, so the prompt is held rather than posted into it.
///
/// Posting it anyway is the 2026-08-13 report: on `cursor-agent` the follow-up
/// went into the mailbox, the turn ended interrupted, and the message sat in the
/// transcript looking sent with the agent never seeing it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_steer_command_holds_for_an_agent_that_takes_no_mid_turn_prompt() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the turn to start",
    )
    .await;

    core.doc_host
        .queue_command(
            CHAT,
            SessionCommandPayload::Steer {
                prompt: "and also this".into(),
                message_id: Some("m-steer".into()),
            },
        )
        .expect("queue steer command");
    wait_for(
        || queue_texts(&core) == vec!["and also this"],
        "the steer to be held in the queue",
    )
    .await;
    // Held means held: not shown as sent, and not with the agent.
    assert!(!user_messages(&core).iter().any(|m| m == "and also this"));
    assert!(!prompts.lock().unwrap().iter().any(|p| p == "and also this"));

    // And it goes on its own when the turn ends — nobody re-sends it.
    let _ = harness.finish.send(());
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "and also this"),
        "the held steer to flush at turn end",
    )
    .await;
    assert!(queue_texts(&core).is_empty());

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// Remote sync can land a whole batch before the host gets a chance to drain.
/// Every steer is a user message, including those preceding a newer steer.
#[tokio::test]
async fn batched_remote_steers_preserve_every_message_in_order_exactly_once() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .unwrap();
    wait_for(|| prompts.lock().unwrap().len() == 1, "opening turn").await;
    let handle = core.doc_host.open(CHAT).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let expected: Vec<_> = (0..20).map(|i| format!("remote message {i}")).collect();
    // No await: all entries are visible together, as with a remote document import.
    for (i, prompt) in expected.iter().enumerate() {
        handle
            .doc()
            .queue_command(&zeron_doc::SessionCommandEntry {
                id: format!("remote-command-{i}"),
                payload: SessionCommandPayload::Steer {
                    prompt: prompt.clone(),
                    message_id: Some(format!("remote-message-{i}")),
                },
                issued_by: "remote-device".into(),
                issued_at: now + i as i64,
                based_on: None,
                expires_at: None,
                status: zeron_doc::SessionCommandStatus::Pending,
                resolution: None,
            })
            .unwrap();
    }
    core.doc_host.drain_commands(&handle).await;
    assert_eq!(queue_texts(&core), expected);
    // Repeated drains must not enqueue the same command again.
    core.doc_host.drain_commands(&handle).await;
    assert_eq!(queue_texts(&core), expected);
    assert!(
        handle
            .doc()
            .read_commands()
            .unwrap()
            .iter()
            .all(|c| c.status == zeron_doc::SessionCommandStatus::Applied)
    );
    for (i, prompt) in expected.iter().enumerate() {
        harness.finish.send(()).unwrap();
        wait_for(
            || prompts.lock().unwrap().len() == i + 2,
            "next queued turn",
        )
        .await;
        assert_eq!(prompts.lock().unwrap()[i + 1], *prompt);
    }
    let mut all = vec!["opening".to_owned()];
    all.extend(expected);
    assert_eq!(*prompts.lock().unwrap(), all);
    assert_eq!(user_messages(&core), all);
    assert!(queue_texts(&core).is_empty());
    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// Even agents that support mid-turn input deliver queue rows one turn at a time.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_text_waits_for_a_steerable_turn_even_with_legacy_policy() {
    let (core, harness, prompts) = setup(SteeringMode::StepBoundary).await;
    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .unwrap();
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "first turn",
    )
    .await;

    // Omitted and explicitly false policies from old clients must both wait.
    core.doc_host
        .queue_message(CHAT, "first queued", Vec::new())
        .unwrap();
    core.doc_host
        .queue_message_with_behavior(CHAT, "second queued", Vec::new(), false)
        .unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(queue_texts(&core), vec!["first queued", "second queued"]);
    assert_eq!(user_messages(&core), vec!["opening"]);

    harness.finish.send(()).unwrap();
    wait_for(
        || {
            user_messages(&core).iter().any(|m| m == "first queued")
                && prompts.lock().unwrap().iter().any(|p| p == "first queued")
                && harness.finish.receiver_count() > 0
        },
        // User entries publish before async admission/checkpoint work. They
        // cannot be used as proof that the next mock turn can receive finish.
        "first queued provider turn ready",
    )
    .await;
    assert_eq!(queue_texts(&core), vec!["second queued"]);
    assert!(!user_messages(&core).iter().any(|m| m == "second queued"));
    // The transcript user row precedes asynchronous orchestration admission;
    // do not finish a harness turn before its broadcast receiver exists.
    wait_for(
        || {
            prompts.lock().unwrap().iter().any(|p| p == "first queued")
                && harness.finish.receiver_count() > 0
        },
        "the first queued harness to be ready",
    )
    .await;
    harness.finish.send(()).unwrap();
    wait_for(
        || user_messages(&core).iter().any(|m| m == "second queued"),
        "second queued turn",
    )
    .await;
    assert!(queue_texts(&core).is_empty());
    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// The legacy explicit-steer RPC remains compatible with older clients;
/// current clients offer only Send now.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_policy_keeps_a_steerable_message_visible_until_steer_now() {
    let (core, harness, prompts) = setup(SteeringMode::StepBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;

    let reply = client
        .call(
            zeron_rpc::methods::QUEUE_MESSAGE,
            serde_json::json!({
                "chatId": CHAT,
                "text": "hold this",
                "holdForTurnEnd": true,
            }),
        )
        .await
        .expect("queue held message");
    let id = reply["id"].as_str().expect("queue id").to_string();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(queue_texts(&core), vec!["hold this"]);
    assert!(!user_messages(&core).iter().any(|m| m == "hold this"));

    let reply = client
        .call(
            zeron_rpc::methods::STEER_QUEUED_MESSAGE_NOW,
            serde_json::json!({ "chatId": CHAT, "id": id }),
        )
        .await
        .expect("steer now");
    assert_eq!(reply["sent"], true);
    wait_for(
        || user_messages(&core).iter().any(|m| m == "hold this"),
        "the held message to steer",
    )
    .await;
    assert!(queue_texts(&core).is_empty());

    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn steer_now_leaves_the_row_when_the_agent_cannot_steer_mid_turn() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;
    let id = core
        .doc_host
        .queue_message(CHAT, "still queued", Vec::new())
        .expect("queue held message");

    assert!(core.doc_host.steer_queued_now(CHAT, &id).await.is_err());
    assert_eq!(queue_texts(&core), vec!["still queued"]);

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// A provider's queue action remains `Steer` even if the active turn finishes
/// before the click lands. With nothing left to interrupt, the selected row
/// becomes the next turn and unpauses normal queue draining.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn steer_now_starts_the_next_turn_when_the_previous_turn_is_already_idle() {
    let (core, harness, prompts) = setup(SteeringMode::StepBoundary).await;

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;

    let client = zeron_rpc::memory_client(core.rpc_service());
    let reply = client
        .call(
            zeron_rpc::methods::QUEUE_MESSAGE,
            serde_json::json!({
                "chatId": CHAT,
                "text": "after cancel",
                "holdForTurnEnd": true,
            }),
        )
        .await
        .expect("queue held message");
    let id = reply["id"].as_str().expect("queue id").to_string();

    core.doc_host
        .queue_command(CHAT, SessionCommandPayload::Interrupt {})
        .expect("queue interrupt");
    wait_for(
        || !core.sessions.turn_in_flight(CHAT),
        "the interrupted turn to settle",
    )
    .await;
    assert_eq!(queue_texts(&core), vec!["after cancel"]);

    assert!(
        core.doc_host
            .steer_queued_now(CHAT, &id)
            .await
            .expect("non-interrupting queue promotion")
    );
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "after cancel"),
        "the selected row to start the next turn",
    )
    .await;
    assert_eq!(
        user_message_id(&core, "after cancel").as_deref(),
        Some(id.as_str()),
        "promoting an idle queued row preserves its transcript identity"
    );
    assert!(queue_texts(&core).is_empty());

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// Attachments never steer: the steer path carries a prompt and nothing else,
/// so a message with files waits for a turn that can inline them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_message_with_attachments_holds_even_for_a_steerable_agent() {
    let (core, harness, prompts) = setup(SteeringMode::StepBoundary).await;

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;

    core.doc_host
        .queue_message(CHAT, "with a file", vec!["att-1".into()])
        .expect("queue attachment message");
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(
        queue_texts(&core),
        vec!["with a file"],
        "a message carrying files must not be steered"
    );
    assert!(
        user_messages(&core)
            .iter()
            .all(|message| !message.contains("with a file")),
        "a held attachment row must not appear in the transcript before dispatch"
    );

    let _ = harness.finish.send(());
    wait_for(
        || {
            prompts.lock().unwrap().iter().any(|p| {
                p == "with a file\n\nAttached images (local files — open them to view):\n- att-1"
            })
        },
        "the held message to flush at turn end",
    )
    .await;
    assert_eq!(
        user_messages(&core),
        ["opening", "with a file"],
        "the attachment transport trailer belongs only in provider input, not app history"
    );

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// "Send now" interrupts the running turn and sends that one message, leaving
/// the rest queued. The UI selects this action for providers that cannot steer.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_now_interrupts_the_running_turn() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;

    let first = core
        .doc_host
        .queue_message(CHAT, "urgent", Vec::new())
        .expect("queue urgent");
    core.doc_host
        .queue_message(CHAT, "later", Vec::new())
        .expect("queue later");

    assert!(
        core.doc_host
            .send_queued_now(CHAT, &first)
            .await
            .expect("send now"),
        "send now takes the row"
    );
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "urgent"),
        "the urgent message to reach the agent",
    )
    .await;
    assert_eq!(
        queue_texts(&core),
        vec!["later"],
        "the rest of the queue is untouched"
    );

    // A row someone else already took is not an error, just `false`.
    assert!(
        !core
            .doc_host
            .send_queued_now(CHAT, &first)
            .await
            .expect("second send now")
    );

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// Editing a queued message to nothing is the delete gesture.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editing_a_queued_message_to_empty_removes_it() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;

    let id = core
        .doc_host
        .queue_message(CHAT, "typo", Vec::new())
        .expect("queue typo");
    assert!(
        core.doc_host
            .update_queued_message(CHAT, &id, "fixed")
            .expect("edit")
    );
    assert_eq!(queue_texts(&core), vec!["fixed"]);

    assert!(
        core.doc_host
            .update_queued_message(CHAT, &id, "   ")
            .expect("empty edit")
    );
    assert!(
        queue_texts(&core).is_empty(),
        "emptying a queued message deletes it"
    );

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// A positive removal acknowledgement is stronger than a local CRDT delete:
/// the host has serialized it against automatic drain, so ending the current
/// turn cannot materialize the cancelled row in the transcript afterwards.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acknowledged_removal_cannot_materialize_after_turn_end() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;

    let id = core
        .doc_host
        .queue_message(CHAT, "cancelled", Vec::new())
        .expect("queue cancelled row");
    assert!(
        core.doc_host
            .remove_queued_message(CHAT, &id)
            .await
            .expect("host removal acknowledgement")
    );

    let _ = harness.finish.send(());
    let handle = core.doc_host.open(CHAT).expect("open chat doc");
    core.doc_host.drain_queue(&handle).await;

    assert!(queue_texts(&core).is_empty());
    assert!(
        !prompts.lock().unwrap().iter().any(|p| p == "cancelled"),
        "an acknowledged removal must never reach the harness"
    );
    assert!(
        !user_messages(&core).iter().any(|p| p == "cancelled"),
        "an acknowledged removal must never reach the transcript"
    );

    core.shutdown().await;
}

/// Reordering, over the RPC surface the UIs actually call.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queue_rpc_reorders_and_streams() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;

    let mut rx = client
        .subscribe(
            zeron_rpc::methods::WATCH_QUEUE,
            serde_json::json!({ "chatId": CHAT }),
        )
        .await
        .expect("WatchQueue");
    let first = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("first queue frame")
        .expect("stream open");
    assert_eq!(
        first["items"].as_array().map(Vec::len),
        Some(0),
        "the stream opens with the current queue"
    );

    for text in ["a", "b", "c"] {
        client
            .call(
                zeron_rpc::methods::QUEUE_MESSAGE,
                serde_json::json!({ "chatId": CHAT, "text": text }),
            )
            .await
            .expect("QueueMessage");
    }
    assert_eq!(queue_texts(&core), vec!["a", "b", "c"]);

    let last_id = core
        .doc_host
        .open(CHAT)
        .unwrap()
        .doc()
        .read_queue()
        .unwrap()
        .last()
        .unwrap()
        .id
        .clone();
    client
        .call(
            zeron_rpc::methods::MOVE_QUEUED_MESSAGE,
            serde_json::json!({ "chatId": CHAT, "id": last_id, "toIndex": 0 }),
        )
        .await
        .expect("MoveQueuedMessage");
    assert_eq!(queue_texts(&core), vec!["c", "a", "b"]);

    client
        .call(
            zeron_rpc::methods::REMOVE_QUEUED_MESSAGE,
            serde_json::json!({ "chatId": CHAT, "id": last_id }),
        )
        .await
        .expect("RemoveQueuedMessage");
    assert_eq!(queue_texts(&core), vec!["a", "b"]);

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// Even a malformed or version-skewed UI must not make a non-host engine take
/// a shared queue row and start the chat on the wrong machine.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_queue_delivery_is_rejected_off_host_without_taking_the_row() {
    let (core, harness, _prompts) = setup(SteeringMode::StepBoundary).await;
    core.workspace
        .set_chat_host(CHAT, "device-remote")
        .expect("move chat host");
    let id = core
        .doc_host
        .queue_message_with_behavior(CHAT, "remote-only", Vec::new(), true)
        .expect("queue held row");

    let send_error = core
        .doc_host
        .send_queued_now(CHAT, &id)
        .await
        .expect_err("non-host send-now must fail");
    assert!(send_error.to_string().contains("does not host chat"));
    assert_eq!(queue_texts(&core), vec!["remote-only"]);

    let steer_error = core
        .doc_host
        .steer_queued_now(CHAT, &id)
        .await
        .expect_err("non-host steer-now must fail");
    assert!(steer_error.to_string().contains("does not host chat"));

    let remove_error = core
        .doc_host
        .remove_queued_message(CHAT, &id)
        .await
        .expect_err("non-host removal must fail");
    assert!(remove_error.to_string().contains("does not host chat"));
    assert_eq!(queue_texts(&core), vec!["remote-only"]);

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// The regression the review caught: `drain_queue` runs from both the
/// doc-change task and the turn-end status watcher, and there is nothing about
/// those two callers that keeps them apart. Driven concurrently against an idle
/// agent, an unserialized drain has both take a different head across the
/// `dispatch` await and both send — the queue empties in one go, which is the
/// "looks sent" failure the whole feature exists to prevent.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_drains_release_one_message() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    let handle = core.doc_host.open(CHAT).expect("open chat");

    for text in ["first", "second", "third"] {
        core.doc_host
            .queue_message(CHAT, text, Vec::new())
            .expect("queue");
    }

    tokio::join!(
        core.doc_host.drain_queue(&handle),
        core.doc_host.drain_queue(&handle),
        core.doc_host.drain_queue(&handle),
    );

    wait_for(
        || !prompts.lock().unwrap().is_empty(),
        "the released message to reach the agent",
    )
    .await;
    // Long enough for a second escapee to show up if the drains interleaved.
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(
        queue_texts(&core),
        vec!["second", "third"],
        "one drain released the head; the others found a busy agent"
    );
    assert_eq!(
        prompts.lock().unwrap().clone(),
        vec!["first".to_string()],
        "the agent was handed exactly one prompt"
    );

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// An agent parked on a question still owns the turn. The composer queues on
/// that state, so the drain has to hold there too — reading `AwaitingInput` as
/// idle sends the follow-up as a fresh turn and abandons the question.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_message_holds_while_the_agent_waits_on_a_question() {
    let (core, harness, prompts) = setup_asking(SteeringMode::TurnBoundary).await;

    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "the first turn to start",
    )
    .await;
    wait_for(
        || {
            core.sessions
                .session_status(CHAT)
                .is_some_and(|s| s.status == zeron_proto::SessionStatus::AwaitingInput)
        },
        "the agent to park on its question",
    )
    .await;

    core.doc_host
        .queue_message(CHAT, "follow-up", Vec::new())
        .expect("queue follow-up");
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(
        queue_texts(&core),
        vec!["follow-up"],
        "the follow-up waits for the question to be answered"
    );
    assert!(
        !prompts.lock().unwrap().iter().any(|p| p == "follow-up"),
        "no fresh turn started under the parked question"
    );

    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// Core regression: once BeginEdit wins the same lock as the drain, ending the
/// current turn cannot promote the row until the lease is explicitly resolved.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_acquired_edit_blocks_turn_end_and_commit_sends_the_new_text() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "opening turn",
    )
    .await;
    let id = core
        .doc_host
        .queue_message(CHAT, "old text", Vec::new())
        .expect("queue editable row");

    let BeginQueueEditOutcome::Acquired {
        lease_id,
        base_text_hash,
        ..
    } = core
        .doc_host
        .begin_queued_message_edit(CHAT, &id, "phone", "view-1")
        .await
        .expect("begin edit")
    else {
        panic!("edit must be acquired");
    };
    let _ = harness.finish.send(());
    wait_for(
        || !core.sessions.turn_in_flight(CHAT),
        "opening turn to end",
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(queue_texts(&core), vec!["old text"]);
    assert!(!prompts.lock().unwrap().iter().any(|p| p == "old text"));
    assert!(core.doc_host.send_queued_now(CHAT, &id).await.is_err());

    assert!(matches!(
        core.doc_host
            .finish_queued_message_edit(
                CHAT,
                &id,
                &lease_id,
                FinishQueueEditAction::Commit,
                Some("new text"),
                Some(&base_text_hash),
            )
            .await
            .expect("finish edit"),
        FinishQueueEditOutcome::Committed
    ));
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "new text"),
        "edited row to dispatch",
    )
    .await;
    assert!(!prompts.lock().unwrap().iter().any(|p| p == "old text"));

    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn only_one_editor_wins_and_an_expired_edit_requires_review() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "opening turn",
    )
    .await;
    let id = core
        .doc_host
        .queue_message(CHAT, "review me", Vec::new())
        .expect("queue row");

    let (a, b) = tokio::join!(
        core.doc_host
            .begin_queued_message_edit(CHAT, &id, "a", "view-a"),
        core.doc_host
            .begin_queued_message_edit(CHAT, &id, "b", "view-b"),
    );
    let outcomes = [a.unwrap(), b.unwrap()];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, BeginQueueEditOutcome::Acquired { .. }))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, BeginQueueEditOutcome::Locked { .. }))
            .count(),
        1
    );

    // Force the persisted deadline into the past; turn-end must convert it to
    // ReviewRequired, never treat expiry as permission to send.
    let row = core
        .doc_host
        .open(CHAT)
        .unwrap()
        .doc()
        .read_queue()
        .unwrap()[0]
        .clone();
    let QueueDeliveryGate::Editing {
        lease_id,
        owner_device_id,
        base_text_hash,
        ..
    } = row.delivery_gate.unwrap()
    else {
        panic!("editing gate expected");
    };
    core.doc_host
        .open(CHAT)
        .unwrap()
        .doc()
        .set_queued_delivery_gate(
            &id,
            Some(&QueueDeliveryGate::Editing {
                lease_id,
                owner_device_id,
                owner_instance_id: "expired".into(),
                acquired_at_ms: 0,
                expires_at_ms: 0,
                base_text_hash,
            }),
        )
        .unwrap();
    let _ = harness.finish.send(());
    wait_for(
        || {
            core.doc_host
                .open(CHAT)
                .unwrap()
                .doc()
                .read_queue()
                .unwrap()
                .first()
                .is_some_and(|row| {
                    matches!(
                        row.delivery_gate,
                        Some(QueueDeliveryGate::ReviewRequired { .. })
                    )
                })
        },
        "expired edit to require review",
    )
    .await;
    assert!(!prompts.lock().unwrap().iter().any(|p| p == "review me"));

    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protected_edit_rpc_round_trips_its_camel_case_protocol() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .expect("queue opening");
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "opening turn",
    )
    .await;
    let id = core
        .doc_host
        .queue_message(CHAT, "rpc edit", Vec::new())
        .expect("queue row");
    let client = zeron_rpc::memory_client(core.rpc_service());
    let begin = client
        .call(
            zeron_rpc::methods::BEGIN_QUEUED_MESSAGE_EDIT,
            serde_json::json!({
                "chatId": CHAT,
                "id": id,
                "editorDeviceId": "phone",
                "editorInstanceId": "view-1",
            }),
        )
        .await
        .expect("begin edit rpc");
    assert_eq!(begin["outcome"], "acquired");
    assert!(begin["baseTextHash"].as_str().is_some());

    let finish = client
        .call(
            zeron_rpc::methods::FINISH_QUEUED_MESSAGE_EDIT,
            serde_json::json!({
                "chatId": CHAT,
                "id": id,
                "leaseId": begin["leaseId"],
                "action": "commit",
                "text": "rpc revised",
                "expectedTextHash": begin["baseTextHash"],
            }),
        )
        .await
        .expect("finish edit rpc");
    assert_eq!(finish["outcome"], "committed");
    // The current turn is still live, so the revised row remains queued.
    assert_eq!(queue_texts(&core), vec!["rpc revised"]);

    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn composer_edit_commits_attachments_in_place_and_cancel_preserves_them() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    core.doc_host
        .queue_message(CHAT, "opening", Vec::new())
        .unwrap();
    wait_for(
        || prompts.lock().unwrap().iter().any(|p| p == "opening"),
        "opening turn",
    )
    .await;
    core.doc_host
        .queue_message(CHAT, "before", Vec::new())
        .unwrap();
    let id = core
        .doc_host
        .queue_message(CHAT, "original", vec!["old.png".into()])
        .unwrap();
    core.doc_host
        .queue_message(CHAT, "after", Vec::new())
        .unwrap();
    let BeginQueueEditOutcome::Acquired {
        lease_id,
        base_text_hash,
        attachments,
        ..
    } = core
        .doc_host
        .begin_queued_message_edit(CHAT, &id, "desktop", "composer")
        .await
        .unwrap()
    else {
        panic!("edit must be acquired");
    };
    assert_eq!(attachments, vec!["old.png"]);
    let paths = vec!["new.png".to_string()];
    assert!(matches!(
        core.doc_host
            .finish_queued_message_edit_with_attachments(
                CHAT,
                &id,
                &lease_id,
                FinishQueueEditAction::Commit,
                Some("revised"),
                Some(&base_text_hash),
                Some(&paths),
            )
            .await
            .unwrap(),
        FinishQueueEditOutcome::Committed
    ));
    assert_eq!(queue_texts(&core), vec!["before", "revised", "after"]);
    let BeginQueueEditOutcome::Acquired {
        lease_id,
        attachments,
        ..
    } = core
        .doc_host
        .begin_queued_message_edit(CHAT, &id, "desktop", "composer")
        .await
        .unwrap()
    else {
        panic!("second edit must be acquired");
    };
    assert_eq!(attachments, paths);
    assert!(matches!(
        core.doc_host
            .finish_queued_message_edit_with_attachments(
                CHAT,
                &id,
                &lease_id,
                FinishQueueEditAction::Cancel,
                None,
                None,
                Some(&[]),
            )
            .await
            .unwrap(),
        FinishQueueEditOutcome::Cancelled
    ));
    let BeginQueueEditOutcome::Acquired { attachments, .. } = core
        .doc_host
        .begin_queued_message_edit(CHAT, &id, "desktop", "composer")
        .await
        .unwrap()
    else {
        panic!("cancel must release the lease");
    };
    assert_eq!(attachments, paths);
    assert_eq!(queue_texts(&core), vec!["before", "revised", "after"]);
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_queue_dispatch_stays_paused_until_explicit_retry() {
    let tmp = tempfile::tempdir().unwrap();
    let (harness, prompts) = HeldHarness::new(SteeringMode::TurnBoundary);
    let core = assemble_at(tmp.path(), harness.clone());
    let handle = core.doc_host.open(CHAT).unwrap();
    let id = core
        .doc_host
        .queue_message(CHAT, "recover me", Vec::new())
        .unwrap();
    core.doc_host.drain_queue(&handle).await;
    let version = handle.doc().doc().oplog_vv();
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(handle.doc().doc().oplog_vv(), version);
    assert_eq!(queue_texts(&core), vec!["recover me"]);
    assert!(prompts.lock().unwrap().is_empty());
    create_chat(&core).await;
    core.doc_host.drain_queue(&handle).await;
    assert_eq!(queue_texts(&core), vec!["recover me"]);
    assert!(core.doc_host.send_queued_now(CHAT, &id).await.unwrap());
    wait_for(|| prompts.lock().unwrap().len() == 1, "explicit recovery").await;
    assert_eq!(user_message_id(&core, "recover me"), Some(id));
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_turn_uses_current_config_at_turn_end_and_send_now() {
    for send_now in [false, true] {
        let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
        let mut config = zeron_proto::ChatConfig {
            instance_id: None,
            harness: HarnessId::Mock,
            model: Some("old-model".into()),
            reasoning: Some(ReasoningLevel::Medium),
            model_options: Default::default(),
            sandbox: zeron_proto::SandboxLevel::WorkspaceWrite,
            runtime_mode: Default::default(),
            interaction_mode: Default::default(),
        };
        core.workspace.set_chat_config(CHAT, &config).unwrap();
        core.doc_host
            .queue_message(CHAT, "opening", Vec::new())
            .unwrap();
        wait_for(|| prompts.lock().unwrap().len() == 1, "first turn").await;
        config.model = Some("new-model".into());
        config.reasoning = None;
        config
            .model_options
            .insert("contextWindow".into(), serde_json::json!("1m"));
        core.workspace.set_chat_config(CHAT, &config).unwrap();
        let id = core
            .doc_host
            .queue_message(CHAT, "follow-up", Vec::new())
            .unwrap();
        if send_now {
            assert!(core.doc_host.send_queued_now(CHAT, &id).await.unwrap());
        } else {
            let _ = harness.finish.send(());
        }
        wait_for(|| prompts.lock().unwrap().len() == 2, "queued turn").await;
        {
            let requests = harness.requests.lock().unwrap();
            assert_eq!(requests[0].model.as_deref(), Some("old-model"));
            assert_eq!(requests[1].model, config.model);
            assert_eq!(requests[1].reasoning, config.reasoning);
            assert_eq!(requests[1].model_options, config.model_options);
            assert!(
                requests[1].resume.is_none(),
                "a changed model cannot resume the old generation"
            );
        }
        wait_for(
            || {
                core.orchestration
                    .store
                    .thread(&CHAT.into())
                    .unwrap()
                    .is_some_and(|p| {
                        p.runs.iter().any(|run| {
                            run.user_message_id.0 == id
                                && run.status
                                    == zeron_proto::orchestration::OrchestrationV2RunStatus::Running
                        })
                    })
            },
            "the queued selection to be accepted canonically",
        )
        .await;
        let projection = core
            .orchestration
            .store
            .thread(&CHAT.into())
            .unwrap()
            .unwrap();
        let selected = projection
            .runs
            .iter()
            .find(|run| run.user_message_id.0 == id)
            .unwrap();
        assert_eq!(selected.model_selection.model, "new-model");
        assert_ne!(
            selected.provider_thread_id,
            projection.runs[0].provider_thread_id
        );
        let attempt = projection
            .attempts
            .iter()
            .find(|attempt| selected.active_attempt_id.as_ref() == Some(&attempt.id))
            .unwrap();
        assert_eq!(
            Some(&attempt.provider_thread_id),
            selected.provider_thread_id.as_ref()
        );
        let _ = harness.finish.send(());
        core.shutdown().await;
    }
}

/// Normal queued turns each publish a completion; Send now's interrupted
/// predecessor does not. The replacement's own completion must still arrive.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queue_completion_markers_distinguish_normal_turns_from_interrupts() {
    for send_now in [false, true] {
        let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
        core.doc_host
            .queue_message(CHAT, "opening", Vec::new())
            .unwrap();
        wait_for(|| prompts.lock().unwrap().len() == 1, "opening turn").await;
        let id = core
            .doc_host
            .queue_message(CHAT, "follow-up", Vec::new())
            .unwrap();
        let completion = || {
            core.sessions
                .session_status(CHAT)
                .and_then(|s| s.last_completed_turn)
        };
        assert_eq!(completion(), None);
        if send_now {
            assert!(core.doc_host.send_queued_now(CHAT, &id).await.unwrap());
        } else {
            harness.finish.send(()).unwrap();
        }
        wait_for(|| prompts.lock().unwrap().len() == 2, "follow-up turn").await;
        let first_completion = completion();
        assert_eq!(
            first_completion.is_some(),
            !send_now,
            "only a naturally completed predecessor should notify"
        );
        harness.finish.send(()).unwrap();
        wait_for(
            || completion().is_some() && completion() != first_completion,
            "follow-up completion marker",
        )
        .await;
        core.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forced_reset_starts_a_fresh_generation_with_portable_history_and_is_replay_safe() {
    use zeron_proto::orchestration::OrchestrationV2RunStatus;
    use zeron_proto::transfer::ResetThreadSessionParams;
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.doc_host
        .queue_message(CHAT, "opening question", vec![])
        .unwrap();
    let sessions = attached_sessions(&core).await;
    let observed = core
        .orchestration
        .store
        .transfer_ui_state(&CHAT.into())
        .unwrap()
        .latest_started_run_id;
    let request = ResetThreadSessionParams {
        chat_id: CHAT.into(),
        client_request_id: "reset-once".into(),
        observed_run_id: observed.clone(),
        provider_sessions: sessions.clone(),
    };
    // A running turn is not silently stopped or rewritten by a reset.
    let refused = client
        .reset_thread_session(request.clone(), &core.device_id)
        .await
        .unwrap();
    assert!(
        refused
            .refusal
            .as_deref()
            .unwrap()
            .contains("Stop the current run"),
        "{:?}",
        refused.refusal
    );
    assert!(core.sessions.has_live_runtime(CHAT));
    let _ = harness.finish.send(());
    wait_for(
        || {
            core.orchestration
                .store
                .thread(&CHAT.into())
                .unwrap()
                .is_some_and(|p| {
                    p.runs
                        .iter()
                        .all(|r| r.status == OrchestrationV2RunStatus::Completed)
                })
        },
        "first turn completion",
    )
    .await;
    let before = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    let old_provider = before.runs[0].provider_thread_id.clone().unwrap();
    let live = core
        .orchestration
        .store
        .transfer_ui_state(&CHAT.into())
        .unwrap()
        .attached_provider_sessions;
    let mut request = ResetThreadSessionParams {
        provider_sessions: live,
        ..request
    };
    let mut stale = request.clone();
    stale.client_request_id = "reset-stale".into();
    stale.observed_run_id = Some("run:an-older-turn".into());
    assert!(
        client
            .reset_thread_session(stale, &core.device_id)
            .await
            .unwrap()
            .refusal
            .as_deref()
            .unwrap()
            .contains("newer turn")
    );
    request.client_request_id = "reset-done".into();
    let first = client
        .reset_thread_session(request.clone(), &core.device_id)
        .await
        .unwrap();
    assert!(first.refusal.is_none(), "{:?}", first.refusal);
    wait_for(|| !core.sessions.has_live_runtime(CHAT), "reset teardown").await;
    let reset = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    // History is kept; only the provider conversation is closed.
    assert_eq!(reset.runs.len(), before.runs.len());
    assert_eq!(
        reset.records["provider-thread"]
            .iter()
            .find(|p| p["id"] == old_provider.0)
            .unwrap()["status"],
        "closed"
    );
    assert_eq!(
        prompts.lock().unwrap().len(),
        1,
        "reset must not start a provider"
    );
    // Retrying the exact request after response loss repeats it, and neither
    // a changed payload under the same id nor a second reset of a thread with
    // nothing left to reset is accepted.
    let replay = client
        .reset_thread_session(request.clone(), &core.device_id)
        .await
        .unwrap();
    assert_eq!(first.sequence, replay.sequence);
    let mut collision = request.clone();
    collision.observed_run_id = Some("run:other".into());
    assert!(
        client
            .reset_thread_session(collision, &core.device_id)
            .await
            .is_err()
    );
    request.client_request_id = "reset-again".into();
    assert!(
        client
            .reset_thread_session(request, &core.device_id)
            .await
            .unwrap()
            .refusal
            .is_some()
    );

    core.doc_host
        .queue_message(CHAT, "after the reset", vec![])
        .unwrap();
    wait_for(
        || prompts.lock().unwrap().len() == 2,
        "fresh generation start",
    )
    .await;
    {
        let requests = harness.requests.lock().unwrap();
        assert_eq!(
            requests[1].resume, None,
            "a reset never rides native resume"
        );
        assert!(
            requests[1].prompt.contains("opening question")
                && requests[1].prompt.contains("after the reset"),
            "{}",
            requests[1].prompt
        );
    }
    let next = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    assert_ne!(
        next.runs[1].provider_thread_id.as_ref(),
        Some(&old_provider)
    );
    let _ = harness.finish.send(());
    wait_for(
        || {
            core.orchestration
                .store
                .thread(&CHAT.into())
                .unwrap()
                .is_some_and(|p| {
                    p.runs
                        .iter()
                        .all(|r| r.status == OrchestrationV2RunStatus::Completed)
                })
        },
        "second turn completion",
    )
    .await;
    // The new generation is an ordinary conversation again: no repeated
    // reconstruction and no tombstone on the following turn.
    core.doc_host
        .queue_message(CHAT, "one more", vec![])
        .unwrap();
    wait_for(
        || prompts.lock().unwrap().len() == 3,
        "continued generation",
    )
    .await;
    {
        let requests = harness.requests.lock().unwrap();
        assert_eq!(requests[2].resume.as_deref(), Some("sess-queue"));
        assert!(!requests[2].prompt.contains("opening question"));
    }
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancelled_first_run_after_reset_does_not_turn_the_next_start_back_into_a_resume() {
    use zeron_proto::transfer::{ResetThreadSessionParams, StopThreadWorkParams};
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.doc_host
        .queue_message(CHAT, "opening question", vec![])
        .unwrap();
    attached_sessions(&core).await;
    for text in ["first after reset", "second after reset"] {
        core.doc_host.queue_message(CHAT, text, vec![]).unwrap();
    }
    // Stop freezes the queue; the two rows stay queued across the reset.
    client
        .stop_thread_work(
            StopThreadWorkParams {
                chat_id: CHAT.into(),
                client_request_id: "stop-before-reset".into(),
            },
            &core.device_id,
        )
        .await
        .unwrap();
    wait_for(
        || !core.sessions.turn_in_flight(CHAT),
        "the stopped turn to settle",
    )
    .await;
    let observed = core
        .orchestration
        .store
        .transfer_ui_state(&CHAT.into())
        .unwrap()
        .latest_started_run_id;
    let live = core
        .orchestration
        .store
        .transfer_ui_state(&CHAT.into())
        .unwrap()
        .attached_provider_sessions;
    let reset = client
        .reset_thread_session(
            ResetThreadSessionParams {
                chat_id: CHAT.into(),
                client_request_id: "reset-then-cancel".into(),
                observed_run_id: observed,
                provider_sessions: live,
            },
            &core.device_id,
        )
        .await
        .unwrap();
    assert!(reset.refusal.is_none(), "{:?}", reset.refusal);
    wait_for(|| !core.sessions.has_live_runtime(CHAT), "reset teardown").await;
    // The first row never reaches a provider; the second is the first to start.
    let rows = core.doc_host.open(CHAT).unwrap().doc().read_queue().unwrap();
    assert_eq!(rows.len(), 2, "{:?}", queue_texts(&core));
    core.doc_host
        .remove_queued_message(CHAT, &rows[0].id)
        .await
        .unwrap();
    assert!(
        core.doc_host
            .send_queued_now(CHAT, &rows[1].id)
            .await
            .unwrap()
    );
    wait_for(
        || prompts.lock().unwrap().len() == 2,
        "the run after the cancelled one",
    )
    .await;
    {
        let requests = harness.requests.lock().unwrap();
        assert_eq!(
            requests[1].resume, None,
            "a cancelled queued run must not hide the reset"
        );
        assert!(
            requests[1].prompt.contains("opening question")
                && requests[1].prompt.contains("second after reset"),
            "{}",
            requests[1].prompt
        );
    }
    let _ = harness.finish.send(());
    core.shutdown().await;
}

fn composer_config(harness: HarnessId, model: &str) -> zeron_proto::ChatConfig {
    zeron_proto::ChatConfig {
        instance_id: None,
        harness,
        model: Some(model.into()),
        reasoning: None,
        model_options: Default::default(),
        sandbox: zeron_proto::SandboxLevel::WorkspaceWrite,
        runtime_mode: Default::default(),
        interaction_mode: Default::default(),
    }
}

/// The desktop composer's own write path (`Mutate setChatConfig`).
async fn set_composer(core: &EngineCore, config: &zeron_proto::ChatConfig) {
    zeron_rpc::memory_client(core.rpc_service())
        .call(
            zeron_rpc::methods::MUTATE,
            serde_json::json!({"op":"setChatConfig","chatId":CHAT,"config":config}),
        )
        .await
        .expect("setChatConfig");
}

async fn runs_completed(core: &EngineCore, count: usize) {
    wait_for(
        || {
            core.orchestration
                .store
                .thread(&CHAT.into())
                .unwrap()
                .is_some_and(|p| {
                    p.runs.len() == count && p.runs.iter().all(|r| {
                        r.status == zeron_proto::orchestration::OrchestrationV2RunStatus::Completed
                    })
                })
        },
        "runs completed",
    )
    .await;
}

fn assemble_two(
    path: &std::path::Path,
    first: Arc<HeldHarness>,
    second: Arc<HeldHarness>,
) -> EngineCore {
    let registry = HarnessRegistry::new();
    registry.register(first);
    registry.register(second);
    EngineCore::assemble(path, Arc::new(registry), HarnessId::Grok, None)
        .expect("engine core assembles")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn next_turn_handoff_on_the_same_instance_starts_fresh_instead_of_resuming_the_remembered_session()
 {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    core.workspace
        .set_chat_config(CHAT, &composer_config(HarnessId::Mock, "model-a"))
        .unwrap();
    core.doc_host
        .queue_message(CHAT, "first question", vec![])
        .unwrap();
    wait_for(|| prompts.lock().unwrap().len() == 1, "first turn").await;
    let _ = harness.finish.send(());
    runs_completed(&core, 1).await;
    // The composer moves to another model of the same instance. This adapter
    // cannot switch models inside a native session, so the planner chooses a new
    // provider generation seeded with portable context.
    set_composer(&core, &composer_config(HarnessId::Mock, "model-b")).await;
    core.doc_host
        .queue_message(CHAT, "second question", vec![])
        .unwrap();
    wait_for(|| prompts.lock().unwrap().len() == 2, "handoff turn").await;
    {
        let requests = harness.requests.lock().unwrap();
        assert_eq!(
            requests[1].resume, None,
            "the engine must not resume the native session it still remembers"
        );
        assert!(
            requests[1].prompt.contains("first question")
                && requests[1].prompt.contains("second question"),
            "{}",
            requests[1].prompt
        );
    }
    let _ = harness.finish.send(());
    runs_completed(&core, 2).await;
    core.doc_host
        .queue_message(CHAT, "third question", vec![])
        .unwrap();
    wait_for(|| prompts.lock().unwrap().len() == 3, "continued turn").await;
    {
        let requests = harness.requests.lock().unwrap();
        assert_eq!(
            requests[2].resume.as_deref(),
            Some("sess-queue"),
            "an unchanged selection keeps native continuity on the new generation"
        );
        assert!(!requests[2].prompt.contains("first question"));
    }
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn composer_model_change_mid_run_drives_a_same_instance_handoff_restart_and_fences_the_old_process()
 {
    use serde_json::json;
    use zeron_proto::{QueuePromotionMode, orchestration::*};
    use zeron_rpc::methods;
    let (mut harness, _) = HeldHarness::new(SteeringMode::TurnBoundary);
    Arc::get_mut(&mut harness).unwrap().partial_text =
        Some("partial answer before the switch".into());
    let tmp = tempfile::tempdir().unwrap();
    let core = assemble_at(&tmp.keep().join("data"), harness.clone());
    create_chat(&core).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    core.workspace
        .set_chat_config(CHAT, &composer_config(HarnessId::Mock, "model-a"))
        .unwrap();
    core.doc_host
        .queue_message(CHAT, "original direction", vec![])
        .unwrap();
    let original = running_canonical(&core).await;
    let active = original.runs[0].id.clone();
    let selected = canonical_input(&core, "new direction").await;
    canonical_input(&core, "later SQL work").await;
    let mut watch = client
        .subscribe_checked(
            methods::WATCH_QUEUE,
            json!({"chatId":CHAT,"includeCanonical":true}),
        )
        .await
        .unwrap();
    let before = canonical_frame(&mut watch, |state| {
        state.queue.len() == 2 && state.promotion_mode.is_some()
    })
    .await;
    assert_eq!(before.promotion_selection, None, "nothing changed yet");
    set_composer(&core, &composer_config(HarnessId::Mock, "model-b")).await;
    let hint = canonical_frame(&mut watch, |state| {
        state.promotion_mode == Some(QueuePromotionMode::InterruptRestartWithHandoff)
    })
    .await;
    let next = hint
        .promotion_selection
        .clone()
        .expect("reviewed selection");
    assert_eq!(&*next.model, "model-b");
    // Changing the composer neither interrupts nor restarts the running turn.
    assert_eq!(harness.requests.lock().unwrap().len(), 1);
    assert_eq!(
        harness.released.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    // A click reviewed against the wrong mode is refused before consuming input.
    let stale = client
        .call(
            methods::MUTATE_QUEUED_RUN,
            json!({"chatId":CHAT,"queuedRunId":selected,"clientRequestId":"wrong-mode",
                "action":{"type":"promoteToRestart","targetRunId":active,"handoff":false,
                    "expectedSelection":next}}),
        )
        .await
        .unwrap();
    assert!(stale["refusal"].is_string(), "{stale}");
    assert_eq!(harness.requests.lock().unwrap().len(), 1);
    core.sessions.mcp_server().credentials.revoke_thread(CHAT);
    let request = json!({"chatId":CHAT,"queuedRunId":selected,"clientRequestId":"handoff-once",
        "action":{"type":"promoteToRestart","targetRunId":active,"handoff":true,
            "expectedSelection":next}});
    let reply = client
        .call(methods::MUTATE_QUEUED_RUN, request.clone())
        .await
        .unwrap();
    assert!(reply["refusal"].is_null(), "{reply}");
    wait_for(
        || harness.requests.lock().unwrap().len() == 2,
        "replacement start on the new generation",
    )
    .await;
    assert_eq!(
        client
            .call(methods::MUTATE_QUEUED_RUN, request)
            .await
            .unwrap(),
        reply,
        "response-loss retry must not restart the replacement"
    );
    let after = running_canonical(&core).await;
    let run = after.runs.iter().find(|r| r.id == active).unwrap();
    assert_eq!(run.model_selection, next);
    assert_ne!(
        run.provider_thread_id, original.runs[0].provider_thread_id,
        "a handoff needs a new provider generation"
    );
    assert_eq!(
        harness.released.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "exactly the old process was torn down"
    );
    assert_eq!(
        after
            .attempts
            .iter()
            .find(|a| Some(&a.id) == original.runs[0].active_attempt_id.as_ref())
            .unwrap()
            .status,
        OrchestrationV2RunAttemptStatus::Superseded
    );
    let requests = harness.requests.lock().unwrap().clone();
    assert_eq!(
        requests[1].resume, None,
        "a fresh generation must not resume the native session the engine remembers"
    );
    assert_eq!(requests[1].model.as_deref(), Some("model-b"));
    assert!(
        requests[1].prompt.ends_with("new direction"),
        "the promoted message is the current input, after the handoff: {}",
        requests[1].prompt
    );
    assert!(
        requests[1].prompt.contains("original direction")
            && requests[1]
                .prompt
                .contains("partial answer before the switch"),
        "the handoff carries the run's partial history: {}",
        requests[1].prompt
    );
    let handoffs: Vec<_> = core
        .orchestration
        .store
        .thread_transfers(&CHAT.into())
        .unwrap()
        .into_iter()
        .filter(|t| t["type"] == "provider_handoff")
        .collect();
    assert_eq!(handoffs.len(), 1, "replay must not duplicate the handoff");
    assert_eq!(
        handoffs[0]["id"],
        format!(
            "provider-handoff:{}:attempt:2",
            active.0.replace(':', "%3A")
        ),
    );
    assert_eq!(
        after
            .runs
            .iter()
            .find(|r| r.user_message_id.0 == "message:later SQL work")
            .unwrap()
            .status,
        OrchestrationV2RunStatus::Queued,
        "other queued rows are untouched"
    );
    assert!(core.orchestration.store.verify_projections().unwrap());
    let _ = harness.finish.send(());
    core.shutdown().await;
}

/// Two adapters on two provider instances: a mid-run composer switch restarts
/// the running turn on the other one, then a later switch back returns to the
/// first instance's earlier native conversation with only the missed delta.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cross_instance_composer_switch_restarts_with_portable_history_and_a_to_b_to_a_resumes_the_earlier_generation()
 {
    use serde_json::json;
    use zeron_proto::QueuePromotionMode;
    use zeron_rpc::methods;
    // The mock adapter is hidden from the migrated catalog once a real driver
    // is registered (`provider_instances::load`), so both instances are real
    // drivers backed by the fake adapter.
    let (mut a, _) = HeldHarness::new(SteeringMode::TurnBoundary);
    {
        let a = Arc::get_mut(&mut a).unwrap();
        a.id = HarnessId::Grok;
        a.session = "sess-a".into();
    }
    let (mut b, _) = HeldHarness::new(SteeringMode::TurnBoundary);
    {
        let b = Arc::get_mut(&mut b).unwrap();
        b.id = HarnessId::Codex;
        b.session = "sess-b".into();
        b.partial_text = Some("partial answer on b".into());
    }
    let tmp = tempfile::tempdir().unwrap();
    let core = assemble_two(&tmp.keep().join("data"), a.clone(), b.clone());
    create_chat(&core).await;
    let client = zeron_rpc::memory_client(core.rpc_service());
    let first_model = |harness: HarnessId, fallback: &str| {
        core.registry
            .provider_instances
            .snapshot(&core.registry)
            .into_iter()
            .find(|p| p.harness_id == Some(harness))
            .and_then(|p| p.models.into_iter().map(|m| m.id).next())
            .unwrap_or_else(|| fallback.into())
    };
    // Startup discovery applies the host's real account state, then fills the
    // model lists; pin readiness only after it has finished.
    wait_for(
        || {
            core.registry
                .provider_instances
                .snapshot(&core.registry)
                .iter()
                .any(|p| p.harness_id == Some(HarnessId::Codex) && !p.models.is_empty())
        },
        "startup provider discovery",
    )
    .await;
    for driver in [HarnessId::Grok, HarnessId::Codex] {
        core.registry.provider_instances.set_authentication(
            driver,
            zeron_engine::provider_instances::Authentication::Authenticated,
        );
    }
    // Turn 1 completes on instance A and leaves its native conversation.
    core.workspace
        .set_chat_config(CHAT, &composer_config(HarnessId::Grok, "a-model"))
        .unwrap();
    core.doc_host
        .queue_message(CHAT, "first turn on a", vec![])
        .unwrap();
    wait_for(|| a.requests.lock().unwrap().len() == 1, "first turn on A").await;
    let _ = a.finish.send(());
    runs_completed(&core, 1).await;
    let first = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    // Turn 2 starts on B (the composer switched while idle). Exact model ids
    // are validated against the live catalog, which fills in after discovery.
    let b_model = first_model(HarnessId::Codex, "b-model");
    set_composer(&core, &composer_config(HarnessId::Codex, &b_model)).await;
    core.doc_host
        .queue_message(CHAT, "second turn on b", vec![])
        .unwrap();
    wait_for(|| b.requests.lock().unwrap().len() == 1, "second turn on B").await;
    assert_eq!(b.requests.lock().unwrap()[0].resume, None);
    let on_b = {
        wait_for(
            || {
                core.orchestration
                    .store
                    .thread(&CHAT.into())
                    .is_ok_and(|p| {
                        p.is_some_and(|p| {
                            p.runs.len() == 2
                                && p.attempts.iter().any(|a| {
                                    a.run_id == p.runs[1].id && a.provider_turn_id.is_some()
                                })
                        })
                    })
            },
            "turn 2 running on B",
        )
        .await;
        core.orchestration
            .store
            .thread(&CHAT.into())
            .unwrap()
            .unwrap()
    };
    let active = on_b.runs[1].id.clone();
    assert_ne!(
        on_b.runs[1].provider_instance_id,
        on_b.runs[0].provider_instance_id
    );
    let selected = canonical_input(&core, "third direction").await;
    let mut watch = client
        .subscribe_checked(
            methods::WATCH_QUEUE,
            json!({"chatId":CHAT,"includeCanonical":true}),
        )
        .await
        .unwrap();
    canonical_frame(&mut watch, |state| {
        state.queue.len() == 1 && state.promotion_mode.is_some()
    })
    .await;
    // Back to A mid-run: the hint is a handoff restart on the earlier instance.
    let a_model = first_model(HarnessId::Grok, "a-model");
    set_composer(&core, &composer_config(HarnessId::Grok, &a_model)).await;
    let hint = canonical_frame(&mut watch, |state| {
        state.promotion_mode == Some(QueuePromotionMode::InterruptRestartWithHandoff)
    })
    .await;
    let next = hint
        .promotion_selection
        .clone()
        .expect("reviewed selection");
    assert_eq!(next.instance_id, first.runs[0].provider_instance_id);
    assert_eq!(b.requests.lock().unwrap().len(), 1);
    assert_eq!(b.released.load(std::sync::atomic::Ordering::SeqCst), 0);
    core.sessions.mcp_server().credentials.revoke_thread(CHAT);
    let request = json!({"chatId":CHAT,"queuedRunId":selected,"clientRequestId":"back-to-a",
        "action":{"type":"promoteToRestart","targetRunId":active,"handoff":true,
            "expectedSelection":next}});
    let reply = client
        .call(methods::MUTATE_QUEUED_RUN, request.clone())
        .await
        .unwrap();
    assert!(reply["refusal"].is_null(), "{reply}");
    wait_for(
        || a.requests.lock().unwrap().len() == 2,
        "restart on instance A",
    )
    .await;
    assert_eq!(
        client
            .call(methods::MUTATE_QUEUED_RUN, request)
            .await
            .unwrap(),
        reply
    );
    assert_eq!(
        b.released.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "exactly B's old process was torn down"
    );
    assert_eq!(
        a.released.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "A's finished turn only"
    );
    let after = core
        .orchestration
        .store
        .thread(&CHAT.into())
        .unwrap()
        .unwrap();
    let run = after.runs.iter().find(|r| r.id == active).unwrap();
    assert_eq!(run.provider_instance_id, first.runs[0].provider_instance_id);
    assert_eq!(
        run.provider_thread_id, first.runs[0].provider_thread_id,
        "A's earlier native generation is reused"
    );
    let requests = a.requests.lock().unwrap().clone();
    assert_eq!(
        requests[1].resume.as_deref(),
        Some("sess-a"),
        "the accepted A conversation continues natively"
    );
    assert!(
        requests[1].prompt.contains("second turn on b")
            && requests[1].prompt.contains("partial answer on b"),
        "{}",
        requests[1].prompt
    );
    assert!(
        !requests[1].prompt.contains("first turn on a"),
        "A already holds its own earlier turn: {}",
        requests[1].prompt
    );
    let handoffs: Vec<_> = core
        .orchestration
        .store
        .thread_transfers(&CHAT.into())
        .unwrap()
        .into_iter()
        .filter(|t| t["type"] == "provider_handoff")
        .collect();
    // One handoff per provider generation of the run: B's first turn (full
    // portable context) and the return to A (only what A has not seen).
    assert_eq!(handoffs.len(), 2);
    let encoded = active.0.replace(':', "%3A");
    let by_id = |id: String| handoffs.iter().find(|t| t["id"] == id).unwrap();
    assert_eq!(
        by_id(format!("provider-handoff:{encoded}:attempt:2"))["resolution"]["strategy"],
        "delta_context"
    );
    assert_eq!(
        by_id(format!("provider-handoff:{encoded}"))["resolution"]["strategy"],
        "portable_context"
    );
    assert!(core.orchestration.store.verify_projections().unwrap());
    let _ = a.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_queued_run_that_starts_on_a_new_generation_does_not_resume_the_remembered_session() {
    let (core, harness, prompts) = setup(SteeringMode::TurnBoundary).await;
    core.workspace
        .set_chat_config(CHAT, &composer_config(HarnessId::Mock, "model-a"))
        .unwrap();
    core.doc_host
        .queue_message(CHAT, "first question", vec![])
        .unwrap();
    running_canonical(&core).await;
    // The selection moves while the first turn runs; the follow-up is queued on
    // the new selection and starts through the runner when the turn ends.
    set_composer(&core, &composer_config(HarnessId::Mock, "model-b")).await;
    canonical_input(&core, "queued follow-up").await;
    assert_eq!(prompts.lock().unwrap().len(), 1);
    let _ = harness.finish.send(());
    wait_for(|| prompts.lock().unwrap().len() == 2, "queued run start").await;
    {
        let requests = harness.requests.lock().unwrap();
        assert_eq!(requests[1].model.as_deref(), Some("model-b"));
        assert_eq!(
            requests[1].resume, None,
            "a rebuilt generation must not resume the remembered native session"
        );
        assert!(
            requests[1].prompt.contains("first question"),
            "{}",
            requests[1].prompt
        );
    }
    let _ = harness.finish.send(());
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn overlapping_composer_writes_leave_the_thread_on_the_chat_rows_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut harness, _prompts) = HeldHarness::new(SteeringMode::TurnBoundary);
    Arc::get_mut(&mut harness).unwrap().id = HarnessId::Codex;
    let registry = HarnessRegistry::new();
    registry.register(harness.clone());
    let core = EngineCore::assemble(
        &tmp.keep().join("data"),
        Arc::new(registry),
        HarnessId::Codex,
        None,
    )
    .unwrap();
    create_chat(&core).await;
    wait_for(
        || {
            core.registry
                .provider_instances
                .snapshot(&core.registry)
                .iter()
                .any(|p| p.harness_id == Some(HarnessId::Codex) && !p.models.is_empty())
        },
        "startup provider discovery",
    )
    .await;
    core.registry.provider_instances.set_authentication(
        HarnessId::Codex,
        zeron_engine::provider_instances::Authentication::Authenticated,
    );
    let advertised: Vec<String> = core
        .registry
        .provider_instances
        .snapshot(&core.registry)
        .into_iter()
        .find(|p| p.harness_id == Some(HarnessId::Codex))
        .unwrap()
        .models
        .into_iter()
        .map(|m| m.id)
        .collect();
    assert!(advertised.len() >= 3, "{advertised:?}");
    for round in 0..8 {
        let models = [
            &advertised[round % 3],
            &advertised[(round + 1) % 3],
            &advertised[(round + 2) % 3],
        ];
        let configs = models.map(|m| composer_config(HarnessId::Codex, m));
        tokio::join!(
            set_composer(&core, &configs[0]),
            set_composer(&core, &configs[1]),
            set_composer(&core, &configs[2]),
        );
        let row = core.workspace.chat_config(CHAT).expect("chat row config");
        let thread = core
            .orchestration
            .store
            .thread(&CHAT.into())
            .unwrap()
            .unwrap()
            .thread
            .model_selection;
        assert_eq!(
            Some(&*thread.model),
            row.model.as_deref(),
            "round {round}: thread selection must follow the chat row"
        );
    }
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn composer_selection_sync_is_catalog_validated_idempotent_and_never_starts_a_turn() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut harness, prompts) = HeldHarness::new(SteeringMode::TurnBoundary);
    Arc::get_mut(&mut harness).unwrap().id = HarnessId::Codex;
    let registry = HarnessRegistry::new();
    registry.register(harness.clone());
    let core = EngineCore::assemble(
        &tmp.keep().join("data"),
        Arc::new(registry),
        HarnessId::Codex,
        None,
    )
    .unwrap();
    create_chat(&core).await;
    wait_for(
        || {
            core.registry
                .provider_instances
                .snapshot(&core.registry)
                .iter()
                .any(|p| p.harness_id == Some(HarnessId::Codex) && !p.models.is_empty())
        },
        "startup provider discovery",
    )
    .await;
    core.registry.provider_instances.set_authentication(
        HarnessId::Codex,
        zeron_engine::provider_instances::Authentication::Authenticated,
    );
    let advertised: Vec<String> = core
        .registry
        .provider_instances
        .snapshot(&core.registry)
        .into_iter()
        .find(|p| p.harness_id == Some(HarnessId::Codex))
        .unwrap()
        .models
        .into_iter()
        .map(|m| m.id)
        .collect();
    assert!(advertised.len() >= 2, "{advertised:?}");
    let selection = |core: &EngineCore| {
        core.orchestration
            .store
            .thread(&CHAT.into())
            .unwrap()
            .map(|p| p.thread.model_selection)
    };
    // The thread exists from chat creation; the composer's pick lands on it
    // before any turn has been admitted.
    set_composer(&core, &composer_config(HarnessId::Codex, &advertised[1])).await;
    assert_eq!(&*selection(&core).unwrap().model, advertised[1]);
    set_composer(&core, &composer_config(HarnessId::Codex, &advertised[0])).await;
    assert_eq!(&*selection(&core).unwrap().model, advertised[0]);
    core.doc_host
        .queue_message(CHAT, "opening", vec![])
        .unwrap();
    wait_for(|| prompts.lock().unwrap().len() == 1, "first turn").await;
    assert_eq!(&*selection(&core).unwrap().model, advertised[0]);
    // A model the live catalog does not advertise never becomes the saved
    // selection (the chat row stays LWW, admission re-validates).
    set_composer(
        &core,
        &composer_config(HarnessId::Codex, "not-a-real-model"),
    )
    .await;
    assert_eq!(&*selection(&core).unwrap().model, advertised[0]);
    set_composer(&core, &composer_config(HarnessId::Codex, &advertised[1])).await;
    assert_eq!(&*selection(&core).unwrap().model, advertised[1]);
    // Re-sending the same config (a retry after response loss, or an unrelated
    // chat-config edit) appends nothing.
    let frontier = core.orchestration.store.projection_frontier().unwrap();
    set_composer(&core, &composer_config(HarnessId::Codex, &advertised[1])).await;
    assert_eq!(
        core.orchestration.store.projection_frontier().unwrap(),
        frontier
    );
    // Returning to an earlier selection is a new change, not a replayed receipt.
    set_composer(&core, &composer_config(HarnessId::Codex, &advertised[0])).await;
    assert_eq!(&*selection(&core).unwrap().model, advertised[0]);
    assert_eq!(
        prompts.lock().unwrap().len(),
        1,
        "syncing the selection never starts or restarts a turn"
    );
    assert!(core.orchestration.store.verify_projections().unwrap());
    let _ = harness.finish.send(());
    core.shutdown().await;
}
