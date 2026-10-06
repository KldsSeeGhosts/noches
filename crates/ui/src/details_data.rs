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
    pub(crate) transfer_actions: HashSet<String>,
    pub(crate) transfer_retries: HashMap<String, ConversationTransfer>,
}

#[derive(Clone)]
pub(crate) enum ConversationTransfer {
    Fork(zeron_proto::transfer::ForkThreadParams),
    Merge(zeron_proto::transfer::MergeThreadBackParams),
}

impl DetailsStore {
    pub(crate) fn transfer_busy(&self, chat: &str) -> bool {
        self.transfer_actions.contains(chat)
    }
}

#[derive(Default)]
pub struct DetailsSnapshot {
    pub prs: Option<ThreadPullRequestsUi>,
    pub launch: Option<LaunchUiState>,
    pub transfer: Option<ThreadTransferState>,
    /// Frozen presentation only; these rows never enter the child's document.
    pub(crate) inherited: Option<std::sync::Arc<Vec<crate::transcript::Row>>>,
    /// The API has no start timestamp: keep first observation stable across reads.
    setup_observed: Option<(String, DateTime<Utc>)>,
    pub error: Option<String>,
}

impl DetailsStore {
    pub fn get(&self, owner: &str, chat: &str) -> Option<&DetailsSnapshot> {
        self.rows.get(&(owner.to_string(), chat.to_string()))
    }

    pub(crate) fn inherited_rows(
        &self,
        chat: &zeron_proto::Chat,
    ) -> Option<&[crate::transcript::Row]> {
        self.get(&chat.device_id, &chat.id)?
            .inherited
            .as_deref()
            .map(Vec::as_slice)
    }
}

/// Text-only history: no private reasoning, replayed approvals, active tools,
/// synthetic provider events, or writes to either conversation. Bound the
/// passive preview; the parent remains the source for its complete transcript.
fn prepare_inherited(
    state: &ThreadTransferState,
) -> Option<std::sync::Arc<Vec<crate::transcript::Row>>> {
    use crate::transcript::{Row, RowKind, rows_for_entry};
    use zeron_doc::{MessagePart, MessageRole, SessionMessageEntry};
    let parent = fork_source(state)?;
    let messages: Vec<_> = state
        .inherited_items
        .iter()
        .filter(|item| {
            matches!(
                item["type"].as_str(),
                Some("user_message" | "assistant_message")
            ) && item["text"].as_str().is_some_and(|text| !text.is_empty())
        })
        .collect();
    let skipped = messages.len().saturating_sub(100);
    let mut rows = Vec::new();
    for item in messages.iter().skip(skipped) {
        let (Some(source), Some(id), Some(text)) = (
            item["sourceThreadId"]
                .as_str()
                .or_else(|| item["threadId"].as_str()),
            item["sourceItemId"]
                .as_str()
                .or_else(|| item["id"].as_str()),
            item["text"].as_str(),
        ) else {
            continue;
        };
        // JSON tuple encoding prevents collisions between nested source IDs.
        let identity = format!(
            "inherited:{}",
            serde_json::to_string(&(source, id)).expect("string identity")
        );
        let mut visible: String = text.chars().take(10_000).collect();
        if visible.len() < text.len() {
            visible.push_str("\n\n[Preview shortened; open the parent for the complete message.]");
        }
        let entry = SessionMessageEntry {
            id: identity,
            role: if item["type"] == "user_message" {
                MessageRole::User
            } else {
                MessageRole::Assistant
            },
            parts: vec![MessagePart::Text {
                id: "text".into(),
                text: visible,
            }],
            // Inherited turn-items need not carry a wall-clock timestamp.
            // Never invent one or offer the inherited attachment transport.
            created_at: 0,
            device_id: String::new(),
            status: None,
            continuation_of: None,
        };
        let mut prepared = rows_for_entry(&entry, false, &mut |_, text| {
            std::sync::Arc::new(crate::markdown::parser::parse_full(text))
        });
        for row in &mut prepared {
            row.timestamp = None;
            if let RowKind::User { attachments, .. } = &mut row.kind {
                *attachments = std::sync::Arc::new(vec![]);
            }
        }
        rows.extend(prepared);
    }
    let label = if skipped > 0 {
        format!("Fork continues here · {skipped} earlier messages in parent")
    } else {
        "Fork continues here · inherited conversation above".into()
    };
    let identity = format!("fork-boundary:{}", state.thread_id);
    rows.push(Row {
        id: identity.clone().into(),
        entry_id: identity.into(),
        version: state.version as u64,
        turn_start: true,
        timestamp: None,
        copy_text: None,
        kind: RowKind::ForkBoundary {
            label: label.into(),
            source_chat_id: parent.into(),
        },
    });
    Some(std::sync::Arc::new(rows))
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
                    ("started", _) => "Start".into(),
                    ("completed", Some(n)) => format!("Turn {n}"),
                    _ => "File checkpoint".into(),
                },
                additions: sum(true),
                deletions: sum(false),
                forkable: file.phase == "completed"
                    && c.run_id.is_some()
                    && c.status
                        == zeron_proto::orchestration::OrchestrationV2CheckpointStatus::Ready,
            })
        })
        .collect();
    rows.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| a.id.cmp(&b.id)));
    // One row per turn: a turn's "before" state is the previous turn's
    // "after", so only the oldest "before" stays, as the Start baseline.
    let start = rows.iter().rposition(|row| row.summary == "Start");
    let mut index = 0;
    rows.retain(|row| {
        let keep = row.summary != "Start" || Some(index) == start;
        index += 1;
        keep
    });
    rows
}

pub fn apply_snapshot(model: &mut DetailsModel, row: &DetailsSnapshot) {
    if let Some(prs) = &row.prs
        && !prs.pull_requests.is_empty()
    {
        model.pull_requests = map_pull_requests(prs);
    }
    if let Some(launch) = &row.launch {
        // A worktree binding is a path other than the project root. A root
        // checkout reports its own path as `worktreePath` with no root.
        let in_worktree = launch
            .worktree_path
            .as_deref()
            .zip(launch.project_workspace_root.as_deref())
            .is_some_and(|(path, root)| path != root);
        model.workspace.worktree_branch =
            in_worktree.then(|| launch.branch.clone().unwrap_or_else(|| "Worktree".into()));
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
        model.fork_run_id = transfer.latest_forkable_run_id.clone();
        model.merge_run_id = transfer.latest_mergeable_run_id.clone();
        model.merge_target = fork_source(transfer).map(str::to_owned);
        model.transfers = map_transfers(transfer);
    }
}

pub fn map_transfers(state: &ThreadTransferState) -> Vec<crate::details::ContextTransferRow> {
    let mut transfers: Vec<_> = state
        .transfers
        .iter()
        .filter_map(|t| {
            let id = t["id"].as_str()?;
            let handoff = state.handoffs.iter().find(|h| h["transferId"] == id);
            let strategy = t["resolution"]["strategy"].as_str();
            let status = t["status"].as_str().unwrap_or("pending");
            // Consuming a transfer reserves it for a run; native acceptance, not
            // logical consumption, confirms delivery of portable history.
            let delivery = handoff.and_then(|h| h["deliveryStatus"].as_str());
            let label = match status {
                "failed" => "Failed",
                "superseded" => "Superseded",
                "consumed"
                    if strategy == Some("native_fork")
                        || matches!(delivery, Some("inline" | "injected")) =>
                {
                    "Delivered"
                }
                "consumed" => "Prepared",
                "resolved_native" | "resolved_portable" => "Ready",
                _ => "Pending",
            };
            let method = match strategy {
                Some("native_fork") => "Native fork",
                Some("delta_context" | "fork_delta_context") => "Delta context",
                Some("portable_context") => "Portable context",
                _ => "Resolves on the next message",
            };
            let mut detail = method.to_owned();
            if let Some(h) = handoff {
                if let (Some(from), Some(to)) = (
                    h["coveredRunOrdinals"]["from"].as_i64(),
                    h["coveredRunOrdinals"]["to"].as_i64(),
                ) {
                    detail.push_str(&format!(" · runs {from}–{to}"));
                }
                if let Some(omitted) = h["omittedItems"].as_u64().filter(|n| *n > 0) {
                    detail.push_str(&format!(" · {omitted} items omitted"));
                }
            }
            let title = match t["type"].as_str() {
                Some("fork") => "Conversation fork",
                Some("merge_back") => "Merge-back context",
                Some("provider_handoff") => "Agent handoff",
                Some("subagent_spawn") => "Delegated task",
                Some("subagent_result") => "Agent result",
                _ => "Context transfer",
            };
            let source = t["sourceThreadId"]
                .as_str()
                .filter(|id| *id != state.thread_id.0);
            let providers = t["sourceProviderInstanceId"]
                .as_str()
                .zip(t["targetProviderInstanceId"].as_str())
                .filter(|(a, b)| a != b)
                .map(|(a, b)| format!("{a} → {b}"));
            Some(crate::details::ContextTransferRow {
                id: id.to_owned(),
                title: title.into(),
                status: label.into(),
                detail,
                source_chat_id: source.map(str::to_owned),
                providers,
                failed: status == "failed",
                error: (status == "failed")
                    .then(|| t["error"].as_str().map(str::to_owned))
                    .flatten(),
            })
        })
        .collect();
    transfers.reverse();
    transfers
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
            let (transfer, inherited) = cx
                .background_executor()
                .spawn(async move {
                    let inherited = transfer.as_ref().ok().and_then(prepare_inherited);
                    (transfer, inherited)
                })
                .await;
            this.update(cx, |state, cx| {
                state.details.pending.remove(&key);
                let row = state.details.rows.entry(key.clone()).or_default();
                row.error = None;
                match prs {
                    Ok(v) => row.prs = Some(v),
                    Err(e) => row.error = Some(e.to_string()),
                }
                match transfer {
                    Ok(v) => {
                        if row
                            .transfer
                            .as_ref()
                            .is_none_or(|current| current.version <= v.version)
                        {
                            row.transfer = Some(v);
                            row.inherited = inherited;
                            state.transcript_revision = state.transcript_revision.wrapping_add(1);
                        }
                    }
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
            "projectWorkspaceRoot":"/repo",
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
        assert_eq!(rows[0].summary, "Turn 4");
        assert_eq!(rows[1].summary, "Start");
        assert!(rows[0].forkable);
        assert!(
            !rows[1].forkable,
            "a before-run file snapshot is not a conversation boundary"
        );
        assert_eq!((rows[0].additions, rows[0].deletions), (12, 3));
        assert_eq!(fork_source(&state), Some("parent"));
    }

    #[test]
    fn root_checkout_is_not_a_worktree_and_only_the_first_before_stays() {
        let launch: LaunchUiState = serde_json::from_value(json!({
            "threadId":"chat", "worktreePath":"/repo", "projectWorkspaceRoot":null
        }))
        .unwrap();
        let mut model = DetailsModel::default();
        apply_snapshot(
            &mut model,
            &DetailsSnapshot {
                launch: Some(launch),
                ..Default::default()
            },
        );
        assert!(model.workspace.worktree_branch.is_none());
        let state: ThreadTransferState = serde_json::from_value(json!({
            "threadId":"chat",
            "checkpoints":[
                checkpoint("s1", "2026-10-04T14:00:00Z", "started"),
                checkpoint("c1", "2026-10-04T14:01:00Z", "completed"),
                checkpoint("s2", "2026-10-04T14:02:00Z", "started"),
                checkpoint("c2", "2026-10-04T14:03:00Z", "completed")
            ]
        }))
        .unwrap();
        let ids: Vec<_> = map_checkpoints(&state).into_iter().map(|r| r.id).collect();
        assert_eq!(ids, ["c2", "c1", "s1"]);
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

    #[test]
    fn transfer_rows_distinguish_prepared_context_from_confirmed_delivery_and_hide_private_content()
    {
        let state: ThreadTransferState = serde_json::from_value(json!({
            "threadId":"target",
            "transfers":[
                {"id":"one","type":"provider_handoff","sourceThreadId":"target","status":"consumed",
                    "sourceProviderInstanceId":"a","targetProviderInstanceId":"b",
                    "resolution":{"strategy":"portable_context","summaryText":"PRIVATE SUMMARY"}},
                {"id":"two","type":"merge_back","sourceThreadId":"fork","status":"consumed",
                    "resolution":{"strategy":"delta_context"}},
                {"id":"three","type":"fork","sourceThreadId":"parent","status":"consumed",
                    "resolution":{"strategy":"native_fork"}},
                {"id":"four","type":"provider_handoff","status":"failed","error":"Provider unavailable"}
            ],
            "handoffs":[
                {"id":"h1","transferId":"one","deliveryStatus":"pending",
                    "coveredRunOrdinals":{"from":1,"to":3},"omittedItems":12,
                    "summaryText":"PRIVATE SUMMARY","history":{"messages":[{"text":"PRIVATE HISTORY"}]}},
                {"id":"h2","transferId":"two","deliveryStatus":"inline","omittedItems":0}
            ]
        })).unwrap();
        let rows = map_transfers(&state);
        assert_eq!(
            rows.iter().map(|r| r.status.as_str()).collect::<Vec<_>>(),
            ["Failed", "Delivered", "Delivered", "Prepared"]
        );
        assert!(rows[0].failed);
        assert_eq!(rows[0].error.as_deref(), Some("Provider unavailable"));
        assert_eq!(rows[2].source_chat_id.as_deref(), Some("fork"));
        assert_eq!(rows[3].providers.as_deref(), Some("a → b"));
        assert!(rows[3].source_chat_id.is_none());
        assert!(rows[3].detail.contains("runs 1–3"));
        assert!(rows[3].detail.contains("12 items omitted"));
        assert!(!format!("{rows:?}").contains("PRIVATE"));
    }

    #[test]
    fn inherited_preview_is_frozen_text_with_unique_source_ids_and_an_explicit_boundary() {
        use crate::transcript::RowKind;
        let state: ThreadTransferState = serde_json::from_value(json!({
            "threadId":"child","forkedFrom":{"threadId":"parent","runId":"pinned"},
            "inheritedItems":[
                {"id":"same","threadId":"grandparent","type":"assistant_message","text":"Earlier decision"},
                {"id":"same","threadId":"parent","type":"user_message","text":"Continue from this"},
                {"id":"private","threadId":"parent","type":"reasoning","text":"PRIVATE REASONING"},
                {"id":"approval","threadId":"parent","type":"approval","text":"LIVE APPROVAL"},
                {"id":"tool","threadId":"parent","type":"command_execution","text":"LIVE TOOL"},
                {"id":"question","threadId":"parent","type":"user_input_request","text":"LIVE QUESTION"}
            ]
        })).unwrap();
        let rows = prepare_inherited(&state).unwrap();
        assert_eq!(rows.len(), 3);
        assert_ne!(rows[0].entry_id, rows[1].entry_id);
        assert!(rows.iter().all(|r| r.timestamp.is_none()));
        assert!(
            matches!(&rows[1].kind, RowKind::User { pending: false, attachments, .. } if attachments.is_empty())
        );
        assert!(
            matches!(&rows[2].kind, RowKind::ForkBoundary { source_chat_id, .. } if source_chat_id == "parent")
        );
        assert!(rows.iter().all(|r| !matches!(
            r.kind,
            RowKind::LiveMarkdown { .. } | RowKind::ToolGroup { .. } | RowKind::InputChip { .. }
        )));
        assert_eq!(
            rows.iter()
                .filter_map(|r| r.copy_text.as_deref())
                .collect::<Vec<_>>(),
            ["Earlier decision", "Continue from this"]
        );
        let root: ThreadTransferState = serde_json::from_value(json!({"threadId":"root"})).unwrap();
        assert!(prepare_inherited(&root).is_none());
    }

    #[test]
    fn inherited_preview_bounds_are_visible_and_unicode_safe() {
        use crate::transcript::RowKind;
        let items: Vec<_> = (0..102)
            .map(|i| {
                json!({
                    "id":format!("item:{i}"), "threadId":"parent","type":"assistant_message",
                    "text": if i == 101 { "🧪".repeat(10_010) } else { format!("Message {i}") }
                })
            })
            .collect();
        let state: ThreadTransferState = serde_json::from_value(json!({
            "threadId":"child","lineage":{"parentThreadId":"parent","relationshipToParent":"fork"},
            "inheritedItems":items,
        }))
        .unwrap();
        let rows = prepare_inherited(&state).unwrap();
        assert_eq!(rows.iter().filter(|r| r.copy_text.is_some()).count(), 100);
        let last_text = rows
            .iter()
            .filter_map(|r| r.copy_text.as_deref())
            .last()
            .unwrap();
        assert_eq!(last_text.matches('🧪').count(), 10_000);
        assert!(last_text.contains("Preview shortened"));
        assert!(
            matches!(&rows.last().unwrap().kind, RowKind::ForkBoundary { label, .. } if label.contains("2 earlier messages"))
        );
    }
}
