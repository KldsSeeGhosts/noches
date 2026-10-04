use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use zeron_proto::transfer::{CheckpointPreviewParams, CheckpointRestoreParams};
use zeron_rpc::{RpcError, RpcReply, RpcService, methods};

struct DetailsApi;
#[async_trait]
impl RpcService for DetailsApi {
    async fn handle(&self, method: &str, params: Value) -> Result<RpcReply, RpcError> {
        assert_eq!(params["targetDeviceId"], "owner");
        assert_eq!(params["chatId"], "chat");
        let value = match method {
            methods::GET_THREAD_PULL_REQUESTS => json!({"threadId":"chat"}),
            methods::GET_THREAD_TRANSFER_STATE => json!({"threadId":"chat"}),
            methods::GET_LAUNCH_STATE => Value::Null,
            methods::CONTROL_WORKTREE_SETUP => {
                assert_eq!(params["runId"], "exact-run");
                assert_eq!(params["action"], "retry");
                json!({"accepted":true})
            }
            methods::PREVIEW_FILE_CHECKPOINT_RESTORE => {
                assert_eq!(params["checkpointId"], "checkpoint");
                json!({"checkpointId":"checkpoint","headSha":null,"checksum":"opaque","allowed":false,"refusal":"Main checkout."})
            }
            methods::RESTORE_FILE_CHECKPOINT => {
                assert_eq!(params["checkpointId"], "checkpoint");
                assert_eq!(params["expectedHeadSha"], Value::Null);
                assert_eq!(params["expectedChecksum"], "opaque");
                json!({"restored":true,"backupCheckpointId":"backup"})
            }
            methods::CHANGE_THREAD_PULL_REQUEST => {
                assert_eq!(params["target"]["url"], "bad");
                return Err(RpcError::Failed("This is not a recognised pull request URL. Pass repository and number instead.".into()));
            }
            methods::HANDOFF_THREAD_WORKTREE => {
                assert_eq!(params["input"]["branch"], "feature");
                assert!(params["input"].get("baseRef").is_none());
                json!({"threadId":"chat","branch":"feature","worktreePath":"/worktree"})
            }
            _ => panic!("unexpected method {method}"),
        };
        Ok(RpcReply::Value(value))
    }
}

#[tokio::test]
async fn typed_details_calls_route_every_operation_and_preserve_guards_and_errors() {
    tokio::time::timeout(std::time::Duration::from_secs(5), details_roundtrip())
        .await
        .expect("details RPCs must finish");
}

async fn details_roundtrip() {
    let client = zeron_rpc::memory_client(Arc::new(DetailsApi));
    client
        .thread_pull_requests_on("chat", "owner")
        .await
        .unwrap();
    client.thread_transfer_state("chat", "owner").await.unwrap();
    assert!(
        client
            .launch_state("chat", Some("owner"))
            .await
            .unwrap()
            .is_none()
    );
    client
        .control_worktree_setup(
            zeron_proto::launch::SetupControlParams {
                chat_id: "chat".into(),
                run_id: "exact-run".into(),
                action: "retry".into(),
            },
            Some("owner"),
        )
        .await
        .unwrap();
    let preview = client
        .preview_file_checkpoint_restore(
            CheckpointPreviewParams {
                chat_id: "chat".into(),
                checkpoint_id: "checkpoint".into(),
            },
            "owner",
        )
        .await
        .unwrap();
    assert!(!preview.allowed);
    assert_eq!(preview.refusal.as_deref(), Some("Main checkout."));
    let restored = client
        .restore_file_checkpoint(
            CheckpointRestoreParams {
                chat_id: "chat".into(),
                checkpoint_id: preview.checkpoint_id,
                expected_head_sha: preview.head_sha,
                expected_checksum: preview.checksum,
            },
            "owner",
        )
        .await
        .unwrap();
    assert_eq!(restored.backup_checkpoint_id, "backup");
    for watching in [None, Some(true), Some(false)] {
        let error = client
            .change_thread_pull_request(
                "chat",
                "owner",
                serde_json::from_value(json!({"url":"bad"})).unwrap(),
                watching,
            )
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "This is not a recognised pull request URL. Pass repository and number instead."
        );
    }
    client
        .handoff_thread_worktree(
            "chat",
            "owner",
            serde_json::from_value(json!({"branch":"feature"})).unwrap(),
        )
        .await
        .unwrap();
}
