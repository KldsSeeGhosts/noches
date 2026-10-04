//! Passive PR-chip read model. MCP schemas stay pinned in orchestration_mcp.
use crate::orchestration::{PullRequestChecksState, PullRequestState, ThreadPullRequestLinkSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThreadPullRequestsUi {
    pub thread_id: String,
    pub version: i64,
    pub pull_requests: Vec<PullRequestUiEntry>,
    pub chains: Vec<PullRequestUiChain>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PullRequestUiEntry {
    pub host: String,
    pub repository: String,
    pub number: i64,
    pub url: String,
    pub source: Option<ThreadPullRequestLinkSource>,
    pub watching: bool,
    pub state: Option<PullRequestState>,
    pub title: Option<String>,
    pub head_branch: Option<String>,
    pub base_branch: Option<String>,
    pub is_draft: Option<bool>,
    pub checks_state: Option<PullRequestChecksState>,
    pub stack: Option<PullRequestUiStack>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PullRequestUiStack {
    pub kind: String,
    pub position: i64,
    pub size: i64,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PullRequestUiChain {
    pub kind: String,
    pub numbers: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_additive_fields_decode_for_older_replicas() {
        let state: ThreadPullRequestsUi =
            serde_json::from_value(serde_json::json!({"threadId":"thread"})).unwrap();
        assert_eq!(state.version, 0);
        assert!(state.pull_requests.is_empty());
        let entry: PullRequestUiEntry =
            serde_json::from_value(serde_json::json!({"number":123,"state":"open"})).unwrap();
        assert!(!entry.watching);
        assert!(entry.checks_state.is_none());
    }
    #[test]
    fn chip_json_keeps_camel_case_and_bottom_to_top_stack_order() {
        let state = ThreadPullRequestsUi {
            thread_id: "thread".into(),
            version: 42,
            pull_requests: vec![PullRequestUiEntry {
                number: 2,
                head_branch: Some("layer-two".into()),
                watching: true,
                checks_state: Some(PullRequestChecksState::Pending),
                stack: Some(PullRequestUiStack {
                    kind: "derived".into(),
                    position: 2,
                    size: 2,
                }),
                ..Default::default()
            }],
            chains: vec![PullRequestUiChain {
                kind: "derived".into(),
                numbers: vec![1, 2],
            }],
        };
        let value = serde_json::to_value(&state).unwrap();
        assert_eq!(value["pullRequests"][0]["headBranch"], "layer-two");
        assert_eq!(value["pullRequests"][0]["checksState"], "pending");
        assert_eq!(value["chains"][0]["numbers"], serde_json::json!([1, 2]));
        assert_eq!(
            serde_json::from_value::<ThreadPullRequestsUi>(value).unwrap(),
            state
        );
    }
}
