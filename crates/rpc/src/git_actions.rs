//! Native Changes/settings/history APIs; all operate on the addressed host.
pub mod methods {
    pub const GET_GIT_STATUS: &str = "GetGitActionStatus";
    pub const PREVIEW_GIT: &str = "PreviewGitAction";
    pub const START_GIT: &str = "StartGitAction";
    pub const GET_GIT_ACTION: &str = "GetGitAction";
    pub const WATCH_GIT_ACTION: &str = "WatchGitAction";
    pub const GET_WRITER: &str = "GetSourceControlSettings";
    pub const SET_WRITER: &str = "SetSourceControlSettings";
    pub const GET_PULL: &str = "GetDefaultBranchPull";
    pub const SET_PULL: &str = "SetDefaultBranchPull";
    pub const RETRY_PULL: &str = "RetryDefaultBranchPull";
    pub const SCAN_HISTORY: &str = "ScanCliHistory";
    pub const WATCH_SCAN: &str = "WatchCliHistoryScan";
    pub const GET_SCAN: &str = "GetCliHistoryScan";
    pub const PREVIEW_HISTORY: &str = "PreviewCliHistory";
    pub const IMPORT_HISTORY: &str = "ImportCliHistory";
    pub const WATCH_IMPORT: &str = "WatchCliHistoryImport";
    pub const GET_IMPORT: &str = "GetCliHistoryImport";
    pub const CANCEL_HISTORY: &str = "CancelCliHistory";
    pub const CONTINUE_HISTORY: &str = "ContinueCliHistory";

    pub fn handles(method: &str) -> bool {
        [
            GET_GIT_STATUS,
            PREVIEW_GIT,
            START_GIT,
            GET_GIT_ACTION,
            WATCH_GIT_ACTION,
            GET_WRITER,
            SET_WRITER,
            GET_PULL,
            SET_PULL,
            RETRY_PULL,
            SCAN_HISTORY,
            WATCH_SCAN,
            GET_SCAN,
            PREVIEW_HISTORY,
            IMPORT_HISTORY,
            WATCH_IMPORT,
            GET_IMPORT,
            CANCEL_HISTORY,
            CONTINUE_HISTORY,
        ]
        .contains(&method)
    }

    pub fn is_stream(method: &str) -> bool {
        [WATCH_GIT_ACTION, WATCH_SCAN, WATCH_IMPORT].contains(&method)
    }
}

use crate::{RpcClient, RpcError, RpcSubscription};
use zeron_proto::git_actions::*;

impl RpcClient {
    pub async fn git_action_status(&self, cwd: &str) -> Result<GitCheckout, RpcError> {
        self.call_as(methods::GET_GIT_STATUS, serde_json::json!({"cwd":cwd}))
            .await
    }
    pub async fn preview_git_action(
        &self,
        request: &GitPreviewRequest,
    ) -> Result<GitMessagePreview, RpcError> {
        self.call_as(
            methods::PREVIEW_GIT,
            serde_json::to_value(request).map_err(|e| RpcError::BadParams(e.to_string()))?,
        )
        .await
    }
    pub async fn start_git_action(
        &self,
        request: &GitActionRequest,
    ) -> Result<GitActionState, RpcError> {
        self.call_as(
            methods::START_GIT,
            serde_json::to_value(request).map_err(|e| RpcError::BadParams(e.to_string()))?,
        )
        .await
    }
    pub async fn get_git_action(&self, action_id: &str) -> Result<GitActionState, RpcError> {
        self.call_as(
            methods::GET_GIT_ACTION,
            serde_json::json!({"actionId":action_id}),
        )
        .await
    }
    pub async fn watch_git_action(&self, action_id: &str) -> Result<RpcSubscription, RpcError> {
        self.subscribe_checked(
            methods::WATCH_GIT_ACTION,
            serde_json::json!({"actionId":action_id}),
        )
        .await
    }
    pub async fn source_control_settings(&self) -> Result<SourceControlSettings, RpcError> {
        self.call_as(methods::GET_WRITER, serde_json::json!({}))
            .await
    }
    pub async fn set_source_control_settings(
        &self,
        settings: &SourceControlSettings,
    ) -> Result<(), RpcError> {
        self.call_as(
            methods::SET_WRITER,
            serde_json::to_value(settings).map_err(|e| RpcError::BadParams(e.to_string()))?,
        )
        .await
    }
    pub async fn default_branch_pull(&self, space_id: &str) -> Result<PullState, RpcError> {
        self.call_as(methods::GET_PULL, serde_json::json!({"spaceId":space_id}))
            .await
    }
    pub async fn set_default_branch_pull(
        &self,
        policy: &PullPolicy,
    ) -> Result<PullState, RpcError> {
        self.call_as(
            methods::SET_PULL,
            serde_json::to_value(policy).map_err(|e| RpcError::BadParams(e.to_string()))?,
        )
        .await
    }
    pub async fn retry_default_branch_pull(&self, space_id: &str) -> Result<PullState, RpcError> {
        self.call_as(methods::RETRY_PULL, serde_json::json!({"spaceId":space_id}))
            .await
    }
    pub async fn scan_cli_history(
        &self,
        request: &HistoryScanRequest,
    ) -> Result<HistoryScanState, RpcError> {
        self.call_as(
            methods::SCAN_HISTORY,
            serde_json::to_value(request).map_err(|e| RpcError::BadParams(e.to_string()))?,
        )
        .await
    }
    pub async fn cli_history_scan(&self, scan_id: &str) -> Result<HistoryScanState, RpcError> {
        self.call_as(methods::GET_SCAN, serde_json::json!({"scanId":scan_id}))
            .await
    }
    pub async fn watch_cli_history_scan(&self, scan_id: &str) -> Result<RpcSubscription, RpcError> {
        self.subscribe_checked(methods::WATCH_SCAN, serde_json::json!({"scanId":scan_id}))
            .await
    }
    pub async fn preview_cli_history(
        &self,
        candidate_id: &str,
    ) -> Result<HistoryPreview, RpcError> {
        self.call_as(
            methods::PREVIEW_HISTORY,
            serde_json::json!({"candidateId":candidate_id}),
        )
        .await
    }
    pub async fn import_cli_history(
        &self,
        request: &HistoryImportRequest,
    ) -> Result<HistoryImportState, RpcError> {
        self.call_as(
            methods::IMPORT_HISTORY,
            serde_json::to_value(request).map_err(|e| RpcError::BadParams(e.to_string()))?,
        )
        .await
    }
    pub async fn cli_history_import(
        &self,
        import_id: &str,
    ) -> Result<HistoryImportState, RpcError> {
        self.call_as(
            methods::GET_IMPORT,
            serde_json::json!({"importId":import_id}),
        )
        .await
    }
    pub async fn watch_cli_history_import(
        &self,
        import_id: &str,
    ) -> Result<RpcSubscription, RpcError> {
        self.subscribe_checked(
            methods::WATCH_IMPORT,
            serde_json::json!({"importId":import_id}),
        )
        .await
    }
    pub async fn cancel_cli_history(&self, operation_id: &str) -> Result<(), RpcError> {
        self.call_as(
            methods::CANCEL_HISTORY,
            serde_json::json!({"operationId":operation_id}),
        )
        .await
    }
    pub async fn continue_cli_history(
        &self,
        space_id: &str,
        candidate_id: &str,
        chat_id: &str,
    ) -> Result<(), RpcError> {
        self.call_as(
            methods::CONTINUE_HISTORY,
            serde_json::json!({"spaceId":space_id,"candidateId":candidate_id,"chatId":chat_id}),
        )
        .await
    }
}
