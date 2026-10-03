//! Inline titles for active thread cards and the compact archived shelf.
use super::*;

/// Which surface hosts the inline editor. The sidebar row and the pane header
/// never mount the same input entity at once.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RenameOrigin {
    Sidebar,
    Header,
}

pub(super) struct ChatRename {
    pub(super) chat_id: String,
    origin: RenameOrigin,
    pub(super) input: Entity<ComposerInput>,
    focus_pending: bool,
    reveal_until: std::time::Instant,
    _events: Subscription,
    _blur: Option<Subscription>,
}

pub(crate) fn chat_title_editor(
    id: SharedString,
    input: Entity<ComposerInput>,
    theme: &Theme,
) -> AnyElement {
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .flex_1()
        .min_w_0()
        .h(px(21.0))
        .px(px(4.0))
        .flex()
        .items_center()
        .rounded(px(4.0))
        .border_1()
        .border_color(theme.border)
        .bg(theme.input_glass_bg())
        .text_color(theme.text)
        .cursor_text()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(|_, _, cx| cx.stop_propagation())
        .child(div().flex_1().min_w_0().overflow_hidden().child(input))
        .into_any_element()
}

/// Pane-header title click waits this long before opening the thread menu.
const HEADER_TITLE_MENU_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

impl Shell {
    pub(super) fn open_rename_chat(&mut self, chat_id: String, cx: &mut Context<Self>) {
        self.begin_rename_chat(chat_id, RenameOrigin::Sidebar, cx);
    }

    /// Double-click on a pane header's title: rename in place, without needing
    /// the sidebar row to be on screen.
    pub(crate) fn open_rename_chat_in_header(&mut self, chat_id: String, cx: &mut Context<Self>) {
        self.header_title_menu_task = None;
        self.begin_rename_chat(chat_id, RenameOrigin::Header, cx);
    }

    /// Click on a pane header's title: the thread menu opens after a beat so a
    /// double-click (rename) can cancel it first.
    pub(crate) fn schedule_chat_title_menu(
        &mut self,
        chat_id: String,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.header_title_menu_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(HEADER_TITLE_MENU_DELAY)
                .await;
            this.update(cx, |shell, cx| {
                shell.header_title_menu_task = None;
                shell.open_chat_title_menu(chat_id, position, cx);
            })
            .ok();
        }));
    }

    /// Open the thread menu (the sidebar row's right-click menu) at `position`.
    pub(crate) fn open_chat_title_menu(
        &mut self,
        chat_id: String,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.header_title_menu_task = None;
        self.chat_menu.open(ChatMenuState {
            chat_id,
            position,
            page: ChatMenuPage::Root,
        });
        cx.notify();
    }

    fn begin_rename_chat(
        &mut self,
        chat_id: String,
        origin: RenameOrigin,
        cx: &mut Context<Self>,
    ) {
        self.close_chat_menu(cx);
        self.finish_rename_chat(true, cx);
        let Some(current) = self
            .state
            .read(cx)
            .chats
            .iter()
            .find(|chat| chat.id == chat_id)
            .map(|chat| chat.title.clone().unwrap_or_default())
        else {
            return;
        };
        if origin == RenameOrigin::Sidebar && !self.reveal_sidebar_chat(&chat_id, cx) {
            cx.notify();
            return;
        }
        let input = cx.new(|cx| {
            ComposerInput::new("Session title", cx)
                .with_single_line()
                .with_text_metrics(13.0, 17.0)
                .with_accessibility_role(gpui::Role::TextInput)
        });
        input.update(cx, |input, cx| {
            input.set_text(current, cx);
            input.select_all_text(cx);
        });
        let events = cx.subscribe(&input, |this: &mut Shell, _, event, cx| {
            if matches!(event, ComposerInputEvent::Submitted) {
                this.finish_rename_chat(true, cx);
                this.focus_composer(cx);
            }
        });
        // The double-click's first click can arm any pane's composer.
        for composer in std::iter::once(&self.composer).chain(
            self.workspace
                .chat_surfaces
                .values()
                .map(|surface| &surface.composer),
        ) {
            composer.update(cx, |composer, _| composer.focus_pending = false);
        }
        self.chat_rename = Some(ChatRename {
            chat_id,
            origin,
            input,
            focus_pending: true,
            reveal_until: std::time::Instant::now()
                + motion::COLLAPSE.total()
                + std::time::Duration::from_millis(150),
            _events: events,
            _blur: None,
        });
        cx.notify();
    }

    pub(super) fn focus_rename_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rename) = self.chat_rename.as_mut() else {
            return;
        };
        if std::mem::take(&mut rename.focus_pending) {
            let focus = rename.input.focus_handle(cx);
            window.focus(&focus, cx);
            rename._blur = Some(cx.on_blur(&focus, window, |this, _, cx| {
                // A click elsewhere owns its new focus; don't pull it back.
                this.finish_rename_chat(true, cx);
            }));
        }
    }

    pub(super) fn finish_rename_chat(&mut self, commit: bool, cx: &mut Context<Self>) {
        let Some(rename) = self.chat_rename.take() else {
            return;
        };
        let title = rename.input.read(cx).text().trim().to_string();
        let unchanged = self
            .state
            .read(cx)
            .chats
            .iter()
            .find(|chat| chat.id == rename.chat_id)
            .is_none_or(|chat| chat.title.as_deref() == Some(title.as_str()));
        if commit && !title.is_empty() && !unchanged {
            self.mutate(
                serde_json::json!({ "op": "renameChat", "chatId": rename.chat_id, "title": title }),
                cx,
            );
        }
        cx.notify();
    }

    pub(super) fn rename_input_for(&self, chat_id: &str) -> Option<Entity<ComposerInput>> {
        self.chat_rename
            .as_ref()
            .filter(|rename| rename.chat_id == chat_id && rename.origin == RenameOrigin::Sidebar)
            .map(|rename| rename.input.clone())
    }

    /// The chat being renamed from a pane header, with its editor.
    pub(super) fn header_rename(&self) -> Option<(String, Entity<ComposerInput>)> {
        self.chat_rename
            .as_ref()
            .filter(|rename| rename.origin == RenameOrigin::Header)
            .map(|rename| (rename.chat_id.clone(), rename.input.clone()))
    }

    /// The editor for a pane header showing `chat_id`, while a header-origin
    /// rename is open.
    pub(crate) fn header_rename_input_for(&self, chat_id: &str) -> Option<Entity<ComposerInput>> {
        self.chat_rename
            .as_ref()
            .filter(|rename| rename.chat_id == chat_id && rename.origin == RenameOrigin::Header)
            .map(|rename| rename.input.clone())
    }

    pub(super) fn rename_row_reveal(
        &self,
        chat_id: &str,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        self.chat_rename
            .as_ref()
            .filter(|rename| {
                rename.chat_id == chat_id
                    && rename.origin == RenameOrigin::Sidebar
                    && std::time::Instant::now() < rename.reveal_until
            })
            .map(|_| {
                let shell = cx.weak_entity();
                gpui::canvas(
                    |_, _, _| {},
                    move |bounds, _, window, cx| {
                        window.defer(cx, move |_, cx| {
                            shell
                                .update(cx, |this, cx| this.keep_rename_row_in_view(bounds, cx))
                                .ok();
                        });
                    },
                )
                .absolute()
                .inset_0()
                .into_any_element()
            })
    }

    fn keep_rename_row_in_view(&mut self, row: gpui::Bounds<Pixels>, cx: &mut Context<Self>) {
        if self
            .chat_rename
            .as_ref()
            .is_none_or(|rename| std::time::Instant::now() >= rename.reveal_until)
        {
            return;
        }
        let viewport = self.sidebar_scroll.bounds();
        let band = px(SIDEBAR_GLASS_FADE_BAND);
        let delta = if row.top() < viewport.top() + band {
            row.top() - (viewport.top() + band)
        } else if row.bottom() > viewport.bottom() - band {
            row.bottom() - (viewport.bottom() - band)
        } else {
            return;
        };
        let offset = self.sidebar_scroll.offset();
        let y = (offset.y - delta).clamp(-self.sidebar_scroll.max_offset().y, px(0.0));
        if y != offset.y {
            self.sidebar_scroll.set_offset(gpui::point(offset.x, y));
            cx.notify();
        }
    }
}
