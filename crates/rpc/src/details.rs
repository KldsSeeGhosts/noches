//! Owner-routed desktop details operations. Reads never acknowledge a run.
use serde_json::json;
use zeron_proto::{
    orchestration_mcp::{LinkPullRequestInput, T3WorktreeHandoffInput},
    pull_requests::ThreadPullRequestsUi,
    transfer::{
        CheckpointPreviewParams, CheckpointRestoreParams, ForkThreadParams, MergeThreadBackParams,
        RestorePreview, RestoreResult, ThreadTransferResult, ThreadTransferState,
    },
};

impl crate::RpcClient {
    pub async fn disconnect_thread_session(
        &self,
        params: zeron_proto::transfer::DisconnectThreadSessionParams,
        owner: &str,
    ) -> Result<zeron_proto::transfer::DisconnectThreadSessionResult, crate::RpcError> {
        let mut value =
            serde_json::to_value(params).map_err(|e| crate::RpcError::BadParams(e.to_string()))?;
        value["targetDeviceId"] = json!(owner);
        self.call_as(crate::methods::DISCONNECT_THREAD_SESSION, value)
            .await
    }

    pub async fn stop_thread_work(
        &self,
        params: zeron_proto::transfer::StopThreadWorkParams,
        owner: &str,
    ) -> Result<zeron_proto::transfer::StopThreadWorkResult, crate::RpcError> {
        let mut value =
            serde_json::to_value(params).map_err(|e| crate::RpcError::BadParams(e.to_string()))?;
        value["targetDeviceId"] = json!(owner);
        self.call_as(crate::methods::STOP_THREAD_WORK, value).await
    }

    pub async fn reset_thread_session(
        &self,
        params: zeron_proto::transfer::ResetThreadSessionParams,
        owner: &str,
    ) -> Result<zeron_proto::transfer::ResetThreadSessionResult, crate::RpcError> {
        let mut value =
            serde_json::to_value(params).map_err(|e| crate::RpcError::BadParams(e.to_string()))?;
        value["targetDeviceId"] = json!(owner);
        self.call_as(crate::methods::RESET_THREAD_SESSION, value)
            .await
    }

    pub async fn fork_thread(
        &self,
        params: ForkThreadParams,
        owner: &str,
    ) -> Result<ThreadTransferResult, crate::RpcError> {
        let mut value =
            serde_json::to_value(params).map_err(|e| crate::RpcError::BadParams(e.to_string()))?;
        value["targetDeviceId"] = json!(owner);
        self.call_as(crate::methods::FORK_THREAD, value).await
    }

    pub async fn merge_thread_back(
        &self,
        params: MergeThreadBackParams,
        owner: &str,
    ) -> Result<ThreadTransferResult, crate::RpcError> {
        let mut value =
            serde_json::to_value(params).map_err(|e| crate::RpcError::BadParams(e.to_string()))?;
        value["targetDeviceId"] = json!(owner);
        self.call_as(crate::methods::MERGE_THREAD_BACK, value).await
    }

    pub async fn thread_pull_requests_on(
        &self,
        chat: &str,
        owner: &str,
    ) -> Result<ThreadPullRequestsUi, crate::RpcError> {
        self.call_as(
            crate::methods::GET_THREAD_PULL_REQUESTS,
            json!({
                "chatId": chat, "targetDeviceId": owner
            }),
        )
        .await
    }

    pub async fn thread_transfer_state(
        &self,
        chat: &str,
        owner: &str,
    ) -> Result<ThreadTransferState, crate::RpcError> {
        self.call_as(
            crate::methods::GET_THREAD_TRANSFER_STATE,
            json!({
                "chatId": chat, "targetDeviceId": owner
            }),
        )
        .await
    }

    pub async fn preview_file_checkpoint_restore(
        &self,
        params: CheckpointPreviewParams,
        owner: &str,
    ) -> Result<RestorePreview, crate::RpcError> {
        let mut value =
            serde_json::to_value(params).map_err(|e| crate::RpcError::BadParams(e.to_string()))?;
        value["targetDeviceId"] = json!(owner);
        self.call_as(crate::methods::PREVIEW_FILE_CHECKPOINT_RESTORE, value)
            .await
    }

    pub async fn restore_file_checkpoint(
        &self,
        params: CheckpointRestoreParams,
        owner: &str,
    ) -> Result<RestoreResult, crate::RpcError> {
        let mut value =
            serde_json::to_value(params).map_err(|e| crate::RpcError::BadParams(e.to_string()))?;
        value["targetDeviceId"] = json!(owner);
        self.call_as(crate::methods::RESTORE_FILE_CHECKPOINT, value)
            .await
    }

    pub async fn change_thread_pull_request(
        &self,
        chat: &str,
        owner: &str,
        target: LinkPullRequestInput,
        watching: Option<bool>,
    ) -> Result<serde_json::Value, crate::RpcError> {
        self.call(
            crate::methods::CHANGE_THREAD_PULL_REQUEST,
            json!({
                "chatId": chat, "targetDeviceId": owner, "target": target, "watching": watching
            }),
        )
        .await
    }

    pub async fn handoff_thread_worktree(
        &self,
        chat: &str,
        owner: &str,
        input: T3WorktreeHandoffInput,
    ) -> Result<serde_json::Value, crate::RpcError> {
        self.call(
            crate::methods::HANDOFF_THREAD_WORKTREE,
            json!({
                "chatId": chat, "targetDeviceId": owner, "input": input
            }),
        )
        .await
    }
}
