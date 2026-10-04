//! Wave-3 Details wiring, kept out of shell chrome.
use super::*;
use crate::details_dialog::{Closed, DetailsDialog, Kind};

impl Shell {
    pub(super) fn open_detail_dialog(&mut self, chat: String, kind: Kind, cx: &mut Context<Self>) {
        let state = self.state.clone();
        let dialog = cx.new(|cx| DetailsDialog::new(state, chat, kind, cx));
        self.details_dialog_events = Some(cx.subscribe(&dialog, |this, _, event: &Closed, cx| {
            this.details_dialog = None;
            this.details_dialog_events = None;
            if let Some(notice) = &event.0 {
                this.sidebar_notice = Some(notice.clone().into());
            }
            cx.notify();
        }));
        self.details_dialog = Some(dialog);
        cx.notify();
    }

    pub(super) fn live_details_actions(&self, chat: String) -> crate::details::DetailsActions {
        let watch_chat = chat.clone();
        let link_chat = chat.clone();
        let restore_chat = chat.clone();
        let retry_chat = chat.clone();
        let continue_chat = chat.clone();
        let move_chat = chat.clone();
        crate::details::DetailsActions {
            open_url: std::rc::Rc::new(|_, url, cx| cx.open_url(&url)),
            toggle_watch: std::rc::Rc::new(move |this, (number, watching), cx| {
                this.state.update(cx, |state, cx| {
                    state.toggle_details_watch(&watch_chat, number, watching, cx)
                });
            }),
            link_pull_request: std::rc::Rc::new(move |this, (), cx| {
                this.open_detail_dialog(link_chat.clone(), Kind::Link, cx)
            }),
            commit: std::rc::Rc::new(move |this, (), cx| {
                let state = this.state.clone();
                let chat = chat.clone();
                let dialog =
                    cx.new(|cx| crate::git_dialog::GitDialog::new(state, chat.clone(), cx));
                this.git_dialog_events = Some(cx.subscribe(
                    &dialog,
                    move |this, _, _: &crate::git_dialog::Closed, cx| {
                        this.git_dialog = None;
                        this.git_dialog_events = None;
                        this.state
                            .update(cx, |state, cx| state.refresh_details(&chat, true, cx));
                        cx.notify();
                    },
                ));
                this.git_dialog = Some(dialog);
                cx.notify();
            }),
            run_automation: std::rc::Rc::new(|this, id, cx| {
                this.state
                    .update(cx, |state, cx| state.run_automation_now(&id, cx));
            }),
            toggle_automation: std::rc::Rc::new(|this, (id, enabled), cx| {
                this.state.update(cx, |state, cx| {
                    state.set_automation_enabled(&id, enabled, cx)
                });
            }),
            manage_automations: std::rc::Rc::new(|this, (), cx| {
                this.open_settings(SettingsSection::Automations, cx)
            }),
            restore_checkpoint: std::rc::Rc::new(move |this, id, cx| {
                this.open_detail_dialog(restore_chat.clone(), Kind::Restore(id), cx)
            }),
            retry_setup: std::rc::Rc::new(move |this, (), cx| {
                this.state.update(cx, |state, cx| {
                    state.control_details_setup(&retry_chat, "retry", cx)
                });
            }),
            continue_setup: std::rc::Rc::new(move |this, (), cx| {
                this.state.update(cx, |state, cx| {
                    state.control_details_setup(&continue_chat, "continue", cx)
                });
            }),
            move_to_worktree: std::rc::Rc::new(move |this, (), cx| {
                this.open_detail_dialog(move_chat.clone(), Kind::Handoff, cx)
            }),
            toggle_pull: std::rc::Rc::new(|this, (space, enabled), cx| {
                this.state
                    .update(cx, |state, cx| state.change_pull(&space, Some(enabled), cx));
            }),
            retry_pull: std::rc::Rc::new(|this, space, cx| {
                this.state
                    .update(cx, |state, cx| state.change_pull(&space, None, cx));
            }),
        }
    }

    pub(super) fn details_related_rows(
        &self,
        chat: &str,
        cx: &App,
    ) -> Vec<crate::delegation::RelationRow> {
        let state = self.state.read(cx);
        let mut rows = crate::delegation::related_rows(&state.delegation, chat, |id| {
            state
                .chats
                .iter()
                .find(|c| c.id == id)
                .and_then(|c| c.title.clone())
        });
        if let Some(owner) = state
            .chats
            .iter()
            .find(|c| c.id == chat)
            .map(|c| &c.device_id)
            && let Some(source) = state
                .details
                .get(owner, chat)
                .and_then(|r| r.transfer.as_ref())
                .and_then(crate::details_data::fork_source)
        {
            let title = state
                .chats
                .iter()
                .find(|c| c.id == source)
                .and_then(|c| c.title.clone())
                .filter(|t| !t.trim().is_empty())
                .unwrap_or_else(|| "Parent thread".into());
            if let Some(row) = rows.iter_mut().find(|r| r.chat_id == source) {
                row.title = format!("Forked from {title}").into();
            } else {
                rows.insert(
                    0,
                    crate::delegation::RelationRow {
                        chat_id: source.into(),
                        title: format!("Forked from {title}").into(),
                        relation: crate::delegation::Relation::Parent(
                            crate::delegation::ThreadLinkKind::Fork,
                        ),
                    },
                );
            }
        }
        rows
    }
}
