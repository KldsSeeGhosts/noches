//! Passive host-owned launch/setup read model, additive to existing contracts.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LaunchUiState {
    pub thread_id: String,
    pub project_id: String,
    /// preparing | ready | failed | cancelled
    pub status: String,
    /// fetch | checkout | bind | setup-script | agent
    pub stage: String,
    pub branch: Option<String>,
    pub worktree_path: Option<String>,
    pub project_workspace_root: Option<String>,
    pub error: Option<String>,
    pub setup: Option<SetupRun>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SetupRun {
    pub run_id: String,
    pub terminal_id: Option<String>,
    /// pending | running | completed | failed | timed_out | cancelled | continued
    pub status: String,
    pub exit_code: Option<i32>,
    pub blocking: bool,
    pub timeout_ms: u64,
    pub script_name: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LaunchProjects {
    /// Exact T3 Project objects, not reduced Space structs.
    pub projects: Vec<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LaunchReadParams {
    pub chat_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SetupControlParams {
    pub chat_id: String,
    pub run_id: String,
    /// cancel | retry | continue
    pub action: String,
}
