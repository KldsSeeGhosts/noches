use super::*;
use crate::dictation::{Event, Phase};
use gpui::{KeyUpEvent, Keystroke, ModifiersChangedEvent};

#[derive(Clone)]
pub(super) enum DictationInputEvent {
    Press,
    Release,
    Changed,
    Submit(u64),
}
impl EventEmitter<DictationInputEvent> for ComposerInput {}

pub(super) struct DictationKey {
    keystroke: Option<Keystroke>,
    _blur: Subscription,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HoldSource {
    Pointer,
    Button,
    Key,
}
pub(super) struct DictationHold {
    source: HoldSource,
    started: Option<Instant>,
}

impl ComposerInput {
    pub(crate) fn cancel_dictation(&mut self) {
        self.dictation.cancel();
        self.transcriber = None;
        self.dictation_task = None;
    }

    pub(super) fn begin_dictation(
        &mut self,
        transcriber: Box<dyn crate::dictation::Transcriber>,
        cx: &mut Context<Self>,
    ) {
        self.cancel_dictation();
        self.dictation.begin(
            &self.content,
            self.projection.normalize_range(self.selected_range.clone()),
        );
        self.transcriber = Some(transcriber);
        self.last_edit = None;
        let generation = self.dictation.generation;
        let started = Instant::now();
        self.dictation_task = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(40))
                    .await;
                if !this
                    .update(cx, |input, cx| {
                        let requesting = input.dictation.phase == Phase::Requesting;
                        let active = input.poll_dictation(generation, cx);
                        if requesting && input.dictation.phase == Phase::Listening {
                            tracing::debug!(
                                target: "zeron_ui::dictation",
                                activation_to_listening_ms = started.elapsed().as_millis(),
                                "Dictation capture ready"
                            );
                        }
                        active
                    })
                    .unwrap_or(false)
                {
                    break;
                }
            }
        }));
        cx.emit(DictationInputEvent::Changed);
        cx.notify();
    }

    pub(crate) fn finish_dictation(&mut self, send: bool, cx: &mut Context<Self>) -> bool {
        if !self.dictation.phase.active() {
            return false;
        }
        if self.dictation.finish(send, Instant::now())
            && let Some(transcriber) = &mut self.transcriber
        {
            transcriber.finish();
        }
        cx.emit(DictationInputEvent::Changed);
        cx.notify();
        true
    }

    fn complete_dictation(&mut self, phase: Phase, cx: &mut Context<Self>) {
        let send = self.dictation.complete(phase);
        self.transcriber = None;
        self.last_edit = None;
        cx.emit(DictationInputEvent::Changed);
        if send {
            cx.emit(DictationInputEvent::Submit(self.dictation.generation));
        }
        cx.notify();
    }

    fn apply_dictation(&mut self, text: &str, cx: &mut Context<Self>) {
        let before =
            (!self.dictation.has_partial && !text.trim().is_empty()).then(|| self.snapshot());
        if let Some(cursor) = self.dictation.replace(&mut self.content, text) {
            if let Some(before) = before {
                self.undo_stack.push(before);
                if self.undo_stack.len() > UNDO_LIMIT {
                    self.undo_stack.remove(0);
                }
                self.redo_stack.clear();
            }
            self.selected_range = cursor..cursor;
            self.selection_reversed = false;
            self.refresh_projection();
            self.invalidate_mention_tooltip();
            self.follow_cursor = true;
            self.reset_blink();
            self.needs_measure = true;
            cx.emit(ComposerInputEvent::Edited);
            cx.notify();
        }
    }

    pub(super) fn poll_dictation(&mut self, generation: u64, cx: &mut Context<Self>) -> bool {
        if generation != self.dictation.generation || !self.dictation.phase.active() {
            return false;
        }
        while let Some(event) = self.transcriber.as_mut().and_then(|service| service.poll()) {
            match event {
                Event::Listening => {
                    if self.dictation.phase == Phase::Requesting {
                        self.dictation.phase = Phase::Listening;
                        self.dictation.meter.start(Instant::now());
                    }
                    cx.emit(DictationInputEvent::Changed);
                    cx.notify();
                }
                Event::Finalizing => {
                    self.finish_dictation(false, cx);
                }
                Event::Partial(text) => self.apply_dictation(&text, cx),
                Event::Final(text) => {
                    if text.trim().is_empty() && !self.dictation.has_partial {
                        self.complete_dictation(Phase::NoSpeech, cx);
                        return false;
                    }
                    self.apply_dictation(&text, cx);
                    self.complete_dictation(Phase::Idle, cx);
                    return false;
                }
                Event::Denied(message) => self.complete_dictation(Phase::Denied(message), cx),
                Event::Unavailable(message) => {
                    self.complete_dictation(Phase::Unavailable(message), cx)
                }
                Event::Failed(message) => self.complete_dictation(Phase::Failed(message), cx),
                Event::Cancelled => {
                    self.cancel_dictation();
                    cx.emit(DictationInputEvent::Changed);
                    cx.notify();
                }
            }
            if !self.dictation.phase.active() {
                self.transcriber = None;
                return false;
            }
        }
        if self.dictation.phase == Phase::Listening
            && let Some(service) = self.transcriber.as_mut()
        {
            self.dictation.meter.record(Instant::now(), service.level());
            cx.emit(DictationInputEvent::Changed);
            cx.notify();
        }
        if self.dictation.timed_out(Instant::now()) {
            self.complete_dictation(
                Phase::Failed("Dictation took too long. Your draft is safe. Wait a moment, then try a shorter recording.".into()),
                cx,
            );
            return false;
        }
        true
    }

    pub(super) fn toggle_dictation_action(
        &mut self,
        _: &ToggleDictation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.dictation_key.is_some() {
            return;
        }
        if !crate::dictation::enabled(cx) && !self.dictation.phase.active() {
            cx.propagate();
            return;
        }
        let parse = |combo: &str| Keystroke::parse(&crate::settings::platform_combo(combo)).ok();
        let keystroke = parse(&crate::settings::current(cx).keymap.toggle_dictation)
            .or_else(|| parse(crate::settings::ShortcutId::ToggleDictation.default_combo()));
        let blur = cx.on_blur(&self.focus_handle, window, |input, _, cx| {
            input.release_dictation_key(cx);
        });
        self.dictation_key = Some(DictationKey {
            keystroke,
            _blur: blur,
        });
        cx.emit(DictationInputEvent::Press);
    }

    fn release_dictation_key(&mut self, cx: &mut Context<Self>) {
        if self.dictation_key.take().is_some() {
            cx.emit(DictationInputEvent::Release);
        }
    }

    pub(super) fn on_dictation_key_up(
        &mut self,
        event: &KeyUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.dictation_key.as_ref().is_some_and(|held| {
            held.keystroke
                .as_ref()
                .is_none_or(|binding| binding.key.eq_ignore_ascii_case(&event.keystroke.key))
        }) {
            self.release_dictation_key(cx);
        }
    }

    pub(super) fn on_dictation_modifiers(
        &mut self,
        event: &ModifiersChangedEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .dictation_key
            .as_ref()
            .and_then(|held| held.keystroke.as_ref())
            .is_some_and(|binding| {
                let (bound, now) = (&binding.modifiers, &event.modifiers);
                (bound.platform && !now.platform)
                    || (bound.control && !now.control)
                    || (bound.alt && !now.alt)
                    || (bound.shift && !now.shift)
            })
        {
            self.release_dictation_key(cx);
        }
    }
}

impl Composer {
    pub(super) fn observe_dictation_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dictation_blur.is_none() {
            self.dictation_blur = Some(cx.on_focus_out(
                &self.dictation_focus,
                window,
                |this, _, _, cx| this.cancel_unfocused_dictation(cx),
            ));
        }
        if self.dictation_activation.is_none() {
            self.dictation_activation =
                Some(cx.observe_window_activation(window, |this, window, cx| {
                    if !window.is_window_active() {
                        this.cancel_unfocused_dictation(cx);
                    }
                }));
        }
    }

    fn cancel_unfocused_dictation(&mut self, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            if input.dictation.phase.active()
                && !(input.dictation.phase == Phase::Requesting
                    && crate::dictation::permission_pending())
            {
                input.cancel_dictation();
                cx.emit(DictationInputEvent::Changed);
                cx.notify();
            }
        });
        cx.notify();
    }

    pub(super) fn toggle_dictation(&mut self, refocus: bool, cx: &mut Context<Self>) {
        if !crate::dictation::enabled(cx) && !self.input.read(cx).dictation.phase.active() {
            return;
        }
        if self.wizard.is_some()
            || self.queue_edit_finishing
            || self.queue_edit_pending_id.is_some()
            || self.sending
            || self.input.read(cx).read_only
        {
            return;
        }
        self.input
            .update(cx, |input, cx| match input.dictation.phase {
                Phase::Requesting => {
                    input.cancel_dictation();
                    cx.emit(DictationInputEvent::Changed);
                    cx.notify();
                }
                Phase::Listening => {
                    input.finish_dictation(false, cx);
                }
                Phase::Finalizing => {}
                _ if input.marked_range.is_some() => {
                    input.dictation.phase = Phase::Unavailable(
                        "Finish composing the current character before starting dictation.".into(),
                    );
                    cx.emit(DictationInputEvent::Changed);
                }
                _ => {
                    if let Some(service) = crate::dictation::start(cx) {
                        input.begin_dictation(service, cx);
                    }
                }
            });
        self.focus_pending |= refocus;
        cx.notify();
    }

    pub(super) fn press_dictation(&mut self, source: HoldSource, cx: &mut Context<Self>) {
        let phase = self.input.read(cx).dictation.phase.clone();
        if phase.active()
            && self
                .dictation_hold
                .as_ref()
                .is_some_and(|h| h.source == source)
        {
            return;
        }
        self.dictation_hold = None;
        match phase {
            Phase::Finalizing => {}
            Phase::Requesting | Phase::Listening => {
                self.dictation_hold = Some(DictationHold {
                    source,
                    started: None,
                });
            }
            _ => {
                self.toggle_dictation(source != HoldSource::Button, cx);
                if self.input.read(cx).dictation.phase.active() {
                    self.dictation_hold = Some(DictationHold {
                        source,
                        started: Some(Instant::now()),
                    });
                }
            }
        }
    }

    pub(super) fn release_dictation(&mut self, source: HoldSource, cx: &mut Context<Self>) {
        if self
            .dictation_hold
            .as_ref()
            .is_none_or(|hold| hold.source != source)
        {
            return;
        }
        let tapped = self
            .dictation_hold
            .take()
            .and_then(|hold| hold.started)
            .is_some_and(|at| at.elapsed() < Duration::from_millis(300));
        self.input
            .update(cx, |input, cx| match input.dictation.phase {
                Phase::Requesting if crate::dictation::permission_pending() => {
                    input.cancel_dictation();
                    cx.emit(DictationInputEvent::Changed);
                    cx.notify();
                }
                Phase::Requesting | Phase::Listening if tapped => {
                    input.cancel_dictation();
                    input.dictation.phase = Phase::Tapped;
                    cx.emit(DictationInputEvent::Changed);
                    cx.notify();
                }
                Phase::Requesting | Phase::Listening => {
                    input.finish_dictation(false, cx);
                }
                _ => {}
            });
        self.focus_pending |= source == HoldSource::Pointer;
        cx.notify();
    }

    pub(super) fn dismiss_dictation(&mut self, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            input.cancel_dictation();
            cx.emit(DictationInputEvent::Changed);
            cx.notify();
        });
        cx.notify();
    }

    pub(super) fn render_dictation_button(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        if !crate::dictation::enabled(cx) || self.wizard.is_some() {
            return None;
        }
        let theme = Theme::of(cx);
        let phase = &self.input.read(cx).dictation.phase;
        let active = phase.active();
        let label = phase.action_label();
        let composer = cx.entity().downgrade();
        let hover_key = composer_hover_key("dictation", cx.entity_id());
        Some(
            div()
                .id("composer-dictation")
                .debug_selector(|| "composer-dictation".into())
                .size(px(28.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .role(Role::Button)
                .aria_label(label)
                .tab_index(0)
                .aria_toggled(if active {
                    gpui::Toggled::True
                } else {
                    gpui::Toggled::False
                })
                .track_focus(&self.microphone_focus)
                .cursor_pointer()
                .bg(motion::hover_blend(
                    &hover_key,
                    gpui::transparent_black(),
                    crate::theme::ink(0.10),
                ))
                .on_hover(motion::hover_listener(hover_key))
                .focus_visible(|s| s.border_1().border_color(theme.accent))
                .tooltip(crate::settings::widgets::text_tooltip(label))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| {
                        window.focus(&this.microphone_focus, cx);
                        this.press_dictation(HoldSource::Pointer, cx);
                        cx.stop_propagation();
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.release_dictation(HoldSource::Pointer, cx)),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.release_dictation(HoldSource::Pointer, cx)),
                )
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        cx.stop_propagation();
                        if !event.is_held {
                            this.press_dictation(HoldSource::Button, cx);
                        }
                    }
                }))
                .on_key_up(cx.listener(|this, event: &KeyUpEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        cx.stop_propagation();
                        this.release_dictation(HoldSource::Button, cx);
                    }
                }))
                .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, cx| {
                    composer
                        .update(cx, |this, cx| this.toggle_dictation(true, cx))
                        .ok();
                })
                .child(
                    crate::icons::icon(crate::icons::MICROPHONE)
                        .size(px(16.0))
                        .text_color(if active { theme.text } else { theme.text_muted }),
                )
                .into_any_element(),
        )
    }

    pub(super) fn render_dictation_status(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let state = &self.input.read(cx).dictation;
        let (title, detail) = state.phase.status()?;
        let theme = Theme::of(cx);
        let now = Instant::now();
        let active = state.phase.active();
        let composer = cx.entity().downgrade();
        let mode = match state.phase {
            Phase::Listening => crate::dictation::waveform::Mode::Live,
            Phase::Finalizing => crate::dictation::waveform::Mode::Processing,
            _ => crate::dictation::waveform::Mode::Waiting,
        };
        Some(
            div()
                .id("dictation-status")
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .text_size(px(11.0))
                .text_color(theme.text_muted)
                .child(
                    div()
                        .id("dictation-live-status")
                        .role(Role::Status)
                        .aria_label(format!("{title}. {detail}"))
                        .child(title.to_owned()),
                )
                .when(active, |el| {
                    el.child(
                        div().flex_1().min_w_0().h(px(16.0)).flex().child(
                            crate::dictation::waveform::waveform(
                                state.meter.bars(now, false),
                                mode,
                                crate::dictation::waveform::Paint {
                                    ink: theme
                                        .text_muted
                                        .opacity(0.72 + 0.28 * state.meter.level(now)),
                                    quiet: theme.text_faint,
                                },
                                crate::dictation::waveform::Phase {
                                    intro: 1.0,
                                    collapse: 0.0,
                                },
                                false,
                            ),
                        ),
                    )
                })
                .when(!active, |el| {
                    el.child(div().flex_1().child(detail.to_owned()))
                })
                .children(state.meter.since_start(now).map(|_| {
                    div()
                        .font_family(theme.font_mono.clone())
                        .child(crate::dictation::waveform::clock(state.meter.elapsed(now)))
                }))
                .child(
                    div()
                        .id("dictation-cancel")
                        .role(Role::Button)
                        .tab_index(0)
                        .aria_label(if active {
                            "Cancel dictation"
                        } else {
                            "Dismiss dictation message"
                        })
                        .cursor_pointer()
                        .focus_visible(|s| s.border_1().border_color(theme.accent))
                        .child(if active { "Cancel" } else { "Dismiss" })
                        .on_click(cx.listener(|this, _, _, cx| this.dismiss_dictation(cx)))
                        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.dismiss_dictation(cx);
                                cx.stop_propagation();
                            }
                        }))
                        .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, cx| {
                            composer
                                .update(cx, |this, cx| this.dismiss_dictation(cx))
                                .ok();
                        }),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictation::Transcriber;
    use gpui::{AppContext, TestAppContext};
    use std::{cell::RefCell, collections::VecDeque};

    #[derive(Default)]
    struct FakeState {
        events: VecDeque<Event>,
        finishes: usize,
        drops: usize,
    }
    struct Fake(Rc<RefCell<FakeState>>);
    impl Transcriber for Fake {
        fn poll(&mut self) -> Option<Event> {
            self.0.borrow_mut().events.pop_front()
        }
        fn finish(&mut self) {
            self.0.borrow_mut().finishes += 1;
        }
    }
    impl Drop for Fake {
        fn drop(&mut self) {
            self.0.borrow_mut().drops += 1;
        }
    }
    fn start(input: &mut ComposerInput, cx: &mut Context<ComposerInput>) -> Rc<RefCell<FakeState>> {
        let fake = Rc::new(RefCell::new(FakeState::default()));
        input.begin_dictation(Box::new(Fake(fake.clone())), cx);
        fake
    }
    fn deliver(
        input: &mut ComposerInput,
        fake: &Rc<RefCell<FakeState>>,
        events: impl IntoIterator<Item = Event>,
        cx: &mut Context<ComposerInput>,
    ) {
        fake.borrow_mut().events.extend(events);
        input.poll_dictation(input.dictation.generation, cx);
    }

    #[gpui::test]
    fn unicode_selection_and_partials_are_one_undo_step(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::dark()));
        let window = cx.add_window(|_, cx| ComposerInput::new("Draft", cx));
        window
            .update(cx, |input, window, cx| {
                input.set_text("👩🏽‍💻 replace café", cx);
                let begin = input.content.find("replace").unwrap();
                input.selected_range = begin..begin + 7;
                let fake = start(input, cx);
                deliver(
                    input,
                    &fake,
                    [
                        Event::Listening,
                        Event::Partial("你好".into()),
                        Event::Partial("bonjour 🌍".into()),
                    ],
                    cx,
                );
                input.finish_dictation(false, cx);
                deliver(input, &fake, [Event::Final("salut 🌍".into())], cx);
                assert_eq!(input.content, "👩🏽‍💻 salut 🌍 café");
                assert_eq!(fake.borrow().drops, 1);
                assert_eq!(input.undo_stack.len(), 1);
                input.undo(&Undo, window, cx);
                assert_eq!(input.content, "👩🏽‍💻 replace café");
                input.redo(&Redo, window, cx);
                assert_eq!(input.content, "👩🏽‍💻 salut 🌍 café");
            })
            .unwrap();
    }

    #[gpui::test]
    fn manual_edit_and_ime_cancel_before_overwriting_speech(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::dark()));
        let window = cx.add_window(|_, cx| ComposerInput::new("Draft", cx));
        window
            .update(cx, |input, window, cx| {
                let fake = start(input, cx);
                let generation = input.dictation.generation;
                deliver(input, &fake, [Event::Partial("Hello ".into())], cx);
                input.replace_and_mark_text_in_range(None, "に", None, window, cx);
                input.replace_and_mark_text_in_range(None, "日本", None, window, cx);
                input.replace_text_in_range(None, "日本語", window, cx);
                assert_eq!(input.content, "Hello 日本語");
                assert_eq!(fake.borrow().drops, 1);
                assert!(!input.poll_dictation(generation, cx));
                input.undo(&Undo, window, cx);
                assert_eq!(input.content, "Hello ");
                input.undo(&Undo, window, cx);
                assert_eq!(input.content, "");
            })
            .unwrap();
    }

    #[gpui::test]
    fn selection_and_empty_undo_redo_cancel(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::dark()));
        let window = cx.add_window(|_, cx| ComposerInput::new("Draft", cx));
        window
            .update(cx, |input, window, cx| {
                for redo in [false, true] {
                    let fake = start(input, cx);
                    if redo {
                        input.redo(&Redo, window, cx);
                    } else {
                        input.undo(&Undo, window, cx);
                    }
                    assert_eq!(fake.borrow().drops, 1);
                    assert!(!input.dictation.phase.active());
                }
                input.set_text("draft", cx);
                let fake = start(input, cx);
                deliver(input, &fake, [Event::Partial(" words".into())], cx);
                input.select_to(0, cx);
                assert_eq!(fake.borrow().drops, 1);
                assert_eq!(input.content, "draft words");
            })
            .unwrap();
    }

    #[gpui::test]
    fn repeated_send_finalizes_once_and_blank_final_keeps_partial(cx: &mut TestAppContext) {
        let input = cx.new(|cx| ComposerInput::new("Draft", cx));
        let mut events = cx.events::<DictationInputEvent, _>(&input);
        input.update(cx, |input, cx| {
            let fake = start(input, cx);
            deliver(
                input,
                &fake,
                [Event::Listening, Event::Partial("keep words".into())],
                cx,
            );
            assert!(input.finish_dictation(true, cx));
            assert!(input.finish_dictation(true, cx));
            deliver(
                input,
                &fake,
                [
                    Event::Listening,
                    Event::Finalizing,
                    Event::Final(" \n\t ".into()),
                    Event::Final("duplicate".into()),
                ],
                cx,
            );
            assert_eq!(input.content, "keep words");
            assert_eq!(input.dictation.phase, Phase::Idle);
            assert_eq!(fake.borrow().finishes, 1);
            assert_eq!(fake.borrow().drops, 1);
        });
        let mut sends = 0;
        while let Ok(event) = events.try_recv() {
            sends += usize::from(matches!(event, DictationInputEvent::Submit(_)));
        }
        assert_eq!(sends, 1);
    }

    #[gpui::test]
    fn no_speech_preserves_selected_draft_without_sending(cx: &mut TestAppContext) {
        let input = cx.new(|cx| ComposerInput::new("Draft", cx));
        let mut events = cx.events::<DictationInputEvent, _>(&input);
        input.update(cx, |input, cx| {
            input.set_text("keep my draft", cx);
            input.selected_range = 0..13;
            let fake = start(input, cx);
            input.finish_dictation(true, cx);
            deliver(input, &fake, [Event::Final(String::new())], cx);
            assert_eq!(input.content, "keep my draft");
            assert_eq!(input.selected_range, 0..13);
            assert_eq!(input.dictation.phase, Phase::NoSpeech);
            assert!(input.undo_stack.is_empty());
        });
        while let Ok(event) = events.try_recv() {
            assert!(!matches!(event, DictationInputEvent::Submit(_)));
        }
    }

    #[gpui::test]
    fn failure_timeout_and_draft_swap_reject_late_results(cx: &mut TestAppContext) {
        let input = cx.new(|cx| ComposerInput::new("Draft", cx));
        let mut events = cx.events::<DictationInputEvent, _>(&input);
        input.update(cx, |input, cx| {
            for mode in 0..3 {
                input.set_text("keep my draft", cx);
                input.selected_range = 0..13;
                let fake = start(input, cx);
                let generation = input.dictation.generation;
                input.finish_dictation(true, cx);
                match mode {
                    0 => deliver(input, &fake, [Event::Failed("load failed".into())], cx),
                    1 => {
                        input.cancel_dictation();
                        let fake = start(input, cx);
                        input
                            .dictation
                            .finish(true, Instant::now() - crate::dictation::FINALIZE_TIMEOUT);
                        input.poll_dictation(input.dictation.generation, cx);
                        assert_eq!(fake.borrow().drops, 1);
                    }
                    _ => input.set_text("different pane draft", cx),
                }
                fake.borrow_mut()
                    .events
                    .push_back(Event::Final("late".into()));
                assert!(!input.poll_dictation(generation, cx));
                assert_eq!(
                    input.content,
                    if mode == 2 {
                        "different pane draft"
                    } else {
                        "keep my draft"
                    }
                );
                assert_eq!(fake.borrow().drops, 1);
            }
        });
        while let Ok(event) = events.try_recv() {
            assert!(!matches!(event, DictationInputEvent::Submit(_)));
        }
    }

    #[gpui::test]
    fn queue_save_waits_and_cancel_releases_capture(cx: &mut TestAppContext) {
        let state = cx.new(|_| AppState::new());
        let composer = cx.new(|cx| Composer::new(state, cx));
        composer.update(cx, |composer, cx| {
            composer.editing_queued = Some("row".into());
            let fake = composer.input.update(cx, start);
            assert!(composer.commit_queue_edit(cx));
            assert!(composer.commit_queue_edit(cx));
            assert_eq!(fake.borrow().finishes, 1);
            assert!(composer.failure.is_none());
            composer.cancel_queue_edit(cx);
            assert_eq!(fake.borrow().drops, 1);
            let fake = composer.input.update(cx, start);
            composer.clear_queue_edit(cx);
            assert_eq!(fake.borrow().drops, 1);
        });
    }

    #[gpui::test]
    fn pending_submission_cannot_send_a_replacement_draft(cx: &mut TestAppContext) {
        let state = cx.new(|_| AppState::new());
        let composer = cx.new(|cx| Composer::new(state, cx));
        composer.update(cx, |composer, cx| {
            composer.input.update(cx, |input, cx| {
                let fake = start(input, cx);
                input.finish_dictation(true, cx);
                deliver(input, &fake, [Event::Final("old".into())], cx);
                input.set_text("new", cx);
            });
        });
        composer.read_with(cx, |composer, cx| {
            assert!(composer.failure.is_none());
            assert!(!composer.sending);
            assert_eq!(composer.input.read(cx).text(), "new");
        });
    }

    #[gpui::test]
    fn each_pane_owns_its_dictation_draft(cx: &mut TestAppContext) {
        let state = cx.new(|_| AppState::new());
        let left = cx.new(|cx| Composer::for_pane(state.clone(), Some("left".into()), cx));
        let right = cx.new(|cx| Composer::for_pane(state.clone(), Some("right".into()), cx));
        right.update(cx, |c, cx| {
            c.input.update(cx, |i, cx| i.set_text("right draft", cx))
        });
        left.update(cx, |c, cx| {
            c.input.update(cx, |i, cx| {
                let fake = start(i, cx);
                deliver(i, &fake, [Event::Final("left speech".into())], cx);
            })
        });
        left.read_with(cx, |c, cx| {
            assert_eq!(c.input.read(cx).text(), "left speech")
        });
        right.read_with(cx, |c, cx| {
            assert_eq!(c.input.read(cx).text(), "right draft")
        });
    }

    #[gpui::test]
    fn tap_cancels_and_another_source_can_finish_a_lost_hold(cx: &mut TestAppContext) {
        let state = cx.new(|_| AppState::new());
        let composer = cx.new(|cx| Composer::new(state, cx));
        composer.update(cx, |composer, cx| {
            let fake = composer.input.update(cx, start);
            composer.dictation_hold = Some(DictationHold {
                source: HoldSource::Pointer,
                started: Some(Instant::now()),
            });
            composer.release_dictation(HoldSource::Button, cx);
            assert_eq!(fake.borrow().drops, 0);
            composer.release_dictation(HoldSource::Pointer, cx);
            assert_eq!(fake.borrow().drops, 1);
            assert_eq!(fake.borrow().finishes, 0);
            assert_eq!(composer.input.read(cx).dictation.phase, Phase::Tapped);

            let fake = composer.input.update(cx, |input, cx| {
                let fake = start(input, cx);
                deliver(input, &fake, [Event::Listening], cx);
                fake
            });
            composer.dictation_hold = Some(DictationHold {
                source: HoldSource::Button,
                started: None,
            });
            composer.press_dictation(HoldSource::Pointer, cx);
            composer.release_dictation(HoldSource::Pointer, cx);
            assert_eq!(fake.borrow().finishes, 1);
            assert_eq!(fake.borrow().drops, 0);
        });
    }

    #[gpui::test]
    fn pointer_send_waits_for_final_result(cx: &mut TestAppContext) {
        let (_dir, handle) = super::super::tests::composer_focus_window(cx);
        let (input, fake) = handle
            .update(cx, |composer, _, cx| {
                let fake = composer.input.update(cx, |input, cx| {
                    let fake = start(input, cx);
                    deliver(
                        input,
                        &fake,
                        [Event::Listening, Event::Partial("unfinished".into())],
                        cx,
                    );
                    fake
                });
                (composer.input.clone(), fake)
            })
            .unwrap();
        let mut events = cx.events::<DictationInputEvent, _>(&input);
        cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear())
            .unwrap();
        let mut visual = gpui::VisualTestContext::from_window(handle.into(), cx);
        let button = visual.debug_bounds("composer-send").unwrap();
        visual.simulate_click(button.center(), gpui::Modifiers::default());
        visual.simulate_click(button.center(), gpui::Modifiers::default());
        assert_eq!(fake.borrow().finishes, 1);
        assert_eq!(fake.borrow().drops, 0);
        input.update(cx, |input, cx| {
            assert_eq!(input.dictation.phase, Phase::Finalizing);
            assert_eq!(input.content, "unfinished");
            deliver(input, &fake, [Event::Final("finished".into())], cx);
        });
        let mut sends = 0;
        while let Ok(event) = events.try_recv() {
            sends += usize::from(matches!(event, DictationInputEvent::Submit(_)));
        }
        assert_eq!(sends, 1);
    }

    #[gpui::test]
    fn focus_within_composer_keeps_capture_and_leaving_cancels(cx: &mut TestAppContext) {
        let (_dir, handle) = super::super::tests::composer_focus_window(cx);
        let fake = handle
            .update(cx, |composer, window, cx| {
                window.activate_window();
                composer.input.update(cx, |input, cx| {
                    let fake = start(input, cx);
                    deliver(
                        input,
                        &fake,
                        [Event::Listening, Event::Partial("keep draft".into())],
                        cx,
                    );
                    fake
                })
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear())
            .unwrap();
        handle
            .update(cx, |composer, window, cx| {
                window.focus(&composer.dictation_focus, cx);
            })
            .unwrap();
        cx.run_until_parked();
        assert_eq!(fake.borrow().drops, 0);
        let elsewhere = cx.update(|cx| cx.focus_handle());
        handle
            .update(cx, |_, window, cx| window.focus(&elsewhere, cx))
            .unwrap();
        cx.run_until_parked();
        assert_eq!(fake.borrow().drops, 1);
        handle
            .read_with(cx, |composer, cx| {
                assert_eq!(composer.input.read(cx).text(), "keep draft");
            })
            .unwrap();
    }

    #[gpui::test]
    fn escape_from_controls_cancels_pending_send(cx: &mut TestAppContext) {
        let (_dir, handle) = super::super::tests::composer_focus_window(cx);
        let fake = handle
            .update(cx, |composer, window, cx| {
                let fake = composer.input.update(cx, |input, cx| {
                    input.set_text("keep draft", cx);
                    let fake = start(input, cx);
                    deliver(input, &fake, [Event::Listening], cx);
                    input.finish_dictation(true, cx);
                    fake
                });
                window.focus(&composer.dictation_focus, cx);
                fake
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear())
            .unwrap();
        cx.simulate_keystrokes(handle.into(), "escape");
        cx.run_until_parked();
        assert_eq!(fake.borrow().drops, 1);
        handle
            .read_with(cx, |composer, cx| {
                assert_eq!(composer.input.read(cx).dictation.phase, Phase::Idle);
                assert_eq!(composer.input.read(cx).text(), "keep draft");
                assert!(composer.failure.is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    fn shortcut_propagates_when_dictation_is_disabled(cx: &mut TestAppContext) {
        gpui::actions!(dictation_test, [Probe]);
        let (_dir, handle) = super::super::tests::composer_focus_window(cx);
        let fired = Rc::new(std::cell::Cell::new(false));
        let probe = fired.clone();
        cx.update(|cx| {
            crate::shell::apply_keymap(
                cx,
                &crate::settings::KeymapConfig::default(),
                ComposerSendBehavior::Enter,
            );
            let combo = crate::settings::platform_combo(
                crate::settings::ShortcutId::ToggleDictation.default_combo(),
            );
            cx.bind_keys([KeyBinding::new(&combo, Probe, None)]);
            cx.on_action(move |_: &Probe, _| probe.set(true));
        });
        let combo = crate::settings::platform_combo(
            crate::settings::ShortcutId::ToggleDictation.default_combo(),
        );
        cx.simulate_keystrokes(handle.into(), &combo);
        assert!(fired.get());
        handle
            .read_with(cx, |composer, cx| {
                assert!(composer.dictation_hold.is_none());
                assert!(composer.input.read(cx).dictation_key.is_none());
            })
            .unwrap();
    }

    #[test]
    fn default_dictation_binding_does_not_take_split_pane_shortcut() {
        let keys = crate::settings::KeymapConfig::default();
        assert_ne!(keys.toggle_dictation, keys.split_pane_right);
        assert!(crate::settings::conflicted_shortcuts(&keys).is_empty());
    }

    #[gpui::test]
    fn microphone_hover_fades_are_scoped_to_each_composer(cx: &mut TestAppContext) {
        let state = cx.new(|_| AppState::new());
        let selected = cx.new(|cx| Composer::new(state.clone(), cx));
        let pane = cx.new(|cx| Composer::for_pane(state.clone(), None, cx));
        let selected_key = composer_hover_key("dictation", selected.entity_id());
        let pane_key = composer_hover_key("dictation", pane.entity_id());
        assert_ne!(selected_key, pane_key);
        motion::set_hover(&selected_key, true, true);
        assert_eq!(motion::hover_t(&selected_key), 1.0);
        assert_eq!(motion::hover_t(&pane_key), 0.0);
        motion::set_hover(&pane_key, true, true);
        motion::set_hover(&selected_key, false, true);
        assert_eq!(motion::hover_t(&selected_key), 0.0);
        assert_eq!(motion::hover_t(&pane_key), 1.0);
        motion::set_hover(&pane_key, false, true);
    }
}
