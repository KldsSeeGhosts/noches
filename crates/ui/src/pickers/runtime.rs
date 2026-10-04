//! Minimal runtime selector; separate from the model picker restyling.
use super::*;
use zeron_proto::RuntimeMode;

impl Pickers {
    pub(super) fn runtime_policy_supported(&self, cx: &App) -> bool {
        let state = self.state.read(cx);
        Self::target_device_id(&self.target, state)
            .is_some_and(|device| state.runtime_policy_supported(&device))
    }

    pub(super) fn runtime_mode(&self, cx: &App) -> RuntimeMode {
        self.target
            .chat(self.state.read(cx))
            .and_then(|c| c.config.as_ref())
            .map(|c| c.runtime_mode)
            .unwrap_or(self.draft_runtime_mode)
    }

    pub(super) fn render_runtime_control(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::of(cx).clone();
        let mode = self.runtime_mode(cx);
        let mut chip = div()
            .id("runtime-mode-picker")
            .track_focus(&self.focus)
            .relative()
            .flex()
            .items_center()
            .gap(px(4.0))
            .h(px(28.0))
            .px(px(6.0))
            .rounded(px(6.0))
            .text_size(px(11.0))
            .text_color(theme.text_muted)
            .cursor_pointer()
            .hover(|s| s.bg(theme.wash(0.08)))
            .on_click(cx.listener(|this, _, window, cx| {
                let open = !this.runtime_menu_open;
                this.dismiss(cx);
                this.runtime_menu_open = open;
                if open {
                    window.focus(&this.focus, cx);
                } else {
                    cx.emit(ReturnComposerFocus);
                }
                cx.notify();
            }))
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if this.runtime_menu_open {
                    this.runtime_menu_open = false;
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.runtime_menu_open = false;
                    cx.emit(ReturnComposerFocus);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .child(
                crate::icons::icon(if mode == RuntimeMode::FullAccess {
                    crate::icons::UNLOCK
                } else {
                    crate::icons::LOCK
                })
                .size(px(12.0)),
            )
            .child(div().truncate().child(mode.label()));
        if self.runtime_menu_open {
            let menu = div()
                .flex()
                .flex_col()
                .w(px(190.0))
                .p(px(4.0))
                .bg(crate::popover::surface_bg(&theme))
                .border_1()
                .border_color(theme.border)
                .rounded(px(6.0))
                .children(
                    RuntimeMode::ALL
                        .into_iter()
                        .enumerate()
                        .map(|(ix, selected)| {
                            let supported = (selected == RuntimeMode::FullAccess
                                || self.runtime_policy_supported(cx))
                                && self.effective_harness(cx).is_none_or(|h| {
                                    zeron_harness::policy::compile(
                                        h,
                                        selected,
                                        self.resolved(cx).interaction_mode,
                                    )
                                    .is_ok()
                                });
                            div()
                                .id(("runtime-option", ix))
                                .h(px(30.0))
                                .px(px(8.0))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .text_size(px(12.0))
                                .text_color(theme.text_muted)
                                .when(selected == mode, |el| el.bg(theme.wash(0.08)))
                                .when(!supported, |el| el.opacity(0.4))
                                .when(supported, |el| {
                                    el.cursor_pointer()
                                        .hover(|s| s.bg(theme.wash(0.08)))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if this.target.chat_id(this.state.read(cx)).is_some() {
                                                this.update_chat_config(cx, |config| {
                                                    config.runtime_mode = selected
                                                });
                                            } else {
                                                this.draft_runtime_mode = selected;
                                            }
                                            this.runtime_menu_open = false;
                                            cx.emit(ReturnComposerFocus);
                                            cx.stop_propagation();
                                            cx.notify();
                                        }))
                                })
                                .child(
                                    crate::icons::icon(if selected == RuntimeMode::FullAccess {
                                        crate::icons::UNLOCK
                                    } else {
                                        crate::icons::LOCK
                                    })
                                    .size(px(12.0)),
                                )
                                .child(selected.label())
                        }),
                );
            chip = chip.child(crate::popover::anchored_menu_below_end(
                "runtime-mode-menu",
                menu.into_any_element(),
                None,
            ));
        }
        chip.into_any_element()
    }
}
