//! The Details surface (`RightSurface::Details`, DESIGN-W3 §1): one quiet
//! panel per chat for the thread's workspace, its pull requests and Git
//! actions, its automations and its file checkpoints. T3's thread details
//! panel, drawn with Noches' rules: color only for state, metadata in mono.
//!
//! The panel renders a [`DetailsModel`]. Engine read models feed it through
//! [`DetailsModel::for_chat`]; every section hides itself when empty, except
//! Workspace.

use std::rc::Rc;

use chrono::{DateTime, Utc};
use gpui::{
    AnyElement, Context, FontWeight, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, div,
    prelude::*, px,
};
use serde::Deserialize;

use crate::icons::{self, icon};
use crate::shell::Shell;
use crate::state::AppState;
use crate::status_palette::SessionState;
use crate::theme::Theme;

const ROW_HEIGHT: f32 = 36.0;
const ROW_RADIUS: f32 = 8.0;
const ROW_PAD_X: f32 = 10.0;
const ROW_GAP: f32 = 10.0;
const ICON_ACTION: f32 = 24.0;
const SECTION_HEADER_MIN: f32 = 32.0;
const CHECKPOINTS_COLLAPSED: usize = 5;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DetailsModel {
    pub workspace: WorkspaceInfo,
    pub pull_requests: Vec<PullRequestLink>,
    pub automations: Vec<Automation>,
    pub checkpoints: Vec<Checkpoint>,
    pub lineage: Vec<ConversationRelation>,
    pub transfers: Vec<ContextTransferRow>,
    pub transfers_supported: bool,
    pub transfer_busy: bool,
    pub fork_run_id: Option<String>,
    pub merge_run_id: Option<String>,
    pub merge_target: Option<String>,
    /// The parent already holds a pending merge-back from another fork;
    /// the host would refuse its next start if a second one were added.
    pub merge_blocked: bool,
    pub session_control_supported: bool,
    pub session_busy: bool,
    pub session_retry: bool,
    pub attached_provider_sessions: Vec<zeron_proto::transfer::ProviderSessionRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConversationRelation {
    pub chat_id: String,
    pub title: String,
    pub label: String,
    pub available: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ContextTransferRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub detail: String,
    pub source_chat_id: Option<String>,
    pub providers: Option<String>,
    pub failed: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WorkspaceInfo {
    /// `None` = the project's root checkout.
    pub worktree_branch: Option<String>,
    pub branch: Option<String>,
    /// Only for a session hosted on another device.
    pub remote_device: Option<String>,
    pub setup: Option<SetupState>,
    pub auto_pull: Option<AutoPullInfo>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AutoPullInfo {
    pub space_id: String,
    pub branch: String,
    pub enabled: bool,
    pub summary: String,
    pub error: Option<String>,
    pub busy: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SetupState {
    Running {
        command: String,
        started_at: DateTime<Utc>,
    },
    Failed {
        command: String,
        exit_code: Option<i32>,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestLink {
    pub summary: zeron_proto::ChangeRequestSummary,
    #[serde(default)]
    pub checks_passed: u32,
    #[serde(default)]
    pub checks_failed: u32,
    #[serde(default)]
    pub checks_pending: u32,
    #[serde(default)]
    pub watching: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunStatus {
    #[default]
    Never,
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Automation {
    pub id: String,
    pub title: String,
    /// "Every 1h", "Weekdays 09:00".
    pub cadence: String,
    pub next_run_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    #[serde(default)]
    pub last_run: RunStatus,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub id: String,
    pub at: DateTime<Utc>,
    pub summary: String,
    pub additions: u32,
    pub deletions: u32,
    #[serde(default)]
    pub forkable: bool,
}

/// Env var naming a JSON file of `{chatTitle: DetailsModel}` for headed QA.
pub const FIXTURE_ENV: &str = "NOCHES_DETAILS_FIXTURE";

impl DetailsModel {
    pub(crate) fn can_disconnect_session(&self) -> bool {
        self.session_control_supported
            && !self.session_busy
            && !self.transfer_busy
            && (self.session_retry || !self.attached_provider_sessions.is_empty())
    }

    /// Whether the action may be attempted. `fork_run_id` is a cached hint
    /// that can lag a turn that just finished, so it only dims the row
    /// (`fork_ready`); the submit path re-reads the state and refuses with a
    /// notice when there is truly no finished turn.
    fn can_fork(&self) -> bool {
        self.transfers_supported && !self.transfer_busy
    }

    fn fork_ready(&self) -> bool {
        self.can_fork() && self.fork_run_id.is_some()
    }

    fn can_merge_back(&self) -> bool {
        self.transfers_supported
            && !self.transfer_busy
            && !self.merge_blocked
            && self.merge_target.as_ref().is_some_and(|target| {
                self.lineage
                    .iter()
                    .any(|r| &r.chat_id == target && r.available)
            })
    }

    pub fn for_chat(state: &AppState, chat_id: &str) -> Self {
        let chat = state.chats.iter().find(|chat| chat.id == chat_id);
        let fixture = chat
            .and_then(|chat| chat.title.as_deref())
            .and_then(fixture_for);
        if let Some(model) = fixture {
            return model;
        }
        let mut model = Self::default();
        if let Some(chat) = chat {
            model.transfers_supported =
                state.chat_host_supports(chat_id, zeron_proto::capabilities::THREAD_TRANSFERS_V1);
            model.transfer_busy = state.details.transfer_busy(chat_id);
            model.session_control_supported = state.chat_host_supports(
                chat_id,
                zeron_proto::capabilities::PROVIDER_SESSION_CONTROL_V1,
            );
            let key = (chat.device_id.clone(), chat.id.clone());
            model.session_busy = state.details.session_actions.contains(&key);
            model.session_retry = state.details.session_retries.contains_key(&key);
            model.lineage = crate::delegation::related_rows(&state.delegation, chat_id, |id| {
                state
                    .chats
                    .iter()
                    .find(|c| c.id == id)
                    .and_then(|c| c.title.clone())
            })
            .into_iter()
            .map(|r| ConversationRelation {
                available: state.chats.iter().any(|c| c.id == r.chat_id),
                chat_id: r.chat_id,
                title: r.title.to_string(),
                label: r.relation.label().into(),
            })
            .collect();
            if let Some(space) = chat
                .space_id
                .as_ref()
                .and_then(|id| state.spaces.iter().find(|s| &s.id == id))
            {
                let root = crate::git_store::is_root_checkout(chat, space);
                if !root {
                    model.workspace.worktree_branch =
                        chat.branch.clone().or_else(|| Some("Worktree".into()));
                } else if let Some(row) = state.git_actions.pulls.get(&space.id) {
                    let branch = if !row.state.policy.default_branch.is_empty() {
                        row.state.policy.default_branch.clone()
                    } else {
                        row.checkout
                            .as_ref()
                            .and_then(|c| c.upstream.as_deref())
                            .and_then(|u| u.split_once('/').map(|(_, b)| b.to_string()))
                            .unwrap_or_else(|| "default branch".into())
                    };
                    model.workspace.auto_pull = Some(AutoPullInfo {
                        space_id: space.id.clone(),
                        branch,
                        enabled: row.state.policy.enabled,
                        summary: crate::git_store::pull_summary(
                            &row.state,
                            Utc::now().timestamp_millis(),
                        ),
                        error: row.error.clone().or_else(|| row.state.last_error.clone()),
                        busy: row.busy,
                    });
                }
            }
            model.workspace.branch =
                crate::change_requests::conversation_branch(chat, &state.spaces)
                    .map(str::to_string);
            if state.local_device_id.as_deref() != Some(chat.device_id.as_str()) {
                model.workspace.remote_device =
                    state.device_name(&chat.device_id).map(str::to_string);
            }
            model.automations = state
                .automations
                .for_thread(&chat.id)
                .map(|row| Automation {
                    id: row.id.clone(),
                    title: row.title.clone(),
                    cadence: row.cadence.clone(),
                    next_run_at: row.next_run_at,
                    enabled: row.enabled,
                    last_run: row.last_run,
                })
                .collect();
            model
                .automations
                .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            if let Some(summary) = state.change_request_for_chat(chat) {
                model.pull_requests.push(PullRequestLink {
                    summary: summary.clone(),
                    checks_passed: 0,
                    checks_failed: 0,
                    checks_pending: 0,
                    watching: false,
                });
            }
        }
        // Engine-owned wave-3 data overlays legacy checkout fallbacks.
        if let Some(chat) = chat
            && let Some(row) = state.details.get(&chat.device_id, chat_id)
        {
            crate::details_data::apply_snapshot(&mut model, row);
        }
        // The parent's own transfer state names the merge-backs waiting on it.
        if let Some(parent) = model
            .merge_target
            .as_deref()
            .and_then(|id| state.chats.iter().find(|c| c.id == id))
        {
            model.merge_blocked = state
                .details
                .get(&parent.device_id, &parent.id)
                .and_then(|row| row.transfer.as_ref())
                .is_some_and(|transfer| {
                    crate::details_data::pending_merge_from_other_fork(transfer, chat_id)
                });
        }
        if let Some(parent) = &model.merge_target
            && !model.lineage.iter().any(|r| &r.chat_id == parent)
        {
            let chat = state.chats.iter().find(|c| &c.id == parent);
            model.lineage.insert(
                0,
                ConversationRelation {
                    chat_id: parent.clone(),
                    title: chat
                        .and_then(|c| c.title.clone())
                        .unwrap_or_else(|| "Parent thread".into()),
                    label: "Forked from".into(),
                    available: chat.is_some(),
                },
            );
        }
        model
    }
}

fn fixture_for(title: &str) -> Option<DetailsModel> {
    let path = std::env::var_os(FIXTURE_ENV).filter(|path| !path.is_empty())?;
    let text = std::fs::read_to_string(path).ok()?;
    let mut all: std::collections::HashMap<String, DetailsModel> =
        serde_json::from_str(&text).ok()?;
    all.remove(title)
}

type ShellAction<T> = Rc<dyn Fn(&mut Shell, T, &mut Context<Shell>)>;

/// What the panel's controls do. The shell wires each to an engine call.
#[derive(Clone)]
pub struct DetailsActions {
    pub open_thread: ShellAction<String>,
    pub fork_thread: ShellAction<Option<String>>,
    pub merge_back: ShellAction<()>,
    pub disconnect_session: ShellAction<()>,
    pub toggle_lineage: ShellAction<()>,
    pub toggle_transfers: ShellAction<()>,
    pub open_url: ShellAction<String>,
    pub toggle_watch: ShellAction<(String, bool)>,
    pub link_pull_request: ShellAction<()>,
    pub commit: ShellAction<()>,
    pub run_automation: ShellAction<String>,
    pub toggle_automation: ShellAction<(String, bool)>,
    pub manage_automations: ShellAction<()>,
    pub restore_checkpoint: ShellAction<String>,
    pub retry_setup: ShellAction<()>,
    pub continue_setup: ShellAction<()>,
    pub move_to_worktree: ShellAction<()>,
    pub toggle_pull: ShellAction<(String, bool)>,
    pub retry_pull: ShellAction<String>,
}

/// Per-chat panel UI state that survives re-renders.
#[derive(Clone, Debug, Default)]
pub struct DetailsUi {
    pub checkpoints_expanded: bool,
    pub lineage_expanded: bool,
    pub transfers_expanded: bool,
}

pub fn details_panel_body(
    chat_id: &str,
    model: &DetailsModel,
    ui: &DetailsUi,
    now: DateTime<Utc>,
    theme: &Theme,
    actions: &DetailsActions,
    toggle_checkpoints: ShellAction<()>,
    cx: &Context<Shell>,
) -> AnyElement {
    let mut sections: Vec<AnyElement> =
        vec![workspace_section(chat_id, model, now, theme, actions, cx)];
    if model.transfers_supported || model.session_control_supported || !model.lineage.is_empty() {
        sections.push(lineage_section(chat_id, model, ui, theme, actions, cx));
    }
    if !model.transfers.is_empty() {
        sections.push(transfers_section(chat_id, model, ui, theme, actions, cx));
    }
    sections.push(version_control_section(chat_id, model, theme, actions, cx));
    if !model.automations.is_empty() {
        sections.push(automations_section(chat_id, model, now, theme, actions, cx));
    }
    if !model.checkpoints.is_empty() {
        sections.push(checkpoints_section(
            chat_id,
            model,
            ui,
            now,
            theme,
            actions,
            toggle_checkpoints,
            cx,
        ));
    }
    div()
        .id(SharedString::from(format!("details-{chat_id}")))
        .size_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .children(sections.into_iter().enumerate().map(|(index, section)| {
            div()
                .px(px(8.0))
                .pt(px(8.0))
                .pb(px(10.0))
                .when(index > 0, |el| el.border_t_1().border_color(theme.border))
                .child(section)
        }))
        .into_any_element()
}

fn section(title: &'static str, actions: Vec<AnyElement>, theme: &Theme) -> gpui::Div {
    div().flex().flex_col().child(
        div()
            .mb(px(4.0))
            .min_h(px(SECTION_HEADER_MIN))
            .px(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(crate::typography::ui_rems(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_muted)
                    .child(SharedString::from(title)),
            )
            .child(div().flex().items_center().gap(px(4.0)).children(actions)),
    )
}

/// A 36px panel row: 16px muted glyph, 13px MEDIUM label at 80%, optional
/// trailing content. Hover is the panel's own quiet wash.
fn row(id: SharedString, glyph: Option<&'static str>, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(ROW_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .gap(px(ROW_GAP))
        .px(px(ROW_PAD_X))
        .rounded(px(ROW_RADIUS))
        .text_size(crate::typography::ui_rems(13.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.text.opacity(0.8))
        .when_some(glyph, |el, glyph| {
            el.child(
                icon(glyph)
                    .size(px(16.0))
                    .flex_none()
                    .text_color(theme.text_muted),
            )
        })
}

fn hover_row(row: gpui::Stateful<gpui::Div>, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    let hover = theme.wash(if theme.appearance == crate::theme::Appearance::Dark {
        0.075
    } else {
        0.055
    });
    row.cursor_pointer().hover(move |el| el.bg(hover))
}

fn label(text: impl Into<SharedString>) -> gpui::Div {
    div().flex_1().min_w_0().truncate().child(text.into())
}

fn mono(text: impl Into<SharedString>, color: Hsla, theme: &Theme) -> gpui::Div {
    div()
        .flex_none()
        .font_family(theme.font_mono.clone())
        .font_weight(FontWeight::NORMAL)
        .text_size(crate::typography::ui_rems(11.0))
        .text_color(color)
        .child(text.into())
}

/// A 24px icon action with a required tooltip. Stops propagation so it never
/// fires its row.
fn icon_action(
    id: SharedString,
    glyph: &'static str,
    tooltip: &'static str,
    theme: &Theme,
) -> gpui::Stateful<gpui::Div> {
    let hover = theme.wash(0.08);
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(tooltip)
        .size(px(ICON_ACTION))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(move |el| el.bg(hover))
        .tooltip(crate::tooltip::text(tooltip))
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(icon(glyph).size(px(14.0)).text_color(theme.text_muted))
}

fn lineage_section(
    chat_id: &str,
    model: &DetailsModel,
    ui: &DetailsUi,
    theme: &Theme,
    actions: &DetailsActions,
    cx: &Context<Shell>,
) -> AnyElement {
    let fork = actions.fork_thread.clone();
    let can_fork = model.can_fork();
    let fork_ready = model.fork_ready();
    let mut body = section("Lineage", vec![], theme);
    if model.transfers_supported {
        body = body.child(
            row(
                format!("details-{chat_id}-fork").into(),
                Some(icons::GIT_BRANCH),
                theme,
            )
            .role(gpui::Role::Button)
            .aria_label("Fork conversation")
            .when(can_fork, |el| hover_row(el, theme))
            .when(!fork_ready, |el| el.opacity(0.45))
            .tooltip(crate::tooltip::text(if model.transfer_busy {
                "Creating context transfer…"
            } else if fork_ready {
                "Explore from the latest finished turn without changing this conversation"
            } else if can_fork {
                "No finished turn seen yet. Checks again when you fork."
            } else {
                "Forking is unavailable on this device"
            }))
            .child(label(if model.transfer_busy {
                "Preparing conversation…"
            } else {
                "Fork conversation"
            }))
            .on_click(cx.listener(move |this, _, _, cx| {
                if can_fork {
                    fork(this, None, cx);
                }
            })),
        );
    }
    if model.session_control_supported
        && (!model.attached_provider_sessions.is_empty()
            || model.session_retry
            || model.session_busy)
    {
        let disconnect = actions.disconnect_session.clone();
        let enabled = model.can_disconnect_session();
        body = body.child(
            row(format!("details-{chat_id}-disconnect").into(), Some(icons::STOP), theme)
                .role(gpui::Role::Button)
                .aria_label("Disconnect agent session")
                .when(enabled, |el| hover_row(el, theme))
                .when(!enabled, |el| el.opacity(0.45))
                .tooltip(crate::tooltip::text(
                    "Stops the live agent session. Keeps this conversation and native history for the next message."))
                .child(label(if model.session_busy {
                    "Disconnecting agent session…"
                } else if model.session_retry {
                    "Retry disconnect"
                } else {
                    "Disconnect agent session"
                }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if enabled { disconnect(this, (), cx); }
                })),
        );
    }
    let shown = if ui.lineage_expanded {
        model.lineage.len()
    } else {
        model.lineage.len().min(6)
    };
    let mut relationships = div()
        .id(SharedString::from(format!(
            "details-{chat_id}-lineage-list"
        )))
        .max_h(px(216.0))
        .overflow_y_scroll()
        .flex()
        .flex_col();
    for relation in model.lineage.iter().take(shown) {
        let open = actions.open_thread.clone();
        let target = relation.chat_id.clone();
        let available = relation.available;
        let is_parent = model.merge_target.as_deref() == Some(&relation.chat_id);
        let can_merge = is_parent && model.can_merge_back();
        let merge = actions.merge_back.clone();
        relationships = relationships.child(
            hover_row(
                row(
                    format!("details-{chat_id}-relation-{}", relation.chat_id).into(),
                    Some(if is_parent {
                        icons::ARROW_LEFT
                    } else {
                        icons::GIT_BRANCH
                    }),
                    theme,
                ),
                theme,
            )
            .role(gpui::Role::Button)
            .aria_label(format!("Open {} {}", relation.label, relation.title))
            .when(!available, |el| el.opacity(0.45))
            .tooltip(crate::tooltip::text(if available {
                "Open related conversation"
            } else {
                "This conversation is unavailable"
            }))
            .child(label(relation.title.clone()))
            .child(mono(relation.label.clone(), theme.text_faint, theme))
            .when(is_parent && model.transfers_supported, |el| {
                el.child(
                    icon_action(
                        format!("details-{chat_id}-merge-back").into(),
                        icons::RETURN,
                        if model.merge_blocked {
                            "The parent already has merged context from another fork waiting for its next message"
                        } else {
                            "Merge conversation context back (does not merge files)"
                        },
                        theme,
                    )
                    .when(!can_merge || model.merge_run_id.is_none(), |el| {
                        el.opacity(0.35)
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        if can_merge {
                            merge(this, (), cx);
                        }
                    })),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                if available {
                    open(this, target.clone(), cx);
                }
            })),
        );
    }
    body = body.child(relationships);
    if model.lineage.len() > 6 {
        let toggle = actions.toggle_lineage.clone();
        body = body.child(
            hover_row(
                row(
                    format!("details-{chat_id}-lineage-more").into(),
                    None,
                    theme,
                ),
                theme,
            )
            .child(label(if ui.lineage_expanded {
                "Show fewer".into()
            } else {
                format!("Show all · {}", model.lineage.len())
            }))
            .on_click(cx.listener(move |this, _, _, cx| toggle(this, (), cx))),
        );
    }
    body.into_any_element()
}

/// Fork and merge-back read as different operations; neutral colour either way.
fn transfer_icon(transfer: &ContextTransferRow) -> &'static str {
    match transfer.title.as_str() {
        "Conversation fork" => icons::GIT_BRANCH,
        "Merge-back context" => icons::RETURN,
        _ => icons::ARROW_TURN_UP_RIGHT,
    }
}

fn transfers_section(
    chat_id: &str,
    model: &DetailsModel,
    ui: &DetailsUi,
    theme: &Theme,
    actions: &DetailsActions,
    cx: &Context<Shell>,
) -> AnyElement {
    let shown = if ui.transfers_expanded {
        model.transfers.len()
    } else {
        model.transfers.len().min(5)
    };
    let mut body = section("Context transfers", vec![], theme);
    let mut history = div()
        .id(SharedString::from(format!(
            "details-{chat_id}-transfer-list"
        )))
        .max_h(px(288.0))
        .overflow_y_scroll()
        .flex()
        .flex_col();
    for transfer in model.transfers.iter().take(shown) {
        let open = actions.open_thread.clone();
        let target = transfer.source_chat_id.clone();
        let tooltip = transfer.error.clone().unwrap_or_else(|| {
            if transfer.status == "Prepared" {
                "Context is prepared; agent acceptance has not been confirmed.".into()
            } else if transfer.status == "Pending" {
                "Context resolves when the target conversation starts its next message.".into()
            } else {
                transfer.detail.clone()
            }
        });
        history = history.child(
            div()
                .id(SharedString::from(format!(
                    "details-{chat_id}-transfer-{}",
                    transfer.id
                )))
                .w_full()
                .min_h(px(48.0))
                .px(px(ROW_PAD_X))
                .py(px(6.0))
                .rounded(px(ROW_RADIUS))
                .flex()
                .flex_col()
                .gap(px(3.0))
                .when(target.is_some(), |el| {
                    el.cursor_pointer().hover(|el| el.bg(theme.wash(0.075)))
                })
                .tooltip(crate::tooltip::text(tooltip))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            icon(if transfer.failed {
                                icons::DANGER_TRIANGLE
                            } else {
                                transfer_icon(transfer)
                            })
                            .size(px(14.0))
                            .text_color(if transfer.failed {
                                theme.danger
                            } else {
                                theme.text_muted
                            }),
                        )
                        .child(
                            label(transfer.title.clone())
                                .text_size(crate::typography::ui_rems(13.0))
                                .text_color(theme.text_muted),
                        )
                        .child(mono(
                            transfer.status.clone(),
                            if transfer.failed {
                                theme.danger
                            } else {
                                theme.text_faint
                            },
                            theme,
                        )),
                )
                .child(
                    div().pl(px(22.0)).child(
                        mono(transfer.detail.clone(), theme.text_faint, theme)
                            .min_w_0()
                            .truncate(),
                    ),
                )
                .when_some(transfer.providers.clone(), |el, providers| {
                    el.child(
                        div().pl(px(22.0)).child(
                            mono(providers, theme.text_faint, theme)
                                .min_w_0()
                                .truncate(),
                        ),
                    )
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(target) = &target {
                        open(this, target.clone(), cx);
                    }
                })),
        );
    }
    body = body.child(history);
    if model.transfers.len() > 5 {
        let toggle = actions.toggle_transfers.clone();
        body = body.child(
            hover_row(
                row(
                    format!("details-{chat_id}-transfer-more").into(),
                    None,
                    theme,
                ),
                theme,
            )
            .child(label(if ui.transfers_expanded {
                "Show fewer".into()
            } else {
                format!("Show all · {}", model.transfers.len())
            }))
            .on_click(cx.listener(move |this, _, _, cx| toggle(this, (), cx))),
        );
    }
    body.into_any_element()
}

fn workspace_section(
    chat_id: &str,
    model: &DetailsModel,
    now: DateTime<Utc>,
    theme: &Theme,
    actions: &DetailsActions,
    cx: &Context<Shell>,
) -> AnyElement {
    let workspace = &model.workspace;
    let mut rows: Vec<AnyElement> = Vec::new();
    let checkout = row(
        format!("details-{chat_id}-checkout").into(),
        Some(icons::FOLDER),
        theme,
    );
    let checkout = match &workspace.worktree_branch {
        Some(branch) => checkout.child(div().flex_none().child("Worktree")).child(
            mono(branch.clone(), theme.text_muted, theme)
                .min_w_0()
                .truncate(),
        ),
        None => checkout
            .child(label("Root checkout"))
            .when_some(workspace.branch.clone(), |el, branch| {
                el.child(mono(branch, theme.text_faint, theme).min_w_0().truncate())
            }),
    };
    rows.push(checkout.into_any_element());
    if let Some(device) = &workspace.remote_device {
        rows.push(
            row(
                format!("details-{chat_id}-device").into(),
                Some(icons::LAPTOP),
                theme,
            )
            .child(mono(device.clone(), theme.text_muted, theme))
            .into_any_element(),
        );
    }
    match &workspace.setup {
        Some(SetupState::Running {
            command,
            started_at,
        }) => {
            let sky = SessionState::Working
                .color(theme)
                .unwrap_or(theme.text_muted);
            rows.push(
                row(format!("details-{chat_id}-setup").into(), None, theme)
                    .child(
                        div()
                            .size(px(16.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(crate::loaders::mini_equalizer(
                                format!("details-{chat_id}-setup-eq"),
                                sky,
                                cx.entity_id(),
                                cx,
                            )),
                    )
                    .child(div().flex_none().child("Setting up"))
                    .child(
                        mono(command.clone(), theme.text_faint, theme)
                            .flex_1()
                            .min_w_0()
                            .truncate(),
                    )
                    .child(mono(
                        crate::shell::format_working_elapsed(
                            now.signed_duration_since(*started_at).num_seconds(),
                        ),
                        theme.text_muted,
                        theme,
                    ))
                    .into_any_element(),
            );
        }
        Some(SetupState::Failed { command, exit_code }) => {
            let retry = actions.retry_setup.clone();
            let proceed = actions.continue_setup.clone();
            rows.push(
                row(format!("details-{chat_id}-setup").into(), None, theme)
                    .child(
                        icon(icons::DANGER_TRIANGLE)
                            .size(px(16.0))
                            .flex_none()
                            .text_color(theme.danger),
                    )
                    .child(label(match exit_code {
                        Some(code) => format!("Setup failed · exit {code}"),
                        None => "Setup failed".to_string(),
                    }))
                    .child(
                        icon_action(
                            format!("details-{chat_id}-setup-retry").into(),
                            icons::RESTART,
                            "Retry setup",
                            theme,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| retry(this, (), cx))),
                    )
                    .child(
                        icon_action(
                            format!("details-{chat_id}-setup-continue").into(),
                            icons::ARROW_RIGHT,
                            "Continue without setup",
                            theme,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| proceed(this, (), cx))),
                    )
                    .tooltip(crate::tooltip::text(SharedString::from(command.clone())))
                    .into_any_element(),
            );
        }
        None => {}
    }
    if workspace.worktree_branch.is_none() {
        if let Some(pull) = &workspace.auto_pull {
            let toggle = actions.toggle_pull.clone();
            let retry = actions.retry_pull.clone();
            let toggle_space = pull.space_id.clone();
            let retry_space = pull.space_id.clone();
            let enabled = pull.enabled;
            let busy = pull.busy;
            rows.push(
                hover_row(
                    row(
                        format!("details-{chat_id}-pull").into(),
                        Some(icons::GIT_BRANCH),
                        theme,
                    ),
                    theme,
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .line_height(px(16.0))
                                .child(format!("Keep {} up to date", pull.branch)),
                        )
                        .child(
                            mono(
                                pull.error.clone().unwrap_or_else(|| pull.summary.clone()),
                                if pull.error.is_some() {
                                    theme.danger
                                } else {
                                    theme.text_muted
                                },
                                theme,
                            )
                            .line_height(px(13.0))
                            .min_w_0()
                            .truncate(),
                        ),
                )
                .when(pull.error.is_some(), |el| {
                    el.child(
                        icon_action(
                            format!("details-{chat_id}-pull-retry").into(),
                            icons::RESTART,
                            "Retry",
                            theme,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !busy {
                                retry(this, retry_space.clone(), cx);
                            }
                        })),
                    )
                })
                .child(
                    crate::settings::widgets::toggle_switch(theme, enabled)
                        .id(SharedString::from(format!("details-{chat_id}-pull-switch")))
                        .when(busy, |el| el.opacity(0.5))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !busy {
                                toggle(this, (toggle_space.clone(), !enabled), cx);
                            }
                        })),
                )
                .into_any_element(),
            );
        }
        let handoff = actions.move_to_worktree.clone();
        rows.push(
            hover_row(
                row(
                    format!("details-{chat_id}-handoff").into(),
                    Some(icons::GIT_BRANCH),
                    theme,
                ),
                theme,
            )
            .child(label("Move to worktree…"))
            .on_click(cx.listener(move |this, _, _, cx| handoff(this, (), cx)))
            .into_any_element(),
        );
    }
    section("Workspace", Vec::new(), theme)
        .children(rows)
        .into_any_element()
}

fn version_control_section(
    chat_id: &str,
    model: &DetailsModel,
    theme: &Theme,
    actions: &DetailsActions,
    cx: &Context<Shell>,
) -> AnyElement {
    let link = actions.link_pull_request.clone();
    let header_actions = vec![
        icon_action(
            format!("details-{chat_id}-link-pr").into(),
            icons::PLUS,
            "Link pull request",
            theme,
        )
        .on_click(cx.listener(move |this, _, _, cx| link(this, (), cx)))
        .into_any_element(),
    ];
    let mut rows: Vec<AnyElement> = Vec::new();
    // Stack order: bottom of the stack first.
    for pr in &model.pull_requests {
        let url = pr.summary.url.clone();
        let watch_url = url.clone();
        let open = actions.open_url.clone();
        let watch = actions.toggle_watch.clone();
        let watching = pr.watching;
        let checks: Option<AnyElement> = if pr.checks_failed > 0 {
            Some(
                div()
                    .flex()
                    .items_center()
                    .gap(px(3.0))
                    .child(icon(icons::CLOSE).size(px(11.0)).text_color(theme.danger))
                    .child(mono(pr.checks_failed.to_string(), theme.danger, theme))
                    .into_any_element(),
            )
        } else if pr.checks_pending > 0 {
            let sky = SessionState::Working
                .color(theme)
                .unwrap_or(theme.text_muted);
            Some(
                div()
                    .flex()
                    .items_center()
                    .gap(px(3.0))
                    .child(icon(icons::CLOCK_CIRCLE).size(px(11.0)).text_color(sky))
                    .child(mono(pr.checks_pending.to_string(), theme.text_muted, theme))
                    .into_any_element(),
            )
        } else if pr.checks_passed > 0 {
            Some(
                div()
                    .flex()
                    .items_center()
                    .gap(px(3.0))
                    .child(
                        icon(icons::CHECK)
                            .size(px(11.0))
                            .text_color(theme.text_faint),
                    )
                    .child(mono(pr.checks_passed.to_string(), theme.text_muted, theme))
                    .into_any_element(),
            )
        } else {
            None
        };
        rows.push(
            hover_row(
                row(format!("details-{chat_id}-pr-{url}").into(), None, theme),
                theme,
            )
            .pr(px(4.0))
            .child(crate::change_requests::pull_request_badge_with_query(
                format!("details-{chat_id}-pr-{url}-badge").into(),
                pr.summary.clone(),
                crate::change_requests::ChangeRequestBadgeSurface::Sidebar,
                None,
                theme,
            ))
            .child(label(pr.summary.title.clone()))
            .children(checks)
            .child(
                icon_action(
                    format!("details-{chat_id}-pr-{url}-watch").into(),
                    icons::EYE,
                    if watching {
                        "Stop watching"
                    } else {
                        "Watch for checks and reviews"
                    },
                    theme,
                )
                // Watching reads as a present eye; not watching recedes.
                .when(!watching, |el| el.opacity(0.45))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    watch(this, (watch_url.clone(), !watching), cx)
                })),
            )
            .on_click(cx.listener(move |this, _, _, cx| open(this, url.clone(), cx)))
            .into_any_element(),
        );
    }
    let commit = actions.commit.clone();
    rows.push(
        hover_row(
            row(
                format!("details-{chat_id}-commit").into(),
                Some(icons::CHECKLIST),
                theme,
            ),
            theme,
        )
        .child(label("Commit…"))
        .on_click(cx.listener(move |this, _, _, cx| commit(this, (), cx)))
        .into_any_element(),
    );
    section("Version control", header_actions, theme)
        .children(rows)
        .into_any_element()
}

pub fn relative_future(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let seconds = (at - now).num_seconds().max(0);
    match seconds {
        0..=59 => "in <1m".to_string(),
        60..=3599 => format!("in {}m", seconds / 60),
        3600..=86_399 => format!("in {}h", seconds / 3600),
        _ => format!("in {}d", seconds / 86_400),
    }
}

fn automations_section(
    chat_id: &str,
    model: &DetailsModel,
    now: DateTime<Utc>,
    theme: &Theme,
    actions: &DetailsActions,
    cx: &Context<Shell>,
) -> AnyElement {
    let manage = actions.manage_automations.clone();
    let header_actions = vec![
        icon_action(
            format!("details-{chat_id}-automations-manage").into(),
            icons::SETTINGS_MINIMALISTIC,
            "Manage scheduled tasks",
            theme,
        )
        .on_click(cx.listener(move |this, _, _, cx| manage(this, (), cx)))
        .into_any_element(),
    ];
    let rows = model.automations.iter().map(|task| {
        let dot = match task.last_run {
            RunStatus::Never => theme.text_faint,
            RunStatus::Running => SessionState::Working
                .color(theme)
                .unwrap_or(theme.text_faint),
            RunStatus::Succeeded => SessionState::Completed
                .color(theme)
                .unwrap_or(theme.text_faint),
            RunStatus::Failed => theme.danger,
        };
        let sub = match (task.enabled, task.next_run_at) {
            (false, _) => format!("{} · paused", task.cadence),
            (true, Some(next)) => format!("{} · next {}", task.cadence, relative_future(next, now)),
            (true, None) => task.cadence.clone(),
        };
        let run_id = task.id.clone();
        let toggle_id = task.id.clone();
        let run = actions.run_automation.clone();
        let toggle = actions.toggle_automation.clone();
        let enabled = task.enabled;
        div()
            .id(SharedString::from(format!(
                "details-{chat_id}-auto-{}",
                task.id
            )))
            .min_h(px(44.0))
            .w_full()
            .flex()
            .items_center()
            .gap(px(ROW_GAP))
            .px(px(ROW_PAD_X))
            .py(px(6.0))
            .rounded(px(ROW_RADIUS))
            .child(
                div()
                    .relative()
                    .size(px(16.0))
                    .flex_none()
                    .child(
                        icon(icons::CALENDAR)
                            .size(px(16.0))
                            .text_color(theme.text_muted),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(-2.0))
                            .right(px(-2.0))
                            .size(px(6.0))
                            .rounded_full()
                            .bg(dot),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .truncate()
                            .text_size(crate::typography::ui_rems(13.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text.opacity(0.8))
                            .child(SharedString::from(task.title.clone())),
                    )
                    .child(mono(sub, theme.text_muted, theme).truncate()),
            )
            .child(
                icon_action(
                    format!("details-{chat_id}-auto-{}-run", task.id).into(),
                    icons::ACTION_PLAY,
                    "Run now",
                    theme,
                )
                .when(task.last_run == RunStatus::Running, |el| el.opacity(0.4))
                .on_click(cx.listener(move |this, _, _, cx| run(this, run_id.clone(), cx))),
            )
            .child(
                crate::settings::widgets::toggle_switch(theme, enabled)
                    .id(SharedString::from(format!(
                        "details-{chat_id}-auto-{}-switch",
                        task.id
                    )))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        toggle(this, (toggle_id.clone(), !enabled), cx)
                    })),
            )
            .into_any_element()
    });
    section("Automations", header_actions, theme)
        .children(rows)
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn checkpoints_section(
    chat_id: &str,
    model: &DetailsModel,
    ui: &DetailsUi,
    now: DateTime<Utc>,
    theme: &Theme,
    actions: &DetailsActions,
    toggle: ShellAction<()>,
    cx: &Context<Shell>,
) -> AnyElement {
    let total = model.checkpoints.len();
    let shown = if ui.checkpoints_expanded {
        total
    } else {
        total.min(CHECKPOINTS_COLLAPSED)
    };
    let mut rows: Vec<AnyElement> = model
        .checkpoints
        .iter()
        .take(shown)
        .map(|checkpoint| {
            let restore = actions.restore_checkpoint.clone();
            let fork = actions.fork_thread.clone();
            let fork_id = checkpoint.id.clone();
            let can_fork = model.transfers_supported && !model.transfer_busy && checkpoint.forkable;
            let id = checkpoint.id.clone();
            let local = checkpoint.at.with_timezone(&chrono::Local);
            let stamp = if now.signed_duration_since(checkpoint.at).num_hours() < 20 {
                local.format("%H:%M").to_string()
            } else {
                local.format("%b %-d").to_string()
            };
            let group: SharedString = format!("details-{chat_id}-cp-{}", checkpoint.id).into();
            hover_row(row(group.clone(), None, theme), theme)
                .group(group.clone())
                .child(mono(stamp, theme.text_faint, theme).w(px(40.0)))
                .child(label(checkpoint.summary.clone()))
                // A checkpoint with no file changes shows no counts.
                .when(checkpoint.additions + checkpoint.deletions > 0, |el| {
                    el.child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .child(mono(
                                format!("+{}", checkpoint.additions),
                                theme.diff_add,
                                theme,
                            ))
                            .child(mono(
                                format!("−{}", checkpoint.deletions),
                                theme.diff_del,
                                theme,
                            )),
                    )
                })
                .when(can_fork, |el| {
                    el.child(
                        icon_action(
                            format!("details-{chat_id}-cp-{}-fork", checkpoint.id).into(),
                            icons::GIT_BRANCH,
                            "Fork conversation from this checkpoint",
                            theme,
                        )
                        .opacity(0.0)
                        .group_hover(group.clone(), |el| el.opacity(1.0))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            fork(this, Some(fork_id.clone()), cx);
                        })),
                    )
                })
                .child(
                    icon_action(
                        format!("details-{chat_id}-cp-{}-restore", checkpoint.id).into(),
                        icons::UNDO,
                        "Restore files to this checkpoint…",
                        theme,
                    )
                    .opacity(0.0)
                    .group_hover(group, |el| el.opacity(1.0))
                    .on_click(cx.listener(move |this, _, _, cx| restore(this, id.clone(), cx))),
                )
                .into_any_element()
        })
        .collect();
    if total > CHECKPOINTS_COLLAPSED {
        let label_text = if ui.checkpoints_expanded {
            "Show fewer".to_string()
        } else {
            format!("Show all · {total}")
        };
        rows.push(
            hover_row(
                row(format!("details-{chat_id}-cp-more").into(), None, theme),
                theme,
            )
            .text_color(theme.text_muted)
            .font_weight(FontWeight::NORMAL)
            .child(label(label_text))
            .on_click(cx.listener(move |this, _, _, cx| toggle(this, (), cx)))
            .into_any_element(),
        );
    }
    section("Checkpoints", Vec::new(), theme)
        .children(rows)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_shape_parses() {
        let model: DetailsModel = serde_json::from_value(serde_json::json!({
            "workspace": {"worktreeBranch": "design/w3", "setup": {"state": "running", "command": "bun install", "startedAt": "2026-10-04T10:00:00Z"}},
            "pullRequests": [{"summary": {"provider": "github", "number": 42, "title": "Lifecycle", "url": "https://x", "state": "open", "baseRef": "dev", "headRef": "w3"}, "checksFailed": 2, "watching": true}],
            "automations": [{"id": "a", "title": "Nightly triage", "cadence": "Weekdays 09:00", "nextRunAt": null, "enabled": false, "lastRun": "failed"}],
            "checkpoints": [{"id": "c", "at": "2026-10-04T10:00:00Z", "summary": "Port shelves", "additions": 120, "deletions": 8}]
        }))
        .unwrap();
        assert_eq!(model.pull_requests[0].checks_failed, 2);
        assert!(matches!(
            model.workspace.setup,
            Some(SetupState::Running { .. })
        ));
    }

    #[test]
    fn relative_future_is_compact() {
        let now = Utc::now();
        assert_eq!(
            relative_future(now + chrono::Duration::minutes(23), now),
            "in 23m"
        );
        assert_eq!(
            relative_future(now + chrono::Duration::hours(5), now),
            "in 5h"
        );
        assert_eq!(
            relative_future(now - chrono::Duration::hours(5), now),
            "in <1m"
        );
    }

    #[test]
    fn session_disconnect_requires_capability_observed_attachment_or_exact_retry() {
        let mut model = DetailsModel::default();
        assert!(!model.can_disconnect_session());
        model
            .attached_provider_sessions
            .push(zeron_proto::transfer::ProviderSessionRef {
                id: "session".into(),
                attachment_sequence: 7,
            });
        assert!(!model.can_disconnect_session());
        model.session_control_supported = true;
        assert!(model.can_disconnect_session());
        model.session_busy = true;
        assert!(!model.can_disconnect_session());
        model.session_busy = false;
        model.transfer_busy = true;
        assert!(!model.can_disconnect_session());
        model.transfer_busy = false;
        model.attached_provider_sessions.clear();
        assert!(!model.can_disconnect_session());
        model.session_retry = true;
        assert!(model.can_disconnect_session());
    }

    #[test]
    fn transfer_actions_need_host_capability_and_available_parent_but_not_a_cached_run() {
        let mut model = DetailsModel::default();
        assert!(!model.can_fork());
        model.fork_run_id = Some("run".into());
        model.merge_run_id = Some("child-run".into());
        model.merge_target = Some("parent".into());
        assert!(!model.can_fork());
        model.transfers_supported = true;
        assert!(model.can_fork());
        assert!(model.fork_ready());
        assert!(!model.can_merge_back());
        model.lineage.push(ConversationRelation {
            chat_id: "parent".into(),
            available: true,
            ..Default::default()
        });
        assert!(model.can_merge_back());
        model.transfer_busy = true;
        assert!(!model.can_fork());
        assert!(!model.can_merge_back());
        model.transfer_busy = false;
        model.lineage[0].available = false;
        assert!(!model.can_merge_back());
        model.lineage[0].available = true;
        // A cached "no finished run" may predate the turn that just ended:
        // it dims the action but must not disable it.
        model.fork_run_id = None;
        model.merge_run_id = None;
        assert!(model.can_fork() && !model.fork_ready());
        assert!(model.can_merge_back());
        // Another fork's pending merge-back would wedge the parent's next start.
        model.merge_blocked = true;
        assert!(!model.can_merge_back());
    }
}
