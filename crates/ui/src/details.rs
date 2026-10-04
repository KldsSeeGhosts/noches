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
}

/// Env var naming a JSON file of `{chatTitle: DetailsModel}` for headed QA.
pub const FIXTURE_ENV: &str = "NOCHES_DETAILS_FIXTURE";

impl DetailsModel {
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
            if let Some(space) = chat.space_id.as_ref().and_then(|id| state.spaces.iter().find(|s| &s.id == id)) {
                let root = crate::git_store::is_root_checkout(chat, space);
                if !root {
                    model.workspace.worktree_branch = chat.branch.clone().or_else(|| Some("Worktree".into()));
                } else if let Some(row) = state.git_actions.pulls.get(&space.id) {
                    let branch = if !row.state.policy.default_branch.is_empty() {
                        row.state.policy.default_branch.clone()
                    } else {
                        row.checkout.as_ref().and_then(|c| c.upstream.as_deref())
                            .and_then(|u| u.split_once('/').map(|(_, b)| b.to_string()))
                            .unwrap_or_else(|| "default branch".into())
                    };
                    model.workspace.auto_pull = Some(AutoPullInfo {
                        space_id: space.id.clone(), branch, enabled: row.state.policy.enabled,
                        summary: crate::git_store::pull_summary(&row.state, Utc::now().timestamp_millis()),
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
                hover_row(row(format!("details-{chat_id}-pull").into(), Some(icons::GIT_BRANCH), theme), theme)
                    .child(div().flex_1().min_w_0().flex().flex_col().gap(px(2.0))
                        .child(div().min_w_0().truncate().line_height(px(16.0)).child(format!("Keep {} up to date", pull.branch)))
                        .child(mono(pull.error.clone().unwrap_or_else(|| pull.summary.clone()),
                            if pull.error.is_some() { theme.danger } else { theme.text_muted }, theme).line_height(px(13.0)).min_w_0().truncate()))
                    .when(pull.error.is_some(), |el| el.child(
                        icon_action(format!("details-{chat_id}-pull-retry").into(), icons::RESTART, "Retry", theme)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !busy { retry(this, retry_space.clone(), cx); }
                            }))))
                    .child(crate::settings::widgets::toggle_switch(theme, enabled)
                        .id(SharedString::from(format!("details-{chat_id}-pull-switch")))
                        .when(busy, |el| el.opacity(0.5)).cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !busy { toggle(this, (toggle_space.clone(), !enabled), cx); }
                        })))
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
                .child(
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
}
