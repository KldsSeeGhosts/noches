//! Passive F1 read API for desktop/replica adapters. This never starts work,
//! acknowledges progress, mutates Git or reads a CLI history home.
pub use zeron_proto::git_actions::{
    GitActionState, GitCheckout, GitMessagePreview, GitProgress, HistoryImportState,
    HistoryPreview, HistoryScanState, PullState, SourceControlSettings,
};

use super::{Store, git_actions::history::ScanJob, git_actions::persistence::get};

impl Store {
    pub fn git_action_state(&self, action_id: &str) -> anyhow::Result<Option<GitActionState>> {
        get(self, "action", action_id)
    }
    pub fn git_message_preview(
        &self,
        preview_id: &str,
    ) -> anyhow::Result<Option<GitMessagePreview>> {
        get(self, "preview", preview_id)
    }
    pub fn default_branch_pull_state(&self, space_id: &str) -> anyhow::Result<Option<PullState>> {
        get(self, "pull", space_id)
    }
    pub fn cli_history_scan_state(
        &self,
        scan_id: &str,
    ) -> anyhow::Result<Option<HistoryScanState>> {
        Ok(get::<ScanJob>(self, "scan", scan_id)?.map(|job| job.state))
    }
    pub fn cli_history_preview(
        &self,
        candidate_id: &str,
    ) -> anyhow::Result<Option<HistoryPreview>> {
        get(self, "candidate", candidate_id)
    }
    pub fn cli_history_import_state(
        &self,
        import_id: &str,
    ) -> anyhow::Result<Option<HistoryImportState>> {
        get(self, "import", import_id)
    }
    pub fn source_control_settings(&self) -> anyhow::Result<SourceControlSettings> {
        Ok(get(self, "settings", "writer")?.unwrap_or_default())
    }
}
