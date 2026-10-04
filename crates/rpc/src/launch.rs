//! Typed UI read/control client. Optional owner routing is explicit.
use crate::{RpcClient, RpcError, methods};
use serde_json::json;
use zeron_proto::launch::{LaunchProjects, LaunchUiState, SetupControlParams};

impl RpcClient {
    pub async fn launch_projects(
        &self,
        target_device_id: Option<&str>,
    ) -> Result<LaunchProjects, RpcError> {
        self.call_as(
            methods::LIST_LAUNCH_PROJECTS,
            json!({"targetDeviceId":target_device_id}),
        )
        .await
    }
    pub async fn launch_state(
        &self,
        chat_id: &str,
        target_device_id: Option<&str>,
    ) -> Result<Option<LaunchUiState>, RpcError> {
        self.call_as(
            methods::GET_LAUNCH_STATE,
            json!({"chatId":chat_id,"targetDeviceId":target_device_id}),
        )
        .await
    }
    pub async fn control_worktree_setup(
        &self,
        params: SetupControlParams,
        target_device_id: Option<&str>,
    ) -> Result<(), RpcError> {
        let mut p = serde_json::to_value(params).map_err(|e| RpcError::BadParams(e.to_string()))?;
        if let Some(id) = target_device_id {
            p["targetDeviceId"] = json!(id);
        }
        self.call(methods::CONTROL_WORKTREE_SETUP, p).await?;
        Ok(())
    }
}
