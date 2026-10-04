//! Desktop F1 contract. Mutations are never inferred from preview or scan.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GitCheckout {
    pub checkout_id: String,
    pub cwd: String,
    pub branch: String,
    pub head: String,
    /// Exact staged tree, not merely a list of filenames.
    pub staged_tree: String,
    pub staged_paths: Vec<String>,
    pub dirty: bool,
    pub upstream: Option<String>,
    pub remotes: Vec<GitRemoteChoice>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GitRemoteChoice {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SourceControlSettings {
    pub style: WritingStyle,
    pub custom_instructions: String,
    /// Explicit provider instance, from ListProviderInstances.
    pub provider_instance_id: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WritingStyle {
    #[default]
    RepoConventions,
    ConventionalCommits,
    Custom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GitPreviewRequest {
    pub thread_id: String,
    pub cwd: String,
    /// None generates; supplied text is previewed without calling a model.
    pub commit_message: Option<String>,
    pub pr_title: Option<String>,
    pub pr_body: Option<String>,
    pub base_branch: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GitMessagePreview {
    pub preview_id: String,
    pub thread_id: String,
    pub checkout: GitCheckout,
    pub commit_message: String,
    pub pr_title: String,
    pub pr_body: String,
    pub base_branch: String,
    pub settings: SourceControlSettings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GitActionRequest {
    /// Durable user-generated idempotency key. Changed payload is refused.
    pub request_id: String,
    pub preview_id: String,
    pub confirmed_checkout: GitCheckout,
    pub authorize_commit: bool,
    /// Independent authorization; a commit never implies permission to push.
    pub authorize_push: bool,
    pub authorize_create_pr: bool,
    pub push_remote: Option<GitRemoteChoice>,
    /// Explicit https://host/owner/repo, matching one inspected Git remote.
    pub pr_repository: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GitProgress {
    pub sequence: u64,
    pub phase: String,
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GitActionState {
    pub action_id: String,
    pub thread_id: String,
    /// running | completed | failed | uncertain (never automatically replayed).
    pub status: String,
    pub commit: Option<String>,
    pub pr_url: Option<String>,
    pub pr_linked: bool,
    pub error: Option<String>,
    pub progress: Vec<GitProgress>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PullPolicy {
    pub space_id: String,
    pub enabled: bool,
    pub remote: GitRemoteChoice,
    pub default_branch: String,
    /// Clamped to 60..3600 seconds, plus deterministic bounded jitter.
    pub cadence_seconds: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PullState {
    pub policy: PullPolicy,
    pub last_checked_at: Option<i64>,
    pub next_check_at: Option<i64>,
    pub last_skip_reason: Option<String>,
    pub last_error: Option<String>,
    pub last_result: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistorySource {
    ClaudeCode,
    #[default]
    Codex,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryProvenance {
    pub source: HistorySource,
    pub native_session_id: String,
    pub file_path: String,
    pub sha256: String,
    pub project_root: String,
    pub truncated: bool,
    pub unsupported_content: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryMessage {
    pub role: String,
    pub text: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryPreview {
    pub candidate_id: String,
    pub provenance: HistoryProvenance,
    pub title: String,
    pub messages: Vec<HistoryMessage>,
    pub already_imported_chat_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryScanRequest {
    pub space_id: String,
    /// Returned scanId resumes the same bounded discovery snapshot.
    pub scan_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryScanState {
    pub scan_id: String,
    pub space_id: String,
    /// running | paused | completed | cancelled | failed
    pub status: String,
    pub scanned_files: usize,
    pub total_files: usize,
    pub bytes_read: u64,
    pub truncated: bool,
    pub candidate_ids: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryImportRequest {
    pub space_id: String,
    pub candidate_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryImportState {
    pub import_id: String,
    pub status: String,
    pub completed_candidate_ids: Vec<String>,
    pub chat_ids: Vec<String>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn additive_contracts_default_without_granting_authority() {
        let action: GitActionRequest = serde_json::from_str("{}").unwrap();
        assert!(!action.authorize_commit && !action.authorize_push && !action.authorize_create_pr);
        let policy: PullPolicy = serde_json::from_str("{}").unwrap();
        assert!(!policy.enabled);
        let preview: HistoryPreview = serde_json::from_str("{}").unwrap();
        assert!(preview.messages.is_empty());
        let settings: SourceControlSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.style, WritingStyle::RepoConventions);
    }

    #[test]
    fn ui_json_names_and_provenance_round_trip() {
        let request: GitActionRequest = serde_json::from_value(serde_json::json!({
            "requestId":"user-key", "previewId":"preview", "confirmedCheckout":{
                "checkoutId":"worktree", "stagedTree":"tree", "stagedPaths":["a b.rs"]
            }, "authorizeCommit":true
        }))
        .unwrap();
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["confirmedCheckout"]["stagedPaths"][0], "a b.rs");
        assert_eq!(value["authorizePush"], false);
        let source = serde_json::to_value(HistoryProvenance {
            source: HistorySource::ClaudeCode,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(source["source"], "claude_code");
        assert_eq!(source["unsupportedContent"], false);
    }
}
