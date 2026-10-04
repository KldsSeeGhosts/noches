//! Owner/chat-keyed passive details snapshots. No background polling.
use crate::{
    details::{Checkpoint, DetailsModel, PullRequestLink, SetupState},
    state::AppState,
};
use chrono::{DateTime, Utc};
use gpui::Context;
use std::collections::{HashMap, HashSet};
use zeron_proto::{
    launch::{LaunchUiState, SetupControlParams},
    orchestration::{PullRequestChecksState, PullRequestState},
    orchestration_mcp::LinkPullRequestInput,
    pull_requests::ThreadPullRequestsUi,
    transfer::ThreadTransferState,
};

#[derive(Default)]
pub struct DetailsStore {
    rows: HashMap<(String, String), DetailsSnapshot>,
    pending: HashSet<(String, String)>,
    refresh_again: HashSet<(String, String)>,
    pub notice: Option<String>,
}

#[derive(Default)]
pub struct DetailsSnapshot {
    pub prs: Option<ThreadPullRequestsUi>,
    pub launch: Option<LaunchUiState>,
    pub transfer: Option<ThreadTransferState>,
    /// The API has no start timestamp: keep first observation stable across reads.
    setup_observed: Option<(String, DateTime<Utc>)>,
    pub error: Option<String>,
}

impl DetailsStore {
    pub fn get(&self, owner: &str, chat: &str) -> Option<&DetailsSnapshot> {
        self.rows.get(&(owner.to_string(), chat.to_string()))
    }
}

/// Honor chain order, retaining insertion order for unrelated links. Native wins.
pub fn map_pull_requests(state: &ThreadPullRequestsUi) -> Vec<PullRequestLink> {
    let mut chains: Vec<_> = state.chains.iter().collect();
    chains.sort_by_key(|c| c.kind != "native");
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();
    for chain in chains {
        // Chain numbers have no repository identity. Never reorder ambiguous numbers.
        for number in &chain.numbers {
            let matches: Vec<_> = state
                .pull_requests
                .iter()
                .enumerate()
                .filter(|(_, p)| p.number == *number)
                .collect();
            if matches.len() == 1 && seen.insert(matches[0].0) {
                ordered.push(matches[0].1);
            }
        }
    }
    for (i, entry) in state.pull_requests.iter().enumerate() {
        if seen.insert(i) {
            ordered.push(entry);
        }
    }
    ordered
        .into_iter()
        .filter_map(|p| {
            Some(PullRequestLink {
                summary: zeron_proto::ChangeRequestSummary {
                    provider: if p.host.contains("gitlab") {
                        "gitlab"
                    } else {
                        "github"
                    }
                    .into(),
                    number: u64::try_from(p.number).ok().filter(|n| *n > 0)?,
                    title: p.title.clone().unwrap_or_else(|| "Pull request".into()),
                    url: p.url.clone(),
                    state: match p.state {
                        Some(PullRequestState::Closed) => zeron_proto::ChangeRequestState::Closed,
                        Some(PullRequestState::Merged) => zeron_proto::ChangeRequestState::Merged,
                        _ => zeron_proto::ChangeRequestState::Open,
                    },
                    base_ref: p.base_branch.clone().unwrap_or_default(),
                    head_ref: p.head_branch.clone().unwrap_or_default(),
                },
                checks_passed: u32::from(p.checks_state == Some(PullRequestChecksState::Passing)),
                checks_failed: u32::from(p.checks_state == Some(PullRequestChecksState::Failing)),
                checks_pending: u32::from(p.checks_state == Some(PullRequestChecksState::Pending)),
                watching: p.watching,
            })
        })
        .collect()
}

pub fn map_checkpoints(state: &ThreadTransferState) -> Vec<Checkpoint> {
    let mut rows: Vec<_> = state
        .checkpoints
        .iter()
        .filter_map(|file| {
            let c = &file.checkpoint;
            let at = DateTime::parse_from_rfc3339(&c.captured_at)
                .ok()?
                .with_timezone(&Utc);
            let sum = |additions| {
                c.files
                    .iter()
                    .map(|f| {
                        u32::try_from(if additions { f.additions } else { f.deletions })
                            .unwrap_or(0)
                    })
                    .fold(0u32, u32::saturating_add)
            };
            Some(Checkpoint {
                id: c.id.0.clone(),
                at,
                // The checkpoint API has no prose turn summary. Don't invent one.
                summary: match (file.phase.as_str(), c.app_run_ordinal) {
                    ("backup", _) => "Restore backup".into(),
                    ("started", Some(n)) => format!("Turn {n} · before"),
                    ("completed", Some(n)) => format!("Turn {n} · after"),
                    _ => "File checkpoint".into(),
                },
                additions: sum(true),
                deletions: sum(false),
            })
        })
        .collect();
    rows.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| a.id.cmp(&b.id)));
    rows
}

pub fn apply_snapshot(model: &mut DetailsModel, row: &DetailsSnapshot) {
    if let Some(prs) = &row.prs
        && !prs.pull_requests.is_empty()
    {
        model.pull_requests = map_pull_requests(prs);
    }
    if let Some(launch) = &row.launch {
        model.workspace.worktree_branch = launch
            .worktree_path
            .as_ref()
            .map(|_| launch.branch.clone().unwrap_or_else(|| "Worktree".into()));
        if launch.branch.is_some() {
            model.workspace.branch = launch.branch.clone();
        }
        if model.workspace.worktree_branch.is_some() {
            model.workspace.auto_pull = None;
        }
        model.workspace.setup =
            launch
                .setup
                .as_ref()
                .and_then(|setup| match setup.status.as_str() {
                    "running" | "pending" => Some(SetupState::Running {
                        command: setup.script_name.clone(),
                        started_at: row.setup_observed.as_ref()?.1,
                    }),
                    "failed" | "timed_out" | "cancelled" => Some(SetupState::Failed {
                        command: setup.script_name.clone(),
                        exit_code: setup.exit_code,
                    }),
                    _ => None,
                });
    }
    if let Some(transfer) = &row.transfer {
        model.checkpoints = map_checkpoints(transfer);
    }
}

pub fn fork_source(state: &ThreadTransferState) -> Option<&str> {
    state
        .forked_from
        .as_ref()
        .and_then(|f| f["threadId"].as_str())
        .or_else(|| {
            (state.lineage["relationshipToParent"] == "fork")
                .then(|| state.lineage["parentThreadId"].as_str())
                .flatten()
        })
}

impl AppState {
    pub fn refresh_details(&mut self, chat_id: &str, force: bool, cx: &mut Context<Self>) {
        let Some(chat) = self.chats.iter().find(|c| c.id == chat_id) else {
            return;
        };
        let key = (chat.device_id.clone(), chat.id.clone());
        if self.details.pending.contains(&key) {
            if force {
                self.details.refresh_again.insert(key);
            }
            return;
        }
        if !force && self.details.rows.contains_key(&key) {
            return;
        }
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        self.details.pending.insert(key.clone());
        cx.spawn(async move |this, cx| {
            let (prs, launch, transfer) = futures::join!(
                engine.client().thread_pull_requests_on(&key.1, &key.0),
                engine.client().launch_state(&key.1, Some(&key.0)),
                engine.client().thread_transfer_state(&key.1, &key.0),
            );
            this.update(cx, |state, cx| {
                state.details.pending.remove(&key);
                let row = state.details.rows.entry(key.clone()).or_default();
                row.error = None;
                match prs {
                    Ok(v) => row.prs = Some(v),
                    Err(e) => row.error = Some(e.to_string()),
                }
                match transfer {
                    Ok(v) => row.transfer = Some(v),
                    Err(e) => row.error = Some(e.to_string()),
                }
                match launch {
                    Ok(v) => {
                        let run = v.as_ref().and_then(|l| l.setup.as_ref()).map(|s| &s.run_id);
                        if run != row.setup_observed.as_ref().map(|(id, _)| id) {
                            row.setup_observed = run.map(|id| (id.clone(), Utc::now()));
                        }
                        row.launch = v;
                    }
                    Err(e) => row.error = Some(e.to_string()),
                }
                // Passive reads never raise a notice: an empty section is the
                // honest rendering of "nothing to show". Only user actions
                // (watch, link, restore, handoff) surface their failures.
                if let Some(error) = &row.error {
                    tracing::debug!(chat = %key.1, %error, "details read failed");
                }
                if state.details.refresh_again.remove(&key) {
                    state.refresh_details(&key.1, true, cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn control_details_setup(&mut self, chat: &str, action: &str, cx: &mut Context<Self>) {
        let Some(owner) = self
            .chats
            .iter()
            .find(|c| c.id == chat)
            .map(|c| c.device_id.clone())
        else {
            return;
        };
        let Some(setup) = self
            .details
            .get(&owner, chat)
            .and_then(|r| r.launch.as_ref())
            .and_then(|l| l.setup.as_ref())
        else {
            return;
        };
        let request = SetupControlParams {
            chat_id: chat.into(),
            run_id: setup.run_id.clone(),
            action: action.into(),
        };
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        let chat = chat.to_string();
        cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .control_worktree_setup(request, Some(&owner))
                .await;
            this.update(cx, |state, cx| {
                state.details.notice = result.err().map(|e| e.to_string());
                state.refresh_details(&chat, true, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn toggle_details_watch(
        &mut self,
        chat: &str,
        url: String,
        watching: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(owner) = self
            .chats
            .iter()
            .find(|c| c.id == chat)
            .map(|c| c.device_id.clone())
        else {
            return;
        };
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        let target: LinkPullRequestInput =
            serde_json::from_value(serde_json::json!({"url": url})).expect("URL target");
        let chat = chat.to_string();
        cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .change_thread_pull_request(&chat, &owner, target, Some(watching))
                .await;
            this.update(cx, |state, cx| {
                state.details.notice = result.err().map(|e| e.to_string());
                state.refresh_details(&chat, true, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn api_pr_json_maps_chain_checks_state_and_watch() {
        let prs: ThreadPullRequestsUi = serde_json::from_value(json!({
            "threadId": "chat",
            "pullRequests": [
                {"host":"github.com", "number": 124, "title":"Top", "state":"merged", "checksState":"passing", "watching":true},
                {"host":"github.com", "number": 123, "title":"Bottom", "state":"open", "checksState":"failing"},
                {"host":"github.com", "number": 125, "title":"Pending", "state":"closed", "checksState":"pending"},
                {"host":"github.com", "number": 126}
            ],
            "chains": [{"kind":"derived", "numbers":[123,124]}]
        })).unwrap();
        let mapped = map_pull_requests(&prs);
        assert_eq!(
            mapped.iter().map(|r| r.summary.number).collect::<Vec<_>>(),
            [123, 124, 125, 126]
        );
        assert_eq!(mapped[0].checks_failed, 1);
        assert_eq!(mapped[1].checks_passed, 1);
        assert_eq!(
            mapped[1].summary.state,
            zeron_proto::ChangeRequestState::Merged
        );
        assert!(mapped[1].watching);
        assert_eq!(mapped[2].checks_pending, 1);
        assert_eq!(
            mapped[2].summary.state,
            zeron_proto::ChangeRequestState::Closed
        );
        assert_eq!(
            mapped[3].checks_passed + mapped[3].checks_pending + mapped[3].checks_failed,
            0
        );
    }

    #[test]
    fn native_chain_wins_and_duplicate_numbers_are_not_guessed() {
        let prs = serde_json::from_value(json!({
            "pullRequests":[{"number":2},{"number":1},{"number":3},{"number":3}],
            "chains":[{"kind":"derived","numbers":[2,1]},{"kind":"native","numbers":[1,2,3]}]
        }))
        .unwrap();
        let rows = map_pull_requests(&prs);
        assert_eq!(
            rows.iter().map(|r| r.summary.number).collect::<Vec<_>>(),
            [1, 2, 3, 3]
        );
    }

    #[test]
    fn launch_json_maps_setup_and_root_without_inferred_cwd() {
        let launch: LaunchUiState = serde_json::from_value(json!({
            "threadId":"chat", "branch":"feature", "worktreePath":"/worktrees/feature",
            "setup":{"runId":"run","status":"running","scriptName":"Install"}
        }))
        .unwrap();
        let now = DateTime::parse_from_rfc3339("2026-10-04T14:02:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut row = DetailsSnapshot {
            launch: Some(launch),
            setup_observed: Some(("run".into(), now)),
            ..Default::default()
        };
        let mut model = DetailsModel::default();
        apply_snapshot(&mut model, &row);
        assert_eq!(model.workspace.worktree_branch.as_deref(), Some("feature"));
        assert_eq!(
            model.workspace.setup,
            Some(SetupState::Running {
                command: "Install".into(),
                started_at: now
            })
        );
        let launch = row.launch.as_mut().unwrap();
        let setup = launch.setup.as_mut().unwrap();
        setup.status = "failed".into();
        setup.exit_code = Some(9);
        apply_snapshot(&mut model, &row);
        assert!(matches!(
            model.workspace.setup,
            Some(SetupState::Failed {
                exit_code: Some(9),
                ..
            })
        ));
        row.launch.as_mut().unwrap().worktree_path = None;
        row.launch.as_mut().unwrap().setup.as_mut().unwrap().status = "continued".into();
        apply_snapshot(&mut model, &row);
        assert!(model.workspace.worktree_branch.is_none());
        assert!(model.workspace.setup.is_none());
    }

    fn checkpoint(id: &str, at: &str, phase: &str) -> Value {
        json!({
            "checkpoint": {
                "id":id, "threadId":"chat", "scopeId":"scope", "runId":"run", "nodeId":"node",
                "parentCheckpointId":null, "ordinalWithinScope":1, "appRunOrdinal":4,
                "ref":"refs/noches/checkpoints/test", "status":"ready", "capturedAt":at,
                "files":[{"path":"src/lib.rs", "kind":"M", "additions":12, "deletions":3}]
            },
            "scope": {
                "id":"scope", "threadId":"chat", "runId":"run", "nodeId":"node", "parentScopeId":null,
                "providerThreadId":null, "kind":"root_run", "ordinalWithinParent":0,
                "advancesAppRunCount":true, "cwd":"/worktree", "createdAt":at
            },
            "phase":phase
        })
    }

    #[test]
    fn transfer_json_maps_newest_first_totals_and_fork_source() {
        let state: ThreadTransferState = serde_json::from_value(json!({
            "threadId":"chat", "forkedFrom":{"type":"run","threadId":"parent","runId":"run"},
            "checkpoints":[
                checkpoint("old", "2026-10-04T14:02:00Z", "started"),
                checkpoint("new", "2026-10-04T14:03:00Z", "completed"),
                checkpoint("invalid", "not a timestamp", "backup")
            ]
        }))
        .unwrap();
        let rows = map_checkpoints(&state);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "new");
        assert_eq!(rows[0].summary, "Turn 4 · after");
        assert_eq!((rows[0].additions, rows[0].deletions), (12, 3));
        assert_eq!(fork_source(&state), Some("parent"));
    }

    #[test]
    fn empty_engine_links_retain_the_checkout_fallback() {
        let prs: ThreadPullRequestsUi = serde_json::from_value(json!({"threadId":"chat"})).unwrap();
        let mut model = DetailsModel::default();
        model.pull_requests = map_pull_requests(
            &serde_json::from_value(json!({"pullRequests":[{"number":42}]})).unwrap(),
        );
        apply_snapshot(
            &mut model,
            &DetailsSnapshot {
                prs: Some(prs),
                ..Default::default()
            },
        );
        assert_eq!(model.pull_requests[0].summary.number, 42);
    }

    #[test]
    fn owner_chat_keys_cannot_share_snapshots() {
        let mut store = DetailsStore::default();
        store
            .rows
            .insert(("host-a".into(), "chat".into()), DetailsSnapshot::default());
        assert!(store.get("host-a", "chat").is_some());
        assert!(store.get("host-b", "chat").is_none());
    }
}
