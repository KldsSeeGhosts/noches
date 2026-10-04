//! Passive designer-facing contract. Files-only restore never rewinds a run.
use crate::orchestration::{OrchestrationV2Checkpoint, OrchestrationV2CheckpointScope, ThreadId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCheckpoint {
    pub checkpoint: OrchestrationV2Checkpoint,
    pub scope: OrchestrationV2CheckpointScope,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub head_sha: Option<String>,
    #[serde(default)]
    pub tree_sha: String,
    #[serde(default)]
    pub index_tree_sha: String,
    #[serde(default)]
    pub phase: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePath {
    #[serde(default)]
    pub path: String,
    /// A/M/D/T relative to the currently observed workspace.
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    #[serde(default)]
    pub checkpoint_id: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub head_sha: Option<String>,
    #[serde(default)]
    pub checksum: String,
    #[serde(default)]
    pub paths: Vec<RestorePath>,
    #[serde(default)]
    pub restores_staging: bool,
    #[serde(default)]
    pub allowed: bool,
    #[serde(default)]
    pub refusal: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    #[serde(default)]
    pub restored: bool,
    #[serde(default)]
    pub backup_checkpoint_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadTransferState {
    pub thread_id: ThreadId,
    #[serde(default)]
    pub version: i64,
    #[serde(default)]
    pub lineage: Value,
    #[serde(default)]
    pub forked_from: Option<Value>,
    #[serde(default)]
    pub transfers: Vec<Value>,
    /// Full private history/delivery contents are intentionally absent.
    #[serde(default)]
    pub handoffs: Vec<Value>,
    #[serde(default)]
    pub checkpoints: Vec<FileCheckpoint>,
    #[serde(default)]
    pub inherited_items: Vec<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferStateParams {
    #[serde(default)]
    pub chat_id: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointPreviewParams {
    #[serde(default)]
    pub chat_id: String,
    #[serde(default)]
    pub checkpoint_id: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointRestoreParams {
    #[serde(default)]
    pub chat_id: String,
    #[serde(default)]
    pub checkpoint_id: String,
    #[serde(default)]
    pub expected_head_sha: Option<String>,
    #[serde(default)]
    pub expected_checksum: String,
}
