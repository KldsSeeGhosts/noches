//! Settings › Automations (DESIGN-W3 §4, T3 `/settings/scheduled-tasks`):
//! every scheduled task this app can see, grouped by project, with run-now,
//! pause and delete. Agents create tasks through `schedule_task`; this page
//! manages them.

use gpui::{
    AnyElement, Context, Entity, FontWeight, SharedString, Subscription, Window, div, prelude::*,
    px,
};

use crate::automations::AutomationRow;
use crate::details::RunStatus;
use crate::icons::{self, icon};
use crate::popover;
use crate::settings::widgets;
use crate::state::AppState;
use crate::status_palette::SessionState;
use crate::theme::Theme;

pub struct AutomationsPage {
    state: Entity<AppState>,
    scroll: widgets::PageScroll,
    /// Task whose Delete is armed (second click confirms).
    confirm_delete: Option<String>,
    _observe: Subscription,
}

/// Projects in first-seen order, tasks by title inside each.
pub fn group_by_project(rows: &[AutomationRow]) -> Vec<(String, Vec<AutomationRow>)> {
    let mut groups: Vec<(String, Vec<AutomationRow>)> = Vec::new();
    for row in rows {
        match groups
            .iter_mut()
            .find(|(project, _)| *project == row.project_id)
        {
            Some((_, list)) => list.push(row.clone()),
            None => groups.push((row.project_id.clone(), vec![row.clone()])),
        }
    }
    for (_, list) in &mut groups {
        list.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    }
    groups
}

impl AutomationsPage {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&state, |_, _, cx| cx.notify());
        // Every device that hosts a project owns its own scheduler.
        state.update(cx, |state, cx| {
            let owners: std::collections::BTreeSet<String> = state
                .spaces
                .iter()
                .map(|space| space.device_id.clone())
                .chain(state.local_device_id.clone())
                .collect();
            for owner in owners {
                state.ensure_automations_watch(&owner, cx);
            }
        });
        Self {
            state,
            scroll: widgets::PageScroll::default(),
            confirm_delete: None,
            _observe: observe,
        }
    }

    fn on_scroll_hovered(&mut self, hovered: &bool, _: &mut Window, cx: &mut Context<Self>) {
        if self.scroll.set_list_hovered(*hovered) {
            cx.notify();
        }
    }

    fn row(
        &self,
        row: &AutomationRow,
        first: bool,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> AnyElement {
        let now = chrono::Utc::now();
        let dot = match row.last_run {
            RunStatus::Never => theme.text_faint,
            RunStatus::Running => SessionState::Working
                .color(theme)
                .unwrap_or(theme.text_faint),
            RunStatus::Succeeded => SessionState::Completed
                .color(theme)
                .unwrap_or(theme.text_faint),
            RunStatus::Failed => theme.danger,
        };
        let sub = match (row.enabled, row.next_run_at) {
            (false, _) => format!("{} · paused", row.cadence),
            (true, Some(next)) => format!(
                "{} · next {}",
                row.cadence,
                crate::details::relative_future(next, now)
            ),
            (true, None) => row.cadence.clone(),
        };
        let thread = row.thread_id.as_ref().and_then(|id| {
            self.state
                .read(cx)
                .chats
                .iter()
                .find(|chat| &chat.id == id)
                .map(|chat| {
                    chat.title
                        .clone()
                        .unwrap_or_else(|| "Untitled session".into())
                })
        });
        let mono = |text: String, color| {
            div()
                .font_family(theme.font_mono.clone())
                .text_size(crate::typography::ui_rems(11.0))
                .text_color(color)
                .child(SharedString::from(text))
        };
        let icon_button = |id: String, glyph: &'static str, tip: &'static str| {
            let hover = theme.wash(0.08);
            div()
                .id(SharedString::from(id))
                .role(gpui::Role::Button)
                .aria_label(tip)
                .size(px(24.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(move |el| el.bg(hover))
                .tooltip(crate::tooltip::text(tip))
                .child(icon(glyph).size(px(14.0)).text_color(theme.text_muted))
        };
        let run_id = row.id.clone();
        let toggle_id = row.id.clone();
        let delete_id = row.id.clone();
        let enabled = row.enabled;
        let armed = self.confirm_delete.as_deref() == Some(row.id.as_str());
        widgets::card_row(theme, first)
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
                    .gap(px(2.0))
                    .child(
                        div()
                            .truncate()
                            .text_size(crate::typography::ui_rems(widgets::ROW_TITLE_SIZE))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(SharedString::from(row.title.clone())),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .min_w_0()
                            .child(mono(sub, theme.text_muted))
                            .when_some(thread, |el, thread| {
                                el.child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(crate::typography::ui_rems(12.0))
                                        .text_color(theme.text_faint)
                                        .child(SharedString::from(thread)),
                                )
                            }),
                    )
                    .when_some(
                        row.last_error
                            .clone()
                            .filter(|_| row.last_run == RunStatus::Failed),
                        |el, error| {
                            el.child(
                                div()
                                    .truncate()
                                    .text_size(crate::typography::ui_rems(12.0))
                                    .text_color(theme.danger)
                                    .child(SharedString::from(error)),
                            )
                        },
                    ),
            )
            .child(
                icon_button(
                    format!("auto-run-{}", row.id),
                    icons::ACTION_PLAY,
                    "Run now",
                )
                .when(row.last_run == RunStatus::Running, |el| el.opacity(0.4))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.state
                        .update(cx, |state, cx| state.run_automation_now(&run_id, cx));
                })),
            )
            .child(if armed {
                div()
                    .id(SharedString::from(format!("auto-delete-{}", row.id)))
                    .flex_none()
                    .px(px(8.0))
                    .h(px(24.0))
                    .flex()
                    .items_center()
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .text_size(crate::typography::ui_rems(12.0))
                    .text_color(theme.danger)
                    .hover(|el| el.bg(theme.danger.opacity(0.08)))
                    .child("Delete")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.confirm_delete = None;
                        this.state
                            .update(cx, |state, cx| state.delete_automation(&delete_id, cx));
                    }))
                    .into_any_element()
            } else {
                icon_button(
                    format!("auto-delete-{}", row.id),
                    icons::TRASH_BIN_MINIMALISTIC,
                    "Delete automation",
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.confirm_delete = Some(delete_id.clone());
                    cx.notify();
                }))
                .into_any_element()
            })
            .child(
                widgets::toggle_switch(theme, enabled)
                    .id(SharedString::from(format!("auto-switch-{}", row.id)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.state.update(cx, |state, cx| {
                            state.set_automation_enabled(&toggle_id, !enabled, cx)
                        });
                    })),
            )
            .into_any_element()
    }
}

impl popover::ScrollRailHost for AutomationsPage {
    fn rail_bar(&mut self) -> &mut popover::MenuScrollbarState {
        self.scroll.rail_bar()
    }

    fn rail_scroll(&self) -> Option<gpui::ScrollHandle> {
        self.scroll.rail_scroll()
    }
}

impl Render for AutomationsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let (rows, error, project_names) = {
            let state = self.state.read(cx);
            let rows: Vec<AutomationRow> = state.automations.rows().cloned().collect();
            let names: std::collections::HashMap<String, String> = state
                .spaces
                .iter()
                .map(|space| (space.id.clone(), space.display_name().to_string()))
                .collect();
            (rows, state.automations.error.clone(), names)
        };
        let count = rows.len();
        let groups = group_by_project(&rows);
        let body: AnyElement = if groups.is_empty() {
            div()
                .mt(px(96.0))
                .flex()
                .flex_col()
                .items_center()
                .text_center()
                .child(
                    icon(icons::CALENDAR)
                        .size(px(28.0))
                        .text_color(theme.text_muted.opacity(0.2)),
                )
                .child(
                    div()
                        .mt(px(12.0))
                        .text_size(crate::typography::ui_rems(14.0))
                        .text_color(theme.text_muted.opacity(0.5))
                        .child("No automations yet"),
                )
                .child(
                    div()
                        .mt(px(4.0))
                        .max_w(px(360.0))
                        .text_size(crate::typography::ui_rems(12.0))
                        .text_color(theme.text_muted.opacity(0.4))
                        .child(
                            "Ask an agent to schedule a task, for example \"every weekday at 9, triage new issues\".",
                        ),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .children(groups.into_iter().map(|(project, list)| {
                    let name = project_names
                        .get(&project)
                        .cloned()
                        .unwrap_or_else(|| project.clone());
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .mt(px(24.0))
                                .px(px(4.0))
                                .text_size(crate::typography::ui_rems(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.text_muted)
                                .child(SharedString::from(name)),
                        )
                        .child(
                            widgets::section_card(&theme).mt(px(8.0)).children(
                                list.iter()
                                    .enumerate()
                                    .map(|(ix, row)| self.row(row, ix == 0, &theme, cx)),
                            ),
                        )
                }))
                .into_any_element()
        };
        let scrollbar = popover::rail(self, "automations-page-scrollbar", &theme, cx);
        div()
            .id("automations-page-host")
            .relative()
            .size_full()
            .on_hover(cx.listener(Self::on_scroll_hovered))
            .child(
                div()
                    .id("automations-page")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll.scroll)
                    .child(
                        widgets::page_column()
                            .child(widgets::page_header(
                                &theme,
                                "Automations",
                                (count > 0).then_some(count),
                            ))
                            .child(widgets::page_subtitle(
                                &theme,
                                "Scheduled prompts that run on the device that owns the project. Paused tasks keep their schedule.",
                            ))
                            .when_some(error, |el, message| {
                                el.child(
                                    widgets::error_strip(&theme, message)
                                        .id("automations-error")
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.update(cx, |state, cx| {
                                                state.automations.error = None;
                                                cx.notify();
                                            });
                                        })),
                                )
                            })
                            .child(body),
                    ),
            )
            .children(scrollbar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, project: &str, title: &str) -> AutomationRow {
        AutomationRow {
            id: id.into(),
            owner_host_id: "mac".into(),
            title: title.into(),
            prompt: String::new(),
            project_id: project.into(),
            thread_id: None,
            enabled: true,
            cadence: "Every 1h".into(),
            next_run_at: None,
            last_run: RunStatus::Never,
            last_error: None,
        }
    }

    #[test]
    fn groups_keep_project_order_and_sort_titles() {
        let groups = group_by_project(&[
            row("1", "b", "zeta"),
            row("2", "a", "beta"),
            row("3", "b", "Alpha"),
        ]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "b");
        let titles: Vec<_> = groups[0].1.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, ["Alpha", "zeta"]);
    }
}
