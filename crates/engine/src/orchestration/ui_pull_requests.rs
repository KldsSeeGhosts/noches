//! One passive contract for local RPC, replicated uiState and sidebar/header chips.
use super::{
    Result, projection,
    pull_requests::{chains, links_of},
};
use zeron_proto::orchestration::{ThreadId, ThreadPullRequestLink};
use zeron_proto::orchestration_mcp::ListThreadPullRequestsResult;
use zeron_proto::pull_requests::*;

pub fn entries(
    links: &[ThreadPullRequestLink],
) -> (Vec<PullRequestUiEntry>, Vec<PullRequestUiChain>) {
    let chains = chains::resolve(links);
    let entries = links
        .iter()
        .filter(chains::visible)
        .map(|link| {
            let id = chains::identity(link);
            let stack = chains.iter().filter(|c| c.layers.len() > 1).find_map(|c| {
                c.layers
                    .iter()
                    .position(|l| chains::identity(l).key() == id.key())
                    .map(|i| PullRequestUiStack {
                        kind: c.kind.into(),
                        position: i as i64 + 1,
                        size: c.layers.len() as i64,
                    })
            });
            let snapshot = link.snapshot.as_ref();
            PullRequestUiEntry {
                host: id.host,
                repository: link.repository.clone(),
                number: link.number,
                url: link.url.clone(),
                source: Some(link.source.clone()),
                watching: link.watch.as_ref().is_some(),
                state: snapshot.map(|s| s.state.clone()),
                title: snapshot.map(|s| s.title.clone()),
                head_branch: snapshot.map(|s| s.head_branch.clone()),
                base_branch: snapshot.map(|s| s.base_branch.clone()),
                is_draft: snapshot.map(|s| s.is_draft),
                checks_state: snapshot.and_then(|s| s.checks_state.as_ref().cloned().flatten()),
                stack,
            }
        })
        .collect();
    (
        entries,
        chains
            .iter()
            .map(|c| PullRequestUiChain {
                kind: c.kind.into(),
                numbers: c.layers.iter().map(|l| l.number).collect(),
            })
            .collect(),
    )
}

pub fn mcp_list(links: &[ThreadPullRequestLink]) -> ListThreadPullRequestsResult {
    let (entries, chains) = entries(links);
    let entries: Vec<_> = entries
        .into_iter()
        .map(|entry| {
            let mut value = serde_json::to_value(entry).unwrap();
            value.as_object_mut().unwrap().remove("checksState");
            value
        })
        .collect();
    serde_json::from_value(serde_json::json!({"pullRequests":entries,"chains":chains}))
        .expect("pinned PR list shape")
}

pub(crate) fn state(conn: &rusqlite::Connection, id: &ThreadId) -> Result<ThreadPullRequestsUi> {
    let projection = projection::read_thread(conn, id)?;
    let links = projection
        .as_ref()
        .map(|p| links_of(&p.thread))
        .unwrap_or_default();
    let (pull_requests, chains) = entries(&links);
    Ok(ThreadPullRequestsUi {
        thread_id: id.0.clone(),
        version: projection.map_or(0, |p| p.through_sequence),
        pull_requests,
        chains,
    })
}
impl super::Store {
    pub fn ui_pull_requests(&self, id: &ThreadId) -> Result<ThreadPullRequestsUi> {
        self.read(|conn| state(conn, id))
    }
}
