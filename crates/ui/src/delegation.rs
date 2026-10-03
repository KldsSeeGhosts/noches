//! Delegated-task read model: the UI half of T3's app-owned subagents.
//!
//! A delegated task is a *real chat* (the child) owned by a parent chat, run by
//! the engine on its own. This module holds only what the surfaces draw: the
//! per-parent task list, the child -> parent links (subagent and fork), the
//! relationship rows, the child banner's view model, and the thin adapter
//! trait the engine read API plugs into. Native vendor subagents stay in
//! [`crate::subagents`] and remain observational; both kinds meet in the one
//! update-owned `SubagentPresentation`.
//!
//! Everything here is pure data plus selectors over it: nothing scans a
//! transcript, and nothing runs from render except lookups in the index.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use gpui::SharedString;
use serde::{Deserialize, Serialize};
use zeron_proto::HarnessId;
use zeron_proto::orchestration_mcp::{
    OrchestratorMcpDelegateTaskResultWorkState as ProtoWorkState,
    OrchestratorMcpDelegatedTaskStatus as ProtoStatus,
};

use crate::subagents::SubagentPhase;

// ---------------------------------------------------------------------------
// Wire-facing model
// ---------------------------------------------------------------------------

/// Lifecycle of the backing run, as `task_status.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegatedStatus {
    Queued,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

impl DelegatedStatus {
    pub fn settled(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Interrupted
        )
    }
}

impl From<ProtoStatus> for DelegatedStatus {
    fn from(status: ProtoStatus) -> Self {
        match status {
            ProtoStatus::Queued => Self::Queued,
            ProtoStatus::Running => Self::Running,
            ProtoStatus::Waiting => Self::Waiting,
            ProtoStatus::Completed => Self::Completed,
            ProtoStatus::Failed => Self::Failed,
            ProtoStatus::Cancelled => Self::Cancelled,
            ProtoStatus::Interrupted => Self::Interrupted,
        }
    }
}

/// `task_status.workState`: whether the *task* still holds work, which is not
/// the same as whether the backing run is alive (a finished child can still be
/// waiting on its own nested tasks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegatedWorkState {
    Working,
    WaitingForChildren,
    ResultAvailable,
}

impl DelegatedWorkState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Working => "Working",
            Self::WaitingForChildren => "Waiting for children",
            Self::ResultAvailable => "Result available",
        }
    }
}

impl From<ProtoWorkState> for DelegatedWorkState {
    fn from(state: ProtoWorkState) -> Self {
        match state {
            ProtoWorkState::Working => Self::Working,
            ProtoWorkState::WaitingForChildren => Self::WaitingForChildren,
            ProtoWorkState::ResultAvailable => Self::ResultAvailable,
        }
    }
}

/// One delegated task. `child_chat_id` is the real chat the task runs in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegatedTask {
    pub task_id: String,
    pub parent_chat_id: String,
    pub child_chat_id: String,
    pub title: String,
    /// Harness behind the provider instance (brand mark); falls back to the
    /// child chat's own config when the engine omits it.
    #[serde(default)]
    pub harness: Option<HarnessId>,
    #[serde(default)]
    pub model: Option<String>,
    /// Effort as the composer names it ("High", "X-High").
    #[serde(default)]
    pub effort: Option<String>,
    pub status: DelegatedStatus,
    pub work_state: DelegatedWorkState,
    /// When the child actually began work (not when a wake began).
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub progress: Option<String>,
    /// `task_cancel` would be accepted (an interruptible active run).
    #[serde(default)]
    pub cancellable: bool,
}

impl DelegatedTask {
    /// The one display phase every subagent surface keys its glyph on.
    pub fn phase(&self) -> SubagentPhase {
        use DelegatedStatus as S;
        match (self.status, self.work_state) {
            (S::Failed, _) => SubagentPhase::Failed,
            (S::Cancelled | S::Interrupted, _) => SubagentPhase::Stopped,
            (_, DelegatedWorkState::WaitingForChildren) => SubagentPhase::Waiting,
            (S::Completed, _) => SubagentPhase::Done,
            (S::Running, _) => SubagentPhase::Running,
            (S::Waiting, _) => SubagentPhase::Waiting,
            (S::Queued, _) => SubagentPhase::Started,
        }
    }

    /// Holds work open: pulses, keeps the tray, accepts Stop.
    pub fn active(&self) -> bool {
        self.phase().active()
    }

    /// T3's detail rule: a settled task prefers its result, a live one its
    /// progress; collapsed to one line and capped.
    pub fn detail(&self, cap: usize) -> Option<String> {
        let (first, second) = if self.phase().active() {
            (&self.progress, &self.result)
        } else {
            (&self.result, &self.progress)
        };
        let text = [first, second]
            .into_iter()
            .flatten()
            .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
            .find(|text| !text.is_empty())?;
        let mut chars = text.chars();
        let mut out: String = chars.by_ref().take(cap).collect();
        if chars.next().is_some() {
            out.truncate(out.trim_end().len());
            out.push('…');
        }
        Some(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadLinkKind {
    Subagent,
    Fork,
}

/// `chat_id` descends from `parent_chat_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadLink {
    pub chat_id: String,
    pub parent_chat_id: String,
    pub kind: ThreadLinkKind,
}

/// Everything the UI reads, in one value: what the engine read API returns and
/// what a fixture file holds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationSnapshot {
    #[serde(default)]
    pub tasks: Vec<DelegatedTask>,
    /// Fork links. Subagent links derive from `tasks`.
    #[serde(default)]
    pub links: Vec<ThreadLink>,
}

// ---------------------------------------------------------------------------
// Index
// ---------------------------------------------------------------------------

/// The snapshot, indexed for the lookups the surfaces make. Rebuilt on update
/// only; selectors never allocate beyond the rows they return.
#[derive(Debug, Default, Clone)]
pub struct DelegationIndex {
    by_parent: HashMap<String, Arc<[DelegatedTask]>>,
    /// child chat id -> (parent chat id, position in the parent's list).
    by_child: HashMap<String, (String, usize)>,
    links: HashMap<String, ThreadLink>,
    forks: HashMap<String, Vec<String>>,
    /// Stop accepted by the engine but not yet reflected as terminal. T3:
    /// acceptance is not terminal confirmation.
    cancel_requested: HashSet<String>,
}

impl DelegationIndex {
    /// Index `snapshot`, carrying forward cancel requests whose task is still
    /// active (a settled task drops its request).
    pub fn build(snapshot: DelegationSnapshot, previous: Option<&DelegationIndex>) -> Self {
        let mut grouped: HashMap<String, Vec<DelegatedTask>> = HashMap::new();
        for task in snapshot.tasks {
            grouped
                .entry(task.parent_chat_id.clone())
                .or_default()
                .push(task);
        }
        let mut index = Self::default();
        for (parent, mut tasks) in grouped {
            // Active first (oldest start first), then settled newest first -
            // the same bucket order the native selector uses.
            tasks.sort_by(|a, b| {
                let bucket = |t: &DelegatedTask| !t.active();
                bucket(a).cmp(&bucket(b)).then_with(|| {
                    if a.active() {
                        a.started_at.cmp(&b.started_at)
                    } else {
                        b.completed_at
                            .or(b.started_at)
                            .cmp(&a.completed_at.or(a.started_at))
                    }
                })
            });
            for (ix, task) in tasks.iter().enumerate() {
                index
                    .by_child
                    .insert(task.child_chat_id.clone(), (parent.clone(), ix));
                index.links.insert(
                    task.child_chat_id.clone(),
                    ThreadLink {
                        chat_id: task.child_chat_id.clone(),
                        parent_chat_id: parent.clone(),
                        kind: ThreadLinkKind::Subagent,
                    },
                );
            }
            index.by_parent.insert(parent, tasks.into());
        }
        for link in snapshot.links {
            if link.kind == ThreadLinkKind::Fork {
                index
                    .forks
                    .entry(link.parent_chat_id.clone())
                    .or_default()
                    .push(link.chat_id.clone());
                index.links.insert(link.chat_id.clone(), link);
            }
        }
        if let Some(previous) = previous {
            index.cancel_requested = previous
                .cancel_requested
                .iter()
                .filter(|id| {
                    index
                        .task_by_id(id)
                        .is_some_and(|task| task.active() && task.cancellable)
                })
                .cloned()
                .collect();
        }
        index
    }

    /// Every chat id that owns tasks or forks, for invalidation.
    pub fn parents(&self) -> impl Iterator<Item = &String> {
        self.by_parent.keys().chain(self.forks.keys())
    }

    pub fn has_parent(&self, chat_id: &str) -> bool {
        self.by_parent.contains_key(chat_id) || self.forks.contains_key(chat_id)
    }

    pub fn tasks_for(&self, parent_chat_id: &str) -> &[DelegatedTask] {
        self.by_parent
            .get(parent_chat_id)
            .map_or(&[], |tasks| &tasks[..])
    }

    pub fn task_for_child(&self, chat_id: &str) -> Option<&DelegatedTask> {
        let (parent, ix) = self.by_child.get(chat_id)?;
        self.by_parent.get(parent)?.get(*ix)
    }

    pub fn task_by_id(&self, task_id: &str) -> Option<&DelegatedTask> {
        self.by_parent
            .values()
            .flat_map(|tasks| tasks.iter())
            .find(|task| task.task_id == task_id)
    }

    /// A chat the engine runs on its own: read-only to the user.
    pub fn is_delegated_child(&self, chat_id: &str) -> bool {
        self.by_child.contains_key(chat_id)
    }

    pub fn link_for(&self, chat_id: &str) -> Option<&ThreadLink> {
        self.links.get(chat_id)
    }

    pub fn forks_of(&self, chat_id: &str) -> &[String] {
        self.forks.get(chat_id).map_or(&[], Vec::as_slice)
    }

    pub fn cancel_requested_set(&self) -> &HashSet<String> {
        &self.cancel_requested
    }

    /// Whether any child -> parent link differs (banner inputs).
    pub fn links_differ(&self, other: &DelegationIndex) -> bool {
        self.links != other.links
    }

    pub fn cancel_requested(&self, task_id: &str) -> bool {
        self.cancel_requested.contains(task_id)
    }

    /// Record an accepted Stop. Returns whether the task could take one.
    pub fn request_cancel(&mut self, task_id: &str) -> bool {
        let stoppable = self
            .task_by_id(task_id)
            .is_some_and(|task| task.active() && task.cancellable);
        if stoppable {
            self.cancel_requested.insert(task_id.to_owned());
        }
        stoppable
    }
}

// ---------------------------------------------------------------------------
// Relationships (T3 ThreadRelationshipsControl)
// ---------------------------------------------------------------------------

/// Rows shown before "Show more", and each further page.
pub const LINEAGE_INITIAL: usize = 6;
pub const LINEAGE_PAGE: usize = 12;

/// How a row relates to the open chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// The chat this one was delegated from or forked from.
    Parent(ThreadLinkKind),
    Fork,
}

impl Relation {
    /// T3's `relationshipLabel`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Parent(ThreadLinkKind::Subagent) => "Parent agent",
            Self::Parent(ThreadLinkKind::Fork) => "Parent thread",
            Self::Fork => "Fork",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationRow {
    pub chat_id: String,
    pub title: SharedString,
    pub relation: Relation,
}

/// The open chat's parent and forks (T3's non-agent relationship rows), parent
/// first. Agents come from the subagent presentation, which already merges
/// native and delegated rows. `title_of` resolves a chat title.
pub fn related_rows(
    index: &DelegationIndex,
    chat_id: &str,
    title_of: impl Fn(&str) -> Option<String>,
) -> Vec<RelationRow> {
    let title = |id: &str, fallback: &str| -> SharedString {
        title_of(id)
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| fallback.to_owned())
            .into()
    };
    let mut rows = Vec::new();
    if let Some(link) = index.link_for(chat_id) {
        rows.push(RelationRow {
            chat_id: link.parent_chat_id.clone(),
            title: title(&link.parent_chat_id, "Parent"),
            relation: Relation::Parent(link.kind),
        });
    }
    rows.extend(index.forks_of(chat_id).iter().map(|fork| RelationRow {
        chat_id: fork.clone(),
        title: title(fork, "Fork"),
        relation: Relation::Fork,
    }));
    rows
}

/// T3's `resolveThreadLineageWindow`: a workable window plus the hidden count.
pub fn lineage_window<T>(rows: &[T], visible: usize) -> (&[T], usize) {
    let shown = visible.min(rows.len());
    (&rows[..shown], rows.len() - shown)
}

// ---------------------------------------------------------------------------
// Child banner view model
// ---------------------------------------------------------------------------

/// What the read-only child banner draws, resolved once per render from the
/// index and the child chat's own config.
#[derive(Debug, Clone, PartialEq)]
pub struct ChildBannerModel {
    pub task: DelegatedTask,
    pub parent_chat_id: String,
    pub parent_title: SharedString,
    pub model: SharedString,
    pub effort: Option<SharedString>,
    pub harness: Option<HarnessId>,
    /// Stop is offered (active, cancellable, not already asked).
    pub can_stop: bool,
    pub stopping: bool,
}

/// Resolve the banner for `chat_id`, or `None` when the chat is not a
/// delegated child. `config` is the child chat's own config (model/effort/
/// harness fall back to it when the task omits them).
pub fn child_banner(
    index: &DelegationIndex,
    chat_id: &str,
    parent_title: Option<String>,
    config: Option<&zeron_proto::ChatConfig>,
) -> Option<ChildBannerModel> {
    let task = index.task_for_child(chat_id)?.clone();
    let model = task
        .model
        .clone()
        .or_else(|| config.and_then(|c| c.model.clone()))
        .map(|m| crate::pickers::chip_model_label(&m).to_owned())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| task.title.clone());
    let effort = task.effort.clone().or_else(|| {
        config
            .and_then(|c| c.reasoning)
            .map(|level| crate::pickers::reasoning_label(level).to_owned())
    });
    let harness = task.harness.or_else(|| config.map(|c| c.harness));
    let stopping = index.cancel_requested(&task.task_id);
    Some(ChildBannerModel {
        parent_chat_id: task.parent_chat_id.clone(),
        parent_title: parent_title.unwrap_or_else(|| "Parent".to_owned()).into(),
        model: model.into(),
        effort: effort.map(Into::into),
        harness,
        can_stop: task.active() && task.cancellable && !stopping,
        stopping,
        task,
    })
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// The engine read API, as the UI needs it.
///
/// TODO(O-G): `docs/orchestration/ui-api.md` and its RPC supersede this shape.
/// Swap [`EngineDelegationApi`]'s two calls for the real read/watch RPC and
/// `task_cancel`; nothing above this trait changes.
pub trait DelegationApi: Send + Sync + 'static {
    /// One consistent read of every task and fork link the UI shows.
    fn snapshot(&self) -> BoxFuture<'static, Result<DelegationSnapshot, String>>;
    /// `task_cancel`. Resolves on *acceptance* (`cancel_requested`), never on
    /// terminal confirmation: the snapshot reports that.
    fn cancel_task(&self, task_id: String) -> BoxFuture<'static, Result<(), String>>;
}

/// No read API wired: the UI shows native subagents only.
pub struct NullDelegationApi;

impl DelegationApi for NullDelegationApi {
    fn snapshot(&self) -> BoxFuture<'static, Result<DelegationSnapshot, String>> {
        Box::pin(async { Ok(DelegationSnapshot::default()) })
    }
    fn cancel_task(&self, _task_id: String) -> BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Err("delegated tasks are unavailable".to_owned()) })
    }
}

/// Reads the snapshot from a JSON file (`NOCHES_DELEGATION_FIXTURE`), for
/// headed visual QA against the mock engine. Cancel rewrites nothing: the
/// fixture is the engine, and a test edits the file to settle the task.
pub struct FixtureDelegationApi {
    pub path: std::path::PathBuf,
}

impl DelegationApi for FixtureDelegationApi {
    fn snapshot(&self) -> BoxFuture<'static, Result<DelegationSnapshot, String>> {
        let path = self.path.clone();
        Box::pin(async move {
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            serde_json::from_str(&text).map_err(|e| e.to_string())
        })
    }
    fn cancel_task(&self, _task_id: String) -> BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}

/// The engine RPC, once it exists.
///
/// TODO(O-G): method names below are placeholders for the ui-api RPC. Until
/// the engine serves them the call errors and the sync loop backs off.
pub struct EngineDelegationApi {
    pub engine: crate::state::EngineHandle,
}

impl DelegationApi for EngineDelegationApi {
    fn snapshot(&self) -> BoxFuture<'static, Result<DelegationSnapshot, String>> {
        let engine = self.engine.clone();
        Box::pin(async move {
            let value = engine
                .client()
                .call(UI_API_SNAPSHOT, serde_json::json!({}))
                .await
                .map_err(|e| e.to_string())?;
            serde_json::from_value(value).map_err(|e| e.to_string())
        })
    }
    fn cancel_task(&self, task_id: String) -> BoxFuture<'static, Result<(), String>> {
        let engine = self.engine.clone();
        Box::pin(async move {
            engine
                .client()
                .call(UI_API_TASK_CANCEL, serde_json::json!({ "taskId": task_id }))
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
    }
}

/// Resolve `call`, or fail after `timeout` (a hung RPC must not wedge the sync
/// loop or leave a Stop spinning).
pub async fn with_timeout<T>(
    call: BoxFuture<'static, Result<T, String>>,
    executor: &gpui::BackgroundExecutor,
    timeout: std::time::Duration,
) -> Result<T, String> {
    match futures::future::select(call, executor.timer(timeout)).await {
        futures::future::Either::Left((result, _)) => result,
        futures::future::Either::Right(_) => Err("delegation request timed out".to_owned()),
    }
}

/// Env var naming a snapshot JSON file for headed QA against the mock engine.
pub const FIXTURE_ENV: &str = "NOCHES_DELEGATION_FIXTURE";

pub fn fixture_configured() -> bool {
    std::env::var_os(FIXTURE_ENV).is_some_and(|path| !path.is_empty())
}

/// The API a window starts with: the QA fixture when configured, else none
/// until the engine connects (the sync loop then prefers the engine).
pub fn api_for_env() -> std::sync::Arc<dyn DelegationApi> {
    match std::env::var_os(FIXTURE_ENV).filter(|path| !path.is_empty()) {
        Some(path) => std::sync::Arc::new(FixtureDelegationApi { path: path.into() }),
        None => std::sync::Arc::new(NullDelegationApi),
    }
}

/// The API to use right now: the QA fixture when configured, else the live
/// engine once connected, else `fallback` (the null API).
pub fn select_api(
    engine: Option<crate::state::EngineHandle>,
    fallback: &std::sync::Arc<dyn DelegationApi>,
) -> std::sync::Arc<dyn DelegationApi> {
    match engine {
        Some(engine) if !fixture_configured() => {
            std::sync::Arc::new(EngineDelegationApi { engine })
        }
        _ => fallback.clone(),
    }
}

/// TODO(O-G): replace with the `ui-api.md` method names.
pub const UI_API_SNAPSHOT: &str = "OrchestrationUiSnapshot";
pub const UI_API_TASK_CANCEL: &str = "OrchestrationTaskCancel";

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    pub(crate) fn t(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::UNIX_EPOCH + TimeDelta::seconds(1_700_000_000 + secs)
    }

    pub(crate) fn task(
        id: &str,
        parent: &str,
        status: DelegatedStatus,
        work: DelegatedWorkState,
    ) -> DelegatedTask {
        DelegatedTask {
            task_id: format!("task-{id}"),
            parent_chat_id: parent.into(),
            child_chat_id: format!("child-{id}"),
            title: format!("Agent {id}"),
            harness: Some(HarnessId::Codex),
            model: Some("gpt-5".into()),
            effort: Some("High".into()),
            status,
            work_state: work,
            started_at: Some(t(0)),
            completed_at: status.settled().then(|| t(90)),
            result: None,
            progress: None,
            cancellable: !status.settled(),
        }
    }

    use DelegatedStatus as S;
    use DelegatedWorkState as W;

    #[test]
    fn phase_prefers_failure_then_stop_then_waiting_children_then_status() {
        assert_eq!(
            task("a", "p", S::Failed, W::ResultAvailable).phase(),
            SubagentPhase::Failed
        );
        assert_eq!(
            task("a", "p", S::Cancelled, W::ResultAvailable).phase(),
            SubagentPhase::Stopped
        );
        assert_eq!(
            task("a", "p", S::Interrupted, W::Working).phase(),
            SubagentPhase::Stopped
        );
        // A finished child that still owns running nested tasks is not done.
        let waiting = task("a", "p", S::Completed, W::WaitingForChildren);
        assert_eq!(waiting.phase(), SubagentPhase::Waiting);
        assert!(waiting.active());
        assert_eq!(
            task("a", "p", S::Completed, W::ResultAvailable).phase(),
            SubagentPhase::Done
        );
        assert_eq!(
            task("a", "p", S::Running, W::Working).phase(),
            SubagentPhase::Running
        );
        assert_eq!(
            task("a", "p", S::Waiting, W::Working).phase(),
            SubagentPhase::Waiting
        );
        // Queued is neutral, never "Done" and never pulsing.
        assert_eq!(
            task("a", "p", S::Queued, W::Working).phase(),
            SubagentPhase::Started
        );
    }

    #[test]
    fn detail_prefers_result_when_settled_and_progress_while_live() {
        let mut t = task("a", "p", S::Completed, W::ResultAvailable);
        t.result = Some("Done:\n  refactored   the parser".into());
        t.progress = Some("reading files".into());
        assert_eq!(
            t.detail(120).as_deref(),
            Some("Done: refactored the parser")
        );
        let mut live = task("a", "p", S::Running, W::Working);
        live.result = t.result.clone();
        live.progress = t.progress.clone();
        assert_eq!(live.detail(120).as_deref(), Some("reading files"));
        live.progress = Some("   ".into());
        assert_eq!(
            live.detail(120).as_deref(),
            Some("Done: refactored the parser")
        );
        live.progress = Some("x".repeat(50));
        assert_eq!(live.detail(10).unwrap().chars().count(), 11);
        assert_eq!(task("b", "p", S::Running, W::Working).detail(10), None);
    }

    #[test]
    fn index_groups_by_parent_orders_active_first_and_links_children() {
        let mut older = task("old", "p", S::Completed, W::ResultAvailable);
        older.completed_at = Some(t(50));
        let mut newer = task("new", "p", S::Failed, W::ResultAvailable);
        newer.completed_at = Some(t(80));
        let mut live = task("live", "p", S::Running, W::Working);
        live.started_at = Some(t(10));
        let other = task("other", "q", S::Running, W::Working);
        let index = DelegationIndex::build(
            DelegationSnapshot {
                tasks: vec![older, newer, live, other],
                links: vec![],
            },
            None,
        );
        let ids: Vec<_> = index
            .tasks_for("p")
            .iter()
            .map(|t| t.task_id.as_str())
            .collect();
        assert_eq!(ids, ["task-live", "task-new", "task-old"]);
        assert!(index.is_delegated_child("child-live"));
        assert!(!index.is_delegated_child("p"));
        assert_eq!(
            index.task_for_child("child-other").unwrap().parent_chat_id,
            "q"
        );
        assert_eq!(
            index.link_for("child-old"),
            Some(&ThreadLink {
                chat_id: "child-old".into(),
                parent_chat_id: "p".into(),
                kind: ThreadLinkKind::Subagent
            })
        );
        assert!(index.has_parent("p") && index.has_parent("q") && !index.has_parent("child-live"));
    }

    #[test]
    fn cancel_request_survives_snapshots_until_the_task_settles() {
        let snapshot = |status| DelegationSnapshot {
            tasks: vec![task("a", "p", status, W::Working)],
            links: vec![],
        };
        let mut index = DelegationIndex::build(snapshot(S::Running), None);
        assert!(!index.cancel_requested("task-a"));
        assert!(index.request_cancel("task-a"));
        // Accepted is not terminal: the task is still running, just "stopping".
        assert!(index.cancel_requested("task-a"));
        assert!(index.task_for_child("child-a").unwrap().active());
        let still = DelegationIndex::build(snapshot(S::Running), Some(&index));
        assert!(still.cancel_requested("task-a"));
        let settled = DelegationIndex::build(snapshot(S::Cancelled), Some(&still));
        assert!(!settled.cancel_requested("task-a"));
        // A settled or non-cancellable task cannot take a request.
        let mut settled = settled;
        assert!(!settled.request_cancel("task-a"));
        let mut background = task("b", "p", S::Running, W::Working);
        background.cancellable = false;
        let mut index = DelegationIndex::build(
            DelegationSnapshot {
                tasks: vec![background],
                links: vec![],
            },
            None,
        );
        assert!(!index.request_cancel("task-b"));
    }

    #[test]
    fn related_rows_lead_with_the_parent_then_forks() {
        let fork = ThreadLink {
            chat_id: "fork-1".into(),
            parent_chat_id: "p".into(),
            kind: ThreadLinkKind::Fork,
        };
        let index = DelegationIndex::build(
            DelegationSnapshot {
                tasks: vec![task("run", "p", S::Running, W::Working)],
                links: vec![fork],
            },
            None,
        );
        let titles = |id: &str| (id == "fork-1").then(|| "Try again".to_owned());
        let rows = related_rows(&index, "p", titles);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].relation, Relation::Fork);
        assert_eq!(rows[0].title.as_ref(), "Try again");
        // A delegated child sees its parent agent.
        let child = related_rows(&index, "child-run", |_| Some("Main".into()));
        assert_eq!(child.len(), 1);
        assert_eq!(
            child[0].relation,
            Relation::Parent(ThreadLinkKind::Subagent)
        );
        assert_eq!(child[0].relation.label(), "Parent agent");
        assert_eq!(child[0].chat_id, "p");
        assert_eq!(child[0].title.as_ref(), "Main");
        // A fork sees its parent thread; a blank title falls back, never empty.
        let fork_view = related_rows(&index, "fork-1", |_| Some("  ".into()));
        assert_eq!(fork_view[0].relation.label(), "Parent thread");
        assert_eq!(fork_view[0].title.as_ref(), "Parent");
        assert!(related_rows(&index, "nobody", |_| None).is_empty());
    }

    #[test]
    fn lineage_window_pages_like_t3() {
        let rows: Vec<u32> = (0..20).collect();
        let (shown, hidden) = lineage_window(&rows, LINEAGE_INITIAL);
        assert_eq!((shown.len(), hidden), (6, 14));
        let (shown, hidden) = lineage_window(&rows, LINEAGE_INITIAL + LINEAGE_PAGE);
        assert_eq!((shown.len(), hidden), (18, 2));
        let (shown, hidden) = lineage_window(&rows, 99);
        assert_eq!((shown.len(), hidden), (20, 0));
        assert_eq!(lineage_window::<u32>(&[], 6), (&[][..], 0));
    }

    #[test]
    fn child_banner_resolves_model_effort_and_stop_state() {
        let mut stale = task("a", "p", S::Running, W::Working);
        stale.model = None;
        stale.effort = None;
        stale.harness = None;
        let mut index = DelegationIndex::build(
            DelegationSnapshot {
                tasks: vec![stale],
                links: vec![],
            },
            None,
        );
        let config = zeron_proto::ChatConfig {
            harness: HarnessId::ClaudeCode,
            model: Some("anthropic/claude-sonnet".into()),
            reasoning: Some(zeron_proto::ReasoningLevel::XHigh),
            model_options: Default::default(),
            sandbox: zeron_proto::SandboxLevel::WorkspaceWrite,
            runtime_mode: Default::default(),
            interaction_mode: Default::default(),
        };
        let banner = child_banner(&index, "child-a", Some("Main".into()), Some(&config)).unwrap();
        assert_eq!(banner.model.as_ref(), "claude-sonnet");
        assert_eq!(banner.effort.as_deref(), Some("X-High"));
        assert_eq!(banner.harness, Some(HarnessId::ClaudeCode));
        assert_eq!(banner.parent_title.as_ref(), "Main");
        assert!(banner.can_stop && !banner.stopping);
        index.request_cancel("task-a");
        let banner = child_banner(&index, "child-a", None, None).unwrap();
        assert!(!banner.can_stop && banner.stopping);
        assert_eq!(banner.parent_title.as_ref(), "Parent");
        // Model falls back to the task title, never blank.
        assert_eq!(banner.model.as_ref(), "Agent a");
        assert!(child_banner(&index, "p", None, None).is_none());
    }

    #[test]
    fn snapshot_round_trips_the_camel_case_wire_shape() {
        let json = r#"{"tasks":[{"taskId":"t","parentChatId":"p","childChatId":"c","title":"Scout",
            "status":"running","workState":"waiting_for_children","startedAt":"2026-10-03T10:00:00Z"}],
            "links":[{"chatId":"f","parentChatId":"p","kind":"fork"}]}"#;
        let snapshot: DelegationSnapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snapshot.tasks[0].work_state, W::WaitingForChildren);
        assert!(!snapshot.tasks[0].cancellable);
        assert_eq!(snapshot.links[0].kind, ThreadLinkKind::Fork);
        let again: DelegationSnapshot =
            serde_json::from_str(&serde_json::to_string(&snapshot).unwrap()).unwrap();
        assert_eq!(again, snapshot);
    }

    #[test]
    fn proto_status_vocabulary_maps_one_to_one() {
        assert_eq!(DelegatedStatus::from(ProtoStatus::Waiting), S::Waiting);
        assert_eq!(
            DelegatedStatus::from(ProtoStatus::Interrupted),
            S::Interrupted
        );
        assert_eq!(
            DelegatedWorkState::from(ProtoWorkState::ResultAvailable),
            W::ResultAvailable
        );
        assert_eq!(W::WaitingForChildren.label(), "Waiting for children");
    }
}
