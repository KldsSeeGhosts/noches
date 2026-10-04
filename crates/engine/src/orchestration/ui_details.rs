//! Narrow owner-user adapters to existing PR and launch services.
use super::service::CallerScope;
use serde::Deserialize;
use serde_json::Value;
use zeron_rpc::{RpcError, RpcReply};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PullRequestChange {
    pub chat_id: String,
    pub target: zeron_proto::orchestration_mcp::LinkPullRequestInput,
    pub watching: Option<bool>,
}

pub(crate) async fn change_pr(
    service: &super::pull_requests::PullRequestService,
    workspace: &crate::WorkspaceHost,
    p: PullRequestChange,
) -> Result<RpcReply, RpcError> {
    let projection = service
        .kernel
        .store
        .thread(&p.chat_id.clone().into())
        .map_err(|e| RpcError::Failed(e.to_string()))?
        .ok_or_else(|| RpcError::Failed(format!("Thread '{}' was not found.", p.chat_id)))?;
    let t = projection.thread;
    if t.deleted_at.is_some() {
        return Err(RpcError::Failed(format!(
            "Thread '{}' was not found.",
            p.chat_id
        )));
    }
    let root = t
        .worktree_path
        .clone()
        .or(service
            .kernel
            .store
            .launch_projects()
            .map_err(|e| RpcError::Failed(e.to_string()))?
            .projects
            .into_iter()
            .find(|p| p["id"].as_str() == Some(t.project_id.0.as_str()))
            .and_then(|p| p["workspaceRoot"].as_str().map(str::to_string)))
        .or_else(|| {
            workspace
                .read_spaces()
                .ok()?
                .into_iter()
                .find(|s| s.id == t.project_id.0 && s.device_id == workspace.device_id())
                .map(|s| s.path)
        })
        .ok_or_else(|| RpcError::Failed("The thread project was not found.".into()))?;
    let caller = CallerScope {
        thread_id: t.id,
        run_id: "ui:details".to_string().into(),
        session_id: "ui:details".into(),
        project_id: t.project_id,
        workspace_root: root.into(),
        runtime_mode: t.runtime_mode,
        interaction_mode: t.interaction_mode,
        provider_instance_id: t.provider_instance_id,
    };
    let name = match p.watching {
        None => "link_pull_request",
        Some(true) => "watch_pull_request",
        Some(false) => "unwatch_pull_request",
    };
    let value = super::pull_requests::mcp::invoke_for_user(
        service,
        &caller,
        name,
        serde_json::to_value(p.target).map_err(|e| RpcError::BadParams(e.to_string()))?,
    )
    .await
    .map_err(|e| RpcError::Failed(e.message))?;
    RpcReply::value(&value)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeHandoff {
    pub chat_id: String,
    pub input: zeron_proto::orchestration_mcp::T3WorktreeHandoffInput,
}

pub(crate) async fn handoff(
    service: &super::launch::HostLaunchService,
    p: WorktreeHandoff,
) -> Result<RpcReply, RpcError> {
    let value: Value = service
        .handoff_for_user(
            &p.chat_id,
            serde_json::to_value(p.input).map_err(|e| RpcError::BadParams(e.to_string()))?,
        )
        .await;
    if let Some(message) = value.get("message").and_then(Value::as_str) {
        return Err(RpcError::Failed(message.into()));
    }
    RpcReply::value(&value)
}
