//! Passive desktop/companion read contract. These APIs never acknowledge mail.
use serde::{Deserialize, Serialize};

use crate::orchestration::ProjectId;
use crate::orchestration_mcp::{
    OrchestratorMcpThreadListItem, T3ThreadReadInput, T3ThreadReadResult,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummariesRequest {
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummaries {
    #[serde(default)]
    pub threads: Vec<OrchestratorMcpThreadListItem>,
}

pub type ThreadTimelineRequest = T3ThreadReadInput;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadTimeline {
    #[serde(default)]
    pub version: i64,
    pub page: T3ThreadReadResult,
}
