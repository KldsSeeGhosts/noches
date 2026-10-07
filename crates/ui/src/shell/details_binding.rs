//! Wave-3 Details wiring, kept out of shell chrome.
use super::*;
use crate::details_data::{ConversationTransfer, TransferIntent};
use crate::details_dialog::{Closed, DetailsDialog, Kind};
use zeron_proto::transfer::{ForkThreadParams, MergeThreadBackParams, ThreadSourcePoint};

#[cfg(test)]
mod tests {
    use super::*;
    use zeron_rpc::RpcError;

    fn fork_request(point: ThreadSourcePoint) -> ConversationTransfer {
        ConversationTransfer::Fork(ForkThreadParams {
            chat_id: "source".into(),
            command_id: "stable".into(),
            target_chat_id: "one-child".into(),
            source_point: point,
            title: None,
        })
    }

    #[test]
    fn plain_notices_are_neutral_and_failures_are_explicit() {
        let copied: SidebarNotice = "Path copied".into();
        assert!(!copied.failed);
        assert!(!SidebarNotice::information("Conversation forked.").failed);
        assert!(SidebarNotice::failure("Conversation transfer refused.").failed);
    }

    #[test]
    fn transfer_pins_are_keyed_by_chat_and_intent() {
        let mut store = crate::details_data::DetailsStore::default();
        let merge = ("chat".to_owned(), TransferIntent::Merge);
        let fork = ("chat".to_owned(), TransferIntent::Fork(None));
        store.transfer_retries.insert(
            merge.clone(),
            fork_request(ThreadSourcePoint::LatestStable),
        );
        // A stuck Merge does not shadow a Fork of the same chat.
        assert!(store.transfer_retry("chat", &TransferIntent::Merge).is_some());
        assert!(store.transfer_retry("chat", &TransferIntent::Fork(None)).is_none());
        store.transfer_retries.insert(
            fork.clone(),
            fork_request(ThreadSourcePoint::Run {
                run_id: "pinned".into(),
            }),
        );
        let checkpoint = TransferIntent::Fork(Some("cp".into()));
        assert!(store.transfer_retry("chat", &checkpoint).is_none());
    }

    #[test]
    fn transfer_pin_survives_only_an_unknown_outcome() {
        let mut store = crate::details_data::DetailsStore::default();
        let pin = ("chat".to_owned(), TransferIntent::Merge);
        let other = ("chat".to_owned(), TransferIntent::Fork(None));
        let arm = |store: &mut crate::details_data::DetailsStore| {
            for key in [&pin, &other] {
                store
                    .transfer_retries
                    .insert(key.clone(), fork_request(ThreadSourcePoint::LatestStable));
            }
        };
        for unknown in [
            RpcError::Transport("link dropped".into()),
            RpcError::Closed,
            RpcError::Failed("transport: no reply from device d for ForkThread within 30s".into()),
        ] {
            arm(&mut store);
            store.settle_transfer_pin(&pin, Some(&unknown));
            assert!(store.transfer_retries.contains_key(&pin), "{unknown}");
        }
        for definite in [
            RpcError::Failed("The transfer was refused by the host.".into()),
            RpcError::BadParams("bad".into()),
            RpcError::UnknownMethod("MergeThreadBack".into()),
        ] {
            arm(&mut store);
            store.settle_transfer_pin(&pin, Some(&definite));
            assert!(!store.transfer_retries.contains_key(&pin), "{definite}");
            // Settling one intent never touches another's pin.
            assert!(store.transfer_retries.contains_key(&other));
        }
        arm(&mut store);
        store.settle_transfer_pin(&pin, None);
        assert!(!store.transfer_retries.contains_key(&pin));
    }
}

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
        let fork_chat = chat.clone();
        let merge_chat = chat.clone();
        let lineage_chat = chat.clone();
        let transfers_chat = chat.clone();
        let disconnect_chat = chat.clone();
        crate::details::DetailsActions {
            open_thread: std::rc::Rc::new(|this, chat, cx| {
                if this.state.read(cx).chats.iter().any(|c| c.id == chat) {
                    this.open_chat(chat, cx);
                }
            }),
            fork_thread: std::rc::Rc::new(move |this, checkpoint, cx| {
                this.fork_conversation(fork_chat.clone(), checkpoint, cx)
            }),
            merge_back: std::rc::Rc::new(move |this, (), cx| {
                this.transfer_conversation(merge_chat.clone(), TransferIntent::Merge, cx)
            }),
            disconnect_session: std::rc::Rc::new(move |this, (), cx| {
                this.disconnect_agent_session(disconnect_chat.clone(), cx);
            }),
            toggle_lineage: std::rc::Rc::new(move |this, (), cx| {
                let ui = this.details_ui.entry(lineage_chat.clone()).or_default();
                ui.lineage_expanded = !ui.lineage_expanded;
                cx.notify();
            }),
            toggle_transfers: std::rc::Rc::new(move |this, (), cx| {
                let ui = this.details_ui.entry(transfers_chat.clone()).or_default();
                ui.transfers_expanded = !ui.transfers_expanded;
                cx.notify();
            }),
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

    pub(super) fn fork_conversation(
        &mut self,
        chat: String,
        checkpoint: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.transfer_conversation(chat, TransferIntent::Fork(checkpoint), cx);
    }

    pub(super) fn disconnect_agent_session(&mut self, chat: String, cx: &mut Context<Self>) {
        let state = self.state.read(cx);
        let model = crate::details::DetailsModel::for_chat(state, &chat);
        if !model.can_disconnect_session() {
            return;
        }
        let Some(owner) = state
            .chats
            .iter()
            .find(|c| c.id == chat)
            .map(|c| c.device_id.clone())
        else {
            return;
        };
        let Some(engine) = state.engine().cloned() else {
            return;
        };
        let key = (owner.clone(), chat.clone());
        let request = state
            .details
            .session_retries
            .get(&key)
            .cloned()
            .unwrap_or_else(|| zeron_proto::transfer::DisconnectThreadSessionParams {
                chat_id: chat.clone(),
                client_request_id: uuid::Uuid::new_v4().to_string(),
                provider_sessions: model.attached_provider_sessions,
            });
        self.state.update(cx, |state, cx| {
            state.details.session_actions.insert(key.clone());
            state
                .details
                .session_retries
                .insert(key.clone(), request.clone());
            cx.notify();
        });
        let app_state = self.state.clone();
        cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .disconnect_thread_session(request, &owner)
                .await;
            let mut succeeded = false;
            let mut notice = String::new();
            app_state.update(cx, |state, cx| {
                state.details.session_actions.remove(&key);
                match result {
                    Ok(reply) => {
                        state.details.session_retries.remove(&key);
                        if let Some(refusal) = reply.refusal {
                            notice = refusal;
                        } else {
                            succeeded = true;
                            notice =
                                "Agent session disconnect requested. Conversation history is kept."
                                    .into();
                        }
                    }
                    Err(error) => {
                        if matches!(
                            error,
                            zeron_rpc::RpcError::BadParams(_)
                                | zeron_rpc::RpcError::UnknownMethod(_)
                        ) {
                            state.details.session_retries.remove(&key);
                        }
                        notice = format!(
                            "{error}{}",
                            if state.details.session_retries.contains_key(&key) {
                                " Retry targets the same sessions, not a replacement."
                            } else {
                                ""
                            }
                        );
                    }
                }
                state.refresh_details(&chat, true, cx);
                cx.notify();
            });
            this.update(cx, |shell, cx| {
                shell.sidebar_notice = Some(if succeeded {
                    SidebarNotice::information(notice)
                } else {
                    SidebarNotice::failure(notice)
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn transfer_conversation(
        &mut self,
        chat: String,
        intent: TransferIntent,
        cx: &mut Context<Self>,
    ) {
        let state = self.state.read(cx);
        if state.details.transfer_busy(&chat)
            || !state.chat_host_supports(&chat, zeron_proto::capabilities::THREAD_TRANSFERS_V1)
        {
            return;
        }
        let Some(owner) = state
            .chats
            .iter()
            .find(|c| c.id == chat)
            .map(|c| c.device_id.clone())
        else {
            return;
        };
        let Some(engine) = state.engine().cloned() else {
            return;
        };
        // Pinned per (chat, intent): an unconfirmed Merge replays only itself
        // and never blocks a Fork.
        let retry = state.details.transfer_retry(&chat, &intent).cloned();
        let pin = (chat.clone(), intent.clone());
        let app_state = self.state.clone();
        app_state.update(cx, |s, cx| {
            s.details.transfer_actions.insert(chat.clone());
            cx.notify();
        });
        cx.spawn(async move |this, cx| {
            let result = async {
                let request = if let Some(retry) = retry {
                    retry
                } else {
                    // Pin a specific run before submitting, even when opened
                    // from a cold sidebar menu. Never retry LatestStable.
                    let snapshot = engine.client().thread_transfer_state(&chat, &owner).await?;
                    let command_id = uuid::Uuid::new_v4().to_string();
                    match intent {
                        TransferIntent::Fork(checkpoint) => {
                            let point = match checkpoint {
                                Some(checkpoint_id) => ThreadSourcePoint::Checkpoint { checkpoint_id },
                                None => ThreadSourcePoint::Run {
                                    run_id: snapshot.latest_forkable_run_id.ok_or_else(||
                                        zeron_rpc::RpcError::BadParams("Finish a turn before forking this conversation.".into()))?,
                                },
                            };
                            ConversationTransfer::Fork(ForkThreadParams {
                                chat_id: chat.clone(), command_id,
                                target_chat_id: uuid::Uuid::new_v4().to_string(),
                                source_point: point, title: None,
                            })
                        }
                        TransferIntent::Merge => ConversationTransfer::Merge(MergeThreadBackParams {
                            chat_id: chat.clone(), command_id,
                            target_chat_id: crate::details_data::fork_source(&snapshot)
                                .ok_or_else(|| zeron_rpc::RpcError::BadParams("Only a direct conversation fork can merge back.".into()))?
                                .to_string(),
                            source_point: ThreadSourcePoint::Run {
                                run_id: snapshot.latest_mergeable_run_id.ok_or_else(||
                                    zeron_rpc::RpcError::BadParams("Finish a turn in this fork before merging back.".into()))?,
                            },
                        }),
                    }
                };
                // Keep the exact target and source point across response loss,
                // including a failure after the host has committed acceptance.
                app_state.update(cx, |s, _| {
                    s.details.transfer_retries.insert(pin.clone(), request.clone());
                });
                let merging = matches!(request, ConversationTransfer::Merge(_));
                let result = match request {
                    ConversationTransfer::Fork(p) => engine.client().fork_thread(p, &owner).await?,
                    ConversationTransfer::Merge(p) => engine.client().merge_thread_back(p, &owner).await?,
                };
                Ok::<_, zeron_rpc::RpcError>((result, merging))
            }.await;
            let mut open_target = None;
            let mut notice = None;
            let mut succeeded = false;
            app_state.update(cx, |s, cx| {
                s.details.transfer_actions.remove(&chat);
                match result {
                    Ok((reply, merging)) => {
                        s.details.settle_transfer_pin(&pin, None);
                        if let Some(refusal) = reply.refusal {
                            notice = Some(refusal);
                        } else {
                            succeeded = true;
                            if let Some(target) = reply.chat {
                                if let Some(existing) = s.chats.iter_mut().find(|c| c.id == target.id) {
                                    *existing = target;
                                } else {
                                    let mut chats = s.chats.clone();
                                    chats.push(target);
                                    s.apply_chats(chats);
                                }
                            }
                            s.refresh_details(&reply.target_chat_id, true, cx);
                            s.nudge_delegation();
                            if s.chats.iter().any(|c| c.id == reply.target_chat_id) {
                                open_target = Some(reply.target_chat_id);
                            }
                            notice = Some(if merging {
                                "Conversation context prepared for the parent's next message. No files were merged."
                            } else {
                                "Conversation forked. Choose an agent and send a message to continue."
                            }.to_string());
                        }
                    }
                    Err(error) => {
                        s.details.settle_transfer_pin(&pin, Some(&error));
                        notice = Some(format!("{error}{}", if s.details.transfer_retries.contains_key(&pin) {
                            " Retry uses the same request to avoid duplicate forks or merges."
                        } else { "" }));
                    }
                }
                s.refresh_details(&chat, true, cx);
                cx.notify();
            });
            this.update(cx, |view, cx| {
                if let Some(notice) = notice {
                    view.sidebar_notice = Some(if succeeded {
                        SidebarNotice::information(notice)
                    } else { SidebarNotice::failure(notice) });
                }
                if let Some(target) = open_target { view.open_chat(target, cx); }
                cx.notify();
            }).ok();
        }).detach();
        cx.notify();
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
