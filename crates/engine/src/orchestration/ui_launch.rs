//! Stable passive read API for the wave-3 designer.
use super::{Result, Store, projection::ThreadProjection};
use zeron_proto::launch::{LaunchProjects, LaunchUiState};
use zeron_proto::orchestration::ThreadId;

impl Store {
    pub(crate) fn launch_attachment_path(&self, id: &str, thread: &str) -> Result<Option<String>> {
        self.read(|conn| {
            use rusqlite::OptionalExtension;
            let raw: Option<String> = conn
                .query_row(
                    "SELECT payload FROM orchestration_launch_claims WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?;
            let row = raw
                .map(|s| serde_json::from_str::<serde_json::Value>(&s))
                .transpose()?;
            Ok(row
                .filter(|r| r["threadId"] == thread)
                .and_then(|r| r["path"].as_str().map(str::to_owned)))
        })
    }
    pub(crate) fn launch_threads(&self) -> Result<Vec<ThreadProjection>> {
        self.read(|conn| {
            let mut stmt =
                conn.prepare("SELECT id FROM orchestration_projection_threads ORDER BY rowid")?;
            let ids = stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids.into_iter()
                .filter_map(|id| super::projection::read_thread(conn, &ThreadId(id)).transpose())
                .collect()
        })
    }
    pub fn launch_projects(&self) -> Result<LaunchProjects> {
        self.read(|conn| {
            let mut stmt =
                conn.prepare("SELECT payload FROM orchestration_launch_projects ORDER BY rowid")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let projects = rows
                .map(|r| Ok(serde_json::from_str::<serde_json::Value>(&r?)?))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .filter(|p| p["deletedAt"].is_null())
                .collect();
            Ok(LaunchProjects { projects })
        })
    }
    pub fn launch_state(&self, id: &ThreadId) -> Result<Option<LaunchUiState>> {
        self.read(|conn| {
            use rusqlite::OptionalExtension;
            let raw: Option<String> = conn
                .query_row(
                    "SELECT payload FROM orchestration_launch_workflows WHERE thread_id=?1",
                    [&id.0],
                    |r| r.get(0),
                )
                .optional()?;
            let mut state = raw
                .map(|s| serde_json::from_str::<LaunchUiState>(&s))
                .transpose()?;
            if let Some(p) = super::projection::read_thread(conn, id)? {
                let state = state.get_or_insert_with(|| LaunchUiState {
                    thread_id: id.0.clone(),
                    project_id: p.thread.project_id.0.clone(),
                    status: "ready".into(),
                    ..Default::default()
                });
                state.branch = p.thread.branch;
                state.worktree_path = p.thread.worktree_path;
            }
            Ok(state)
        })
    }
}
