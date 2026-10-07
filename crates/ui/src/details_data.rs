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
    /// Transfer-state-only reads (selection, turn completion) are tracked
    /// apart from the full Details read so neither blocks the other.
    transfer_pending: HashSet<(String, String)>,
    transfer_again: HashSet<(String, String)>,
    transfer_debounce: HashSet<(String, String)>,
    /// Inherited-history page reads in flight, one per chat.
    history_pending: HashSet<(String, String)>,
    /// Chats whose first inherited page failed to load: refreshes stay quiet
    /// until the backoff passes, then the chat's own row offers a manual retry.
    history_first_failed: HashMap<(String, String), FirstPageFailure>,
    pub notice: Option<String>,
    pub(crate) transfer_actions: HashSet<String>,
    /// One pinned request per (chat, intent): an unresolved Merge never blocks
    /// a Fork, and a retry can only replay the request it was minted for.
    pub(crate) transfer_retries: HashMap<(String, TransferIntent), ConversationTransfer>,
    pub(crate) session_actions: HashSet<(String, String)>,
    pub(crate) session_retries:
        HashMap<(String, String), zeron_proto::transfer::DisconnectThreadSessionParams>,
    /// A reset and a whole-thread Stop each pin one request per (owner, chat),
    /// so a retry after a lost response repeats the same request identity.
    pub(crate) reset_actions: HashSet<(String, String)>,
    pub(crate) reset_retries:
        HashMap<(String, String), zeron_proto::transfer::ResetThreadSessionParams>,
    pub(crate) stop_actions: HashSet<(String, String)>,
    pub(crate) stop_retries:
        HashMap<(String, String), zeron_proto::transfer::StopThreadWorkParams>,
}

/// What the user asked for; keys a pinned retry together with the chat.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TransferIntent {
    Fork(Option<String>),
    Merge,
}

#[derive(Clone)]
pub(crate) enum ConversationTransfer {
    Fork(zeron_proto::transfer::ForkThreadParams),
    Merge(zeron_proto::transfer::MergeThreadBackParams),
}

/// Automatic retries of a failed first inherited page, spaced out; once spent
/// only the retry row in the transcript tries again. Retries ride the next
/// refresh after the delay, never a timer of their own.
const FIRST_PAGE_BACKOFF: [std::time::Duration; 3] = [
    std::time::Duration::from_secs(5),
    std::time::Duration::from_secs(30),
    std::time::Duration::from_secs(120),
];

struct FirstPageFailure {
    attempts: usize,
    /// `None` once the automatic retries are spent.
    retry_at: Option<std::time::Instant>,
}

impl DetailsStore {
    /// Whether a refresh may (re)issue the first inherited page now.
    fn first_page_due(&self, key: &(String, String)) -> bool {
        self.history_first_failed.get(key).is_none_or(|failure| {
            failure
                .retry_at
                .is_some_and(|at| std::time::Instant::now() >= at)
        })
    }

    fn note_first_page_failure(
        failed: &mut HashMap<(String, String), FirstPageFailure>,
        key: &(String, String),
    ) {
        let attempts = failed.get(key).map_or(0, |failure| failure.attempts) + 1;
        failed.insert(
            key.clone(),
            FirstPageFailure {
                attempts,
                retry_at: FIRST_PAGE_BACKOFF
                    .get(attempts - 1)
                    .map(|delay| std::time::Instant::now() + *delay),
            },
        );
    }

    pub(crate) fn transfer_busy(&self, chat: &str) -> bool {
        self.transfer_actions.contains(chat)
    }

    pub(crate) fn transfer_retry(
        &self,
        chat: &str,
        intent: &TransferIntent,
    ) -> Option<&ConversationTransfer> {
        self.transfer_retries
            .get(&(chat.to_owned(), intent.clone()))
    }

    /// Settle a transfer's pinned request after the RPC returned. The pin
    /// survives only while the host may have committed the request (transport
    /// loss or timeout); any answer the host itself gave, success, refusal or
    /// error, is definite and releases it.
    pub(crate) fn settle_transfer_pin(
        &mut self,
        pin: &(String, TransferIntent),
        error: Option<&zeron_rpc::RpcError>,
    ) {
        if error.is_none_or(|error| !rpc_outcome_unknown(error)) {
            self.transfer_retries.remove(pin);
        }
    }

    /// Drop everything keyed by a chat that no longer exists.
    pub(crate) fn retain_chats(&mut self, chats: &[zeron_proto::Chat]) {
        let live: HashSet<(&str, &str)> = chats
            .iter()
            .map(|c| (c.device_id.as_str(), c.id.as_str()))
            .collect();
        self.rows
            .retain(|(owner, id), _| live.contains(&(owner.as_str(), id.as_str())));
        let ids: HashSet<&str> = chats.iter().map(|c| c.id.as_str()).collect();
        self.transfer_retries.retain(|(id, _), _| ids.contains(id.as_str()));
        self.history_first_failed
            .retain(|(owner, id), _| live.contains(&(owner.as_str(), id.as_str())));
    }
}

#[derive(Default)]
pub struct DetailsSnapshot {
    pub prs: Option<ThreadPullRequestsUi>,
    pub launch: Option<LaunchUiState>,
    pub transfer: Option<ThreadTransferState>,
    /// Frozen presentation only; these rows never enter the child's document.
    pub(crate) inherited: Option<std::sync::Arc<Vec<crate::transcript::Row>>>,
    /// Full inherited transcript, loaded newest page first. `None` until the
    /// first page lands (or when the host cannot page); `inherited` then holds
    /// the frozen text preview.
    history: Option<InheritedHistory>,
    /// The API has no start timestamp: keep first observation stable across reads.
    setup_observed: Option<(String, DateTime<Utc>)>,
    pub error: Option<String>,
    /// Set once a full Details read has completed; a transfer-only row
    /// (chat selection) does not satisfy the panel.
    full: bool,
}

#[doc(hidden)]
#[cfg(feature = "orchestration-fixture")]
#[derive(Debug, Default)]
pub struct InheritedShape {
    pub tool_rows: usize,
    pub image_attachments: usize,
    pub load_earlier: bool,
}

/// Pages of a fork's inherited transcript already rendered into rows.
struct InheritedHistory {
    /// Oldest first; each page is prepared off the UI thread once.
    rows: Vec<crate::transcript::Row>,
    next_before: Option<String>,
    remaining: u64,
    /// The last page read failed; the "load earlier" row offers a retry.
    failed: bool,
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
    rows.push(boundary_row(&state.thread_id.0, state.version as u64, parent, label));
    Some(std::sync::Arc::new(rows))
}

fn boundary_row(
    thread_id: &str,
    version: u64,
    parent: &str,
    label: String,
) -> crate::transcript::Row {
    let identity = format!("fork-boundary:{thread_id}");
    crate::transcript::Row {
        id: identity.clone().into(),
        entry_id: identity.into(),
        version,
        turn_start: true,
        timestamp: None,
        copy_text: None,
        kind: crate::transcript::RowKind::ForkBoundary {
            label: label.into(),
            source_chat_id: parent.into(),
        },
    }
}

/// Turn one inherited-history page into read-only rows with the ordinary
/// transcript builders. Runs on the background executor, once per page.
fn prepare_history_rows(
    page: &zeron_proto::transfer::InheritedHistoryPage,
    owner: &str,
) -> Vec<crate::transcript::Row> {
    use crate::transcript::rows_for_entry;
    use zeron_doc::SessionMessageEntry;
    let mut rows = Vec::new();
    for value in &page.entries {
        let Ok(mut entry) = serde_json::from_value::<SessionMessageEntry>(value.clone()) else {
            continue;
        };
        // The owner device serves media through its attachment transport.
        entry.device_id = owner.to_owned();
        let mut prepared = rows_for_entry(&entry, false, &mut |_, text| {
            std::sync::Arc::new(crate::markdown::parser::parse_full(text))
        });
        for row in &mut prepared {
            // The host sends no wall-clock for inherited items; never invent one.
            row.timestamp = None;
        }
        rows.extend(prepared);
    }
    rows
}

/// The clickable row above a fork's inherited history: load earlier pages, or
/// retry the one that failed.
fn more_row(chat: &str, label: String, version: u64) -> crate::transcript::Row {
    let identity = format!("inherited-more:{chat}");
    crate::transcript::Row {
        id: identity.clone().into(),
        entry_id: identity.into(),
        version,
        turn_start: true,
        timestamp: None,
        copy_text: None,
        kind: crate::transcript::RowKind::InheritedMore {
            label: label.into(),
            chat_id: chat.into(),
        },
    }
}

/// The rows a fork shows above its own conversation: an optional "load
/// earlier" row, the loaded pages, then the explicit boundary.
fn assemble_history(
    state: &ThreadTransferState,
    chat: &str,
    history: &InheritedHistory,
) -> Option<std::sync::Arc<Vec<crate::transcript::Row>>> {
    let parent = fork_source(state)?;
    let mut rows = Vec::with_capacity(history.rows.len() + 2);
    if history.next_before.is_some() {
        let label = if history.failed {
            format!(
                "Could not load earlier history · retry ({} earlier)",
                history.remaining
            )
        } else {
            format!("Load earlier history · {} earlier", history.remaining)
        };
        // Re-keys the row when the count or failure state changes.
        rows.push(more_row(
            chat,
            label,
            history.remaining << 1 | u64::from(history.failed),
        ));
    }
    rows.extend(history.rows.iter().cloned());
    rows.push(boundary_row(
        &state.thread_id.0,
        state.version as u64,
        parent,
        "Fork continues here · inherited conversation above".into(),
    ));
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
        model.attached_provider_sessions = transfer.attached_provider_sessions.clone();
        model.latest_started_run_id = transfer.latest_started_run_id.clone();
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

/// Did the request possibly reach the host without its answer coming back?
/// Only then may a minted request be replayed. Wire errors reach the client as
/// `Failed(text)`, so a relayed transport failure is recognised by its prefix.
pub(crate) fn rpc_outcome_unknown(error: &zeron_rpc::RpcError) -> bool {
    use zeron_rpc::RpcError;
    match error {
        RpcError::Transport(_) | RpcError::Closed => true,
        RpcError::Failed(text) => rpc_message_outcome_unknown(text),
        RpcError::BadParams(_) | RpcError::UnknownMethod(_) => false,
    }
}

/// [`rpc_outcome_unknown`] for an error already rendered with `to_string()`.
pub(crate) fn rpc_message_outcome_unknown(message: &str) -> bool {
    message.starts_with("transport: ") || message == "connection closed"
}

/// Does `parent` hold a pending merge-back from a fork other than `fork`?
/// The host refuses the parent's next start while two forks are pending.
pub fn pending_merge_from_other_fork(parent: &ThreadTransferState, fork: &str) -> bool {
    parent.transfers.iter().any(|t| {
        t["type"] == "merge_back"
            && t["status"] == "pending"
            && t["targetThreadId"] == parent.thread_id.0.as_str()
            && t["sourceThreadId"].as_str().is_some_and(|source| source != fork)
    })
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

/// Does the frozen inherited preview need rebuilding for `next`? Compares the
/// inputs, so an unchanged parent history is never re-parsed or re-synced.
fn inherited_changed(current: Option<&ThreadTransferState>, next: &ThreadTransferState) -> bool {
    current.is_none_or(|current| {
        fork_source(current) != fork_source(next) || current.inherited_items != next.inherited_items
    })
}

/// Delay that coalesces the burst of session frames around a turn ending
/// into one transfer-state read.
const TRANSFER_REFRESH_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(600);

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
        if !force && self.details.rows.get(&key).is_some_and(|row| row.full) {
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
            let inherited = prepare_if_changed(&this, cx, &key, &transfer).await;
            this.update(cx, |state, cx| {
                state.details.pending.remove(&key);
                let row = state.details.rows.entry(key.clone()).or_default();
                row.error = None;
                row.full = true;
                match prs {
                    Ok(v) => row.prs = Some(v),
                    Err(e) => row.error = Some(e.to_string()),
                }
                state.apply_transfer_state(&key, transfer, inherited);
                state.ensure_inherited_history(&key, cx);
                let parent = state
                    .details
                    .rows
                    .get(&key)
                    .and_then(|row| row.transfer.as_ref())
                    .and_then(fork_source)
                    .map(str::to_owned);
                if let Some(parent) = parent {
                    // The panel's merge gating reads the parent's pending list.
                    state.fetch_transfer_state(&parent, true, false, cx);
                }
                let row = state.details.rows.entry(key.clone()).or_default();
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

    /// Fetch only the thread transfer state: what chat selection needs for the
    /// inherited preview, and what keeps the fork/merge gating current. Without
    /// `force`, a chat whose transfer state is already cached is left alone.
    /// A fork's parent state is read once too (merge-back conflicts live there).
    pub fn refresh_transfer_state(&mut self, chat_id: &str, force: bool, cx: &mut Context<Self>) {
        self.fetch_transfer_state(chat_id, force, true, cx);
    }

    fn fetch_transfer_state(
        &mut self,
        chat_id: &str,
        force: bool,
        follow_parent: bool,
        cx: &mut Context<Self>,
    ) {
        if !self.chat_host_supports(chat_id, zeron_proto::capabilities::THREAD_TRANSFERS_V1) {
            return;
        }
        let Some(chat) = self.chats.iter().find(|c| c.id == chat_id) else {
            return;
        };
        let key = (chat.device_id.clone(), chat.id.clone());
        if self.details.pending.contains(&key) {
            // The full read carries the transfer state; rerun it afterwards
            // only when the caller knows the cached value is out of date.
            if force {
                self.details.refresh_again.insert(key);
            }
            return;
        }
        if self.details.transfer_pending.contains(&key) {
            if force {
                self.details.transfer_again.insert(key);
            }
            return;
        }
        if !force
            && self
                .details
                .rows
                .get(&key)
                .is_some_and(|row| row.transfer.is_some())
        {
            return;
        }
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        self.details.transfer_pending.insert(key.clone());
        cx.spawn(async move |this, cx| {
            let transfer = engine.client().thread_transfer_state(&key.1, &key.0).await;
            let inherited = prepare_if_changed(&this, cx, &key, &transfer).await;
            this.update(cx, |state, cx| {
                state.details.transfer_pending.remove(&key);
                let parent = transfer
                    .as_ref()
                    .ok()
                    .and_then(fork_source)
                    .filter(|_| follow_parent)
                    .map(str::to_owned);
                state.apply_transfer_state(&key, transfer, inherited);
                state.ensure_inherited_history(&key, cx);
                if let Some(parent) = parent {
                    state.fetch_transfer_state(&parent, false, false, cx);
                }
                if state.details.transfer_again.remove(&key) {
                    state.fetch_transfer_state(&key.1, true, follow_parent, cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// A turn just ended (indicator left Working): re-read the transfer state
    /// of every chat that already has a Details row, once per burst. Event
    /// driven; nothing is armed while idle.
    pub(crate) fn schedule_transfer_refresh(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        let Some(chat) = self.chats.iter().find(|c| c.id == chat_id) else {
            return;
        };
        let key = (chat.device_id.clone(), chat.id.clone());
        let tracked = self.selected_chat.as_deref() == Some(chat_id)
            || self.details.rows.contains_key(&key);
        if !tracked || !self.details.transfer_debounce.insert(key.clone()) {
            return;
        }
        let chat = chat_id.to_owned();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(TRANSFER_REFRESH_DEBOUNCE)
                .await;
            this.update(cx, |state, cx| {
                state.details.transfer_debounce.remove(&key);
                state.refresh_transfer_state(&chat, true, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Store a transfer-state read. The inherited preview and the transcript
    /// revision only change when the inherited inputs did.
    fn apply_transfer_state(
        &mut self,
        key: &(String, String),
        transfer: Result<ThreadTransferState, zeron_rpc::RpcError>,
        inherited: Option<std::sync::Arc<Vec<crate::transcript::Row>>>,
    ) {
        let row = self.details.rows.entry(key.clone()).or_default();
        let mut bump = false;
        match transfer {
            Ok(v) => {
                if row
                    .transfer
                    .as_ref()
                    .is_none_or(|current| current.version <= v.version)
                {
                    if inherited_changed(row.transfer.as_ref(), &v) {
                        // `None` here means the read raced a change after the
                        // up-front comparison; build it now.
                        let next = inherited.or_else(|| prepare_inherited(&v));
                        bump = row.inherited.is_some() || next.is_some();
                        row.inherited = next;
                        // The paged transcript belongs to the old inputs.
                        row.history = None;
                        self.details.history_first_failed.remove(key);
                    }
                    row.transfer = Some(v);
                }
            }
            Err(e) => row.error = Some(e.to_string()),
        }
        if bump {
            self.transcript_revision = self.transcript_revision.wrapping_add(1);
        }
    }

    /// Load the newest page of a fork's inherited transcript once per set of
    /// inherited inputs. Hosts without the capability keep the text preview.
    fn ensure_inherited_history(&mut self, key: &(String, String), cx: &mut Context<Self>) {
        let wanted = self.details.rows.get(key).is_some_and(|row| {
            row.history.is_none() && row.transfer.as_ref().and_then(fork_source).is_some()
        });
        if wanted && self.details.first_page_due(key) {
            self.fetch_inherited_page(key, None, cx);
        }
    }

    /// The transcript's "load earlier history" row.
    pub fn load_more_inherited_history(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        let Some(chat) = self.chats.iter().find(|c| c.id == chat_id) else {
            return;
        };
        let key = (chat.device_id.clone(), chat.id.clone());
        let history = self
            .details
            .rows
            .get(&key)
            .and_then(|row| row.history.as_ref());
        match history {
            Some(history) => {
                if let Some(before) = history.next_before.clone() {
                    self.fetch_inherited_page(&key, Some(before), cx);
                }
            }
            // The first page failed: this is the manual retry.
            None if self.details.history_first_failed.remove(&key).is_some() => {
                self.fetch_inherited_page(&key, None, cx);
            }
            None => {}
        }
    }

    fn fetch_inherited_page(
        &mut self,
        key: &(String, String),
        before: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if !self.chat_host_supports(&key.1, zeron_proto::capabilities::THREAD_INHERITED_HISTORY_V1)
            || self.details.history_pending.contains(key)
        {
            return;
        }
        let Some(engine) = self.engine().cloned() else {
            return;
        };
        self.details.history_pending.insert(key.clone());
        let key = key.clone();
        cx.spawn(async move |this, cx| {
            let page = engine
                .client()
                .thread_inherited_history(&key.1, &key.0, before.as_deref(), None)
                .await;
            let rows = match &page {
                Ok(page) => {
                    let page = page.clone();
                    let owner = key.0.clone();
                    cx.background_executor()
                        .spawn(async move { prepare_history_rows(&page, &owner) })
                        .await
                }
                Err(_) => Vec::new(),
            };
            this.update(cx, |state, cx| {
                state.details.history_pending.remove(&key);
                if state.apply_inherited_page(&key, before, page, rows) {
                    state.ensure_inherited_history(&key, cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn apply_inherited_page(
        &mut self,
        key: &(String, String),
        before: Option<String>,
        page: Result<zeron_proto::transfer::InheritedHistoryPage, zeron_rpc::RpcError>,
        rows: Vec<crate::transcript::Row>,
    ) -> bool {
        let Some(row) = self.details.rows.get_mut(key) else {
            return false;
        };
        match page {
            Ok(page) if before.is_none() => {
                self.details.history_first_failed.remove(key);
                row.history = Some(InheritedHistory {
                    rows,
                    next_before: page.next_before,
                    remaining: page.remaining,
                    failed: false,
                });
            }
            Ok(page) => {
                // A reset (new inherited inputs) or a duplicate click may have
                // moved the cursor since this read was issued.
                let Some(history) = row
                    .history
                    .as_mut()
                    .filter(|h| h.next_before.as_deref() == before.as_deref())
                else {
                    return false;
                };
                history.rows.splice(0..0, rows);
                history.next_before = page.next_before;
                history.remaining = page.remaining;
                history.failed = false;
            }
            Err(error) => {
                tracing::debug!(chat = %key.1, %error, "inherited history read failed");
                // The host refuses a cursor it no longer knows (its typed
                // code, not any message that happens to mention a cursor):
                // start over from the newest page instead of skipping history.
                if before.is_some()
                    && matches!(
                        &error,
                        zeron_rpc::RpcError::Failed(message)
                            if message.contains(zeron_proto::transfer::INHERITED_CURSOR_EXPIRED)
                    )
                    && row.history.take().is_some()
                {
                    return true;
                }
                match row.history.as_mut() {
                    Some(history) if before.is_some() => history.failed = true,
                    Some(_) => return false,
                    // A first page that failed keeps the preview and shows a
                    // retry row; refreshes back off instead of re-asking.
                    None => {
                        DetailsStore::note_first_page_failure(&mut self.details.history_first_failed, key);
                        let Some(preview) = row.inherited.as_ref() else {
                            return false;
                        };
                        let mut rows = preview.as_ref().clone();
                        if rows
                            .first()
                            .is_some_and(|first| matches!(first.kind, crate::transcript::RowKind::InheritedMore { .. }))
                        {
                            rows.remove(0);
                        }
                        rows.insert(0, more_row(&key.1, "Could not load inherited history · retry".into(), 0));
                        row.inherited = Some(std::sync::Arc::new(rows));
                        self.transcript_revision = self.transcript_revision.wrapping_add(1);
                        return false;
                    }
                }
            }
        }
        let (Some(transfer), Some(history)) = (row.transfer.as_ref(), row.history.as_ref()) else {
            return false;
        };
        if let Some(assembled) = assemble_history(transfer, &key.1, history) {
            row.inherited = Some(assembled);
            self.transcript_revision = self.transcript_revision.wrapping_add(1);
        }
        false
    }

    /// Fixture assertions over the rows a fork shows above its conversation.
    #[doc(hidden)]
    #[cfg(feature = "orchestration-fixture")]
    pub fn fixture_inherited_shape(&self, chat_id: &str) -> InheritedShape {
        use crate::transcript::RowKind;
        let mut shape = InheritedShape::default();
        let rows = self
            .chats
            .iter()
            .find(|c| c.id == chat_id)
            .and_then(|chat| self.details.inherited_rows(chat));
        for row in rows.into_iter().flatten() {
            match &row.kind {
                RowKind::ToolGroup { tools, .. } => shape.tool_rows += tools.len(),
                RowKind::User { attachments, .. } => shape.image_attachments += attachments.len(),
                RowKind::GeneratedImage { .. } => shape.image_attachments += 1,
                RowKind::InheritedMore { .. } => shape.load_earlier = true,
                _ => {}
            }
        }
        shape
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

/// Parse the inherited preview off the UI thread, and only when the transfer
/// state's inherited inputs differ from what the row already holds.
async fn prepare_if_changed(
    this: &gpui::WeakEntity<AppState>,
    cx: &mut gpui::AsyncApp,
    key: &(String, String),
    transfer: &Result<ThreadTransferState, zeron_rpc::RpcError>,
) -> Option<std::sync::Arc<Vec<crate::transcript::Row>>> {
    let Ok(next) = transfer else {
        return None;
    };
    let changed = this
        .update(cx, |state, _| {
            inherited_changed(
                state
                    .details
                    .rows
                    .get(key)
                    .and_then(|row| row.transfer.as_ref()),
                next,
            )
        })
        .unwrap_or(false);
    if !changed {
        return None;
    }
    let next = next.clone();
    cx.background_executor()
        .spawn(async move { prepare_inherited(&next) })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn transfer_state(version: i64, items: Vec<Value>) -> ThreadTransferState {
        serde_json::from_value(json!({
            "threadId": "child",
            "version": version,
            "lineage": {"relationshipToParent": "fork", "parentThreadId": "parent"},
            "inheritedItems": items,
        }))
        .unwrap()
    }

    fn inherited_message(id: &str) -> Value {
        json!({"type": "user_message", "sourceThreadId": "parent", "sourceItemId": id, "text": id})
    }

    #[test]
    fn unchanged_inherited_history_does_not_resync_the_transcript() {
        let mut state = AppState::new();
        let key = ("dev".to_owned(), "child".to_owned());
        let first = transfer_state(1, vec![inherited_message("m1")]);
        let prepared = prepare_inherited(&first);
        assert!(prepared.is_some());
        state.apply_transfer_state(&key, Ok(first), prepared);
        let revision = state.transcript_revision;
        // A later read with identical inherited inputs keeps the preview.
        let second = transfer_state(2, vec![inherited_message("m1")]);
        assert!(!inherited_changed(
            state.details.rows[&key].transfer.as_ref(),
            &second
        ));
        state.apply_transfer_state(&key, Ok(second), None);
        assert_eq!(state.transcript_revision, revision);
        assert!(state.details.rows[&key].inherited.is_some());
        assert_eq!(state.details.rows[&key].transfer.as_ref().unwrap().version, 2);
        // New inherited history rebuilds the preview and resyncs once.
        let third = transfer_state(3, vec![inherited_message("m1"), inherited_message("m2")]);
        state.apply_transfer_state(&key, Ok(third), None);
        assert_eq!(state.transcript_revision, revision.wrapping_add(1));
    }

    fn history_page(
        ids: &[&str],
        next_before: Option<&str>,
        remaining: u64,
    ) -> zeron_proto::transfer::InheritedHistoryPage {
        let entries = ids
            .iter()
            .map(|id| match *id {
                "tool" => json!({"id": "inherited:tool", "role": "assistant", "createdAt": 0, "deviceId": "",
                    "parts": [
                        {"kind": "tool", "id": "t", "call": {"kind": "exec", "command": "ls"},
                         "resolved": true, "output": "a\nb"},
                        {"kind": "text", "id": "x", "text": "listed"},
                        {"kind": "image", "id": "i", "path": "/work/a.png", "name": "a.png",
                         "mimeType": "image/png"}]}),
                "image" => json!({"id": "inherited:image", "role": "user", "createdAt": 0, "deviceId": "",
                    "parts": [{"kind": "text", "id": "text",
                        "text": "look\n\nAttached images (local files — open them to view):\n- /work/a.png"}]}),
                id => json!({"id": format!("inherited:{id}"), "role": "user", "createdAt": 0,
                    "deviceId": "", "parts": [{"kind": "text", "id": "text", "text": id}]}),
            })
            .collect();
        zeron_proto::transfer::InheritedHistoryPage {
            thread_id: "child".into(),
            entries,
            next_before: next_before.map(str::to_owned),
            remaining,
            total: 10,
            shortened: 0,
        }
    }

    fn fork_state() -> (AppState, (String, String)) {
        let mut state = AppState::new();
        let key = ("dev".to_owned(), "child".to_owned());
        state.apply_transfer_state(&key, Ok(transfer_state(1, vec![inherited_message("m1")])), None);
        (state, key)
    }

    #[test]
    fn history_pages_render_tools_media_and_the_boundary_read_only() {
        let page = history_page(&["image", "tool"], Some("inherited:image"), 4);
        let rows = prepare_history_rows(&page, "dev");
        use crate::transcript::RowKind;
        let RowKind::User { attachments, .. } = &rows[0].kind else {
            panic!("user row first");
        };
        assert_eq!(attachments.len(), 1, "inherited media keeps its thumbnail");
        assert!(rows.iter().any(|r| matches!(&r.kind, RowKind::ToolGroup { tools, .. } if tools.len() == 1)));
        assert!(rows.iter().any(|r| matches!(&r.kind, RowKind::GeneratedImage { owner, .. } if owner == "dev")));
        assert!(rows.iter().all(|r| r.timestamp.is_none()), "no invented wall-clock");
    }

    #[test]
    fn first_page_replaces_the_preview_and_older_pages_prepend_in_order() {
        let (mut state, key) = fork_state();
        let preview = state.details.rows[&key].inherited.clone().unwrap();
        assert_eq!(preview.len(), 2, "text preview + boundary");
        let revision = state.transcript_revision;
        let newest = history_page(&["c", "d"], Some("inherited:c"), 2);
        let rows = prepare_history_rows(&newest, "dev");
        assert!(!state.apply_inherited_page(&key, None, Ok(newest), rows));
        let shown = state.details.rows[&key].inherited.clone().unwrap();
        // load-more row, two entries, boundary
        assert!(matches!(shown[0].kind, crate::transcript::RowKind::InheritedMore { .. }));
        assert_eq!(shown.len(), 4);
        assert!(matches!(shown.last().unwrap().kind, crate::transcript::RowKind::ForkBoundary { .. }));
        assert_eq!(state.transcript_revision, revision.wrapping_add(1));

        let older = history_page(&["a", "b"], None, 0);
        let rows = prepare_history_rows(&older, "dev");
        state.apply_inherited_page(&key, Some("inherited:c".into()), Ok(older), rows);
        let shown = state.details.rows[&key].inherited.clone().unwrap();
        let ids: Vec<_> = shown.iter().map(|r| r.id.to_string()).collect();
        assert_eq!(
            ids[..4],
            ["inherited:a", "inherited:b", "inherited:c", "inherited:d"],
            "{ids:?}"
        );
        assert!(
            !shown.iter().any(|r| matches!(r.kind, crate::transcript::RowKind::InheritedMore { .. })),
            "no load-more row once the start is reached"
        );
    }

    #[test]
    fn a_stale_older_page_is_dropped_and_failures_keep_loaded_history() {
        let (mut state, key) = fork_state();
        let newest = history_page(&["c"], Some("inherited:c"), 5);
        let rows = prepare_history_rows(&newest, "dev");
        state.apply_inherited_page(&key, None, Ok(newest), rows);
        let revision = state.transcript_revision;
        // A page issued for a cursor the row no longer holds changes nothing.
        let stale = history_page(&["z"], None, 0);
        let rows = prepare_history_rows(&stale, "dev");
        state.apply_inherited_page(&key, Some("inherited:other".into()), Ok(stale), rows);
        assert_eq!(state.transcript_revision, revision);
        // A failed older read keeps what is loaded and offers a retry.
        let restart = state.apply_inherited_page(
            &key,
            Some("inherited:c".into()),
            Err(zeron_rpc::RpcError::Failed("offline".into())),
            vec![],
        );
        assert!(!restart);
        let shown = state.details.rows[&key].inherited.clone().unwrap();
        assert!(matches!(&shown[0].kind, crate::transcript::RowKind::InheritedMore { label, .. } if label.contains("retry")));
        assert_eq!(shown.len(), 3);
        // A failure that merely mentions a cursor is not the typed code: the
        // loaded pages stay.
        let restart = state.apply_inherited_page(
            &key,
            Some("inherited:c".into()),
            Err(zeron_rpc::RpcError::Transport("cursor stream reset".into())),
            vec![],
        );
        assert!(!restart);
        assert!(state.details.rows[&key].history.is_some());
        // The host forgetting the cursor (its typed code) restarts from the
        // newest page.
        let restart = state.apply_inherited_page(
            &key,
            Some("inherited:c".into()),
            Err(zeron_rpc::RpcError::Failed(format!(
                "{}: The inherited-history cursor is no longer valid.",
                zeron_proto::transfer::INHERITED_CURSOR_EXPIRED
            ))),
            vec![],
        );
        assert!(restart);
        assert!(state.details.rows[&key].history.is_none());
    }

    #[test]
    fn a_failed_first_page_shows_a_retry_row_and_backs_off_instead_of_refetching() {
        use crate::transcript::RowKind;
        let (mut state, key) = fork_state();
        let revision = state.transcript_revision;
        assert!(state.details.first_page_due(&key), "nothing failed yet");
        let restart = state.apply_inherited_page(
            &key,
            None,
            Err(zeron_rpc::RpcError::Failed("offline".into())),
            vec![],
        );
        assert!(!restart);
        // The preview stays, with a retry row on top.
        let shown = state.details.rows[&key].inherited.clone().unwrap();
        assert!(matches!(&shown[0].kind, RowKind::InheritedMore { label, .. } if label.contains("retry")));
        assert_eq!(shown.len(), 3, "retry row + text preview + boundary");
        assert_eq!(state.transcript_revision, revision.wrapping_add(1));
        // Refreshes do not re-issue the read until the backoff passes.
        assert!(!state.details.first_page_due(&key));
        // A second failure replaces the row instead of stacking another.
        state.details.history_first_failed.get_mut(&key).unwrap().retry_at =
            Some(std::time::Instant::now());
        assert!(state.details.first_page_due(&key));
        state.apply_inherited_page(&key, None, Err(zeron_rpc::RpcError::Closed), vec![]);
        assert_eq!(state.details.rows[&key].inherited.as_ref().unwrap().len(), 3);
        assert_eq!(state.details.history_first_failed[&key].attempts, 2);
        // Once the automatic retries are spent only the retry row tries again.
        state.apply_inherited_page(&key, None, Err(zeron_rpc::RpcError::Closed), vec![]);
        assert!(state.details.history_first_failed[&key].retry_at.is_some());
        state.apply_inherited_page(&key, None, Err(zeron_rpc::RpcError::Closed), vec![]);
        assert!(state.details.history_first_failed[&key].retry_at.is_none());
        assert!(!state.details.first_page_due(&key));
        // A success clears the failure and replaces the preview.
        let newest = history_page(&["c"], None, 0);
        let rows = prepare_history_rows(&newest, "dev");
        state.apply_inherited_page(&key, None, Ok(newest), rows);
        assert!(state.details.first_page_due(&key));
        let shown = state.details.rows[&key].inherited.clone().unwrap();
        assert!(!shown.iter().any(|r| matches!(&r.kind, RowKind::InheritedMore { .. })));
        // New inherited inputs forget an old failure.
        state.apply_inherited_page(&key, None, Err(zeron_rpc::RpcError::Closed), vec![]);
        state.details.history_first_failed.insert(
            key.clone(),
            FirstPageFailure { attempts: 3, retry_at: None },
        );
        let next = transfer_state(3, vec![inherited_message("m1"), inherited_message("m2")]);
        state.apply_transfer_state(&key, Ok(next), None);
        assert!(state.details.first_page_due(&key));
    }

    #[test]
    fn changed_inherited_inputs_discard_the_loaded_pages() {
        let (mut state, key) = fork_state();
        let newest = history_page(&["c"], None, 0);
        let rows = prepare_history_rows(&newest, "dev");
        state.apply_inherited_page(&key, None, Ok(newest), rows);
        assert!(state.details.rows[&key].history.is_some());
        let same = transfer_state(2, vec![inherited_message("m1")]);
        state.apply_transfer_state(&key, Ok(same), None);
        assert!(state.details.rows[&key].history.is_some(), "unchanged inputs keep the pages");
        let next = transfer_state(3, vec![inherited_message("m1"), inherited_message("m2")]);
        state.apply_transfer_state(&key, Ok(next), None);
        assert!(state.details.rows[&key].history.is_none());
    }

    #[test]
    fn a_chat_without_inherited_history_never_bumps_the_revision() {
        let mut state = AppState::new();
        let key = ("dev".to_owned(), "plain".to_owned());
        let revision = state.transcript_revision;
        let plain: ThreadTransferState =
            serde_json::from_value(json!({"threadId": "plain", "version": 4})).unwrap();
        state.apply_transfer_state(&key, Ok(plain), None);
        assert_eq!(state.transcript_revision, revision);
        assert!(state.details.rows[&key].inherited.is_none());
    }

    #[test]
    fn removed_chats_are_evicted_from_the_details_store() {
        let mut store = DetailsStore::default();
        for id in ["keep", "gone"] {
            store
                .rows
                .insert(("dev".into(), id.into()), DetailsSnapshot::default());
            store
                .transfer_retries
                .insert((id.into(), TransferIntent::Merge), unreachable_retry());
        }
        let mut keep = crate::state::AppState::new();
        keep.chats = vec![serde_json::from_value(json!({
            "id": "keep", "deviceId": "dev", "archived": false,
            "createdAt": "2026-07-19T12:00:00Z"
        }))
        .unwrap()];
        store.retain_chats(&keep.chats);
        assert!(store.rows.contains_key(&("dev".into(), "keep".into())));
        assert!(!store.rows.contains_key(&("dev".into(), "gone".into())));
        assert_eq!(store.transfer_retries.len(), 1);
    }

    fn unreachable_retry() -> ConversationTransfer {
        ConversationTransfer::Merge(zeron_proto::transfer::MergeThreadBackParams {
            chat_id: "c".into(),
            command_id: "k".into(),
            target_chat_id: "p".into(),
            source_point: zeron_proto::transfer::ThreadSourcePoint::LatestStable,
        })
    }

    #[test]
    fn another_forks_pending_merge_back_blocks_a_second_merge() {
        let parent = |transfers: Value| -> ThreadTransferState {
            serde_json::from_value(json!({"threadId": "parent", "transfers": transfers})).unwrap()
        };
        let merge = |source: &str, status: &str| {
            json!({"type": "merge_back", "status": status,
                   "sourceThreadId": source, "targetThreadId": "parent"})
        };
        assert!(!pending_merge_from_other_fork(&parent(json!([])), "fork-a"));
        // The fork's own pending merge is superseded by merging again.
        assert!(!pending_merge_from_other_fork(
            &parent(json!([merge("fork-a", "pending")])),
            "fork-a"
        ));
        assert!(pending_merge_from_other_fork(
            &parent(json!([merge("fork-b", "pending")])),
            "fork-a"
        ));
        assert!(!pending_merge_from_other_fork(
            &parent(json!([merge("fork-b", "consumed")])),
            "fork-a"
        ));
    }

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
