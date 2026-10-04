//! Settings › Import history (DESIGN-W3 §8). Discovery and import never
//! execute a harness; original-session binding is a separate confirmation.
use std::collections::{BTreeSet, HashMap, HashSet};

use gpui::{
    AnyElement, Context, Entity, EventEmitter, FontWeight, ListAlignment, ListState, SharedString,
    Subscription, Window, div, list, prelude::*, px,
};
use serde_json::json;
use zeron_proto::git_actions::{HistoryPreview, HistorySource};
use zeron_rpc::git_actions::methods;

use crate::{
    controls::{self, Size, Variant},
    git_dialog::{checkbox, mono},
    git_store::call,
    icons::{self, icon},
    motion, popover,
    settings::widgets,
    state::AppState,
    theme::Theme,
};

pub fn source_label(source: HistorySource) -> &'static str {
    match source {
        HistorySource::ClaudeCode => "Claude Code",
        HistorySource::Codex => "Codex",
    }
}

/// Deterministic source order, discovery order preserved inside each source.
pub fn group_candidates<'a>(
    rows: impl IntoIterator<Item = &'a HistoryPreview>,
) -> Vec<(HistorySource, Vec<&'a HistoryPreview>)> {
    let mut claude = Vec::new();
    let mut codex = Vec::new();
    for row in rows {
        match row.provenance.source {
            HistorySource::ClaudeCode => claude.push(row),
            HistorySource::Codex => codex.push(row),
        }
    }
    [
        (HistorySource::ClaudeCode, claude),
        (HistorySource::Codex, codex),
    ]
    .into_iter()
    .filter(|(_, rows)| !rows.is_empty())
    .collect()
}

#[derive(Clone, PartialEq)]
enum Item {
    Header(HistorySource, usize),
    Candidate(String),
    Pending(String),
}
pub struct OpenChat(pub String);
pub struct ImportPage {
    state: Entity<AppState>,
    project: Option<String>,
    project_menu: bool,
    selected: HashMap<String, BTreeSet<String>>,
    expanded: HashSet<String>,
    confirm: Option<HistoryPreview>,
    continuing: bool,
    list: ListState,
    items: Vec<Item>,
    _observe: Subscription,
}
impl EventEmitter<OpenChat> for ImportPage {}

impl ImportPage {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        motion::init_hover_owner(cx);
        let project = state
            .read(cx)
            .selected_space
            .clone()
            .or_else(|| state.read(cx).spaces.first().map(|s| s.id.clone()));
        let observe = cx.observe(&state, |page, _, cx| {
            page.list.remeasure();
            cx.notify();
        });
        Self {
            state,
            project,
            project_menu: false,
            selected: HashMap::new(),
            expanded: HashSet::new(),
            confirm: None,
            continuing: false,
            list: ListState::new(0, ListAlignment::Top, px(100.0)),
            items: Vec::new(),
            _observe: observe,
        }
    }

    fn sync_items(&mut self, cx: &mut Context<Self>) {
        let mut items = Vec::new();
        if let Some(project_id) = &self.project {
            if let Some(project) = self.state.read(cx).history_import.projects.get(project_id) {
                let ids = project
                    .scan
                    .as_ref()
                    .map(|s| s.candidate_ids.as_slice())
                    .unwrap_or_default();
                let rows: Vec<_> = ids
                    .iter()
                    .filter_map(|id| project.previews.get(id))
                    .collect();
                for (source, rows) in group_candidates(rows) {
                    items.push(Item::Header(source, rows.len()));
                    items.extend(
                        rows.into_iter()
                            .map(|r| Item::Candidate(r.candidate_id.clone())),
                    );
                }
                items.extend(
                    ids.iter()
                        .filter(|id| !project.previews.contains_key(*id))
                        .cloned()
                        .map(Item::Pending),
                );
                if let Some(selected) = self.selected.get_mut(project_id) {
                    selected.retain(|id| {
                        ids.contains(id)
                            && !project
                                .previews
                                .get(id)
                                .is_some_and(|p| p.already_imported_chat_id.is_some())
                    });
                }
            }
        }
        if self.items != items {
            let mut anchor = self.list.logical_scroll_top();
            let at_top = anchor.item_ix == 0 && anchor.offset_in_item == px(0.0);
            let key = self.items.get(anchor.item_ix).cloned();
            self.list.splice(0..self.items.len(), items.len());
            if at_top {
                anchor.item_ix = 0;
            } else if let Some(key) = key {
                anchor.item_ix = items
                    .iter()
                    .position(|item| match (&key, item) {
                        (Item::Header(a, _), Item::Header(b, _)) => a == b,
                        (
                            Item::Candidate(a) | Item::Pending(a),
                            Item::Candidate(b) | Item::Pending(b),
                        ) => a == b,
                        _ => false,
                    })
                    .unwrap_or(anchor.item_ix.min(items.len().saturating_sub(1)));
            }
            self.list.scroll_to(anchor);
            self.items = items;
        }
    }

    fn render_item(&mut self, index: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::of(cx).clone();
        let Some(item) = self.items.get(index).cloned() else {
            return div().into_any_element();
        };
        let Some(space_id) = self.project.clone() else {
            return div().into_any_element();
        };
        match item {
            Item::Header(source, count) => {
                let harness = match source {
                    HistorySource::ClaudeCode => zeron_proto::HarnessId::ClaudeCode,
                    HistorySource::Codex => zeron_proto::HarnessId::Codex,
                };
                let (mark, tint) = crate::pickers::harness_brand_icon(harness);
                div()
                    .h(px(24.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        icon(mark)
                            .size(px(16.0))
                            .text_color(tint.unwrap_or(theme.text_muted)),
                    )
                    .child(
                        div()
                            .text_size(crate::typography::ui_rems(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_muted)
                            .child(source_label(source)),
                    )
                    .child(mono(count.to_string(), &theme, theme.text_faint))
                    .into_any_element()
            }
            Item::Pending(id) => {
                let error = self
                    .state
                    .read(cx)
                    .history_import
                    .projects
                    .get(&space_id)
                    .and_then(|p| p.preview_errors.get(&id))
                    .cloned();
                // ListState only calls this for visible/overscan rows.
                if error.is_none() {
                    self.state
                        .update(cx, |s, cx| s.ensure_history_preview(&space_id, &id, cx));
                }
                let failed = error.is_some();
                div()
                    .h(px(44.0))
                    .flex()
                    .items_center()
                    .child(mono(
                        error.unwrap_or_else(|| "Loading history…".into()),
                        &theme,
                        if failed {
                            theme.danger
                        } else {
                            theme.text_muted
                        },
                    ))
                    .into_any_element()
            }
            Item::Candidate(id) => {
                let Some(preview) = self
                    .state
                    .read(cx)
                    .history_import
                    .projects
                    .get(&space_id)
                    .and_then(|p| p.previews.get(&id))
                    .cloned()
                else {
                    return div().into_any_element();
                };
                let imported = preview.already_imported_chat_id.clone();
                let selected = self
                    .selected
                    .get(&space_id)
                    .is_some_and(|ids| ids.contains(&id));
                let expanded = self.expanded.contains(&id);
                let date = preview
                    .messages
                    .first()
                    .and_then(|m| chrono::DateTime::from_timestamp_millis(m.created_at))
                    .map(|at| at.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "Unknown date".into());
                let meta = format!(
                    "{date} · {} messages{}",
                    preview.messages.len(),
                    if preview.provenance.truncated {
                        " · truncated"
                    } else {
                        ""
                    }
                );
                let select_id = id.clone();
                let select_view = cx.entity().downgrade();
                let expand_id = id.clone();
                let open = imported.clone();
                let mut row = div().flex().flex_col().child(
                    div()
                        .id(SharedString::from(format!("history-row-{id}")))
                        .h(px(44.0))
                        .px(px(10.0))
                        .rounded(px(8.0))
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .cursor_pointer()
                        .hover(|el| {
                            el.bg(theme.wash(if theme.appearance.is_dark() {
                                0.075
                            } else {
                                0.055
                            }))
                        })
                        .when(imported.is_none(), |el| {
                            el.child(
                                checkbox(
                                    format!("history-checkbox-{id}"),
                                    selected,
                                    false,
                                    "Select history",
                                    &theme,
                                )
                                .cursor_pointer()
                                .on_change(move |_, _, _, cx| {
                                    cx.stop_propagation();
                                    select_view
                                        .update(cx, |page, cx| {
                                            let ids =
                                                page.selected.entry(space_id.clone()).or_default();
                                            if !ids.remove(&select_id) && ids.len() < 100 {
                                                ids.insert(select_id.clone());
                                            }
                                            cx.notify();
                                        })
                                        .ok();
                                }),
                            )
                        })
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
                                        .text_size(crate::typography::ui_rems(13.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.text)
                                        .child(preview.title.clone()),
                                )
                                .child(mono(meta, &theme, theme.text_muted)),
                        )
                        .when(imported.is_some(), |el| {
                            el.child(
                                mono("Imported", &theme, theme.text_muted)
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(theme.wash(0.055)),
                            )
                        })
                        .child(
                            controls::icon_button(
                                format!("history-expand-{id}"),
                                cx.entity_id(),
                                &theme,
                                Variant::Ghost,
                                Size::Xs,
                                if expanded {
                                    icons::ALT_ARROW_DOWN
                                } else {
                                    icons::ALT_ARROW_RIGHT
                                },
                                if expanded {
                                    "Collapse preview"
                                } else {
                                    "Preview messages"
                                },
                            )
                            .tooltip(crate::tooltip::text(if expanded {
                                "Collapse preview"
                            } else {
                                "Preview messages"
                            }))
                            .on_click(cx.listener(
                                move |page, _, _, cx| {
                                    cx.stop_propagation();
                                    if !page.expanded.remove(&expand_id) {
                                        page.expanded.insert(expand_id.clone());
                                    }
                                    page.list.remeasure_items(index..index + 1);
                                    cx.notify();
                                },
                            )),
                        )
                        .on_click(cx.listener(move |_, _, _, cx| {
                            if let Some(chat) = &open {
                                cx.emit(OpenChat(chat.clone()));
                            }
                        })),
                );
                if expanded {
                    row = row.child(div().pl(px(36.0)).pr(px(10.0)).children(
                        preview.messages.iter().take(3).map(|m| {
                            div()
                                .py(px(4.0))
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(crate::typography::ui_rems(11.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.text_faint)
                                        .child(m.role.clone()),
                                )
                                .child(
                                    div()
                                        .max_h(px(32.0))
                                        .overflow_hidden()
                                        .text_size(crate::typography::ui_rems(12.0))
                                        .line_height(px(16.0))
                                        .text_color(theme.text_muted)
                                        .child(m.text.clone()),
                                )
                        }),
                    ));
                }
                if imported.is_some() && expanded {
                    row = row.child(
                        controls::button(
                            format!("history-continue-{id}"),
                            cx.entity_id(),
                            &theme,
                            Variant::Ghost,
                            Size::Sm,
                            "Continue session…",
                        )
                        .on_click(cx.listener(move |page, _, _, cx| {
                            page.confirm = Some(preview.clone());
                            cx.notify();
                        })),
                    );
                }
                row.into_any_element()
            }
        }
    }

    fn continue_session(&mut self, cx: &mut Context<Self>) {
        if self.continuing {
            return;
        }
        let (Some(project), Some(preview), Some(engine)) = (
            self.project.clone(),
            self.confirm.clone(),
            self.state.read(cx).engine().cloned(),
        ) else {
            return;
        };
        let Some(owner) = self
            .state
            .read(cx)
            .spaces
            .iter()
            .find(|s| s.id == project)
            .map(|s| s.device_id.clone())
        else {
            return;
        };
        let Some(chat) = preview.already_imported_chat_id.clone() else {
            return;
        };
        self.continuing = true;
        cx.spawn(async move |this, cx| {
            let result: Result<(), _> = call(
                engine.client(),
                &owner,
                methods::CONTINUE_HISTORY,
                json!({"spaceId":project, "candidateId":preview.candidate_id, "chatId":chat}),
            )
            .await;
            this.update(cx, |page, cx| {
                page.continuing = false;
                match result {
                    Ok(()) => {
                        page.confirm = None;
                        cx.emit(OpenChat(chat));
                    }
                    Err(error) => {
                        page.confirm = None;
                        page.state.update(cx, |state, cx| {
                            state
                                .history_import
                                .projects
                                .entry(project)
                                .or_default()
                                .error = Some(error.to_string());
                            cx.notify();
                        });
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}

impl Render for ImportPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self
            .project
            .as_ref()
            .is_some_and(|id| self.state.read(cx).spaces.iter().any(|s| &s.id == id))
        {
            self.project = self.state.read(cx).spaces.first().map(|s| s.id.clone());
        }
        self.sync_items(cx);
        let theme = Theme::of(cx).clone();
        let owner = cx.entity_id();
        let (spaces, project) = {
            let state = self.state.read(cx);
            (
                state.spaces.clone(),
                self.project
                    .as_ref()
                    .and_then(|id| state.history_import.projects.get(id))
                    .map(|p| crate::history_import::ProjectHistory {
                        scan: p.scan.clone(),
                        import: p.import.clone(),
                        busy: p.busy,
                        error: p.error.clone(),
                        ..Default::default()
                    })
                    .unwrap_or_default(),
            )
        };
        let selected: Vec<String> = self
            .project
            .as_ref()
            .and_then(|id| self.selected.get(id))
            .map(|ids| ids.iter().cloned().collect())
            .unwrap_or_default();
        let count = selected.len();
        let running = project.scan.as_ref().is_some_and(|s| s.status == "running");
        let resume = project
            .scan
            .as_ref()
            .is_some_and(|s| matches!(s.status.as_str(), "paused" | "cancelled"));
        let importing = project
            .import
            .as_ref()
            .is_some_and(|i| i.status == "running");
        let selected_space = self
            .project
            .as_ref()
            .and_then(|id| spaces.iter().find(|s| &s.id == id));
        let name = selected_space
            .map(|s| s.display_name().to_string())
            .unwrap_or_else(|| "Choose project".into());
        let mut header = widgets::page_column()
            .pb(px(16.0))
            .child(widgets::page_header(&theme, "Import CLI history", None))
            .child(widgets::page_subtitle(&theme, "Bring Claude Code and Codex sessions for a project into Noches. Nothing runs until you continue a session."))
            .child(div().mt(px(16.0)).flex().items_center().gap(px(8.0))
                .child(controls::button("history-project", owner, &theme, Variant::Outline, Size::Sm, name)
                    .when_some(selected_space, |el, space| el.child(
                        div().size(px(14.0)).child(crate::shell::project_monogram(space.display_name(), &space.id, &theme))))
                    .child(icon(icons::ALT_ARROW_DOWN).size(px(12.0)).text_color(theme.text_muted))
                    .on_click(cx.listener(|page, _, _, cx| { page.project_menu = !page.project_menu; cx.notify(); })))
                .child(controls::button("history-scan", owner, &theme, Variant::Primary, Size::Md, if resume { "Resume" } else { "Scan" })
                    .when(running || importing || project.busy || self.project.is_none(), |el| el.opacity(controls::DISABLED_OPACITY))
                    .on_click(cx.listener(move |page, _, _, cx| {
                        if !running && !importing && !project.busy {
                            if let Some(id) = &page.project { page.state.update(cx, |s, cx| s.scan_history(id, resume, cx)); }
                        }
                    }))));
        if self.project_menu {
            header = header.child(popover::popover_card(&theme).w_full().p(px(4.0)).children(
                spaces.into_iter().map(|space| {
                    popover::menu_row_owned(
                        owner,
                        &theme,
                        false,
                        format!("history-project-{}", space.id),
                    )
                    .id(SharedString::from(format!("history-project-{}", space.id)))
                    .gap(px(8.0))
                    .child(div().size(px(14.0)).child(crate::shell::project_monogram(
                        space.display_name(),
                        &space.id,
                        &theme,
                    )))
                    .child(space.display_name().to_string())
                    .on_click(cx.listener(move |page, _, _, cx| {
                        page.project = Some(space.id.clone());
                        page.project_menu = false;
                        page.expanded.clear();
                        cx.notify();
                    }))
                }),
            ));
        }
        if let Some(scan) = &project.scan {
            if running {
                let id = scan.scan_id.clone();
                header = header.child(
                    div()
                        .mt(px(8.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(mono(
                            format!(
                                "{}/{} files · {:.1} MB",
                                scan.scanned_files,
                                scan.total_files,
                                scan.bytes_read as f64 / 1048576.0
                            ),
                            &theme,
                            theme.text_muted,
                        ))
                        .child(
                            controls::button(
                                "history-cancel-scan",
                                owner,
                                &theme,
                                Variant::Ghost,
                                Size::Sm,
                                "Cancel",
                            )
                            .on_click(cx.listener(
                                move |page, _, _, cx| {
                                    if let Some(space) = &page.project {
                                        page.state.update(cx, |s, cx| {
                                            s.cancel_history(space, id.clone(), cx)
                                        });
                                    }
                                },
                            )),
                        ),
                );
            }
            if scan.truncated {
                header = header.child(mono(
                    "Some files were skipped or truncated by scan limits.",
                    &theme,
                    theme.text_muted,
                ));
            }
        }
        if let Some(error) = &project.error {
            header = header.child(widgets::error_strip(&theme, error.clone()));
        }
        let footer = if importing {
            let import = project.import.as_ref().unwrap();
            let id = import.import_id.clone();
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(mono(
                    format!("{} imported", import.completed_candidate_ids.len()),
                    &theme,
                    theme.text_muted,
                ))
                .child(
                    controls::button(
                        "history-cancel-import",
                        owner,
                        &theme,
                        Variant::Ghost,
                        Size::Md,
                        "Cancel",
                    )
                    .on_click(cx.listener(move |page, _, _, cx| {
                        if let Some(space) = &page.project {
                            page.state
                                .update(cx, |s, cx| s.cancel_history(space, id.clone(), cx));
                        }
                    })),
                )
        } else {
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(mono(format!("{count} selected"), &theme, theme.text_muted))
                .child(
                    controls::button(
                        "history-import",
                        owner,
                        &theme,
                        Variant::Primary,
                        Size::Md,
                        format!("Import {count}"),
                    )
                    .when(count == 0 || count > 100 || project.busy, |el| {
                        el.opacity(controls::DISABLED_OPACITY)
                    })
                    .on_click(cx.listener(move |page, _, _, cx| {
                        if count > 0 && count <= 100 {
                            if let Some(space) = &page.project {
                                page.state.update(cx, |s, cx| {
                                    s.import_history(space, selected.clone(), cx)
                                });
                            }
                        }
                    })),
                )
        };
        let confirm = self.confirm.clone().map(|preview| {
            let card = popover::dialog_card(&theme)
                .child(popover::dialog_title(&theme, "Continue session?"))
                .child(popover::dialog_body(&theme, format!("Bind this chat to the original {} session? Your next message resumes it with approval-required permissions.", source_label(preview.provenance.source))))
                .child(popover::dialog_footer(&theme)
                    .child(controls::button("history-continue-cancel", owner, &theme, Variant::Ghost, Size::Md, "Cancel")
                        .on_click(cx.listener(|page, _, _, cx| { if !page.continuing { page.confirm = None; cx.notify(); } })))
                    .child(controls::button("history-continue-confirm", owner, &theme, Variant::Primary, Size::Md, "Continue session")
                        .when(self.continuing, |el| el.opacity(controls::DISABLED_OPACITY))
                        .on_click(cx.listener(|page, _, _, cx| page.continue_session(cx)))));
            popover::modal("history-continue-dialog", window.viewport_size(), card.into_any_element())
        });
        let page =
            div()
                .size_full()
                .relative()
                .flex()
                .flex_col()
                .child(header)
                .child(
                    div().flex_1().min_h_0().px(px(32.0)).child(
                        list(self.list.clone(), cx.processor(Self::render_item)).size_full(),
                    ),
                )
                .child(
                    div()
                        .px(px(32.0))
                        .py(px(14.0))
                        .border_t_1()
                        .border_color(theme.border)
                        .child(footer),
                )
                .children(confirm);
        motion::drive_hover_owner(owner, window);
        page
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn candidate(id: &str, source: HistorySource) -> HistoryPreview {
        HistoryPreview {
            candidate_id: id.into(),
            provenance: zeron_proto::git_actions::HistoryProvenance {
                source,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    #[test]
    fn candidate_groups_have_fixed_source_and_stable_discovery_order() {
        let rows = [
            candidate("c1", HistorySource::Codex),
            candidate("a", HistorySource::ClaudeCode),
            candidate("c2", HistorySource::Codex),
        ];
        let groups = group_candidates(&rows);
        assert_eq!(groups[0].0, HistorySource::ClaudeCode);
        assert_eq!(
            groups[1]
                .1
                .iter()
                .map(|r| r.candidate_id.as_str())
                .collect::<Vec<_>>(),
            ["c1", "c2"]
        );
        assert!(group_candidates(&[]).is_empty());
    }
    #[test]
    fn source_copy_is_not_protocol_spelling() {
        assert_eq!(source_label(HistorySource::ClaudeCode), "Claude Code");
        assert_eq!(source_label(HistorySource::Codex), "Codex");
    }
}
