//! Existing dialog/input primitives with wave-3 owner-routed actions.
use crate::{
    composer::{ComposerInput, ComposerInputEvent},
    controls::{self, Size, Variant},
    git_dialog::{checkbox, dialog_field, mono},
    motion, popover,
    settings::widgets,
    state::AppState,
    theme::Theme,
};
use gpui::{AnyElement, Context, Entity, EventEmitter, Subscription, Window, div, prelude::*, px};
use serde_json::json;
use zeron_proto::{
    orchestration::Optional,
    orchestration_mcp::{LinkPullRequestInput, T3WorktreeHandoffInput},
    transfer::{CheckpointPreviewParams, CheckpointRestoreParams, RestorePreview},
};

#[derive(Clone)]
pub(crate) enum Kind {
    Link,
    Handoff,
    Restore(String),
}
pub(crate) struct Closed(pub Option<String>);

fn handoff_input(
    branch: String,
    base: String,
    origin: bool,
    run_setup: bool,
    prompt: String,
) -> T3WorktreeHandoffInput {
    T3WorktreeHandoffInput {
        branch,
        base_ref: if base.is_empty() {
            Optional::Absent
        } else {
            Optional::Present(base)
        },
        start_from_origin: Optional::Present(origin),
        path: Optional::Absent,
        run_setup_script: Optional::Present(run_setup),
        continuation_prompt: if prompt.trim().is_empty() {
            Optional::Absent
        } else {
            Optional::Present(prompt)
        },
    }
}

fn restore_request(chat: &str, preview: &RestorePreview) -> CheckpointRestoreParams {
    CheckpointRestoreParams {
        chat_id: chat.into(),
        checkpoint_id: preview.checkpoint_id.clone(),
        expected_head_sha: preview.head_sha.clone(),
        expected_checksum: preview.checksum.clone(),
    }
}

pub(crate) struct DetailsDialog {
    state: Entity<AppState>,
    chat: String,
    owner: String,
    kind: Kind,
    main: Entity<ComposerInput>,
    base: Entity<ComposerInput>,
    prompt: Entity<ComposerInput>,
    origin: bool,
    run_setup: bool,
    preview: Option<RestorePreview>,
    busy: bool,
    error: Option<String>,
    _inputs: Vec<Subscription>,
}
impl EventEmitter<Closed> for DetailsDialog {}

impl DetailsDialog {
    pub(crate) fn new(
        state: Entity<AppState>,
        chat: String,
        kind: Kind,
        cx: &mut Context<Self>,
    ) -> Self {
        motion::init_hover_owner(cx);
        let owner = state
            .read(cx)
            .chats
            .iter()
            .find(|c| c.id == chat)
            .map(|c| c.device_id.clone())
            .unwrap_or_default();
        let main = cx.new(|cx| {
            ComposerInput::new(
                if matches!(kind, Kind::Link) {
                    "Pull request URL"
                } else {
                    "Branch"
                },
                cx,
            )
            .with_single_line()
            .with_monospace()
            .with_text_metrics(11.0, 18.0)
        });
        let base = cx.new(|cx| {
            ComposerInput::new("Base ref (default if empty)", cx)
                .with_single_line()
                .with_monospace()
                .with_text_metrics(11.0, 18.0)
        });
        let prompt = cx.new(|cx| {
            ComposerInput::new("Continuation prompt", cx)
                .with_text_metrics(13.0, 18.0)
                .with_max_visible_lines(6)
        });
        let inputs = [&main, &base, &prompt]
            .into_iter()
            .map(|i| cx.subscribe(i, |_, _, _: &ComposerInputEvent, cx| cx.notify()))
            .collect();
        let mut view = Self {
            state,
            chat,
            owner,
            kind,
            main,
            base,
            prompt,
            origin: false,
            run_setup: true,
            preview: None,
            busy: false,
            error: None,
            _inputs: inputs,
        };
        if matches!(view.kind, Kind::Restore(_)) {
            view.load_preview(cx);
        }
        view
    }

    fn load_preview(&mut self, cx: &mut Context<Self>) {
        let Kind::Restore(checkpoint) = &self.kind else {
            return;
        };
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            self.error = Some("Engine unavailable.".into());
            return;
        };
        self.busy = true;
        let request = CheckpointPreviewParams {
            chat_id: self.chat.clone(),
            checkpoint_id: checkpoint.clone(),
        };
        let owner = self.owner.clone();
        cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .preview_file_checkpoint_restore(request, &owner)
                .await;
            this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(preview) => view.preview = Some(preview),
                    Err(error) => view.error = Some(error.to_string()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn enabled(&self, cx: &Context<Self>) -> bool {
        !self.busy
            && match &self.kind {
                Kind::Restore(_) => self
                    .preview
                    .as_ref()
                    .is_some_and(|p| p.allowed && p.refusal.is_none()),
                _ => !self.main.read(cx).text().trim().is_empty(),
            }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if !self.enabled(cx) {
            return;
        }
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            self.error = Some("Engine unavailable.".into());
            cx.notify();
            return;
        };
        let main = self.main.read(cx).text().trim().to_string();
        let base = self.base.read(cx).text().trim().to_string();
        let prompt = self.prompt.read(cx).text().to_string();
        let input = handoff_input(main.clone(), base, self.origin, self.run_setup, prompt);
        let target: LinkPullRequestInput =
            serde_json::from_value(json!({"url": main})).expect("URL target");
        let restore = self
            .preview
            .as_ref()
            .map(|p| restore_request(&self.chat, p));
        let (chat, owner, kind, state) = (
            self.chat.clone(),
            self.owner.clone(),
            self.kind.clone(),
            self.state.clone(),
        );
        self.busy = true;
        self.error = None;
        cx.spawn(async move |this, cx| {
            let result = match kind {
                Kind::Link => engine
                    .client()
                    .change_thread_pull_request(&chat, &owner, target, None)
                    .await
                    .map(|_| "Pull request linked.".to_string()),
                Kind::Handoff => engine
                    .client()
                    .handoff_thread_worktree(&chat, &owner, input)
                    .await
                    .map(|_| "Moved to worktree.".to_string()),
                Kind::Restore(_) => engine
                    .client()
                    .restore_file_checkpoint(restore.expect("confirmed preview"), &owner)
                    .await
                    .and_then(|r| {
                        if r.restored {
                            Ok(format!(
                                "Files restored. Backup checkpoint: {}",
                                r.backup_checkpoint_id
                            ))
                        } else {
                            Err(zeron_rpc::RpcError::Failed(
                                "Files were not restored.".into(),
                            ))
                        }
                    }),
            };
            // Refresh even if the dialog was closed while the mutation ran.
            state.update(cx, |state, cx| state.refresh_details(&chat, true, cx));
            this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(notice) => cx.emit(Closed(Some(notice))),
                    Err(error) => view.error = Some(error.to_string()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}

impl Render for DetailsDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let owner = cx.entity_id();
        let (title, primary) = match self.kind {
            Kind::Link => ("Link pull request", "Link pull request"),
            Kind::Handoff => ("Move to worktree", "Move to worktree"),
            Kind::Restore(_) => ("Restore checkpoint files?", "Restore files"),
        };
        let mut body = div().flex().flex_col().gap(px(12.0));
        if let Some(error) = &self.error {
            body = body.child(widgets::error_strip(&theme, error.clone()));
        }
        match &self.kind {
            Kind::Link => {
                body = body.child(dialog_field(&self.main, 1, 1, &theme, cx));
            }
            Kind::Handoff => {
                let weak = cx.entity().downgrade();
                let tabs =
                    div()
                        .flex()
                        .gap(px(4.0))
                        .children([(false, "Local"), (true, "Origin")].map(|(origin, label)| {
                            controls::button(
                                label,
                                owner,
                                &theme,
                                if self.origin == origin {
                                    Variant::Outline
                                } else {
                                    Variant::Ghost
                                },
                                Size::Sm,
                                label,
                            )
                            .when(self.busy, |el| el.opacity(controls::DISABLED_OPACITY))
                            .on_click(cx.listener(
                                move |view, _, _, cx| {
                                    if !view.busy {
                                        view.origin = origin;
                                        cx.notify();
                                    }
                                },
                            ))
                        }));
                body = body
                    .child(popover::dialog_body(&theme, "Branch"))
                    .child(dialog_field(&self.main, 1, 1, &theme, cx))
                    .child(popover::dialog_body(&theme, "Base ref"))
                    .child(tabs)
                    .child(dialog_field(&self.base, 1, 1, &theme, cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                checkbox(
                                    "handoff-setup",
                                    self.run_setup,
                                    self.busy,
                                    "Run setup",
                                    &theme,
                                )
                                .on_change(move |_, _, _, cx| {
                                    weak.update(cx, |view, cx| {
                                        if !view.busy {
                                            view.run_setup = !view.run_setup;
                                            cx.notify();
                                        }
                                    })
                                    .ok();
                                }),
                            )
                            .child("Run setup"),
                    )
                    .child(dialog_field(&self.prompt, 3, 6, &theme, cx));
            }
            Kind::Restore(_) => {
                body = body.child(popover::dialog_body(
                    &theme,
                    "Only files are restored. Conversation and branch HEAD are unchanged.",
                ));
                if let Some(preview) = &self.preview {
                    if let Some(refusal) = &preview.refusal {
                        body = body.child(widgets::error_strip(&theme, refusal.clone()));
                    }
                    body = body.child(
                        div()
                            .id("restore-paths")
                            .max_h(px(240.0))
                            .overflow_y_scroll()
                            .children(preview.paths.iter().map(|p| {
                                mono(format!("{}  {}", p.kind, p.path), &theme, theme.text_muted)
                            })),
                    );
                    if preview.paths.is_empty() {
                        body = body.child(popover::dialog_body(&theme, "No file paths change."));
                    }
                    if preview.restores_staging {
                        body = body.child(popover::dialog_body(
                            &theme,
                            "Staging is reset relative to the current HEAD.",
                        ));
                    }
                } else if self.busy {
                    body = body.child(popover::dialog_body(&theme, "Inspecting checkpoint…"));
                }
            }
        }
        let enabled = self.enabled(cx);
        popover::dialog_card(&theme)
            .w(px(if matches!(self.kind, Kind::Handoff) {
                480.0
            } else {
                420.0
            }))
            .max_h(window.viewport_size().height * 0.8)
            .text_size(crate::typography::ui_rems(13.0))
            .child(popover::dialog_title(&theme, title))
            .child(
                div()
                    .id("details-dialog-body")
                    .mt(px(16.0))
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(body),
            )
            .child(
                popover::dialog_footer(&theme)
                    .child(
                        controls::button(
                            "details-cancel",
                            owner,
                            &theme,
                            Variant::Ghost,
                            Size::Md,
                            "Cancel",
                        )
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(Closed(None)))),
                    )
                    .child(
                        controls::button(
                            "details-submit",
                            owner,
                            &theme,
                            if matches!(self.kind, Kind::Restore(_)) {
                                Variant::Destructive
                            } else {
                                Variant::Primary
                            },
                            Size::Md,
                            if self.busy { "Working…" } else { primary },
                        )
                        .when(!enabled, |el| el.opacity(controls::DISABLED_OPACITY))
                        .on_click(cx.listener(|view, _, _, cx| view.submit(cx))),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_omits_blank_base_and_preserves_user_options() {
        let wire = serde_json::to_value(handoff_input(
            "feature".into(),
            "".into(),
            true,
            false,
            "Continue here".into(),
        ))
        .unwrap();
        assert!(wire.get("baseRef").is_none());
        assert!(wire.get("path").is_none());
        assert_eq!(wire["startFromOrigin"], true);
        assert_eq!(wire["runSetupScript"], false);
        assert_eq!(wire["continuationPrompt"], "Continue here");
        let wire = serde_json::to_value(handoff_input(
            "feature".into(),
            "main".into(),
            false,
            true,
            "".into(),
        ))
        .unwrap();
        assert_eq!(wire["baseRef"], "main");
        assert!(wire.get("continuationPrompt").is_none());
    }

    #[test]
    fn restore_uses_exact_preview_guards_including_unborn_head() {
        for head in [None, Some("exact-observed-head".to_string())] {
            let p = RestorePreview {
                checkpoint_id: "checkpoint:one".into(),
                head_sha: head.clone(),
                checksum: "opaque-checksum".into(),
                allowed: true,
                ..Default::default()
            };
            let request = restore_request("chat", &p);
            assert_eq!(request.expected_head_sha, head);
            assert_eq!(request.expected_checksum, p.checksum);
            assert_eq!(request.checkpoint_id, p.checkpoint_id);
        }
    }
}
