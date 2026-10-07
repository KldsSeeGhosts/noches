//! Codex-style subagent inventory: a pure selector over a chat's transcript
//! (spawn tool parts) plus the surfaces that render it - the composer agents
//! tray, the right-pane Agents panel, and the sidebar's nested child rows.
//! Status hues come only from [`SessionState`]; everything else stays on
//! neutral theme tokens.

use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use chrono::{DateTime, TimeZone, Utc};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, px,
};
use zeron_doc::{MessagePart, MessageRole, MessageStatus, SessionMessageEntry, SubagentStatus};
use zeron_proto::ToolCall;

use crate::elapsed_label::{ClockSpec, elapsed_label};
use crate::shell::Shell;
use crate::state::AppState;
use crate::status_palette::SessionState;
use crate::theme::Theme;
use crate::{icons, loaders, transcript};

/// Display lifecycle of one spawned subagent. `Started` is the honest
/// neutral state for a spawn that returned without lifecycle proof - the
/// "eager-done" window and `run_in_background` spawns live here (never
/// "Done" just because the spawn call resolved).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubagentPhase {
    Running,
    /// A delegated task whose own nested work is still open: alive, but not
    /// pulsing (only work that is actually running pulses).
    Waiting,
    Started,
    Done,
    Failed,
    /// Cancelled or interrupted: settled without a result, neither success
    /// nor failure.
    Stopped,
}

impl SubagentPhase {
    pub fn active(self) -> bool {
        matches!(self, Self::Running | Self::Waiting | Self::Started)
    }
}

/// What distinguishes an app-owned delegated task from a native vendor
/// subagent: it is a real chat the engine runs on its own, so rows open that
/// chat, show its provider/model/workState and offer Stop.
#[derive(Debug, Clone)]
pub struct DelegatedLink {
    pub task_id: String,
    /// The real chat the task runs in.
    pub child_chat_id: String,
    pub harness: Option<zeron_proto::HarnessId>,
    pub effort: Option<SharedString>,
    pub work_state: crate::delegation::DelegatedWorkState,
    pub cancellable: bool,
    /// Stop accepted, terminal not yet confirmed.
    pub stopping: bool,
}

/// One spawned subagent, reduced to what the tray/panel/sidebar draw.
#[derive(Debug, Clone)]
pub struct SubagentSummary {
    /// The spawn tool part id (`parent_tool_use_id` for tagged traffic).
    pub id: String,
    pub title: SharedString,
    pub agent_type: Option<SharedString>,
    pub model: Option<SharedString>,
    pub status: SubagentPhase,
    pub started: Option<DateTime<Utc>>,
    pub finished: Option<DateTime<Utc>>,
    /// First line of the result text (<=120 chars), or the live tail.
    pub summary: Option<SharedString>,
    /// The spawned transcript doc id; `None` for doc-less harnesses (Pi).
    pub doc_ref: Option<SharedString>,
    /// Sits in the chat's latest turn (after the last user entry).
    pub latest_turn: bool,
    /// `Some` for an app-owned delegated task; native subagents are `None`
    /// and stay observational.
    pub delegated: Option<Arc<DelegatedLink>>,
}

/// One update-owned presentation shared by all consumers. Subsets retain only
/// indices into the same immutable summaries; render does no history work.
#[derive(Debug, Default)]
pub(crate) struct SubagentPresentation {
    summaries: Arc<[SubagentSummary]>,
    tray: Vec<usize>,
    running: Vec<usize>,
}

impl std::ops::Deref for SubagentPresentation {
    type Target = [SubagentSummary];
    fn deref(&self) -> &Self::Target {
        &self.summaries
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SubagentSubset<'a> {
    summaries: &'a [SubagentSummary],
    indices: &'a [usize],
}

impl SubagentSubset<'_> {
    fn iter(&self) -> impl ExactSizeIterator<Item = &SubagentSummary> {
        self.indices.iter().map(|&ix| &self.summaries[ix])
    }
    pub(crate) fn len(&self) -> usize {
        self.indices.len()
    }
    fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

impl SubagentPresentation {
    fn new(summaries: Vec<SubagentSummary>) -> Self {
        let tray = summaries
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.latest_turn || matches!(s.status, SubagentPhase::Running | SubagentPhase::Waiting)
            })
            .map(|(ix, _)| ix)
            .collect();
        let running = summaries
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s.status, SubagentPhase::Running | SubagentPhase::Waiting))
            .map(|(ix, _)| ix)
            .collect();
        Self {
            summaries: summaries.into(),
            tray,
            running,
        }
    }
    pub(crate) fn running(&self) -> SubagentSubset<'_> {
        SubagentSubset {
            summaries: &self.summaries,
            indices: &self.running,
        }
    }
}

impl SubagentSummary {
    /// `45s` / `2m` / `1h 4m` - live for active phases, frozen at finish.
    /// Settled agents with no observed finish time show nothing rather
    /// than a guessed duration.
    pub fn elapsed(&self, now: DateTime<Utc>) -> Option<String> {
        self.clock().label(now)
    }

    /// The self-ticking clock this row's elapsed label runs on.
    pub fn clock(&self) -> ClockSpec {
        ClockSpec {
            started: self.started,
            finished: self.finished,
            active: self.status.active(),
        }
    }

    /// Stop is offered: an active delegated task the engine can interrupt that
    /// has not already been asked to.
    pub fn stoppable(&self) -> Option<&DelegatedLink> {
        self.delegated
            .as_deref()
            .filter(|link| self.status.active() && link.cancellable && !link.stopping)
    }
}

/// The 12px provider mark for a delegated row: the harness brand, monochrome
/// unless the brand carries its own tint. No mark for native subagents.
fn provider_mark(link: &DelegatedLink, theme: &Theme) -> Option<AnyElement> {
    let (mark, tint) = crate::pickers::harness_brand_icon(link.harness?);
    Some(
        icons::icon(mark)
            .size(px(12.0))
            .flex_none()
            .text_color(tint.unwrap_or(theme.text_faint))
            .into_any_element(),
    )
}

/// The hover card for a delegated row (T3's `SubagentTooltipContent`): title,
/// provider and model, workState with elapsed, then the latest result.
fn delegated_tooltip(
    summary: SubagentSummary,
) -> impl Fn(&mut gpui::Window, &mut App) -> gpui::AnyView + 'static {
    // Built per hover, so the elapsed it shows is the hover's, not the last
    // frame's.
    crate::tooltip::lines(move || {
        let now = Utc::now();
        let mut card = crate::tooltip::Lines::new(summary.title.clone());
        if let Some(link) = summary.delegated.as_deref() {
            let model = [summary.model.as_deref(), link.effort.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" \u{00b7} ");
            if !model.is_empty() {
                card = card.detail(model);
            }
            let mut state = if link.stopping {
                "Stopping".to_owned()
            } else {
                link.work_state.label().to_owned()
            };
            if let Some(elapsed) = summary.elapsed(now) {
                state = format!("{state} \u{00b7} {elapsed}");
            }
            card = card.detail(state);
        }
        if let Some(detail) = summary.summary.clone() {
            card = card.detail(detail);
        }
        card
    })
}

fn spawn_input(call: &ToolCall) -> Option<&serde_json::Value> {
    match call {
        ToolCall::Unknown { input, .. } | ToolCall::Mcp { input, .. } => input.as_ref(),
        _ => None,
    }
}

fn input_str<'a>(input: Option<&'a serde_json::Value>, key: &str) -> Option<&'a str> {
    input?
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// A spawn that returns immediately and keeps running detached. Harnesses
/// send the flag as a JSON bool; tolerate a stringly `"true"` too.
fn is_background_spawn(input: Option<&serde_json::Value>) -> bool {
    match input.and_then(|i| i.get("run_in_background")) {
        Some(serde_json::Value::Bool(flag)) => *flag,
        Some(serde_json::Value::String(flag)) => flag.trim().eq_ignore_ascii_case("true"),
        _ => false,
    }
}

/// Strip a leading `Agent:`/`Task:` genus from a spawn's name, then take the
/// first non-empty line. Mirrors the spawn chip's title rules.
fn spawn_title(call: &ToolCall) -> SharedString {
    let (name, input) = match call {
        ToolCall::Unknown { name, input } => (name.as_str(), input.as_ref()),
        ToolCall::Mcp { tool, input, .. } => (tool.as_str(), input.as_ref()),
        _ => return "Agent".into(),
    };
    let bare = name
        .strip_prefix("Agent: ")
        .or_else(|| name.strip_prefix("Task: "))
        .unwrap_or(name);
    let candidates = [
        Some(bare),
        input.and_then(|i| i.get("description")?.as_str()),
    ];
    for text in candidates.into_iter().flatten() {
        let line = transcript::single_line(text);
        if !line.is_empty() && !line.eq_ignore_ascii_case("agent") {
            let capped: String = line.chars().take(40).collect();
            return SharedString::from(capped);
        }
    }
    "Agent".into()
}

/// First meaningful line of a result, capped at 120 chars.
fn one_line(text: &str) -> Option<SharedString> {
    let line = text.lines().find(|l| !l.trim().is_empty())?.trim();
    let mut chars = line.chars();
    let mut out: String = chars.by_ref().take(120).collect();
    if chars.next().is_some() {
        out.push('…');
    }
    Some(SharedString::from(out))
}

fn millis(ms: i64) -> Option<DateTime<Utc>> {
    if ms > 0 {
        Utc.timestamp_millis_opt(ms).single()
    } else {
        None
    }
}

fn part_key(chat_id: &str, part_id: &str) -> String {
    format!("{chat_id}/{part_id}")
}

/// Doc id for a doc-less subagent's synthetic result snapshot (Pi parity:
/// the spawn call's own result opens as a read-only transcript). Matches
/// the engine's `{chat}--sub--{id}` shape without colliding with real docs.
pub fn result_doc_id(chat_id: &str, part_id: &str) -> String {
    format!("{chat_id}--sub--result:{part_id}")
}

/// The chat's spawn calls as display summaries. `chat_id` resolves its
/// transcript: the selected chat's joined transcript, or a pane-opened
/// chat's doc watch (`sub_transcripts`). A chat with no loaded transcript
/// selects nothing - that is the "transcript loaded" gate the sidebar uses.
///
/// Order: running first (oldest first), then finished (newest first).
pub(crate) fn subagents_for(state: &AppState, chat_id: &str) -> Arc<SubagentPresentation> {
    crate::perf_trace::subagent_cache_hit();
    state
        .subagent_presentations
        .get(chat_id)
        .cloned()
        .unwrap_or_else(|| {
            static EMPTY: OnceLock<Arc<SubagentPresentation>> = OnceLock::new();
            EMPTY
                .get_or_init(|| Arc::new(SubagentPresentation::default()))
                .clone()
        })
}

fn derive_subagents(state: &AppState, chat_id: &str) -> Vec<SubagentSummary> {
    crate::perf_trace::subagent_scan();
    let entries: &[SessionMessageEntry] = if state.selected_chat.as_deref() == Some(chat_id) {
        &state.transcript
    } else {
        state.sub_transcript(chat_id)
    };
    if entries.is_empty() {
        return Vec::new();
    }
    let last_user = entries
        .iter()
        .rposition(|e| e.role == MessageRole::User)
        .unwrap_or(0);
    let mut out: Vec<SubagentSummary> = Vec::new();
    for (ix, entry) in entries.iter().enumerate() {
        if entry.role == MessageRole::User {
            continue;
        }
        let streaming = entry.status == Some(MessageStatus::Streaming);
        for part in &entry.parts {
            let MessagePart::Tool {
                id,
                call,
                is_error,
                resolved,
                output,
                subagent_ref,
                subagent_status,
                subagent_tail,
                ..
            } = part
            else {
                continue;
            };
            if !call.is_subagent_spawn() {
                continue;
            }
            let background = is_background_spawn(spawn_input(call));
            let status = if *is_error {
                SubagentPhase::Failed
            } else {
                match subagent_status {
                    Some(SubagentStatus::Running) => SubagentPhase::Running,
                    Some(SubagentStatus::Done) => SubagentPhase::Done,
                    Some(SubagentStatus::Failed) => SubagentPhase::Failed,
                    None => {
                        if background {
                            SubagentPhase::Started
                        } else if !resolved {
                            // A turn that ended without the spawn resolving
                            // can never report again - it died with the run.
                            if streaming {
                                SubagentPhase::Running
                            } else {
                                SubagentPhase::Failed
                            }
                        } else if output.is_some() {
                            // The call's own result IS the subagent's final
                            // report (ACP harnesses, e.g. Pi - no nested doc).
                            SubagentPhase::Done
                        } else if streaming {
                            // Eager-done window: the call returned while the
                            // tagged lifecycle is still owed.
                            SubagentPhase::Started
                        } else {
                            SubagentPhase::Done
                        }
                    }
                }
            };
            let doc_ref = subagent_ref.clone().map(SharedString::from);
            let summary = output
                .as_deref()
                .and_then(one_line)
                .or_else(|| subagent_tail.as_deref().and_then(one_line))
                .or_else(|| {
                    doc_ref.as_ref().and_then(|doc| {
                        state.sub_transcript(doc).iter().rev().find_map(|e| {
                            e.parts.iter().find_map(|p| match p {
                                MessagePart::Text { text, .. } => one_line(text),
                                _ => None,
                            })
                        })
                    })
                });
            let finished = if status.active() {
                None
            } else {
                doc_ref
                    .as_ref()
                    .and_then(|doc| state.sub_transcript(doc).iter().map(|e| e.created_at).max())
                    .and_then(millis)
                    .or_else(|| {
                        state
                            .subagent_finished_obs
                            .get(&part_key(chat_id, id))
                            .and_then(|ms| millis(*ms))
                    })
            };
            out.push(SubagentSummary {
                id: id.clone(),
                title: spawn_title(call),
                agent_type: input_str(spawn_input(call), "subagent_type").map(SharedString::from),
                model: call.subagent_model().map(SharedString::from),
                status,
                started: millis(entry.created_at),
                finished,
                summary,
                doc_ref,
                latest_turn: ix >= last_user,
                delegated: None,
            });
        }
    }
    out
}

/// The chat's delegated tasks as display summaries. Reads the update-owned
/// delegation index and the chat's config; never a history beyond locating the
/// last user turn (the tray's "latest turn" cut), and only from update.
fn delegated_summaries(state: &AppState, chat_id: &str) -> Vec<SubagentSummary> {
    let tasks = state.delegation.tasks_for(chat_id);
    if tasks.is_empty() {
        return Vec::new();
    }
    let entries: &[SessionMessageEntry] = if state.selected_chat.as_deref() == Some(chat_id) {
        &state.transcript
    } else {
        state.sub_transcript(chat_id)
    };
    let last_user_ms = entries
        .iter()
        .rev()
        .find(|e| e.role == MessageRole::User)
        .map_or(0, |e| e.created_at);
    tasks
        .iter()
        .map(|task| {
            let child_config = state
                .chats
                .iter()
                .find(|c| c.id == task.child_chat_id)
                .and_then(|c| c.config.as_ref());
            let model = task
                .model
                .clone()
                .or_else(|| child_config.and_then(|c| c.model.clone()))
                .map(|m| crate::pickers::chip_model_label(&m).to_owned());
            let effort = task.effort.clone().or_else(|| {
                child_config
                    .and_then(|c| c.reasoning)
                    .map(|level| crate::pickers::reasoning_label(level).to_owned())
            });
            let phase = task.phase();
            let latest_turn = phase.active()
                || task
                    .completed_at
                    .or(task.started_at)
                    .is_some_and(|at| at.timestamp_millis() >= last_user_ms);
            SubagentSummary {
                id: task.task_id.clone(),
                title: SharedString::from(task.title.clone()),
                agent_type: None,
                model: model.map(SharedString::from),
                status: phase,
                started: task.started_at,
                finished: task.completed_at,
                summary: task.detail(120).map(SharedString::from),
                doc_ref: None,
                latest_turn,
                delegated: Some(Arc::new(DelegatedLink {
                    task_id: task.task_id.clone(),
                    child_chat_id: task.child_chat_id.clone(),
                    harness: task.harness.or_else(|| child_config.map(|c| c.harness)),
                    effort: effort.map(SharedString::from),
                    work_state: task.work_state,
                    cancellable: task.cancellable,
                    stopping: state.delegation.cancel_requested(&task.task_id),
                })),
            }
        })
        .collect()
}

fn sort_summaries(out: &mut [SubagentSummary]) {
    out.sort_by(|a, b| {
        let bucket = |s: &SubagentSummary| !s.status.active();
        bucket(a).cmp(&bucket(b)).then_with(|| {
            if a.status.active() {
                a.started.cmp(&b.started)
            } else {
                b.finished.or(b.started).cmp(&a.finished.or(a.started))
            }
        })
    });
}

impl AppState {
    /// Runs only on a doc update, never from a selector/render. Child-doc
    /// changes rebuild only parents that actually reference that document.
    pub(crate) fn refresh_subagents(&mut self, doc_id: &str) {
        // A delegated parent or child transcript moved: its task state may too.
        if self.delegation.has_parent(doc_id) || self.delegation.is_delegated_child(doc_id) {
            self.nudge_delegation();
        }
        let parents = self
            .subagent_parents
            .get(doc_id)
            .cloned()
            .unwrap_or_default();
        self.prepare_subagents(doc_id);
        for parent in parents {
            if parent != doc_id {
                self.prepare_subagents(&parent);
            }
        }
    }

    /// Text/reasoning appends cannot change this doc's spawn inventory or
    /// lifecycle. Keep its shared presentation, but refresh referencing
    /// parents whose child preview may have changed (including self-links).
    pub(crate) fn refresh_subagents_after_text(&mut self, doc_id: &str) {
        if self.delegation.has_parent(doc_id) || self.delegation.is_delegated_child(doc_id) {
            self.nudge_delegation();
        }
        let parents = self
            .subagent_parents
            .get(doc_id)
            .cloned()
            .unwrap_or_default();
        for parent in parents {
            self.prepare_subagents(&parent);
        }
    }

    /// Replace the delegation read model and rebuild only the presentations
    /// whose tasks or links changed. Returns whether anything did, so the
    /// caller notifies exactly once. Runs from the sync loop, never render.
    pub(crate) fn apply_delegation_snapshot(
        &mut self,
        snapshot: crate::delegation::DelegationSnapshot,
    ) -> bool {
        let mut snapshot = snapshot;
        // The engine reports a provider instance, not a brand: take the mark
        // from the child chat's own config.
        for task in snapshot.tasks.iter_mut().filter(|t| t.harness.is_none()) {
            task.harness = self
                .chats
                .iter()
                .find(|chat| chat.id == task.child_chat_id)
                .and_then(|chat| chat.config.as_ref())
                .map(|config| config.harness);
        }
        let next = crate::delegation::DelegationIndex::build(snapshot, Some(&self.delegation));
        let affected: HashSet<String> = self
            .delegation
            .parents()
            .chain(next.parents())
            .filter(|parent| {
                self.delegation.tasks_for(parent) != next.tasks_for(parent)
                    || self.delegation.forks_of(parent) != next.forks_of(parent)
                    || self.delegation.cancel_requested_set() != next.cancel_requested_set()
            })
            .cloned()
            .collect();
        // A child's own parent link is part of its banner.
        let links_changed = self.delegation.links_differ(&next);
        self.delegation = next;
        for parent in &affected {
            self.prepare_subagents(parent);
        }
        self.prune_subagent_presentations();
        links_changed || !affected.is_empty()
    }

    /// Record an accepted Stop (`cancel_requested`) so rows read "Stopping…"
    /// until the snapshot reports a terminal state.
    pub(crate) fn mark_delegated_cancel_requested(&mut self, task_id: &str) {
        let parent = self
            .delegation
            .task_by_id(task_id)
            .map(|task| task.parent_chat_id.clone());
        if self.delegation.request_cancel(task_id)
            && let Some(parent) = parent
        {
            self.prepare_subagents(&parent);
        }
    }

    pub(crate) fn prune_subagent_presentations(&mut self) {
        let retained: HashSet<_> = self
            .subagent_presentations
            .keys()
            .filter(|id| self.subagent_source_retained(id))
            .cloned()
            .collect();
        self.subagent_presentations
            .retain(|id, _| retained.contains(id));
        self.subagent_parents.retain(|_, parents| {
            parents.retain(|parent| self.subagent_presentations.contains_key(parent));
            !parents.is_empty()
        });
    }

    fn prepare_subagents(&mut self, chat_id: &str) {
        let mut summaries = derive_subagents(self, chat_id);
        summaries.extend(delegated_summaries(self, chat_id));
        // Only agents observed active in this app lifetime get an inferred
        // finish time. A terminal replay after restart never stamps "now".
        let now = Utc::now().timestamp_millis();
        for summary in &mut summaries {
            let key = part_key(chat_id, &summary.id);
            if summary.status.active() {
                self.subagent_active_obs.insert(key);
            } else if summary.finished.is_none() && self.subagent_active_obs.contains(&key) {
                summary.finished = millis(*self.subagent_finished_obs.entry(key).or_insert(now));
            }
        }
        sort_summaries(&mut summaries);
        if let Some(previous) = self.subagent_presentations.get(chat_id) {
            for doc in previous.iter().filter_map(|s| s.doc_ref.as_deref()) {
                if let Some(parents) = self.subagent_parents.get_mut(doc) {
                    parents.remove(chat_id);
                    if parents.is_empty() {
                        self.subagent_parents.remove(doc);
                    }
                }
            }
        }
        for doc in summaries.iter().filter_map(|s| s.doc_ref.as_deref()) {
            self.subagent_parents
                .entry(doc.to_owned())
                .or_default()
                .insert(chat_id.to_owned());
        }
        self.subagent_presentations.insert(
            chat_id.to_owned(),
            Arc::new(SubagentPresentation::new(summaries)),
        );
    }
}

/// The tray's visible subset: the latest turn's subagents, plus anything
/// still live from earlier turns.
pub(crate) fn strip_visible(summaries: &SubagentPresentation) -> SubagentSubset<'_> {
    SubagentSubset {
        summaries: &summaries.summaries,
        indices: &summaries.tray,
    }
}

// ---------------------------------------------------------------------------
// Status glyph (SessionState hues only)
// ---------------------------------------------------------------------------

/// Equalizer Running, check Done (emerald until seen), danger triangle
/// Failed, ring Waiting, stop mark Stopped, neutral dot Started.
pub fn status_glyph(
    key: SharedString,
    phase: SubagentPhase,
    seen: bool,
    theme: &Theme,
    view: gpui::EntityId,
    cx: &App,
) -> AnyElement {
    match phase {
        SubagentPhase::Running => {
            loaders::mini_equalizer(key, SessionState::Working.color(theme).unwrap(), view, cx)
                .into_any_element()
        }
        SubagentPhase::Done => icons::icon(icons::CHECK)
            .size(px(12.0))
            .flex_none()
            .text_color(if seen {
                theme.text_faint
            } else {
                SessionState::Completed.color(theme).unwrap()
            })
            .into_any_element(),
        SubagentPhase::Failed => icons::icon(icons::DANGER_TRIANGLE)
            .size(px(12.0))
            .flex_none()
            .text_color(theme.danger)
            .into_any_element(),
        // Alive but not pulsing: a static ring in the Working hue.
        SubagentPhase::Waiting => div()
            .size(px(8.0))
            .flex_none()
            .rounded_full()
            .border_1()
            .border_color(SessionState::Working.color(theme).unwrap())
            .into_any_element(),
        SubagentPhase::Stopped => icons::icon(icons::STOP)
            .size(px(10.0))
            .flex_none()
            .text_color(theme.text_faint)
            .into_any_element(),
        SubagentPhase::Started => div()
            .size(px(4.0))
            .flex_none()
            .rounded_full()
            .bg(theme.text_faint)
            .into_any_element(),
    }
}

// ---------------------------------------------------------------------------
// Child-thread banner
// ---------------------------------------------------------------------------

/// The summary behind an open child thread: its spawned doc, or (for
/// doc-less harnesses) the synthetic result doc of the spawn call.
pub fn summary_for_doc<'a>(
    summaries: &'a [SubagentSummary],
    parent_chat_id: &str,
    doc_id: &str,
) -> Option<&'a SubagentSummary> {
    summaries.iter().find(|s| {
        s.doc_ref.as_deref() == Some(doc_id) || result_doc_id(parent_chat_id, &s.id) == doc_id
    })
}

/// What the banner needs from the open child thread. The thread is read-only
/// (the agent runs on its own), so this stands where a composer would.
pub struct ChildBanner<'a> {
    /// Unique per pane tab; keys the live equalizer.
    pub key: &'a str,
    /// The spawn's strip title; the model name when the summary carries none.
    pub title: &'a SharedString,
    /// `None` while the parent's transcript is not loaded.
    pub summary: Option<&'a SubagentSummary>,
    /// The parent harness's brand mark, when known.
    pub mark: Option<(&'static str, Option<gpui::Hsla>)>,
}

/// The read-only banner under a subagent thread: the agent's mark, model
/// (medium), type (muted), ticking status, "Runs on its own", and a way back
/// to the parent. `open_parent` runs on the button.
pub fn child_thread_banner(
    banner: &ChildBanner<'_>,
    theme: &Theme,
    view: gpui::EntityId,
    open_parent: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let summary = banner.summary;
    let (mark, tint) = banner.mark.unwrap_or((icons::BOT, None));
    let model = summary
        .and_then(|s| s.model.clone())
        .unwrap_or_else(|| banner.title.clone());
    let status: Option<AnyElement> = summary.map(|s| {
        div()
            .flex_none()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .child(status_glyph(
                SharedString::from(format!("child-banner-{}", banner.key)),
                s.status,
                true,
                theme,
                view,
                cx,
            ))
            .child(elapsed_label(
                format!("child-banner-clock-{}", banner.key),
                s.clock(),
                theme.text_muted,
            ))
            .into_any_element()
    });
    banner_frame(theme, 12.0)
        .child(
            icons::icon(mark)
                .size(px(16.0))
                .flex_none()
                .text_color(tint.unwrap_or(theme.text_muted)),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.text)
                .child(model),
        )
        .children(summary.and_then(|s| s.agent_type.clone()).map(|kind| {
            div()
                .min_w_0()
                .truncate()
                .text_color(theme.text_muted)
                .child(kind)
        }))
        .children(status)
        .child(div().flex_1())
        .child(
            div()
                .flex_none()
                .text_size(crate::typography::ui_rems(12.0))
                .text_color(theme.text_faint)
                .child("Runs on its own"),
        )
        .child(
            crate::controls::button(
                format!("child-banner-parent-{}", banner.key),
                view,
                theme,
                crate::controls::Variant::Ghost,
                crate::controls::Size::Xs,
                "Open parent",
            )
            .on_click(open_parent),
        )
        .into_any_element()
}

/// The read-only pill both banners share: 44px, the composer's outline and
/// glass, `margin` on every side.
fn banner_frame(theme: &Theme, margin: f32) -> gpui::Div {
    div()
        .flex_none()
        .m(px(margin))
        .h(px(CHILD_BANNER_HEIGHT))
        .pl(px(16.0))
        .pr(px(8.0))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .rounded(px(CHILD_BANNER_HEIGHT / 2.0))
        .border_1()
        .border_color(theme.composer_outline())
        .bg(theme.input_glass_bg())
        .text_size(crate::typography::ui_rems(13.0))
}

/// The banner under a delegated child chat (T3's `ProviderSubagentBar`): the
/// provider mark, model (medium), effort (muted), a self-ticking status,
/// "Runs on its own", Stop while the task can take one, and a way back to
/// the parent. The chat is read-only to the user; the engine runs it.
/// The banner's state word. A stopped or failed task keeps `workState`
/// `result_available` (it has a final transcript), but must not read as a
/// finished result.
pub(crate) fn banner_state_word(
    task: &crate::delegation::DelegatedTask,
    stopping: bool,
) -> &'static str {
    match (stopping, task.phase()) {
        (true, _) => "Stopping\u{2026}",
        (_, SubagentPhase::Failed) => "Failed",
        (_, SubagentPhase::Stopped) => "Stopped",
        _ => task.work_state.label(),
    }
}

pub fn delegated_child_banner(
    key: &str,
    model: &crate::delegation::ChildBannerModel,
    theme: &Theme,
    view: gpui::EntityId,
    open_parent: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
    stop: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let task = &model.task;
    let phase = task.phase();
    let (mark, tint) = model
        .harness
        .map_or((icons::BOT, None), crate::pickers::harness_brand_icon);
    let clock = ClockSpec {
        started: task.started_at,
        finished: task.completed_at,
        active: phase.active(),
    };
    let state_word = banner_state_word(task, model.stopping);
    banner_frame(theme, 0.0)
        .child(
            icons::icon(mark)
                .size(px(16.0))
                .flex_none()
                .text_color(tint.unwrap_or(theme.text_muted)),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.text)
                .child(model.model.clone()),
        )
        .children(
            model
                .effort
                .clone()
                .map(|effort| div().flex_none().text_color(theme.text_muted).child(effort)),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(status_glyph(
                    SharedString::from(format!("delegated-banner-{key}")),
                    phase,
                    true,
                    theme,
                    view,
                    cx,
                ))
                .child(
                    div()
                        .text_size(crate::typography::ui_rems(12.0))
                        .text_color(theme.text_muted)
                        .child(state_word),
                )
                .child(elapsed_label(
                    format!("delegated-banner-clock-{key}"),
                    clock,
                    theme.text_muted,
                )),
        )
        .child(div().flex_1())
        .child(
            div()
                .flex_none()
                .text_size(crate::typography::ui_rems(12.0))
                .text_color(theme.text_faint)
                .child("Runs on its own"),
        )
        .when(model.can_stop, |el| {
            el.child(
                crate::controls::button(
                    format!("delegated-banner-stop-{key}"),
                    view,
                    theme,
                    crate::controls::Variant::Ghost,
                    crate::controls::Size::Xs,
                    "Stop",
                )
                .tooltip(crate::tooltip::text("Stop this task"))
                .on_click(stop),
            )
        })
        .child(
            crate::controls::button(
                format!("delegated-banner-parent-{key}"),
                view,
                theme,
                crate::controls::Variant::Ghost,
                crate::controls::Size::Xs,
                "Open parent",
            )
            .tooltip(crate::tooltip::text(format!("Open {}", model.parent_title)))
            .on_click(open_parent),
        )
        .into_any_element()
}

/// Banner height: the pill the composer morphs through, minus its action row.
pub const CHILD_BANNER_HEIGHT: f32 = 44.0;

// ---------------------------------------------------------------------------
// Composer agents tray
// ---------------------------------------------------------------------------

/// The tray content row: `Agents` label, pills, `+N`, chevron (the surface's
/// own top radius and bottom tuck come from the queue-tray metrics).
pub const TRAY_ROW_HEIGHT: f32 = 32.0;
const PILL_GAP: f32 = 6.0;

/// Estimated pill width (12px glyph + ≤22ch title + mono elapsed + pads) -
/// the tray packs greedily off this estimate; `+N` covers the rest.
/// `open(chat_id, summary)` - sidebar click → select chat + open thread.
pub type OpenAgent = Rc<dyn Fn(&mut Shell, String, SubagentSummary, &mut Context<Shell>)>;
/// `open_panel(chat_id)` - sidebar `+N more` → select chat + Agents tab.
pub type OpenPanel = Rc<dyn Fn(&mut Shell, String, &mut Context<Shell>)>;

fn pill_width(title_chars: usize, extras: f32) -> f32 {
    8.0 * 2.0 + 12.0 + 6.0 + title_chars.min(22) as f32 * 6.6 + 6.0 + 34.0 + extras
}

/// Extra pill width a delegated row spends on its provider mark (12 + gap)
/// and the Stop slot (14 + gap).
fn pill_extras(summary: &SubagentSummary) -> f32 {
    match summary.delegated.as_deref() {
        None => 0.0,
        Some(_) => {
            12.0 + 6.0
                + if summary.stoppable().is_some() {
                    14.0 + 6.0
                } else {
                    0.0
                }
        }
    }
}

/// How many leading pills fit `width` (the tray's inner width), keeping
/// room for the leading label, the `+N` overflow pill and the trailing
/// chevron.
fn fitting(summaries: SubagentSubset<'_>, width: f32) -> usize {
    let mut used = 74.0 + 28.0 + PILL_GAP;
    let mut shown = 0usize;
    for s in summaries.iter() {
        let left_after = summaries.len() - shown - 1;
        let reserve = if left_after > 0 { 44.0 } else { 0.0 };
        let pill = pill_width(s.title.chars().count(), pill_extras(s));
        if used + pill + reserve > width {
            break;
        }
        used += pill + PILL_GAP;
        shown += 1;
    }
    shown.min(summaries.len())
}

/// The tray's pills that fit `width`; `+N` covers the remainder.
fn tray_layout(summaries: SubagentSubset<'_>, width: f32) -> (usize, usize) {
    let shown = fitting(summaries, width);
    (shown, summaries.len() - shown)
}

/// The agents tray content row: `Agents {done}/{total}` label, fitted
/// pills, `+N` overflow, and the trailing chevron that toggles the Agents
/// panel. Rendered INSIDE the queue-tray surface by the composer itself
/// (its stack order is the composer's), so it is one continuous surface
/// with the queue tray and the pill on every route.
///
/// Pills lose their own borders inside the tray (the tray frames them);
/// fills stay on `wash`. `seen` = subagent keys whose thread the user
/// already opened. Fitting uses the tray's inner width.
#[allow(clippy::too_many_arguments)] // render fn; params are the tray's props
pub fn agents_tray_row(
    chat_id: &str,
    summaries: SubagentSubset<'_>,
    inner_width: f32,
    panel_open: bool,
    seen: &HashSet<String>,
    theme: &Theme,
    view: gpui::EntityId,
    cx: &mut Context<crate::composer::Composer>,
) -> AnyElement {
    let done = summaries.iter().filter(|s| !s.status.active()).count();
    let (shown, more) = tray_layout(summaries, inner_width);
    let mut row = div()
        .id("subagent-agents-tray")
        .h(px(TRAY_ROW_HEIGHT))
        .w_full()
        .pl(px(12.0))
        .pr(px(6.0))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(PILL_GAP))
        .overflow_hidden()
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(4.0))
                .pr(px(2.0))
                .text_size(crate::typography::ui_rems(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.text_faint)
                .child("Agents")
                .child(
                    div()
                        .font_family(theme.font_mono.clone())
                        .text_size(crate::typography::ui_rems(11.0))
                        .text_color(theme.text_faint)
                        .child(SharedString::from(format!("{done}/{}", summaries.len()))),
                ),
        );
    for s in summaries.iter().take(shown) {
        let summary = s.clone();
        let chat = chat_id.to_string();
        let phase = s.status;
        let pill_id = SharedString::from(format!("agent-pill-{}", s.id));
        let group = SharedString::from(format!("agent-pill-group-{}", s.id));
        let mark = s
            .delegated
            .as_deref()
            .and_then(|link| provider_mark(link, theme));
        let stopping = s.delegated.as_deref().is_some_and(|link| link.stopping);
        let stop = s.stoppable().map(|link| link.task_id.clone());
        let mut pill = div()
            .id(pill_id)
            .group(group.clone())
            .h(px(24.0))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(8.0))
            .rounded(px(12.0))
            .bg(crate::theme::wash(0.06))
            .cursor_pointer()
            .hover(|s| s.bg(crate::theme::wash(0.10)))
            .on_click(cx.listener(move |_, _, _, cx| {
                cx.emit(crate::composer::ComposerEvent::OpenSubagentSummary {
                    chat_id: chat.clone(),
                    summary: summary.clone(),
                });
            }))
            .child(status_glyph(
                SharedString::from(format!("agent-pill-glyph-{}", s.id)),
                phase,
                seen.contains(&s.id) || seen.contains(&part_key(chat_id, &s.id)),
                theme,
                view,
                cx,
            ))
            .children(mark)
            .child(
                div()
                    .flex_none()
                    .max_w(px(150.0))
                    .truncate()
                    .text_size(crate::typography::ui_rems(12.0))
                    .text_color(if stopping {
                        theme.text_faint
                    } else {
                        theme.text_muted
                    })
                    .child(s.title.clone()),
            )
            .child(elapsed_label(
                format!("tray-clock-{chat_id}-{}", s.id),
                s.clock(),
                theme.text_faint,
            ));
        if s.delegated.is_some() {
            pill = pill.tooltip(delegated_tooltip(s.clone()));
        }
        if let Some(task_id) = stop {
            // Space is reserved; the glyph appears on pill hover so a tray of
            // tasks does not read as a row of buttons.
            pill = pill.child(
                div()
                    .id(SharedString::from(format!("agent-pill-stop-{}", s.id)))
                    .size(px(14.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .invisible()
                    .group_hover(group, |style| style.visible())
                    .hover(|style| style.bg(crate::theme::wash(0.12)))
                    .tooltip(crate::tooltip::text("Stop task"))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        cx.stop_propagation();
                        cx.emit(crate::composer::ComposerEvent::StopDelegatedTask {
                            task_id: task_id.clone(),
                        });
                    }))
                    .child(
                        icons::icon(icons::STOP)
                            .size(px(8.0))
                            .text_color(theme.text_muted),
                    ),
            );
        }
        row = row.child(pill);
    }
    if more > 0 {
        row = row.child(
            div()
                .id("agent-pill-more")
                .h(px(24.0))
                .flex_none()
                .flex()
                .items_center()
                .px(px(8.0))
                .rounded(px(12.0))
                .bg(crate::theme::wash(0.06))
                .cursor_pointer()
                .hover(|s| s.bg(crate::theme::wash(0.10)))
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.emit(crate::composer::ComposerEvent::ToggleAgentsPanel);
                }))
                .child(
                    div()
                        .font_family(theme.font_mono.clone())
                        .text_size(crate::typography::ui_rems(11.0))
                        .text_color(theme.text_faint)
                        .child(SharedString::from(format!("+{more}"))),
                ),
        );
    }
    row = row.child(div().flex_1().min_w_0());
    row.child(
        div()
            .id("agents-panel-toggle")
            .debug_selector(|| "agents-panel-toggle".into())
            .size(px(24.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|s| s.bg(crate::theme::wash(0.08)))
            .on_click(cx.listener(move |_, _, _, cx| {
                cx.emit(crate::composer::ComposerEvent::ToggleAgentsPanel);
            }))
            .child(
                icons::icon(if panel_open {
                    icons::ALT_ARROW_DOWN
                } else {
                    icons::ALT_ARROW_RIGHT
                })
                .size(px(12.0))
                .text_color(theme.text_muted),
            ),
    )
    .into_any_element()
}

// ---------------------------------------------------------------------------
// Sidebar nested rows
// ---------------------------------------------------------------------------

pub const SIDEBAR_CHILD_HEIGHT: f32 = 22.0;
pub const SIDEBAR_CHILD_MAX: usize = 3;
/// Breathing room between a card's context line and its first child row.
pub const SIDEBAR_CHILD_GAP: f32 = 2.0;
/// Extra bottom inset inside the child disclosure; the card keeps its base padding.
pub const SIDEBAR_CHILD_PAD_BOTTOM: f32 = 4.0;

/// Up to `SIDEBAR_CHILD_MAX` running subagents as EXTRA LINES inside the
/// chat card (after the context line), sharing its wash and radius. Each row: 12px
/// status glyph at the card's text-start x, 6px gap, 12px `text_muted`
/// title truncating, mono 11px `text_faint` elapsed flush to the card's
/// right edge. No tree stubs, no hairlines. `+N more` opens the Agents
/// panel. Children stop click propagation (they open the thread, not the
/// card's plain select).
#[allow(clippy::too_many_arguments)]
pub fn sidebar_children(
    chat_id: &str,
    summaries: SubagentSubset<'_>,
    theme: &Theme,
    view: gpui::EntityId,
    open: OpenAgent,
    open_panel: OpenPanel,
    cx: &Context<Shell>,
) -> AnyElement {
    let more = summaries.len().saturating_sub(SIDEBAR_CHILD_MAX);
    let mut col = div()
        .w_full()
        .flex()
        .flex_col()
        .pt(px(SIDEBAR_CHILD_GAP))
        .pb(px(SIDEBAR_CHILD_PAD_BOTTOM));
    for s in summaries.iter().take(SIDEBAR_CHILD_MAX) {
        let summary = s.clone();
        let chat = chat_id.to_string();
        let open = open.clone();
        // A delegated child is a real chat: its row opens it and drags to a
        // split like any session row. Native subagents have no chat to drag.
        let child_chat = s
            .delegated
            .as_deref()
            .map(|link| link.child_chat_id.clone());
        let mark = s
            .delegated
            .as_deref()
            .and_then(|link| provider_mark(link, theme));
        let mut row = div()
            .id(SharedString::from(format!("sub-child-{}", s.id)))
            .h(px(SIDEBAR_CHILD_HEIGHT))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                if !this.sidebar_drag_suppressed_click {
                    open(this, chat.clone(), summary.clone(), cx);
                }
            }));
        if let Some(child_chat) = child_chat {
            row = row.on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.press_sidebar_session(child_chat.clone(), event.position, cx);
                }),
            );
        }
        if s.delegated.is_some() {
            row = row.tooltip(delegated_tooltip(s.clone()));
        }
        col = col.child(
            row.child(
                div()
                    .size(px(12.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(status_glyph(
                        SharedString::from(format!("sub-glyph-{}", s.id)),
                        s.status,
                        true,
                        theme,
                        view,
                        cx,
                    )),
            )
            .children(mark)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(crate::typography::ui_rems(12.0))
                    .line_height(px(SIDEBAR_CHILD_HEIGHT))
                    .text_color(theme.text_muted)
                    .child(s.title.clone()),
            )
            .child(elapsed_label(
                format!("sidebar-clock-{chat_id}-{}", s.id),
                s.clock(),
                theme.text_faint,
            )),
        );
    }
    if more > 0 {
        let parent = chat_id.to_string();
        col = col.child(
            div()
                .id(SharedString::from(format!("sub-more-{chat_id}")))
                .h(px(SIDEBAR_CHILD_HEIGHT))
                .w_full()
                .flex()
                .items_center()
                // No glyph: indent to the child rows' title start.
                .pl(px(12.0 + 6.0))
                .text_size(crate::typography::ui_rems(12.0))
                .text_color(theme.text_faint)
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    open_panel(this, parent.clone(), cx);
                }))
                .child(SharedString::from(format!("+{more} more"))),
        );
    }
    col.into_any_element()
}

// ---------------------------------------------------------------------------
// Agents panel (RightSurface::Agents)
// ---------------------------------------------------------------------------

const AGENTS_ROW_HEIGHT: f32 = 44.0;
const AGENTS_ROW_HEIGHT_BARE: f32 = 32.0;
const AGENTS_SECTION_HEIGHT: f32 = 30.0;
/// T3's relationship row: h 36.
const RELATION_ROW_HEIGHT: f32 = 36.0;

/// One section header: label (+ count while collapsed), optionally clickable.
fn agents_section(label: String, dot: Option<gpui::Hsla>, theme: &Theme) -> gpui::Div {
    div()
        .h(px(AGENTS_SECTION_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(Theme::SPACE_SM))
        .font_weight(FontWeight::MEDIUM)
        .text_size(crate::typography::ui_rems(11.5))
        .text_color(theme.text_faint)
        .children(dot.map(|hue| div().size(px(6.0)).flex_none().rounded_full().bg(hue)))
        .child(SharedString::from(label))
}

/// The three row groups the panel pages independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineageGroup {
    Related,
    Active,
    Previous,
}

/// Per-chat panel state owned by the shell: whether "Previous agents" is open
/// and how many rows of each group are showing (T3: 6, then +12 a click).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelUi {
    pub previous_open: bool,
    shown: [usize; 3],
}

impl Default for PanelUi {
    fn default() -> Self {
        Self {
            previous_open: false,
            shown: [crate::delegation::LINEAGE_INITIAL; 3],
        }
    }
}

impl PanelUi {
    fn slot(group: LineageGroup) -> usize {
        match group {
            LineageGroup::Related => 0,
            LineageGroup::Active => 1,
            LineageGroup::Previous => 2,
        }
    }

    pub fn shown(&self, group: LineageGroup) -> usize {
        self.shown[Self::slot(group)]
    }

    pub fn show_more(&mut self, group: LineageGroup) {
        self.shown[Self::slot(group)] += crate::delegation::LINEAGE_PAGE;
    }
}

/// What a panel click does, as shell callbacks (so the panel stays a pure
/// element builder).
#[derive(Clone)]
pub struct PanelActions {
    pub open: OpenAgent,
    /// Navigate to a related chat (parent, fork).
    pub open_chat: Rc<dyn Fn(&mut Shell, String, &mut Context<Shell>)>,
    /// `task_cancel` for one task.
    pub stop: Rc<dyn Fn(&mut Shell, String, &mut Context<Shell>)>,
    /// Whole-thread Stop: the run, background work and every owned task.
    pub stop_all: Rc<dyn Fn(&mut Shell, &mut Context<Shell>)>,
    pub toggle_previous: Rc<dyn Fn(&mut Shell, &mut Context<Shell>)>,
    pub show_more: Rc<dyn Fn(&mut Shell, LineageGroup, &mut Context<Shell>)>,
}

/// Whether the Active header offers "Stop all" (the host supports it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopAll {
    Unavailable,
    Ready,
    Stopping,
}

/// The header's quiet 20px Stop target, sized like a row's Stop.
fn stop_all_button(state: StopAll, theme: &Theme, actions: &PanelActions, cx: &Context<Shell>) -> AnyElement {
    let stop_all = actions.stop_all.clone();
    let stopping = state == StopAll::Stopping;
    div()
        .id("agents-stop-all")
        .role(gpui::Role::Button)
        .aria_label("Stop all agents")
        .size(px(20.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .when(!stopping, |el| {
            el.cursor_pointer()
                .hover(|el| el.bg(crate::theme::wash(0.10)))
                .tab_index(0)
                .focus_visible(|el| el.bg(crate::theme::wash(0.10)))
        })
        .when(stopping, |el| el.opacity(0.45))
        .tooltip(crate::tooltip::text(if stopping {
            "Stopping…"
        } else {
            "Stop all agents and background work in this thread"
        }))
        .on_click(cx.listener(move |this, _, _, cx| {
            cx.stop_propagation();
            if !stopping {
                stop_all(this, cx);
            }
        }))
        .child(
            icons::icon(icons::STOP)
                .size(px(9.0))
                .text_color(theme.text_muted),
        )
        .into_any_element()
}

/// A parent / fork row (T3's non-agent relationship rows): relation glyph,
/// title, relation word, hover arrow.
fn relation_row(
    row: &crate::delegation::RelationRow,
    theme: &Theme,
    actions: &PanelActions,
    cx: &Context<Shell>,
) -> AnyElement {
    use crate::delegation::{Relation, ThreadLinkKind};
    let glyph = match row.relation {
        Relation::Parent(ThreadLinkKind::Subagent) => icons::ALT_ARROW_LEFT,
        Relation::Parent(ThreadLinkKind::Fork) | Relation::Fork => icons::GIT_BRANCH,
    };
    let chat = row.chat_id.clone();
    let open_chat = actions.open_chat.clone();
    div()
        .id(SharedString::from(format!("relation-row-{}", row.chat_id)))
        .group("relation-row")
        .h(px(RELATION_ROW_HEIGHT))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(Theme::SPACE_SM))
        .rounded(px(8.0))
        .cursor_pointer()
        .hover(|el| el.bg(crate::theme::wash(0.06)))
        // Keyboard: Tab reaches the row, ↑/↓ step between rows, Enter opens.
        .tab_index(0)
        .focus_visible(|el| el.bg(crate::theme::wash(0.10)))
        .on_key_down(|event, window, cx| {
            let modifiers = event.keystroke.modifiers;
            if modifiers.control || modifiers.alt || modifiers.platform {
                return;
            }
            match event.keystroke.key.as_str() {
                "down" => window.focus_next(cx),
                "up" => window.focus_prev(cx),
                _ => return,
            }
            cx.stop_propagation();
        })
        .tooltip(crate::tooltip::text(format!(
            "Open {} in this chat",
            row.relation.label().to_lowercase()
        )))
        .on_click(cx.listener(move |this, _, _, cx| open_chat(this, chat.clone(), cx)))
        .child(
            icons::icon(glyph)
                .size(px(12.0))
                .flex_none()
                .text_color(theme.text_faint),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(crate::typography::ui_rems(13.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.text)
                .child(row.title.clone()),
        )
        .child(
            div()
                .flex_none()
                .invisible()
                .group_hover("relation-row", |style| style.visible())
                .child(
                    icons::icon(icons::ARROW_RIGHT)
                        .size(px(12.0))
                        .text_color(theme.text_faint),
                ),
        )
        .child(
            div()
                .flex_none()
                .text_size(crate::typography::ui_rems(11.0))
                .text_color(theme.text_faint)
                .child(row.relation.label()),
        )
        .into_any_element()
}

/// "Show N more" under a paged group.
fn show_more_row(
    group: LineageGroup,
    hidden: usize,
    theme: &Theme,
    actions: &PanelActions,
    cx: &Context<Shell>,
) -> AnyElement {
    let show_more = actions.show_more.clone();
    div()
        .id(SharedString::from(format!("agents-show-more-{group:?}")))
        .h(px(32.0))
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(Theme::SPACE_SM))
        .rounded(px(8.0))
        .cursor_pointer()
        .text_size(crate::typography::ui_rems(12.0))
        .text_color(theme.text_faint)
        .hover(|el| el.bg(crate::theme::wash(0.06)).text_color(theme.text_muted))
        .on_click(cx.listener(move |this, _, _, cx| show_more(this, group, cx)))
        .child(icons::icon(icons::PLUS).size(px(12.0)).flex_none())
        .child(SharedString::from(format!(
            "Show {} more",
            hidden.min(crate::delegation::LINEAGE_PAGE)
        )))
        .into_any_element()
}

/// The panel's two agent groups and the counts its headers speak.
#[derive(Debug)]
pub(crate) struct AgentGroups<'a> {
    pub active: Vec<&'a SubagentSummary>,
    /// Settled agents. A delegated task still waiting on nested work is
    /// active, whatever its own run did.
    pub previous: Vec<&'a SubagentSummary>,
    /// Pulsing work: only agents actually running.
    pub running: usize,
    pub failed: usize,
}

pub(crate) fn agent_groups(summaries: &[SubagentSummary]) -> AgentGroups<'_> {
    let (active, previous): (Vec<_>, Vec<_>) = summaries.iter().partition(|s| s.status.active());
    AgentGroups {
        running: active
            .iter()
            .filter(|s| s.status == SubagentPhase::Running)
            .count(),
        failed: previous
            .iter()
            .filter(|s| s.status == SubagentPhase::Failed)
            .count(),
        active,
        previous,
    }
}

/// One agent row: native subagents (observational) and delegated tasks (real
/// chats, with provider, workState and Stop) share the geometry.
fn agent_row(
    s: &SubagentSummary,
    chat_id: &str,
    seen: &HashSet<String>,
    theme: &Theme,
    view: gpui::EntityId,
    actions: &PanelActions,
    cx: &Context<Shell>,
) -> AnyElement {
    let summary = s.clone();
    let chat = chat_id.to_string();
    let open = actions.open.clone();
    let is_seen = seen.contains(&s.id)
        || s.doc_ref
            .as_ref()
            .is_some_and(|d| seen.contains(d.as_str()))
        || s.delegated
            .as_deref()
            .is_some_and(|link| seen.contains(&link.child_chat_id));
    let link = s.delegated.as_deref();
    // Right meta: `agent_type · model [· effort]` (mono 11px) - any part may
    // be absent, and the dot is omitted when only one exists.
    let meta = [
        s.agent_type.as_deref(),
        s.model.as_deref(),
        link.and_then(|l| l.effort.as_deref()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" \u{00b7} ");
    // Line 2 (left): a delegated row leads with its workState word and the
    // latest result after it; a native row is just its one-line summary.
    let state_word: Option<SharedString> = link.map(|l| {
        SharedString::from(if l.stopping {
            "Stopping\u{2026}"
        } else {
            l.work_state.label()
        })
    });
    let summary_line = s.summary.clone();
    let line_two: SharedString = match (&state_word, &summary_line) {
        (Some(word), Some(line)) => format!("{word} \u{00b7} {line}").into(),
        (Some(word), None) => word.clone(),
        (None, line) => line.clone().unwrap_or_default(),
    };
    // No summary and no meta (and no workState): the row collapses to the
    // title-only 32px (a line-2 slot with nothing on either side).
    let bare = summary_line.is_none() && meta.is_empty() && state_word.is_none();
    let mark = link.and_then(|l| provider_mark(l, theme));
    let mark_inset = if mark.is_some() { 12.0 + 8.0 } else { 0.0 };
    let stop = s.stoppable().map(|l| l.task_id.clone());
    let stop_action = actions.stop.clone();
    let mut row = div()
        .id(SharedString::from(format!("agents-row-{}", s.id)))
        .h(px(if bare {
            AGENTS_ROW_HEIGHT_BARE
        } else {
            AGENTS_ROW_HEIGHT
        }))
        .w_full()
        .flex()
        .flex_col()
        .justify_center()
        .px(px(Theme::SPACE_SM))
        .cursor_pointer()
        .hover(|el| el.bg(crate::theme::wash(0.06)))
        .on_click(cx.listener(move |this, _, _, cx| {
            open(this, chat.clone(), summary.clone(), cx);
        }));
    if let Some(child_chat) = link.map(|l| l.child_chat_id.clone()) {
        // A delegated child is a real chat: the row drags into a split.
        row = row.on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                this.press_sidebar_session(child_chat.clone(), event.position, cx);
            }),
        );
        row = row.tooltip(delegated_tooltip(s.clone()));
    }
    row
        // Line 1 (18px): 12px glyph + 8px + (provider mark) + 13px title
        // truncating, then Stop and the self-ticking mono elapsed.
        .child(
            div()
                .w_full()
                .h(px(18.0))
                .flex()
                .flex_row()
                // Centered, not baseline: the glyph box has no text
                // baseline, so baseline alignment dropped it below the
                // title.
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .size(px(12.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(status_glyph(
                            SharedString::from(format!("agents-glyph-{}", s.id)),
                            s.status,
                            is_seen,
                            theme,
                            view,
                            cx,
                        )),
                )
                .children(mark)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(crate::typography::ui_rems(13.0))
                        .text_color(theme.text)
                        .child(s.title.clone()),
                )
                .children(stop.map(|task_id| {
                    // A quiet 20px target with a small stop mark: the full
                    // 14px glyph read as a black block next to 13px text.
                    div()
                        .id(SharedString::from(format!("agents-stop-{}", s.id)))
                        .size(px(20.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .hover(|el| el.bg(crate::theme::wash(0.10)))
                        .tooltip(crate::tooltip::text("Stop task"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            stop_action(this, task_id.clone(), cx);
                        }))
                        .child(
                            icons::icon(icons::STOP)
                                .size(px(9.0))
                                .text_color(theme.text_muted),
                        )
                }))
                .child(elapsed_label(
                    format!("panel-clock-{chat_id}-{}", s.id),
                    s.clock(),
                    theme.text_faint,
                )),
        )
        // Line 2 (16px) starts at the title's x: workState + one-line
        // summary truncating, `agent_type · model · effort` right-aligned.
        // Either side may be absent (empty rows collapse above).
        .when(!bare, |row| {
            row.child(
                div()
                    .w_full()
                    .h(px(16.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .pl(px(12.0 + 8.0 + mark_inset))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(crate::typography::ui_rems(12.0))
                            .text_color(theme.text_muted)
                            .child(line_two),
                    )
                    .when(!meta.is_empty(), |el| {
                        el.child(
                            div()
                                .flex_none()
                                .font_family(theme.font_mono.clone())
                                .text_size(crate::typography::ui_rems(11.0))
                                .text_color(theme.text_faint)
                                .child(SharedString::from(meta)),
                        )
                    }),
            )
        })
        .into_any_element()
}

/// The right-pane inventory: lineage (parent, forks) first, then every agent
/// the selector sees for `chat_id` (not just the strip-visible subset) -
/// Active, then a collapsible Previous group - each clickable to its thread.
/// Groups page 6 rows then +12 a click (T3's `ThreadRelationshipsControl`).
#[allow(clippy::too_many_arguments)]
pub fn agents_panel_body(
    chat_id: &str,
    related: &[crate::delegation::RelationRow],
    summaries: &[SubagentSummary],
    seen: &HashSet<String>,
    ui: &PanelUi,
    stop_all: StopAll,
    theme: &Theme,
    view: gpui::EntityId,
    actions: &PanelActions,
    cx: &Context<Shell>,
) -> AnyElement {
    if summaries.is_empty() && related.is_empty() {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_size(crate::typography::ui_rems(12.0))
            .text_color(theme.text_faint)
            .child("No agents yet")
            .into_any_element();
    }
    let AgentGroups {
        active,
        previous,
        running,
        failed,
    } = agent_groups(summaries);
    let mut children: Vec<AnyElement> = Vec::new();
    if !related.is_empty() {
        let (rows, hidden) =
            crate::delegation::lineage_window(related, ui.shown(LineageGroup::Related));
        children.push(agents_section("Lineage".into(), None, theme).into_any_element());
        children.extend(rows.iter().map(|row| relation_row(row, theme, actions, cx)));
        if hidden > 0 {
            children.push(show_more_row(
                LineageGroup::Related,
                hidden,
                theme,
                actions,
                cx,
            ));
        }
    }
    if !active.is_empty() {
        let (rows, hidden) =
            crate::delegation::lineage_window(&active, ui.shown(LineageGroup::Active));
        // T3: `Lineage · N running`; the heading carries the live count.
        let label = if running > 0 {
            format!("Active \u{00b7} {running} running")
        } else {
            "Active".to_owned()
        };
        let mut header = agents_section(label, SessionState::Working.color(theme), theme);
        if running > 0 && stop_all != StopAll::Unavailable {
            header = header
                .child(div().flex_1())
                .child(stop_all_button(stop_all, theme, actions, cx));
        }
        children.push(header.into_any_element());
        children.extend(
            rows.iter()
                .map(|s| agent_row(s, chat_id, seen, theme, view, actions, cx)),
        );
        if hidden > 0 {
            children.push(show_more_row(
                LineageGroup::Active,
                hidden,
                theme,
                actions,
                cx,
            ));
        }
    }
    if !previous.is_empty() {
        let open = ui.previous_open;
        let toggle = actions.toggle_previous.clone();
        let mut label = format!("Previous agents \u{00b7} {}", previous.len());
        if !open && failed > 0 {
            label = format!("{label} \u{00b7} {failed} failed");
        }
        children.push(
            agents_section(label, None, theme)
                .id("agents-previous-toggle")
                .cursor_pointer()
                .hover(|el| el.text_color(theme.text_muted))
                .on_click(cx.listener(move |this, _, _, cx| toggle(this, cx)))
                .child(div().flex_1())
                .child(
                    icons::icon(if open {
                        icons::ALT_ARROW_DOWN
                    } else {
                        icons::ALT_ARROW_RIGHT
                    })
                    .size(px(11.0))
                    .text_color(theme.text_faint),
                )
                .into_any_element(),
        );
        if open {
            let (rows, hidden) =
                crate::delegation::lineage_window(&previous, ui.shown(LineageGroup::Previous));
            children.extend(
                rows.iter()
                    .map(|s| agent_row(s, chat_id, seen, theme, view, actions, cx)),
            );
            if hidden > 0 {
                children.push(show_more_row(
                    LineageGroup::Previous,
                    hidden,
                    theme,
                    actions,
                    cx,
                ));
            }
        }
    }
    div()
        .id("agents-panel-rows")
        .size_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .pb(px(Theme::SPACE_SM))
        .children(children)
        .into_any_element()
}

impl crate::composer::Composer {
    /// The read-only banner when this composer's chat is a delegated child,
    /// else `None` (an ordinary, sendable chat). Reads only the update-owned
    /// delegation index.
    pub(crate) fn render_delegated_child_banner(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let (chat_id, model) = {
            let state = self.state.read(cx);
            let chat_id = self.target.chat_id(state)?.to_owned();
            let link = state.delegation.link_for(&chat_id)?;
            let title_of = |id: &str| {
                state
                    .chats
                    .iter()
                    .find(|chat| chat.id == id)
                    .and_then(|chat| chat.title.clone())
            };
            let config = state
                .chats
                .iter()
                .find(|chat| chat.id == chat_id)
                .and_then(|chat| chat.config.as_ref());
            let model = crate::delegation::child_banner(
                &state.delegation,
                &chat_id,
                title_of(&link.parent_chat_id),
                config,
            )?;
            (chat_id, model)
        };
        let theme = Theme::of(cx).clone();
        let parent = model.parent_chat_id.clone();
        let task_id = model.task.task_id.clone();
        Some(delegated_child_banner(
            &chat_id,
            &model,
            &theme,
            cx.entity_id(),
            cx.listener(move |_, _, _, cx| {
                cx.emit(crate::composer::ComposerEvent::OpenChat {
                    chat_id: parent.clone(),
                });
            }),
            cx.listener(move |_, _, _, cx| {
                cx.emit(crate::composer::ComposerEvent::StopDelegatedTask {
                    task_id: task_id.clone(),
                });
            }),
            cx,
        ))
    }

    /// The agents tray: a queue-tray surface over this composer's chat, or
    /// `None` when nothing qualifies (no chat, or no visible subagents).
    /// `tucked` (queue tray rendered below) drops the tray's own radius:
    /// the queue tray's top edge becomes the shared seam.
    pub(crate) fn render_agents_tray(
        &mut self,
        tucked: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let chat_id = self
            .target
            .chat_id(self.state.read(cx))
            .map(str::to_owned)?;
        let presentation = subagents_for(self.state.read(cx), &chat_id);
        let summaries = strip_visible(&presentation);
        if summaries.is_empty() {
            return None;
        }
        let theme = Theme::of(cx).clone();
        // Fitting budget: the tray's inner width = the measured composer
        // column minus the tray inset on both sides minus the row pads.
        let inner_width = self
            .last_available_width()
            .unwrap_or(crate::composer::COMPOSER_MAX_WIDTH)
            - 2.0 * crate::composer::QUEUE_SIDE_INSET
            - 12.0
            - 6.0;
        let panel_open = self.agents_panel_open;
        let seen = std::mem::take(&mut self.subagent_seen);
        let row = agents_tray_row(
            &chat_id,
            summaries,
            inner_width,
            panel_open,
            &seen,
            &theme,
            cx.entity_id(),
            cx,
        );
        self.subagent_seen = seen;
        let surface = crate::queue::queue_panel_surface(&theme)
            .when(tucked, |el| el.rounded_bl(px(0.0)).rounded_br(px(0.0)))
            .child(row);
        Some(
            crate::frost::frosted(crate::queue::PANEL_RADIUS, crate::frost::MENU_BLUR, surface)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeron_doc::{MessagePart, MessageRole};
    use zeron_proto::ToolCall;

    fn entry(id: &str, role: MessageRole, parts: Vec<MessagePart>) -> SessionMessageEntry {
        SessionMessageEntry {
            id: id.into(),
            role,
            parts,
            created_at: 1_700_000_000_000,
            device_id: "dev".into(),
            status: Some(MessageStatus::Complete),
            continuation_of: None,
        }
    }

    fn spawn(id: &str, resolved: bool, status: Option<SubagentStatus>) -> MessagePart {
        MessagePart::Tool {
            id: id.into(),
            call: ToolCall::Unknown {
                name: "Agent: scout".into(),
                input: Some(serde_json::json!({"description": "scout"})),
            },
            is_error: false,
            resolved,
            output: None,
            diff: None,
            output_ref: None,
            output_bytes: None,
            diff_ref: None,
            diff_stats: None,
            subagent_ref: status.map(|_| "doc-1".to_string()),
            subagent_status: status,
            subagent_tail: None,
        }
    }

    #[test]
    fn selector_orders_running_then_finished_and_tags_latest_turn() {
        let mut state = AppState::new();
        state.selected_chat = Some("c".into());
        let done = spawn("a", true, Some(SubagentStatus::Done));
        let running = spawn("b", false, Some(SubagentStatus::Running));
        let mut started = spawn("s", true, None);
        if let MessagePart::Tool { call, .. } = &mut started {
            *call = ToolCall::Unknown {
                name: "Agent: bg".into(),
                input: Some(serde_json::json!({"run_in_background": true})),
            };
        }
        state.apply_transcript(vec![
            entry("u1", MessageRole::User, vec![]),
            entry(
                "m1",
                MessageRole::Assistant,
                vec![done.clone(), running.clone()],
            ),
            entry("u2", MessageRole::User, vec![]),
            entry("m2", MessageRole::Assistant, vec![started]),
        ]);
        let out = subagents_for(&state, "c");
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].status, SubagentPhase::Running);
        assert_eq!(out[0].id, "b");
        assert_eq!(out[1].status, SubagentPhase::Started);
        assert_eq!(out[2].status, SubagentPhase::Done);
        assert!(out[1].latest_turn);
        assert!(!out[2].latest_turn);
    }

    #[test]
    fn pi_spawn_lifecycle_comes_from_the_tool_call() {
        let mut state = AppState::new();
        state.selected_chat = Some("c".into());
        // Unresolved in a streaming turn = Running; resolved with output = Done.
        let unresolved = spawn("p1", false, None);
        let mut resolved = spawn("p2", true, None);
        if let MessagePart::Tool { output, .. } = &mut resolved {
            *output = Some("agent report: all green".into());
        }
        let mut live = entry("m", MessageRole::Assistant, vec![unresolved.clone()]);
        live.status = Some(MessageStatus::Streaming);
        state.apply_transcript(vec![
            entry("u", MessageRole::User, vec![]),
            live,
            entry("m2", MessageRole::Assistant, vec![resolved]),
        ]);
        let out = subagents_for(&state, "c");
        assert_eq!(out[0].status, SubagentPhase::Running);
        assert_eq!(out[1].status, SubagentPhase::Done);
        assert_eq!(out[1].summary.as_deref(), Some("agent report: all green"));
    }

    #[test]
    fn eager_done_spawn_reads_started_not_done() {
        let mut state = AppState::new();
        state.selected_chat = Some("c".into());
        let resolved_no_output = spawn("e", true, None);
        let mut live = entry("m", MessageRole::Assistant, vec![resolved_no_output]);
        live.status = Some(MessageStatus::Streaming);
        state.apply_transcript(vec![entry("u", MessageRole::User, vec![]), live]);
        let out = subagents_for(&state, "c");
        assert_eq!(out[0].status, SubagentPhase::Started);
    }

    #[test]
    fn finished_observation_requires_an_active_observation_this_session() {
        let mut state = AppState::new();
        state.selected_chat = Some("c".into());
        // A settled agent first observed after an app restart (no doc, no
        // output): never stamp a finish time and never show a guessed
        // elapsed.
        state.apply_transcript(vec![
            entry("u", MessageRole::User, vec![]),
            entry(
                "m",
                MessageRole::Assistant,
                vec![spawn("a", true, Some(SubagentStatus::Done))],
            ),
        ]);
        let out = subagents_for(&state, "c");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].status, SubagentPhase::Done);
        assert!(out[0].finished.is_none());
        assert!(out[0].elapsed(Utc::now()).is_none());
        assert!(state.subagent_finished_obs.is_empty());

        // The same agent seen Running first, then Done: the transition
        // stamps a finish observation and elapsed freezes.
        let mut live = entry(
            "m",
            MessageRole::Assistant,
            vec![spawn("a", false, Some(SubagentStatus::Running))],
        );
        live.status = Some(MessageStatus::Streaming);
        state.apply_transcript(vec![entry("u", MessageRole::User, vec![]), live]);
        let out = subagents_for(&state, "c");
        assert_eq!(out[0].status, SubagentPhase::Running);

        state.apply_transcript(vec![
            entry("u", MessageRole::User, vec![]),
            entry(
                "m",
                MessageRole::Assistant,
                vec![spawn("a", true, Some(SubagentStatus::Done))],
            ),
        ]);
        // Update handling stamps the observation, not the first render.
        let out = subagents_for(&state, "c");
        assert_eq!(out[0].status, SubagentPhase::Done);
        assert!(out[0].finished.is_some());
        assert!(out[0].elapsed(Utc::now()).is_some());
    }

    #[test]
    fn a_child_thread_finds_its_summary_by_doc_or_result_doc() {
        let summary = |id: &str, doc: Option<&str>| SubagentSummary {
            id: id.into(),
            title: id.to_string().into(),
            agent_type: None,
            model: None,
            status: SubagentPhase::Running,
            started: None,
            finished: None,
            summary: None,
            doc_ref: doc.map(|d| d.to_string().into()),
            latest_turn: true,
            delegated: None,
        };
        let all = Arc::new(SubagentPresentation::new(vec![
            summary("a", Some("doc-a")),
            summary("b", None),
        ]));
        assert_eq!(
            summary_for_doc(&all, "chat", "doc-a").map(|s| s.id.as_str()),
            Some("a")
        );
        // A doc-less harness opens its synthetic result doc.
        assert_eq!(
            summary_for_doc(&all, "chat", &result_doc_id("chat", "b")).map(|s| s.id.as_str()),
            Some("b")
        );
        assert!(summary_for_doc(&all, "chat", "elsewhere").is_none());
        assert!(
            std::ptr::eq(summary_for_doc(&all, "chat", "doc-a").unwrap(), &all[0],),
            "the banner borrows the shared presentation rather than cloning a summary"
        );
    }

    #[test]
    fn warm_picker_consumers_share_empty_and_nonempty_presentations_without_scans() {
        for agents in [false, true] {
            let mut state = AppState::new();
            state.selected_chat = Some("c".into());
            // 21 MB-class loaded history, including the important zero-agent
            // case. This is an attribution test, not a native timing claim.
            let text = "x".repeat(2_160);
            let mut entries: Vec<_> = (0..10_000)
                .map(|ix| {
                    entry(
                        &format!("m{ix}"),
                        MessageRole::Assistant,
                        vec![MessagePart::Text {
                            id: format!("t{ix}"),
                            text: text.clone(),
                        }],
                    )
                })
                .collect();
            if agents {
                entries.push(entry(
                    "spawns",
                    MessageRole::Assistant,
                    vec![
                        spawn("live", false, Some(SubagentStatus::Running)),
                        spawn("finished", true, Some(SubagentStatus::Done)),
                    ],
                ));
            }
            state.apply_transcript(entries);
            let before = crate::perf_trace::snapshot();
            for _ in 0..300 {
                let _ = derive_subagents(&state, "c");
            }
            let baseline = crate::perf_trace::snapshot().subagent_scans - before.subagent_scans;
            let presentation = subagents_for(&state, "c");
            let before = crate::perf_trace::snapshot();
            // 50 open/close cycles, three consumers per frame.
            for _ in 0..50 {
                for _ in 0..2 {
                    for _ in 0..3 {
                        let hit = subagents_for(&state, "c");
                        assert!(Arc::ptr_eq(&presentation, &hit));
                        let _ = strip_visible(&hit).len();
                        let _ = hit.running().len();
                        let banner = summary_for_doc(&hit, "c", "doc-1");
                        if agents {
                            assert!(std::ptr::eq(banner.unwrap(), &hit[0]));
                        } else {
                            assert!(banner.is_none());
                        }
                    }
                }
            }
            let after = crate::perf_trace::snapshot();
            assert_eq!(baseline, 300);
            assert_eq!(after.subagent_scans - before.subagent_scans, 0);
            assert_eq!(after.subagent_cache_hits - before.subagent_cache_hits, 300);
            eprintln!(
                "subagent evidence: agents={agents}, 50 cycles/3 consumers: history scans {baseline} -> 0, shared hits=300"
            );
            if agents {
                assert!(std::ptr::eq(
                    presentation.running().iter().next().unwrap(),
                    &presentation[0],
                ));
                assert!(std::ptr::eq(
                    strip_visible(&presentation).iter().next().unwrap(),
                    &presentation[0],
                ));
            } else {
                assert!(presentation.is_empty());
            }
        }
    }

    #[test]
    #[ignore = "comparative optimized-test benchmark; run without concurrent builds"]
    fn profile_streaming_subagent_projection() {
        for agents in [false, true] {
            let mut state = AppState::new();
            state.selected_chat = Some("c".into());
            let text = "x".repeat(2_160);
            let mut entries: Vec<_> = (0..10_000)
                .map(|ix| {
                    entry(
                        &format!("m{ix}"),
                        MessageRole::Assistant,
                        vec![MessagePart::Text {
                            id: format!("t{ix}"),
                            text: text.clone(),
                        }],
                    )
                })
                .collect();
            if agents {
                entries.push(entry(
                    "spawns",
                    MessageRole::Assistant,
                    vec![
                        spawn("live", false, Some(SubagentStatus::Running)),
                        spawn("finished", true, Some(SubagentStatus::Done)),
                    ],
                ));
            }
            let mut tail = entry(
                "tail",
                MessageRole::Assistant,
                vec![MessagePart::Text {
                    id: "text".into(),
                    text: String::new(),
                }],
            );
            tail.status = Some(MessageStatus::Streaming);
            entries.push(tail);
            let count = entries.len();
            state.apply_transcript(entries);
            let before = crate::perf_trace::snapshot();
            let start = std::time::Instant::now();
            for len in 1..=500 {
                state
                    .apply_transcript_frame(zeron_doc::TranscriptFrame::Delta {
                        upsert: vec![],
                        append: vec![zeron_doc::TextAppend {
                            entry: "tail".into(),
                            part: "text".into(),
                            text: "x".into(),
                            len,
                        }],
                        remove: vec![],
                        count,
                    })
                    .unwrap();
            }
            let elapsed_us = start.elapsed().as_micros();
            let scans = crate::perf_trace::snapshot().subagent_scans - before.subagent_scans;
            eprintln!(
                "stream_projection_profile agents={agents} history_entries=10000 \
                 history_bytes=21600000 appends=500 elapsed_us={elapsed_us} subagent_scans={scans}"
            );
        }
    }

    fn text_append(
        entry: &str,
        part: &str,
        text: &str,
        len: usize,
        count: usize,
    ) -> zeron_doc::TranscriptFrame {
        zeron_doc::TranscriptFrame::Delta {
            upsert: vec![],
            append: vec![zeron_doc::TextAppend {
                entry: entry.into(),
                part: part.into(),
                text: text.into(),
                len,
            }],
            remove: vec![],
            count,
        }
    }

    #[test]
    fn pure_text_and_reasoning_keep_shared_spawn_presentations_in_both_reducers() {
        for agents in [false, true] {
            let mut state = AppState::new();
            state.selected_chat = Some("c".into());
            let mut parts = vec![
                MessagePart::Text {
                    id: "text".into(),
                    text: String::new(),
                },
                MessagePart::Reasoning {
                    id: "thought".into(),
                    text: String::new(),
                },
            ];
            if agents {
                parts.push(spawn("live", false, Some(SubagentStatus::Running)));
                parts.push(spawn("done", true, Some(SubagentStatus::Done)));
            }
            let entries = vec![entry("m", MessageRole::Assistant, parts)];
            state.apply_transcript(entries.clone());
            state.set_subagent_snapshot("pane".into(), entries);
            let primary = subagents_for(&state, "c");
            let pane = subagents_for(&state, "pane");
            let before = crate::perf_trace::snapshot().subagent_scans;
            for part in ["text", "thought"] {
                state
                    .apply_transcript_frame(text_append("m", part, "x", 1, 1))
                    .unwrap();
                state
                    .apply_sub_transcript_frame("pane", text_append("m", part, "x", 1, 1))
                    .unwrap();
            }
            assert_eq!(crate::perf_trace::snapshot().subagent_scans, before);
            assert!(Arc::ptr_eq(&primary, &subagents_for(&state, "c")));
            assert!(Arc::ptr_eq(&pane, &subagents_for(&state, "pane")));

            // A structural/status update still rebuilds the inventory.
            state
                .apply_transcript_frame(zeron_doc::TranscriptFrame::Delta {
                    upsert: vec![zeron_doc::TranscriptUpsert {
                        after: None,
                        entry: entry(
                            "m",
                            MessageRole::Assistant,
                            vec![spawn("new", true, Some(SubagentStatus::Failed))],
                        ),
                    }],
                    append: vec![],
                    remove: vec![],
                    count: 1,
                })
                .unwrap();
            assert!(!Arc::ptr_eq(&primary, &subagents_for(&state, "c")));
            assert_eq!(subagents_for(&state, "c")[0].status, SubagentPhase::Failed);
        }
    }

    #[test]
    fn child_text_refreshes_just_its_referencing_parents_not_its_own_inventory() {
        let mut state = AppState::new();
        state.selected_chat = Some("parent".into());
        state.apply_transcript(vec![entry(
            "spawn",
            MessageRole::Assistant,
            vec![spawn("a", true, Some(SubagentStatus::Done))],
        )]);
        state.set_subagent_snapshot("unrelated".into(), vec![]);
        state.set_subagent_snapshot(
            "doc-1".into(),
            vec![entry(
                "child",
                MessageRole::Assistant,
                vec![MessagePart::Text {
                    id: "result".into(),
                    text: "Child".into(),
                }],
            )],
        );
        let parent = subagents_for(&state, "parent");
        let child = subagents_for(&state, "doc-1");
        let unrelated = subagents_for(&state, "unrelated");
        let before = crate::perf_trace::snapshot().subagent_scans;
        state
            .apply_sub_transcript_frame(
                "doc-1",
                text_append("child", "result", " finished.", 15, 1),
            )
            .unwrap();
        assert_eq!(crate::perf_trace::snapshot().subagent_scans - before, 1);
        let after = subagents_for(&state, "parent");
        assert!(!Arc::ptr_eq(&parent, &after));
        assert_eq!(after[0].summary.as_deref(), Some("Child finished."));
        assert!(Arc::ptr_eq(&child, &subagents_for(&state, "doc-1")));
        assert!(Arc::ptr_eq(&unrelated, &subagents_for(&state, "unrelated")));
    }

    #[test]
    fn self_referencing_spawn_preview_still_refreshes_after_text() {
        let mut state = AppState::new();
        let mut self_spawn = spawn("self", true, Some(SubagentStatus::Done));
        if let MessagePart::Tool { subagent_ref, .. } = &mut self_spawn {
            *subagent_ref = Some("self-doc".into());
        }
        state.set_subagent_snapshot(
            "self-doc".into(),
            vec![entry(
                "m",
                MessageRole::Assistant,
                vec![
                    self_spawn,
                    MessagePart::Text {
                        id: "text".into(),
                        text: String::new(),
                    },
                ],
            )],
        );
        state
            .apply_sub_transcript_frame("self-doc", text_append("m", "text", "result", 6, 1))
            .unwrap();
        assert_eq!(
            subagents_for(&state, "self-doc")[0].summary.as_deref(),
            Some("result")
        );
    }

    #[test]
    fn only_referenced_child_updates_invalidate_parent_presentation() {
        let mut state = AppState::new();
        state.selected_chat = Some("c".into());
        state.apply_transcript(vec![entry(
            "spawn",
            MessageRole::Assistant,
            vec![spawn("a", true, Some(SubagentStatus::Done))],
        )]);
        let before = subagents_for(&state, "c");
        state.set_subagent_snapshot("unrelated".into(), vec![]);
        assert!(Arc::ptr_eq(&before, &subagents_for(&state, "c")));
        let mut child = entry(
            "child",
            MessageRole::Assistant,
            vec![MessagePart::Text {
                id: "result".into(),
                text: "Child finished.".into(),
            }],
        );
        child.created_at += 5_000;
        state.set_subagent_snapshot("doc-1".into(), vec![child]);
        let after = subagents_for(&state, "c");
        assert!(!Arc::ptr_eq(&before, &after));
        assert_eq!(after[0].summary.as_deref(), Some("Child finished."));
        assert!(after[0].finished.is_some());
        state.unwatch_subagent_doc("doc-1");
        assert!(subagents_for(&state, "c")[0].summary.is_none());
    }

    #[test]
    fn finish_is_recorded_by_updates_even_without_any_render_or_selector() {
        let mut state = AppState::new();
        state.selected_chat = Some("c".into());
        state.apply_transcript(vec![entry(
            "spawn",
            MessageRole::Assistant,
            vec![spawn("a", false, Some(SubagentStatus::Running))],
        )]);
        state.apply_transcript(vec![entry(
            "spawn",
            MessageRole::Assistant,
            vec![spawn("a", true, Some(SubagentStatus::Done))],
        )]);
        let finish = state.subagent_finished_obs["c/a"];
        for _ in 0..10 {
            assert_eq!(subagents_for(&state, "c")[0].finished, millis(finish));
        }
        assert_eq!(state.subagent_finished_obs.len(), 1);
    }

    #[test]
    fn one_line_caps_unicode_and_preserves_meaningful_line_rules() {
        assert_eq!(one_line("\n \n hello \n ignored").as_deref(), Some("hello"));
        assert_eq!(one_line(&"é".repeat(120)).unwrap().chars().count(), 120);
        assert_eq!(
            one_line(&"é".repeat(1_000_000)).unwrap().chars().count(),
            121
        );
    }

    // ---- delegated tasks (app-owned child chats) ----

    use crate::delegation::{
        DelegatedStatus as DS, DelegatedTask, DelegatedWorkState as DW, DelegationSnapshot,
        ThreadLink, ThreadLinkKind,
    };

    fn delegated(id: &str, parent: &str, status: DS, work: DW) -> DelegatedTask {
        let started = Utc::now() - chrono::TimeDelta::seconds(30);
        DelegatedTask {
            task_id: format!("task-{id}"),
            parent_chat_id: parent.into(),
            child_chat_id: format!("child-{id}"),
            title: format!("Agent {id}"),
            harness: Some(zeron_proto::HarnessId::Codex),
            model: Some("openai/gpt-5".into()),
            effort: Some("High".into()),
            status,
            work_state: work,
            started_at: Some(started),
            completed_at: status
                .settled()
                .then(|| started + chrono::TimeDelta::seconds(20)),
            result: Some("All green".into()),
            progress: None,
            cancellable: !status.settled(),
        }
    }

    fn snapshot(tasks: Vec<DelegatedTask>) -> DelegationSnapshot {
        DelegationSnapshot {
            tasks,
            links: vec![],
        }
    }

    #[test]
    fn delegated_tasks_join_native_subagents_in_one_presentation() {
        let mut state = AppState::new();
        state.selected_chat = Some("c".into());
        state.apply_transcript(vec![
            entry("u", MessageRole::User, vec![]),
            entry(
                "m",
                MessageRole::Assistant,
                vec![spawn("native", false, Some(SubagentStatus::Running))],
            ),
        ]);
        assert!(state.apply_delegation_snapshot(snapshot(vec![
            delegated("run", "c", DS::Running, DW::Working),
            delegated("wait", "c", DS::Completed, DW::WaitingForChildren),
            delegated("done", "c", DS::Completed, DW::ResultAvailable),
            delegated("stopped", "c", DS::Cancelled, DW::ResultAvailable),
        ])));
        let out = subagents_for(&state, "c");
        let ids: Vec<_> = out.iter().map(|s| s.id.as_str()).collect();
        // Active first (oldest start first), settled after.
        assert_eq!(out.len(), 5);
        assert!(ids[..3].contains(&"native") && ids[..3].contains(&"task-run"));
        let by_id = |id: &str| out.iter().find(|s| s.id == id).unwrap();
        assert!(
            by_id("native").delegated.is_none(),
            "native stays observational"
        );
        let run = by_id("task-run");
        assert_eq!(run.status, SubagentPhase::Running);
        assert_eq!(run.model.as_deref(), Some("gpt-5"));
        let link = run.delegated.as_deref().unwrap();
        assert_eq!(link.child_chat_id, "child-run");
        assert_eq!(link.effort.as_deref(), Some("High"));
        assert!(run.stoppable().is_some());
        assert_eq!(by_id("task-wait").status, SubagentPhase::Waiting);
        assert!(by_id("task-wait").status.active());
        assert_eq!(by_id("task-done").status, SubagentPhase::Done);
        assert_eq!(by_id("task-stopped").status, SubagentPhase::Stopped);
        assert!(by_id("task-done").stoppable().is_none());
        // Pulsing set: running work only; waiting work stays in the tray.
        assert_eq!(out.running().len(), 3);
        assert!(strip_visible(&out).iter().any(|s| s.id == "task-wait"));
        let groups = agent_groups(&out);
        assert_eq!((groups.active.len(), groups.previous.len()), (3, 2));
        assert_eq!(groups.running, 2);
    }

    #[test]
    fn model_and_effort_fall_back_to_the_child_chats_own_config() {
        let mut state = AppState::new();
        let mut child = zeron_proto::Chat {
            id: "child-a".into(),
            device_id: "dev".into(),
            title: Some("Scout".into()),
            archived: false,
            cwd: None,
            branch: None,
            checkout_id: None,
            source_context: None,
            config: Some(zeron_proto::ChatConfig {
                instance_id: None,
                harness: zeron_proto::HarnessId::ClaudeCode,
                model: Some("anthropic/claude-opus".into()),
                reasoning: Some(zeron_proto::ReasoningLevel::High),
                model_options: Default::default(),
                sandbox: zeron_proto::SandboxLevel::WorkspaceWrite,
                runtime_mode: Default::default(),
                interaction_mode: Default::default(),
            }),
            last_message_preview: None,
            last_message_at: None,
            created_at: Utc::now(),
            harness_session_id: None,
            harness_session_cwd: None,
            harness_session_instance_id: None,
            space_id: None,
            last_seen_at: None,
            room_gen: None,
        };
        child.title = Some("Scout".into());
        state.apply_chats(vec![child]);
        let mut task = delegated("a", "c", DS::Running, DW::Working);
        task.model = None;
        task.effort = None;
        task.harness = None;
        state.apply_delegation_snapshot(snapshot(vec![task]));
        let out = subagents_for(&state, "c");
        let link = out[0].delegated.as_deref().unwrap();
        assert_eq!(out[0].model.as_deref(), Some("claude-opus"));
        assert_eq!(link.effort.as_deref(), Some("High"));
        assert_eq!(link.harness, Some(zeron_proto::HarnessId::ClaudeCode));
    }

    #[test]
    fn banner_word_names_stopped_and_failed_tasks_not_result_available() {
        let word = |status, work| banner_state_word(&delegated("a", "p", status, work), false);
        assert_eq!(word(DS::Interrupted, DW::ResultAvailable), "Stopped");
        assert_eq!(word(DS::Cancelled, DW::ResultAvailable), "Stopped");
        assert_eq!(word(DS::Failed, DW::ResultAvailable), "Failed");
        assert_eq!(word(DS::Completed, DW::ResultAvailable), "Result available");
        assert_eq!(word(DS::Running, DW::Working), "Working");
        assert_eq!(
            banner_state_word(&delegated("a", "p", DS::Running, DW::Working), true),
            "Stopping\u{2026}"
        );
    }

    #[test]
    fn delegation_sync_is_nudged_by_updates_and_only_heartbeats_while_work_is_live() {
        let mut state = AppState::new();
        let mut nudges = state.take_delegation_nudges().expect("first take");
        assert!(state.take_delegation_nudges().is_none(), "single consumer");
        let nudged = |rx: &mut futures::channel::mpsc::UnboundedReceiver<()>| {
            let mut any = false;
            while rx.try_next().is_ok_and(|n| n.is_some()) {
                any = true;
            }
            any
        };
        assert!(!nudged(&mut nudges));
        // A chat-row frame (a new child chat, a parent's publication) nudges.
        state.apply_chats(vec![]);
        assert!(nudged(&mut nudges));
        assert!(!state.delegation_busy() && state.delegation_always_parents().is_empty());

        state.apply_delegation_snapshot(snapshot(vec![
            delegated("a", "p1", DS::Running, DW::Working),
            delegated("b", "p2", DS::Completed, DW::ResultAvailable),
        ]));
        assert!(state.delegation_busy());
        // Only the parent with live work is re-read on every pass.
        assert_eq!(state.delegation_always_parents(), ["p1"]);
        // A delegated child's or parent's transcript moving nudges; a stranger does not.
        state.refresh_subagents("child-a");
        assert!(nudged(&mut nudges));
        state.refresh_subagents("p2");
        assert!(nudged(&mut nudges));
        state.refresh_subagents("unrelated");
        assert!(!nudged(&mut nudges));
        state.refresh_subagents_after_text("child-a");
        assert!(nudged(&mut nudges));
        state.refresh_subagents_after_text("p2");
        assert!(nudged(&mut nudges));
        state.refresh_subagents_after_text("unrelated");
        assert!(!nudged(&mut nudges));
        // Settled: no heartbeat, nothing to force-read.
        state.apply_delegation_snapshot(snapshot(vec![delegated(
            "a",
            "p1",
            DS::Completed,
            DW::ResultAvailable,
        )]));
        assert!(!state.delegation_busy() && state.delegation_always_parents().is_empty());
    }

    #[test]
    fn applying_a_snapshot_rebuilds_only_changed_parents_and_never_scans_histories() {
        let mut state = AppState::new();
        state.apply_delegation_snapshot(snapshot(vec![
            delegated("a", "p1", DS::Running, DW::Working),
            delegated("b", "p2", DS::Running, DW::Working),
        ]));
        let p2_before = subagents_for(&state, "p2");
        let before = crate::perf_trace::snapshot();
        // Only p1's task changed: p2's presentation is the same allocation.
        let mut changed = delegated("a", "p1", DS::Completed, DW::ResultAvailable);
        changed.started_at = state.delegation.tasks_for("p1")[0].started_at;
        changed.completed_at = changed
            .started_at
            .map(|s| s + chrono::TimeDelta::seconds(5));
        let unchanged_b = state.delegation.tasks_for("p2")[0].clone();
        assert!(state.apply_delegation_snapshot(snapshot(vec![changed, unchanged_b.clone()])));
        assert!(Arc::ptr_eq(&p2_before, &subagents_for(&state, "p2")));
        // Re-applying an identical snapshot is a no-op: nothing to notify.
        let same = snapshot(vec![
            state.delegation.tasks_for("p1")[0].clone(),
            unchanged_b,
        ]);
        assert!(!state.apply_delegation_snapshot(same));
        // Selecting is a cache hit, never a scan.
        for _ in 0..10 {
            let _ = subagents_for(&state, "p1");
        }
        let after = crate::perf_trace::snapshot();
        assert_eq!(
            after.subagent_scans - before.subagent_scans,
            1,
            "only p1 rebuilt"
        );
        assert!(after.subagent_cache_hits - before.subagent_cache_hits >= 10);
    }

    #[test]
    fn stop_acceptance_reads_stopping_until_the_snapshot_settles_the_task() {
        let mut state = AppState::new();
        state.apply_delegation_snapshot(snapshot(vec![delegated(
            "a",
            "p",
            DS::Running,
            DW::Working,
        )]));
        assert!(subagents_for(&state, "p")[0].stoppable().is_some());
        state.mark_delegated_cancel_requested("task-a");
        let row = subagents_for(&state, "p")[0].clone();
        assert!(row.delegated.as_deref().unwrap().stopping);
        assert!(
            row.stoppable().is_none(),
            "no second Stop while one is in flight"
        );
        assert!(row.status.active(), "acceptance is not terminal");
        // The engine settles it; the request is dropped with it.
        state.apply_delegation_snapshot(snapshot(vec![delegated(
            "a",
            "p",
            DS::Cancelled,
            DW::ResultAvailable,
        )]));
        let row = subagents_for(&state, "p")[0].clone();
        assert_eq!(row.status, SubagentPhase::Stopped);
        assert!(!row.delegated.as_deref().unwrap().stopping);
        // Unknown tasks cannot take a request.
        state.mark_delegated_cancel_requested("task-missing");
    }

    #[test]
    fn delegated_children_are_never_ordinary_sidebar_rows_but_stay_in_state() {
        let mut state = AppState::new();
        let mk = |id: &str| zeron_proto::Chat {
            id: id.into(),
            device_id: "dev".into(),
            title: None,
            archived: false,
            cwd: None,
            branch: None,
            checkout_id: None,
            source_context: None,
            config: None,
            last_message_preview: None,
            last_message_at: None,
            created_at: Utc::now(),
            harness_session_id: None,
            harness_session_cwd: None,
            harness_session_instance_id: None,
            space_id: None,
            last_seen_at: None,
            room_gen: None,
        };
        state.apply_chats(vec![mk("p"), mk("child-a"), mk("fork-1")]);
        let visible = |s: &AppState| {
            let mut ids = s.visible_chats().map(|c| c.id.clone()).collect::<Vec<_>>();
            ids.sort();
            ids
        };
        assert_eq!(visible(&state), ["child-a", "fork-1", "p"]);
        state.apply_delegation_snapshot(DelegationSnapshot {
            tasks: vec![delegated("a", "p", DS::Running, DW::Working)],
            // A fork is an ordinary top-level thread with lineage.
            links: vec![ThreadLink {
                chat_id: "fork-1".into(),
                parent_chat_id: "p".into(),
                kind: ThreadLinkKind::Fork,
            }],
        });
        assert_eq!(visible(&state), ["fork-1", "p"]);
        // The child remains a real chat the panes and engine can address.
        assert!(state.chats.iter().any(|c| c.id == "child-a"));
        assert!(state.delegation.is_delegated_child("child-a"));
        // Dropping the task returns it to the ordinary list.
        state.apply_delegation_snapshot(DelegationSnapshot::default());
        assert_eq!(visible(&state), ["child-a", "fork-1", "p"]);
    }

    #[test]
    fn presentations_for_delegating_parents_survive_pruning_without_a_transcript() {
        let mut state = AppState::new();
        state.apply_delegation_snapshot(snapshot(vec![delegated(
            "a",
            "unopened",
            DS::Running,
            DW::Working,
        )]));
        state.prune_subagent_presentations();
        assert_eq!(subagents_for(&state, "unopened").len(), 1);
        state.apply_delegation_snapshot(DelegationSnapshot::default());
        assert_eq!(subagents_for(&state, "unopened").len(), 0);
    }

    #[test]
    fn row_clock_runs_while_active_and_freezes_at_finish() {
        let mut state = AppState::new();
        state.apply_delegation_snapshot(snapshot(vec![
            delegated("live", "p", DS::Running, DW::Working),
            delegated("done", "p", DS::Completed, DW::ResultAvailable),
        ]));
        let out = subagents_for(&state, "p");
        let live = out.iter().find(|s| s.id == "task-live").unwrap();
        let done = out.iter().find(|s| s.id == "task-done").unwrap();
        assert!(live.clock().active && live.clock().next_change(Utc::now()).is_some());
        assert!(!done.clock().active && done.clock().next_change(Utc::now()).is_none());
        assert_eq!(
            done.elapsed(Utc::now() + chrono::TimeDelta::hours(5))
                .as_deref(),
            Some("20s")
        );
    }

    #[test]
    fn tray_fitting_budgets_the_mark_and_stop_slot_of_delegated_pills() {
        let mut state = AppState::new();
        state.apply_delegation_snapshot(snapshot(vec![delegated(
            "a",
            "p",
            DS::Running,
            DW::Working,
        )]));
        let pres = subagents_for(&state, "p");
        let row = &pres[0];
        assert_eq!(pill_extras(row), 12.0 + 6.0 + 14.0 + 6.0);
        let mut native = row.clone();
        native.delegated = None;
        assert_eq!(pill_extras(&native), 0.0);
        let mut settled = row.clone();
        settled.status = SubagentPhase::Done;
        assert_eq!(
            pill_extras(&settled),
            12.0 + 6.0,
            "no Stop slot once settled"
        );
    }

    #[test]
    fn panel_ui_pages_each_group_independently() {
        let mut ui = PanelUi::default();
        assert_eq!(ui.shown(LineageGroup::Active), 6);
        ui.show_more(LineageGroup::Active);
        assert_eq!(ui.shown(LineageGroup::Active), 18);
        assert_eq!(ui.shown(LineageGroup::Previous), 6);
        assert!(
            !ui.previous_open,
            "Previous agents starts collapsed, like T3"
        );
    }
}
