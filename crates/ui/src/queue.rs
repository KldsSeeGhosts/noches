//! The pending-message queue, docked above the composer.
//!
//! Typed intents and canonical agent/automation work, in delivery order. Typed
//! rows live on the session doc; SQL-only rows are a passive projection, never
//! synthetic intents. Every mutation is routed to its actual authority.
//!
//! Typed rows retain `Send now` and their host edit leases. Canonical work can
//! steer a compatible active attempt or explicitly interrupt/restart it;
//! edits preserve context, can replace uploaded attachments,
//! and retain the draft if another editor or automatic drain wins the race.

use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled, Window, div, prelude::*, px,
};

use zeron_doc::{QueueDeliveryGate, QueuedMessage};
use zeron_rpc::methods;

use crate::composer::{Composer, QUEUE_COMPOSER_OVERLAP};
use crate::icons::{self, icon};
use crate::motion::{self, AnimationExt as _, TAB_SLIDE};
use crate::settings::shortcuts::modifier_send_label;
use crate::state::{AppState, ChatTarget};
use crate::terminal::panel::{drop_index, slide_offset};
use crate::theme::Theme;

/// Queue rows are replicated CRDT state, so ordinary mutations deliberately
/// land on the local engine. Delivery and cancellation must execute on the
/// chat's owning device because they race over the same row.
fn queue_action_needs_host(method: &str) -> bool {
    matches!(
        method,
        methods::SEND_QUEUED_MESSAGE_NOW
            | methods::STEER_QUEUED_MESSAGE_NOW
            | methods::REMOVE_QUEUED_MESSAGE
            | methods::BEGIN_QUEUED_MESSAGE_EDIT
            | methods::RENEW_QUEUED_MESSAGE_EDIT
            | methods::FINISH_QUEUED_MESSAGE_EDIT
    )
}

/// Queue mutation replies are deliberately explicit. A false or malformed
/// acknowledgement means an optimistic local edit may not match the document
/// (for example, another device removed the same row first).
fn queue_mutation_acknowledged(method: &str, reply: &serde_json::Value) -> bool {
    let field = match method {
        methods::UPDATE_QUEUED_MESSAGE | methods::MOVE_QUEUED_MESSAGE => "changed",
        methods::REMOVE_QUEUED_MESSAGE => "removed",
        methods::SEND_QUEUED_MESSAGE_NOW | methods::STEER_QUEUED_MESSAGE_NOW => "sent",
        _ => return true,
    };
    reply.get(field).and_then(serde_json::Value::as_bool) == Some(true)
}

/// Compact, borderless rows inside the queue's single glass surface.
const ROW_HEIGHT: f32 = 36.0;
const QUEUE_TEXT_SIZE: f32 = 12.5;
const ROW_GAP: f32 = 0.0;
const ROW_SLOT: f32 = ROW_HEIGHT + ROW_GAP;
const ROW_PAD_X: f32 = 8.0;
const ROW_RADIUS: f32 = 8.0;
pub(crate) const PANEL_RADIUS: f32 = 16.0;
const PANEL_PAD_TOP: f32 = 0.0;
/// The custom 24px queue glyphs have quieter geometry than the legacy set, so
/// render them slightly larger to preserve the previous optical weight.
const QUEUE_ICON_SIZE: f32 = 13.0;

/// The single trailing action a queue row advertises and executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueuePrimaryAction {
    SendNow,
    Steer,
    Restart,
    /// The saved selection needs a new provider generation seeded with context.
    RestartWithHandoff,
}

impl QueuePrimaryAction {
    fn tooltip(self) -> &'static str {
        match self {
            Self::SendNow => "Send now (interrupt)",
            Self::Steer => "Steer active response",
            Self::Restart => "Send now (interrupt and restart)",
            Self::RestartWithHandoff => "Send now (restart with handoff)",
        }
    }
}

/// The row action's tooltip, naming the selection it will run on and why it is
/// unavailable. The host decides all of it; this only words the hint.
fn primary_tooltip(
    action: QueuePrimaryAction,
    enabled: bool,
    queue: Option<&zeron_proto::QueueUiState>,
) -> String {
    let model = queue
        .and_then(|queue| queue.promotion_selection.as_ref())
        .map(|selection| selection.model.to_string());
    if !enabled {
        return queue
            .and_then(|queue| queue.promotion_blocked.clone())
            .unwrap_or_else(|| "Waiting for provider capabilities".into());
    }
    match (action, model) {
        (QueuePrimaryAction::Steer, Some(_))
            if queue.is_some_and(|queue| queue.promotion_selection_deferred) =>
        {
            "Steer active response (the new model applies next turn)".into()
        }
        (QueuePrimaryAction::Restart, Some(model)) => {
            format!("Send now on {model} (interrupt and restart)")
        }
        (QueuePrimaryAction::RestartWithHandoff, Some(model)) => {
            format!("Send now on {model} (restart with handoff)")
        }
        (action, _) => action.tooltip().into(),
    }
}

fn canonical_primary_action(queue: &zeron_proto::QueueUiState) -> Option<QueuePrimaryAction> {
    use zeron_proto::QueuePromotionMode;
    match queue.promotion_mode {
        Some(QueuePromotionMode::ActiveSteering) => Some(QueuePrimaryAction::Steer),
        Some(QueuePromotionMode::InterruptRestart) => Some(QueuePrimaryAction::Restart),
        Some(QueuePromotionMode::InterruptRestartWithHandoff) => {
            Some(QueuePrimaryAction::RestartWithHandoff)
        }
        None if queue.can_promote_to_steer => Some(QueuePrimaryAction::Steer),
        None => None,
    }
}

/// Unlike a document lease, this edit cannot stop an automatic drain. The
/// owner uses the exact run and original text; a lost race retains the draft.
pub(crate) struct CanonicalQueueEdit {
    run_id: String,
    base_text: String,
    /// What the pinned retry identity covers: the saved text plus the staged
    /// attachment set. Any change mints a new identity.
    request: Option<(String, String)>,
    /// Attachments the host keeps that the composer cannot show (claimed by an
    /// agent): never removed by this edit.
    attachment_count: usize,
    /// `queue_attachment_fingerprint` of the entry when the edit began.
    expected_attachments: String,
    /// Composer strip id → host path of the uploads loaded for this edit.
    loaded: Vec<(String, String)>,
    /// False while the uploads load, or when they could not be read back: the
    /// edit then changes text only and the host keeps them.
    attachments_editable: bool,
}

impl CanonicalQueueEdit {
    pub(crate) fn attachments_editable(&self) -> bool {
        self.attachments_editable
    }
}

/// Whether the composer's attachment set differs from the loaded one.
fn staged_changed(
    loaded: &[(String, String)],
    staged: &[crate::attachments::StagedAttachment],
) -> bool {
    loaded.len() != staged.len()
        || loaded
            .iter()
            .zip(staged)
            .any(|((id, _), staged)| *id != staged.id)
}

/// Merge only for presentation. Never resurrect a consumed Loro intent from
/// a delayed SQL publication, and never write SQL-only messages into Loro.
fn queue_rows_for_display(
    document: &[QueuedMessage],
    canonical: Option<&zeron_proto::QueueUiState>,
) -> Vec<QueuedMessage> {
    let Some(canonical) = canonical.filter(|queue| queue.schema_version == 1) else {
        return document.to_vec();
    };
    let mut seen = std::collections::HashSet::new();
    let mut rows = Vec::new();
    for entry in &canonical.queue {
        seen.insert(entry.message_id.as_str());
        if entry.automatic {
            continue;
        }
        if let Some(row) = document.iter().find(|row| row.id == entry.message_id) {
            rows.push(row.clone());
        } else if !entry.document_backed {
            let mut row = QueuedMessage::new(&entry.message_id, &entry.text, "");
            row.attachments = entry.attachment_paths.clone();
            row.hold_for_turn_end = entry.held;
            row.delivery_gate = entry
                .delivery_gate
                .clone()
                .and_then(|value| serde_json::from_value(value).ok());
            rows.push(row);
        }
    }
    rows.extend(
        document
            .iter()
            .filter(|row| !seen.contains(row.id.as_str()))
            .cloned(),
    );
    rows
}

/// Map a display-row move onto the document slice the legacy RPC targets:
/// the moved row's id plus its document index and the final document index.
/// The destination is the display row currently at `to` (after the moved row
/// is lifted out); no such row means "to the end".
fn doc_move_indices(
    rows: &[QueuedMessage],
    document: &[QueuedMessage],
    from: usize,
    to: usize,
) -> Option<(String, usize, usize)> {
    let id = rows.get(from)?.id.clone();
    let from_doc = document.iter().position(|row| row.id == id)?;
    let anchor = rows
        .iter()
        .filter(|row| row.id != id)
        .nth(to)
        .and_then(|row| document.iter().position(|doc| doc.id == row.id));
    let to_doc = match anchor {
        Some(position) => position - usize::from(from_doc < position),
        None => document.len() - 1,
    };
    Some((id, from_doc, to_doc))
}

/// Would [`queue_rows_for_display`] list `id`? The allocation-free twin for
/// the state-notification path, which only needs to know a row still exists.
fn queue_contains(
    document: &[QueuedMessage],
    canonical: Option<&zeron_proto::QueueUiState>,
    id: &str,
) -> bool {
    let in_document = document.iter().any(|row| row.id == id);
    let Some(canonical) = canonical.filter(|queue| queue.schema_version == 1) else {
        return in_document;
    };
    let mut listed = false;
    for entry in canonical.queue.iter().filter(|entry| entry.message_id == id) {
        listed = true;
        if !entry.automatic && (in_document || !entry.document_backed) {
            return true;
        }
    }
    // Document rows the snapshot has not caught up with are appended last.
    !listed && in_document
}

/// Typed document rows use Send now. Only host support and edit/review gates
/// determine whether the action is available.
fn available_queue_primary_action(
    delivery_blocked: bool,
    host_supports_actions: bool,
) -> Option<QueuePrimaryAction> {
    (!delivery_blocked && host_supports_actions).then_some(QueuePrimaryAction::SendNow)
}

fn queue_latest_shortcut_visible(
    index: usize,
    count: usize,
    reveal_requested: bool,
    action_available: bool,
) -> bool {
    index.checked_add(1) == Some(count) && reveal_requested && action_available
}

fn latest_queued_message(items: &[QueuedMessage]) -> Option<&QueuedMessage> {
    items.last()
}

/// Translate a pointer inside the whole panel into a row slot. The top pad
/// belongs to slot zero; the bottom pad clamps to the final row.
fn queue_drop_index(panel_y: f32, count: usize) -> usize {
    drop_index(panel_y - PANEL_PAD_TOP, ROW_SLOT, count)
}

/// Paint-only start and target positions for the PR #90 reorder treatment.
/// The dragged row travels to the hovered slot while every row in its path
/// slides into the space it leaves behind.
fn queue_drag_offsets(ix: usize, from: usize, prev_over: usize, over: usize) -> (f32, f32) {
    if ix == from {
        (
            (prev_over as f32 - from as f32) * ROW_SLOT,
            (over as f32 - from as f32) * ROW_SLOT,
        )
    } else {
        (
            slide_offset(ix, from, prev_over) * ROW_SLOT,
            slide_offset(ix, from, over) * ROW_SLOT,
        )
    }
}

/// A queue row being dragged (gpui drag-and-drop). Scoped to its chat so a
/// drag can't land in a queue it didn't come from.
pub struct QueueDragPayload {
    chat: String,
    from: usize,
    message_id: String,
}

/// Where the dragged row would land, including the previous slot needed to
/// restart the short PR #90-style slide from its current visual position.
pub struct QueueDragState {
    pub from: usize,
    pub over: usize,
    pub prev_over: usize,
    pub epoch: usize,
}

/// Invisible cursor ghost: the real row stays in the queue and moves between
/// slots, instead of following the pointer as a detached tooltip.
struct QueueGhost;

impl Render for QueueGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        gpui::Empty
    }
}

/// One line of a queued message: the newlines that make it a paragraph in the
/// composer make it three rows here, and the row is one line tall.
fn one_line(text: &str) -> SharedString {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    SharedString::from(flat)
}

/// New queue rows contain only editable user text. During a rolling upgrade,
/// an older client may still have stored the attachment trailer in `text`.
/// Hide it only when the parsed paths exactly match the row's attachment field.
fn queue_visible_text(text: &str, attachments: &[String]) -> String {
    let text = crate::appshots::strip_context_for_display(text);
    if text.trim().is_empty() && !attachments.is_empty() {
        return crate::attachments::ATTACHMENT_ONLY_TEXT.to_string();
    }
    if attachments.is_empty() {
        return text.to_string();
    }
    let parsed = crate::attachments::parse_user_message_images(text);
    let paths_match = parsed.attachments.len() == attachments.len()
        && parsed
            .attachments
            .iter()
            .zip(attachments)
            .all(|(parsed, stored)| parsed.path == *stored);
    if !paths_match {
        return text.to_string();
    }
    if parsed.text.trim().is_empty() {
        crate::attachments::ATTACHMENT_ONLY_TEXT.to_string()
    } else {
        parsed.text
    }
}

const CONTEXT_HREF: &str = "](t3-context://v1/";
const MAX_CONTEXT_LABEL: usize = 512;

/// T3's inline context references (`[label](t3-context://v1/<kind>/<id>)`, or
/// `![label](..)` for images) as their labels. Image chips vanish when the
/// row already summarizes its attachments. The link syntax is position only;
/// anything malformed stays as typed.
fn replace_context_references(text: &str, drop_images: bool) -> String {
    if !text.contains(CONTEXT_HREF) {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(found) = text[cursor..].find(CONTEXT_HREF) {
        let close = cursor + found;
        let href_start = close + CONTEXT_HREF.len();
        let label_start = text[..close]
            .rfind('[')
            .filter(|&start| start >= cursor && close - start <= MAX_CONTEXT_LABEL);
        let href_end = text[href_start..]
            .find([')', ' ', '\n'])
            .map(|end| href_start + end)
            .filter(|&end| text[end..].starts_with(')') && end - href_start <= 200);
        let valid = match (label_start, href_end) {
            (Some(start), Some(end)) => {
                let label = &text[start + 1..close];
                let mut parts = text[href_start..end].split('/');
                let (kind, id, extra) = (parts.next(), parts.next(), parts.next());
                (!label.contains(['\n', ']']) && extra.is_none())
                    .then_some((start, end, kind.unwrap_or(""), id.unwrap_or("")))
                    .filter(|(_, _, kind, id)| !kind.is_empty() && !id.is_empty())
            }
            _ => None,
        };
        let Some((start, end, kind, _)) = valid else {
            out.push_str(&text[cursor..href_start]);
            cursor = href_start;
            continue;
        };
        let image = text[..start].ends_with('!');
        out.push_str(&text[cursor..start - usize::from(image)]);
        if !(image && kind == "image" && drop_images) {
            out.push_str(&text[start + 1..close]);
        }
        cursor = end + 1;
    }
    out.push_str(&text[cursor..]);
    out
}

/// Metadata chips for a row's retained context. Image/file records are the
/// attachments the row already lists, so only the other kinds are named.
fn queue_context_chips(context: &[zeron_proto::QueueContextRef]) -> Vec<String> {
    context
        .iter()
        .filter(|record| !matches!(record.kind.as_str(), "image" | "file"))
        .map(|record| {
            let what = if record.detail.is_empty() {
                &record.label
            } else {
                &record.detail
            };
            if what.is_empty() {
                record.kind.clone()
            } else {
                format!("{} {what}", record.kind)
            }
        })
        .collect()
}

/// Project file references for display without changing the stored delivery text.
fn queue_row_text(text: &str, attachments: &[String]) -> SharedString {
    let visible = queue_visible_text(text, attachments);
    let display = crate::composer::sent_mention_display(&visible)
        .map(|(display, _)| display)
        .unwrap_or(visible);
    one_line(&display)
}

/// Presentation-only metadata. Never expose the observed accessibility payload.
fn queue_attachment_labels(text: &str, paths: &[String]) -> Vec<String> {
    let presentations = crate::appshots::presentations(text);
    paths
        .iter()
        .map(|path| match presentations.get(path) {
            Some(appshot) => format!("{} Appshot", appshot.app_name),
            None => std::path::Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Image")
                .to_owned(),
        })
        .collect()
}

pub(crate) fn queue_panel_surface(theme: &Theme) -> gpui::Div {
    div()
        .occlude()
        .rounded_t(px(PANEL_RADIUS))
        .bg(crate::popover::surface_bg(theme))
        .border_1()
        .border_color(theme.border)
        .when(!theme.is_frost(), |el| el.shadow_lg())
        // Keep visible rows flush with the tray; only the portion tucked behind
        // the composer needs padding.
        .pb(px(QUEUE_COMPOSER_OVERLAP))
        .flex()
        .flex_col()
}

fn queue_rows(
    scroll: &gpui::ScrollHandle,
    max_height: gpui::Pixels,
    rows: impl IntoIterator<Item = AnyElement>,
) -> crate::edge_fade::EdgeFaded {
    crate::edge_fade::edge_faded(
        Theme::TRANSCRIPT_FADE_BAND,
        true,
        true,
        div()
            .id("message-queue-rows")
            .max_h(max_height)
            .overflow_y_scroll()
            .track_scroll(scroll)
            .flex()
            .flex_col()
            .gap(px(ROW_GAP))
            .children(rows),
    )
    .fade_overflow_y(scroll)
    // GPUI samples glyph fades at baseline + font size.
    .outset_bottom(QUEUE_TEXT_SIZE)
}

fn preview_load_gate() -> &'static futures::lock::Mutex<()> {
    static GATE: std::sync::OnceLock<futures::lock::Mutex<()>> = std::sync::OnceLock::new();
    GATE.get_or_init(|| futures::lock::Mutex::new(()))
}

pub(crate) struct QueuePreview {
    image: Option<crate::attachments::CachedAttachmentImage>,
    finished: bool,
    // Keeping the task here cancels offscreen transfers on eviction.
    _task: gpui::Task<()>,
}

fn visible_queue_rows(offset: f32, height: f32, count: usize) -> std::ops::Range<usize> {
    let first = ((-offset).max(0.0) / ROW_SLOT).floor() as usize;
    let last = first.saturating_add((height.max(0.0) / ROW_SLOT).ceil() as usize + 1);
    first.min(count)..last.min(count)
}

impl Composer {
    /// This composer's queue projection: the selected chat's `state.queue`,
    /// a pane-fixed chat's own `pane_queues` entry (fed by its dedicated
    /// watch), or nothing on the new-chat canvas.
    pub(crate) fn target_queue<'a>(
        target: &'a ChatTarget,
        state: &'a AppState,
    ) -> &'a [QueuedMessage] {
        match target {
            ChatTarget::Selected => state.queue.as_slice(),
            ChatTarget::Fixed(Some(chat_id)) => state.pane_queue(chat_id),
            ChatTarget::Fixed(None) => &[],
        }
    }

    pub(crate) fn target_queue_rows(target: &ChatTarget, state: &AppState) -> Vec<QueuedMessage> {
        queue_rows_for_display(
            Self::target_queue(target, state),
            target
                .chat_id(state)
                .and_then(|id| state.canonical_queues.get(id)),
        )
    }

    pub(crate) fn target_queue_contains(target: &ChatTarget, state: &AppState, id: &str) -> bool {
        queue_contains(
            Self::target_queue(target, state),
            target
                .chat_id(state)
                .and_then(|chat| state.canonical_queues.get(chat)),
            id,
        )
    }

    fn canonical_queue_entry<'a>(
        &self,
        id: &str,
        state: &'a AppState,
    ) -> Option<&'a zeron_proto::QueueUiEntry> {
        let chat = self.target.chat_id(state)?;
        state
            .canonical_queues
            .get(chat)
            .filter(|queue| queue.schema_version == 1)?
            .queue
            .iter()
            .find(|entry| {
                entry.message_id == id
                    && !entry.document_backed
                    && !entry.automatic
                    && !Self::target_queue(&self.target, state)
                        .iter()
                        .any(|row| row.id == id)
            })
    }

    /// The queue panel, or `None` when nothing is waiting. Like the composer,
    /// it is one frosted surface; rows use spacing and hover wash rather than
    /// nesting raised cards inside it.
    pub(crate) fn render_queue_panel(
        &mut self,
        show_latest_shortcut: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        // A drop outside the panel ends GPUI's active drag without invoking our
        // `on_drop`. Never leave the source row replaced by a stale gap.
        if self.queue_drag.is_some() && !cx.has_active_drag() {
            self.queue_drag = None;
        }
        let (items, chat_id, host_supports_actions) = {
            let state = self.state.read(cx);
            let chat_id = self.target.chat_id(state).map(str::to_owned)?;
            let host_supports_actions = state.chat_host_supports(
                &chat_id,
                zeron_proto::capabilities::MESSAGE_QUEUE_ACTIONS_V1,
            );
            (
                Self::target_queue_rows(&self.target, state),
                chat_id,
                host_supports_actions,
            )
        };
        self.prepare_queue_previews(&items, window, cx);
        if items.is_empty() {
            return None;
        }
        let theme = Theme::of(cx).clone();
        let count = items.len();
        let drag = self
            .queue_drag
            .as_ref()
            .map(|d| (d.from, d.over, d.prev_over, d.epoch));
        let editing = self.editing_queued.clone();

        let list_chat = chat_id.clone();
        let drop_chat = chat_id.clone();
        let rows = queue_rows(
            &self.queue_scroll,
            window.viewport_size().height * 0.3,
            items.iter().enumerate().map(|(ix, item)| {
                self.queue_row(
                    &chat_id,
                    ix,
                    count,
                    item,
                    drag,
                    &editing,
                    host_supports_actions,
                    show_latest_shortcut,
                    &theme,
                    cx,
                )
            }),
        );

        let panel = queue_panel_surface(&theme)
            .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.notify()))
            // The complete glass surface is a drop target, including its
            // padding.
            .on_drag_move::<QueueDragPayload>(cx.listener(
                move |this, event: &gpui::DragMoveEvent<QueueDragPayload>, _, cx| {
                    let payload = event.drag(cx);
                    if payload.chat != list_chat {
                        return;
                    }
                    let from = payload.from;
                    let rel_y = f32::from(event.event.position.y)
                        - f32::from(event.bounds.top())
                        - f32::from(this.queue_scroll.offset().y);
                    let over = queue_drop_index(rel_y, count);
                    this.update_queue_drag_over(from, over, cx);
                },
            ))
            .on_drop::<QueueDragPayload>(cx.listener(
                move |this, payload: &QueueDragPayload, _, cx| {
                    if payload.chat != drop_chat {
                        this.queue_drag = None;
                        cx.notify();
                        return;
                    }
                    let to = this
                        .queue_drag
                        .as_ref()
                        .map(|d| d.over)
                        .unwrap_or(payload.from);
                    this.queue_drag = None;
                    let state = this.state.read(cx);
                    if this.target.chat_id(state) != Some(drop_chat.as_str()) {
                        return;
                    }
                    let from = Self::target_queue_rows(&this.target, state)
                        .iter()
                        .position(|row| row.id == payload.message_id);
                    if let Some(from) = from {
                        this.move_queued(from, to, cx);
                    }
                },
            ))
            .on_mouse_up_out(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| this.cancel_queue_drag(cx)),
            )
            .child(rows);
        Some(crate::frost::frosted(PANEL_RADIUS, crate::frost::MENU_BLUR, panel).into_any_element())
    }

    /// One queued message: a quiet queue marker, the text, edit controls, and
    /// one explicit primary delivery action.
    #[allow(clippy::too_many_arguments)]
    fn queue_row(
        &self,
        chat_id: &str,
        ix: usize,
        count: usize,
        item: &QueuedMessage,
        drag: Option<(usize, usize, usize, usize)>,
        editing: &Option<String>,
        host_supports_actions: bool,
        show_latest_shortcut: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = SharedString::from(format!("queue-{}", item.id));
        let being_edited = editing.as_deref() == Some(item.id.as_str());
        let being_removed = self.queue_removing.contains(&item.id);
        let state = self.state.read(cx);
        let canonical = self.canonical_queue_entry(&item.id, state).cloned();
        let canonical_supported =
            state.chat_host_supports(chat_id, zeron_proto::capabilities::CANONICAL_QUEUE_V1);
        let delivery_blocked = item.delivery_gate.is_some();
        let interaction_blocked = delivery_blocked || being_removed;
        let text = match &item.delivery_gate {
            Some(QueueDeliveryGate::Editing {
                owner_device_id, ..
            }) if !being_edited => SharedString::from(format!("Editing on {owner_device_id}")),
            Some(QueueDeliveryGate::ReviewRequired { .. }) if !being_edited => {
                SharedString::from("Needs review")
            }
            _ => {
                let summarized = !item.attachments.is_empty()
                    || canonical
                        .as_ref()
                        .is_some_and(|entry| !entry.attachments.is_empty());
                queue_row_text(
                    &replace_context_references(&item.text, summarized),
                    &item.attachments,
                )
            }
        };

        let edit_id = item.id.clone();
        let edit = self.queue_action(
            &key,
            "edit",
            "Edit",
            icons::PEN,
            !being_removed
                && self.queue_edit_controls_ready()
                && (canonical.is_none() || canonical_supported),
            theme,
            cx.listener(move |this, _, _, cx| {
                this.begin_queue_edit(edit_id.clone(), cx);
            }),
        );
        let drop_id = item.id.clone();
        let discard = self.queue_action(
            &key,
            "drop",
            if being_removed {
                "Updating…"
            } else {
                "Remove"
            },
            icons::TRASH_BIN_MINIMALISTIC,
            !being_removed && (canonical.is_none() || canonical_supported),
            theme,
            cx.listener(move |this, _, _, cx| {
                this.remove_queued(drop_id.clone(), cx);
            }),
        );
        let resolved_primary = if canonical.is_some() {
            (!interaction_blocked && canonical_supported)
                .then(|| {
                    state
                        .canonical_queues
                        .get(chat_id)
                        .and_then(canonical_primary_action)
                })
                .flatten()
        } else {
            available_queue_primary_action(interaction_blocked, host_supports_actions)
        };
        let primary_action = resolved_primary.unwrap_or(if canonical.is_some() {
            QueuePrimaryAction::Steer
        } else {
            QueuePrimaryAction::SendNow
        });
        let primary_id = item.id.clone();
        let primary_tooltip = primary_tooltip(
            primary_action,
            resolved_primary.is_some(),
            state.canonical_queues.get(chat_id),
        );
        let primary = self.queue_primary_action_button(
            &key,
            primary_action,
            primary_tooltip,
            resolved_primary.is_some(),
            queue_latest_shortcut_visible(
                ix,
                count,
                show_latest_shortcut,
                resolved_primary.is_some() && !being_removed,
            ),
            theme,
            cx.listener(move |this, _, _, cx| {
                this.activate_queued_primary(primary_id.clone(), primary_action, cx);
            }),
        );
        let save = self.queue_action(
            &key,
            "save",
            "Save to queue",
            icons::QUEUE_CHECK,
            !self.queue_edit_finishing,
            theme,
            cx.listener(|this, _, _, cx| {
                this.commit_queue_edit(cx);
            }),
        );
        let cancel = self.queue_action(
            &key,
            "cancel",
            "Cancel",
            icons::QUEUE_CLOSE,
            !self.queue_edit_finishing,
            theme,
            cx.listener(|this, _, _, cx| {
                this.cancel_queue_edit(cx);
            }),
        );

        let drag_chat = chat_id.to_string();
        let queue_marker = div()
            .id(SharedString::from(format!("{key}-drag")))
            .w(px(14.0))
            .h(px(22.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.0))
            .cursor_pointer()
            .when(interaction_blocked, |el| {
                el.cursor(gpui::CursorStyle::Arrow).opacity(0.35)
            })
            .child(
                icon(icons::QUEUE_DRAG_HANDLE)
                    .size(px(QUEUE_ICON_SIZE))
                    .text_color(theme.text_muted.opacity(0.5)),
            );

        let row = div()
            .id(SharedString::from(format!("{key}-row")))
            .h(px(ROW_HEIGHT))
            .flex_none()
            .px(px(ROW_PAD_X))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(if self.queue_preview_limit() == 1 {
                4.0
            } else {
                8.0
            }))
            .rounded(px(ROW_RADIUS))
            .when(being_edited, |el| el.bg(crate::theme::ink(0.06)))
            .when(!being_edited && !being_removed, |el| {
                el.hover(|s| s.bg(crate::theme::ink(0.04)))
            })
            .when(being_removed, |el| el.opacity(0.55))
            .cursor(gpui::CursorStyle::Arrow)
            // The marker hints that the row belongs to the queue, while the
            // proven full-row drag hitbox keeps reordering easy. Editing
            // disables it so selection cannot become a reorder gesture.
            .when(!being_edited && !interaction_blocked, |el| {
                el.on_drag(
                    QueueDragPayload {
                        chat: drag_chat,
                        from: ix,
                        message_id: item.id.clone(),
                    },
                    move |_payload, _point, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| QueueGhost)
                    },
                )
            })
            .when(!being_edited, |el| el.child(queue_marker))
            // Preserve text alignment while removing the disabled drag glyph
            // from the editing state.
            .when(being_edited, |el| el.child(div().w(px(14.0)).flex_none()))
            .when(!being_edited, |el| {
                let mut labels = queue_attachment_labels(&item.text, &item.attachments);
                if let Some(entry) = &canonical {
                    labels.extend(entry.attachments.iter().map(|attachment| {
                        attachment["name"]
                            .as_str()
                            .unwrap_or("Attachment")
                            .to_owned()
                    }));
                }
                let mut summary = if labels.len() > 1 {
                    format!("{} attachments · {}", labels.len(), labels.join(" · "))
                } else {
                    labels.join(" · ")
                };
                let chips = canonical
                    .as_ref()
                    .map(|entry| queue_context_chips(&entry.context))
                    .unwrap_or_default();
                let has_meta = !labels.is_empty() || !chips.is_empty();
                if !chips.is_empty() {
                    if !summary.is_empty() {
                        summary.push_str(" · ");
                    }
                    summary.push_str(&chips.join(" · "));
                }
                let only_images = text.as_ref() == crate::attachments::ATTACHMENT_ONLY_TEXT;
                let title = if only_images {
                    summary.clone().into()
                } else {
                    text
                };
                let mut content = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(QUEUE_TEXT_SIZE))
                            .line_height(px(16.0))
                            .text_color(theme.text.opacity(0.9))
                            .child(title),
                    );
                if has_meta && !only_images {
                    content = content.child(
                        div()
                            .truncate()
                            .font_family(theme.font_mono.clone())
                            .text_size(px(11.0))
                            .line_height(px(13.0))
                            .text_color(theme.text_muted)
                            .child(summary),
                    );
                }
                el.children(
                    item.attachments
                        .iter()
                        .take(self.queue_preview_limit())
                        .enumerate()
                        .map(|(index, path)| self.queue_thumbnail(&key, index, path, cx)),
                )
                .when(item.attachments.len() > self.queue_preview_limit(), |el| {
                    let remaining = item.attachments.len() - self.queue_preview_limit();
                    el.child(
                        div()
                            .id(SharedString::from(format!("{key}-more-attachments")))
                            .w(px(28.0))
                            .h(px(28.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.0))
                            .bg(crate::theme::ink(0.06))
                            .text_size(px(11.0))
                            .text_color(theme.text_muted)
                            .aria_label(format!(
                                "{remaining} more attachments; edit message to view all"
                            ))
                            .child(format!("+{remaining}")),
                    )
                })
                .child(content)
            })
            .when(being_edited, |el| {
                el.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(QUEUE_TEXT_SIZE))
                        .text_color(theme.text_muted)
                        .child(if self.queue_edit_finishing {
                            SharedString::from("Saving…")
                        } else if let Some(edit) = &self.canonical_queue_edit
                            && edit.attachment_count > 0
                        {
                            SharedString::from(format!(
                                "Editing · {} agent attachments kept",
                                edit.attachment_count
                            ))
                        } else if let Some(edit) = &self.canonical_queue_edit
                            && !edit.attachments_editable
                        {
                            SharedString::from("Loading attachments…")
                        } else {
                            SharedString::from("Editing in composer")
                        }),
                )
            })
            .when(being_edited, |el| {
                el.child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(3.0))
                        .child(save)
                        .child(cancel),
                )
            })
            .when(!being_edited, |el| {
                el.child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(3.0))
                        .child(discard)
                        .child(edit)
                        .child(primary),
                )
            });

        let Some((from, over, prev_over, epoch)) = drag else {
            return row.into_any_element();
        };
        let (start, target) = queue_drag_offsets(ix, from, prev_over, over);
        if cx.reduce_motion() {
            return div()
                .relative()
                .top(px(target))
                .child(row)
                .into_any_element();
        }
        div()
            .child(row)
            .with_animation(
                ("queue-row-slide", (ix as u64) | ((epoch as u64) << 32)),
                TAB_SLIDE.animation(),
                move |el, t| el.relative().top(px(motion::lerp(start, target, t))),
            )
            .into_any_element()
    }

    pub(crate) fn release_queue_previews(&mut self, cx: &mut gpui::App) {
        for (_, preview) in self.queue_previews.drain() {
            if let Some(image) = preview.image {
                gpui::ImageSource::Image(image.image).evict(None, cx);
            }
        }
    }

    fn prepare_queue_previews(
        &mut self,
        items: &[QueuedMessage],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::attachments;
        let state = self.state.read(cx);
        let device = self
            .target
            .chat(state)
            .map(|chat| chat.device_id.clone())
            .unwrap_or_default();
        let engine = state.engine().cloned();
        let target =
            (state.local_device_id.as_deref() != Some(device.as_str())).then(|| device.clone());
        let visible = visible_queue_rows(
            f32::from(self.queue_scroll.offset().y),
            f32::from(window.viewport_size().height) * 0.3,
            items.len(),
        );
        let keys: std::collections::HashSet<_> = items[visible]
            .iter()
            .flat_map(|item| item.attachments.iter().take(self.queue_preview_limit()))
            .map(|path| (device.clone(), path.clone()))
            .take(64)
            .collect();
        self.queue_previews.retain(|key, preview| {
            if keys.contains(key) {
                return true;
            }
            if let Some(image) = &preview.image {
                gpui::ImageSource::Image(image.image.clone()).evict(Some(window), cx);
            }
            false
        });
        let Some(engine) = engine else { return };
        for key in keys {
            if self.queue_previews.contains_key(&key) {
                continue;
            }
            let engine = engine.clone();
            let target = target.clone();
            let task_key = key.clone();
            let task = cx.spawn(async move |this, cx| {
                let _permit = preview_load_gate().lock().await;
                let source = match attachments::attachment_snapshot(&task_key.0, &task_key.1) {
                    attachments::AttachmentSnapshot::Loaded(image) => {
                        Some(attachments::LoadedAttachmentImage {
                            name: image.name.to_string(),
                            image: image.image,
                        })
                    }
                    _ => {
                        attachments::read_attachment_image(
                            &engine,
                            cx.background_executor(),
                            target.as_deref(),
                            &task_key.1,
                            None,
                        )
                        .await
                    }
                };
                let image = if let Some(source) = source {
                    cx.background_executor()
                        .spawn(async move {
                            attachments::queue_thumbnail_image(&source.image).map(|image| {
                                attachments::CachedAttachmentImage {
                                    name: source.name.into(),
                                    image,
                                }
                            })
                        })
                        .await
                } else {
                    None
                };
                this.update(cx, |this, cx| {
                    if let Some(preview) = this.queue_previews.get_mut(&task_key) {
                        preview.image = image;
                        preview.finished = true;
                    }
                    cx.notify();
                })
                .ok();
            });
            self.queue_previews.insert(
                key,
                QueuePreview {
                    image: None,
                    finished: false,
                    _task: task,
                },
            );
        }
    }

    fn load_queue_full_preview(&mut self, device: String, path: String, cx: &mut Context<Self>) {
        use crate::attachments;
        if let attachments::AttachmentSnapshot::Loaded(image) =
            attachments::attachment_snapshot(&device, &path)
        {
            self.queue_full_preview = None;
            self.show_queue_image(
                crate::attachments::PreviewImage::new(image.name, image.image),
                cx,
            );
            return;
        }
        let state = self.state.read(cx);
        let Some(engine) = state.engine().cloned() else {
            return;
        };
        let target = (state.local_device_id.as_deref() != Some(device.as_str())).then_some(device);
        let chat = self.target.chat_id(state).map(str::to_owned);
        self.queue_full_preview = Some(cx.spawn(async move |this, cx| {
            let image = attachments::read_attachment_image(
                &engine,
                cx.background_executor(),
                target.as_deref(),
                &path,
                None,
            )
            .await;
            this.update(cx, |this, cx| {
                this.queue_full_preview = None;
                if this.target.chat_id(this.state.read(cx)) != chat.as_deref() {
                    return;
                }
                if let Some(image) = image {
                    this.show_queue_image(
                        crate::attachments::PreviewImage::new(image.name, image.image),
                        cx,
                    );
                } else {
                    this.show_appshot_error(
                        "Could not load the image. Try opening it again.".into(),
                        cx,
                    );
                }
            })
            .ok();
        }));
    }

    fn queue_thumbnail(
        &self,
        key: &SharedString,
        index: usize,
        path: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let device = self
            .target
            .chat(self.state.read(cx))
            .map(|chat| chat.device_id.clone())
            .unwrap_or_default();
        let cache_key = (device.clone(), path.to_string());
        let failed = self
            .queue_previews
            .get(&cache_key)
            .is_some_and(|preview| preview.finished && preview.image.is_none());
        let snapshot = self
            .queue_previews
            .get(&cache_key)
            .and_then(|preview| preview.image.clone());
        let frame = div()
            .id(SharedString::from(format!("{key}-image-{index}")))
            .w(px(40.0))
            .h(px(28.0))
            .flex_none()
            .rounded(px(5.0))
            .border_1()
            .border_color(crate::theme::hairline(0.1))
            .bg(crate::theme::ink(0.035))
            .overflow_hidden();
        match snapshot {
            Some(image) => {
                let label = image.name.clone();
                let path = path.to_owned();
                let accent = Theme::of(cx).accent;
                frame
                    .role(gpui::Role::Button)
                    .aria_label(format!("Preview {}", label))
                    .tab_index(0)
                    .focus_visible(move |style| style.border_color(accent))
                    .hover(move |style| style.border_color(accent))
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.load_queue_full_preview(device.clone(), path.clone(), cx);
                    }))
                    .child(
                        gpui::img(image.image)
                            .w(px(38.0))
                            .h(px(26.0))
                            .rounded(px(4.0))
                            .object_fit(gpui::ObjectFit::Cover),
                    )
                    .into_any_element()
            }
            _ => frame
                .when(failed, |frame| {
                    let accent = Theme::of(cx).accent;
                    frame
                        .role(gpui::Role::Button)
                        .aria_label("Open attachment preview")
                        .tab_index(0)
                        .focus_visible(move |style| style.border_color(accent))
                        .cursor_pointer()
                        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.queue_previews.remove(&cache_key);
                            this.load_queue_full_preview(
                                cache_key.0.clone(),
                                cache_key.1.clone(),
                                cx,
                            );
                            cx.notify();
                        }))
                })
                .flex()
                .items_center()
                .justify_center()
                .child(icon(icons::QUEUE_PAPERCLIP).size(px(14.0)))
                .into_any_element(),
        }
    }

    /// A permanently-visible trailing glyph button. The queue reference keeps
    /// edit and remove present instead of revealing them only on hover.
    fn queue_action(
        &self,
        key: &SharedString,
        slot: &str,
        label: &'static str,
        glyph: &'static str,
        enabled: bool,
        theme: &Theme,
        on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    ) -> AnyElement {
        let own = SharedString::from(format!("{key}-{slot}-grp"));
        let accent = theme.accent;
        div()
            .id(SharedString::from(format!("{key}-{slot}")))
            .group(own.clone())
            .role(gpui::Role::Button)
            .aria_label(label)
            .size(px(28.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(5.0))
            .opacity(0.72)
            .when(enabled, |el| {
                el.cursor_pointer()
                    .hover(|s| s.opacity(1.0).bg(crate::theme::ink(0.07)))
                    .tab_index(0)
                    .focus_visible(move |s| s.bg(accent.opacity(0.18)).text_color(accent))
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(move |event, window, cx| {
                        cx.stop_propagation();
                        on_click(event, window, cx);
                    })
            })
            .when(!enabled, |el| {
                el.cursor(gpui::CursorStyle::Arrow).opacity(0.45)
            })
            .tooltip(crate::tooltip::text(label))
            .tooltip_show_delay(std::time::Duration::from_millis(350))
            .child(
                icon(glyph)
                    .size(px(QUEUE_ICON_SIZE))
                    .text_color(theme.text_muted.opacity(0.8))
                    .group_hover(own, |s| s.text_color(theme.text)),
            )
            .into_any_element()
    }

    /// Send now interrupts the current response before delivering the row.
    fn queue_primary_action_button(
        &self,
        key: &SharedString,
        action: QueuePrimaryAction,
        tooltip: String,
        enabled: bool,
        show_shortcut: bool,
        theme: &Theme,
        on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    ) -> AnyElement {
        let accent = theme.accent;
        let compact = self.queue_preview_limit() == 1;
        div()
            .id(SharedString::from(format!("{key}-primary")))
            .role(gpui::Role::Button)
            .aria_label(tooltip.clone())
            // Both labels occupy the same slot; modifier previews never move
            // the message text, thumbnails, or adjacent actions.
            .w(px(if compact { 28.0 } else { 72.0 }))
            .h(px(28.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(5.0))
            .text_size(px(11.5))
            .text_color(theme.text_muted)
            .when(enabled, |el| {
                el.cursor_pointer()
                    .tab_index(0)
                    .hover(|s| s.bg(crate::theme::ink(0.07)).text_color(theme.text))
                    .focus_visible(move |s| s.bg(accent.opacity(0.18)).text_color(accent))
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(move |event, window, cx| {
                        cx.stop_propagation();
                        on_click(event, window, cx);
                    })
            })
            .when(!enabled, |el| el.opacity(0.45))
            .tooltip(crate::tooltip::text(tooltip))
            .tooltip_show_delay(std::time::Duration::from_millis(350))
            .child(if show_shortcut {
                div()
                    .child(if compact {
                        if cfg!(target_os = "macos") {
                            "⌘↵"
                        } else {
                            "⌃↵"
                        }
                    } else {
                        modifier_send_label(cfg!(target_os = "macos"))
                    })
                    .into_any_element()
            } else if compact {
                icon(icons::QUEUE_SEND)
                    .size(px(QUEUE_ICON_SIZE))
                    .text_color(theme.text_muted)
                    .into_any_element()
            } else {
                div()
                    .child(match action {
                        QueuePrimaryAction::SendNow
                        | QueuePrimaryAction::Restart
                        | QueuePrimaryAction::RestartWithHandoff => "Send now",
                        QueuePrimaryAction::Steer => "Steer",
                    })
                    .into_any_element()
            })
            .into_any_element()
    }

    /// Track the drop slot while a row is dragged over the list.
    fn update_queue_drag_over(&mut self, from: usize, over: usize, cx: &mut Context<Self>) {
        match &mut self.queue_drag {
            Some(drag) if drag.from == from => {
                if drag.over != over {
                    drag.prev_over = drag.over;
                    drag.over = over;
                    drag.epoch = drag.epoch.wrapping_add(1);
                    cx.notify();
                }
            }
            _ => {
                self.queue_drag = Some(QueueDragState {
                    from,
                    over,
                    prev_over: from,
                    epoch: 0,
                });
                cx.notify();
            }
        }
    }

    /// Restore a row whose pointer was released outside the queue's drop zone.
    fn cancel_queue_drag(&mut self, cx: &mut Context<Self>) {
        if self.queue_drag.take().is_some() {
            cx.notify();
        }
    }

    /// Move the row at `from` to `to`, optimistically here and for real on the
    /// doc (the watch frame is what everyone else sees).
    pub(crate) fn move_queued(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from == to {
            cx.notify();
            return;
        }
        let state = self.state.read(cx);
        let rows = Self::target_queue_rows(&self.target, state);
        let canonical = self
            .target
            .chat_id(state)
            .and_then(|id| state.canonical_queues.get(id))
            .filter(|queue| queue.schema_version == 1);
        if canonical.is_some_and(|queue| {
            queue
                .queue
                .iter()
                .any(|entry| !entry.automatic && !entry.document_backed)
        }) {
            let Some(queue) = canonical else { return };
            let Some(row) = rows.get(from) else { return };
            let Some(run) = queue.queue.iter().find(|entry| entry.message_id == row.id) else {
                self.failure = Some("The queue is still syncing; try reordering again".into());
                cx.notify();
                return;
            };
            let run_id = run.queued_run_id.clone();
            let mut order = rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>();
            order.remove(from);
            let before = order.get(to.min(order.len())).copied();
            let before_run_id = match before {
                None => None,
                Some(id) => {
                    let Some(entry) = queue.queue.iter().find(|entry| entry.message_id == id)
                    else {
                        self.failure =
                            Some("The queue is still syncing; try reordering again".into());
                        cx.notify();
                        return;
                    };
                    Some(entry.queued_run_id.clone())
                }
            };
            let message_id = row.id.clone();
            self.canonical_queue_action(
                message_id,
                run_id,
                zeron_proto::QueuedRunAction::Reorder { before_run_id },
                cx,
            );
            return;
        }
        // `from`/`to` index the display rows, but the host and the optimistic
        // mutate address the document order, which canonical ordering can
        // differ from. Resolve the row by id against the document slice.
        let (id, from, to, chat_id) = {
            let state = self.state.read(cx);
            let Some((id, from, to)) = doc_move_indices(
                &rows,
                Self::target_queue(&self.target, state),
                from,
                to,
            ) else {
                return;
            };
            let Some(chat_id) = self.target.chat_id(state).map(str::to_owned) else {
                return;
            };
            (id, from, to, chat_id)
        };
        self.state.update(cx, |state, cx| {
            state.mutate_chat_queue(&chat_id, |queue| {
                if from < queue.len() {
                    let item = queue.remove(from);
                    queue.insert(to.min(queue.len()), item);
                }
            });
            cx.notify();
        });
        self.queue_rpc(
            methods::MOVE_QUEUED_MESSAGE,
            serde_json::json!({ "id": id, "toIndex": to }),
            "Couldn't reorder the queue",
            cx,
        );
    }

    /// Cancel a queued message at its host. The row remains visible and inert
    /// until the host acknowledges winning the race against automatic drain.
    pub(crate) fn remove_queued(&mut self, id: String, cx: &mut Context<Self>) {
        if self.queue_removing.contains(&id) {
            return;
        }
        if let Some(entry) = self
            .canonical_queue_entry(&id, self.state.read(cx))
            .cloned()
        {
            self.canonical_queue_action(
                id,
                entry.queued_run_id,
                zeron_proto::QueuedRunAction::Cancel,
                cx,
            );
            return;
        }
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let (chat_id, host_device_id, supported) = {
            let state = self.state.read(cx);
            let Some(chat_id) = self.target.chat_id(state).map(str::to_owned) else {
                return;
            };
            let Some(host_device_id) = self.target.chat(state).map(|chat| chat.device_id.clone())
            else {
                return;
            };
            let supported = state.chat_host_supports(
                &chat_id,
                zeron_proto::capabilities::MESSAGE_QUEUE_ACTIONS_V1,
            );
            (chat_id, host_device_id, supported)
        };
        if !supported {
            self.failure = Some("The chat host does not support safe queue removal".into());
            cx.notify();
            return;
        }
        if self.editing_queued.as_deref() == Some(id.as_str()) {
            self.clear_queue_edit(cx);
        }
        self.queue_removing.insert(id.clone());
        self.queue_drag = None;
        cx.notify();

        let params = serde_json::json!({
            "chatId": chat_id,
            "id": id,
            "targetDeviceId": host_device_id,
        });
        cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::REMOVE_QUEUED_MESSAGE, params)
                .await;
            this.update(cx, |composer, cx| {
                composer.queue_removing.remove(&id);
                // The composer reports while it still serves the chat; the
                // queue projection for that chat reconciles either way.
                let target_matches =
                    composer.target.chat_id(composer.state.read(cx)) == Some(chat_id.as_str());
                match result {
                    Ok(reply)
                        if queue_mutation_acknowledged(methods::REMOVE_QUEUED_MESSAGE, &reply) =>
                    {
                        composer.state.update(cx, |state, cx| {
                            state.mutate_chat_queue(&chat_id, |queue| {
                                queue.retain(|item| item.id != id)
                            });
                            cx.notify();
                        });
                    }
                    Ok(reply) => {
                        tracing::debug!(
                            ?reply,
                            "queued message had already left the queue before removal"
                        );
                        if target_matches {
                            composer.failure =
                                Some("That message had already left the queue".into());
                        }
                        composer
                            .state
                            .update(cx, |state, cx| state.refresh_chat_queue(&chat_id, cx));
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "host-authoritative queue removal failed");
                        if target_matches {
                            composer.failure = Some("Couldn't remove the message".into());
                        }
                        composer
                            .state
                            .update(cx, |state, cx| state.refresh_chat_queue(&chat_id, cx));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Send one now: the host stops the turn and hands this message over. Not
    /// optimistic — the row leaves the queue when the host has actually taken
    /// it, so a failed interrupt doesn't lose the text.
    pub(crate) fn send_queued_now(&mut self, id: String, cx: &mut Context<Self>) {
        if self.editing_queued.as_deref() == Some(id.as_str()) {
            self.clear_queue_edit(cx);
        }
        self.queue_rpc(
            methods::SEND_QUEUED_MESSAGE_NOW,
            serde_json::json!({ "id": id }),
            "Couldn't send that message",
            cx,
        );
    }

    /// Hand one queued message to the live turn without interrupting it. The
    /// row stays queued if the host cannot steer (unsupported harness, a sync
    /// that has not landed yet), so a failed steer never loses the text.
    pub(crate) fn steer_queued_now(&mut self, id: String, cx: &mut Context<Self>) {
        self.queue_rpc(
            methods::STEER_QUEUED_MESSAGE_NOW,
            serde_json::json!({ "id": id }),
            "Couldn't steer that message",
            cx,
        );
    }

    /// Execute the same resolved action advertised on the row. Both pointer
    /// clicks and the empty-composer Enter gesture come through here.
    fn activate_queued_primary(
        &mut self,
        id: String,
        action: QueuePrimaryAction,
        cx: &mut Context<Self>,
    ) {
        match action {
            QueuePrimaryAction::SendNow => self.send_queued_now(id, cx),
            QueuePrimaryAction::Steer
            | QueuePrimaryAction::Restart
            | QueuePrimaryAction::RestartWithHandoff => {
                let state = self.state.read(cx);
                let Some(entry) = self.canonical_queue_entry(&id, state) else {
                    return;
                };
                let Some((target_run_id, expected_selection)) = self
                    .target
                    .chat_id(state)
                    .and_then(|id| state.canonical_queues.get(id))
                    .filter(|queue| canonical_primary_action(queue) == Some(action))
                    .and_then(|queue| {
                        Some((queue.active_run_id.clone()?, queue.promotion_selection.clone()))
                    })
                else {
                    return;
                };
                let run_id = entry.queued_run_id.clone();
                // The host refuses if the selection or mode shown here moved.
                self.canonical_queue_action(
                    id,
                    run_id,
                    match action {
                        QueuePrimaryAction::Steer => zeron_proto::QueuedRunAction::PromoteToSteer {
                            target_run_id,
                            expected_selection,
                        },
                        _ => zeron_proto::QueuedRunAction::PromoteToRestart {
                            target_run_id,
                            handoff: action == QueuePrimaryAction::RestartWithHandoff,
                            expected_selection,
                        },
                    },
                    cx,
                );
            }
        }
    }

    #[cfg(feature = "orchestration-fixture")]
    pub(crate) fn fixture_restart_queued(&mut self, id: String, cx: &mut Context<Self>) {
        let state = self.state.read(cx);
        assert!(self.canonical_queue_entry(&id, state).is_some());
        assert_eq!(
            self.target
                .chat_id(state)
                .and_then(|chat| state.canonical_queues.get(chat))
                .and_then(canonical_primary_action),
            Some(QueuePrimaryAction::Restart),
        );
        self.activate_queued_primary(id, QueuePrimaryAction::Restart, cx);
    }

    /// Cmd/Ctrl+Enter activates the most recent row's advertised action:
    /// interrupt for typed intents, or the advertised exact-attempt promotion.
    /// Unsupported capabilities and edit/review gates remain a no-op.
    pub(crate) fn activate_latest_queued(&mut self, cx: &mut Context<Self>) {
        if self.editing_queued.is_some() {
            return;
        }
        let state = self.state.read(cx);
        if let Some(row) = Self::target_queue_rows(&self.target, state).last()
            && self.canonical_queue_entry(&row.id, state).is_some()
        {
            let id = row.id.clone();
            let action = self
                .target
                .chat_id(state)
                .and_then(|chat| state.canonical_queues.get(chat))
                .and_then(canonical_primary_action);
            if let Some(action) = action {
                self.activate_queued_primary(id, action, cx);
            }
            return;
        }
        let (id, delivery_blocked, host_supports_actions) = {
            let state = self.state.read(cx);
            let Some(chat_id) = self.target.chat_id(state) else {
                return;
            };
            let Some(item) = latest_queued_message(Self::target_queue(&self.target, state)) else {
                return;
            };
            (
                item.id.clone(),
                item.delivery_gate.is_some(),
                state.chat_host_supports(
                    chat_id,
                    zeron_proto::capabilities::MESSAGE_QUEUE_ACTIONS_V1,
                ),
            )
        };
        let Some(action) = available_queue_primary_action(delivery_blocked, host_supports_actions)
        else {
            return;
        };
        self.activate_queued_primary(id, action, cx);
    }

    /// Borrow the composer while the leased row reserves its queue position.
    pub(crate) fn begin_queue_edit(&mut self, id: String, cx: &mut Context<Self>) {
        if !self.queue_edit_controls_ready() {
            return;
        }
        if let Some(entry) = self
            .canonical_queue_entry(&id, self.state.read(cx))
            .cloned()
        {
            self.begin_canonical_queue_edit(id, entry, cx);
            return;
        }
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let (chat_id, host_device_id, supported) = {
            let state = self.state.read(cx);
            let Some(chat_id) = self.target.chat_id(state).map(str::to_owned) else {
                return;
            };
            let Some(host_device_id) = self.target.chat(state).map(|chat| chat.device_id.clone())
            else {
                return;
            };
            let capability = zeron_proto::capabilities::MESSAGE_QUEUE_EDIT_LEASE_V1;
            let supported = engine.engine_info().supports(capability)
                && state.chat_host_supports(&chat_id, capability);
            (chat_id, host_device_id, supported)
        };
        if !supported {
            self.failure = Some("Update the chat host to edit queued messages safely".into());
            cx.notify();
            return;
        }
        if !Self::target_queue(&self.target, self.state.read(cx))
            .iter()
            .any(|item| item.id == id)
        {
            return;
        }
        let owner_device_id = engine.engine_info().device_id.clone();
        let instance_id = self.queue_edit_instance_id.clone();
        self.queue_edit_pending_id = Some(id.clone());
        self.queue_drag = None;
        cx.notify();
        let params = serde_json::json!({
            "chatId": chat_id,
            "id": id,
            "editorDeviceId": owner_device_id,
            "editorInstanceId": instance_id,
            "targetDeviceId": host_device_id,
        });
        let task = cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::BEGIN_QUEUED_MESSAGE_EDIT, params)
                .await;
            let mut loaded_attachments = Vec::new();
            let mut loaded_appshots = Vec::new();
            if let Ok(reply) = &result
                && reply.get("outcome").and_then(|v| v.as_str()) == Some("acquired")
            {
                let paths = reply.get("attachments")
                    .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok());
                let mut load_failed = paths.is_none();
                for path in paths.unwrap_or_default() {
                    let loaded = crate::attachments::read_attachment_image(
                        &engine, cx.background_executor(), Some(&host_device_id), &path,
                        None,
                    ).await;
                    match loaded {
                        Some(loaded) => loaded_attachments.push(crate::attachments::StagedAttachment {
                            id: uuid::Uuid::new_v4().to_string(), name: loaded.name, image: loaded.image,
                        }),
                        None => { load_failed = true; break; }
                    }
                }
                if !load_failed {
                    let paths: Vec<String> = serde_json::from_value(reply["attachments"].clone()).unwrap_or_default();
                    match crate::appshots::restore_queued_appshots(
                        reply.get("text").and_then(|v| v.as_str()).unwrap_or_default(),
                        &paths, &loaded_attachments,
                    ) {
                        Ok((ordinary, shots)) => { loaded_attachments = ordinary; loaded_appshots = shots; }
                        Err(_) => { load_failed = true; }
                    }
                }
                if load_failed {
                    let _ = engine.client().call(methods::FINISH_QUEUED_MESSAGE_EDIT, serde_json::json!({
                        "chatId": chat_id, "id": id, "leaseId": reply.get("leaseId"),
                        "action": "cancel", "targetDeviceId": host_device_id,
                    })).await;
                    this.update(cx, |composer, cx| {
                        composer.queue_edit_pending_id = None;
                        composer.failure = Some("Couldn't load the queued attachments or Appshot context. Check the connection and update the chat host.".into());
                        cx.notify();
                    }).ok();
                    return;
                }
            }
            this.update(cx, |composer, cx| {
                composer.queue_edit_pending_id = None;
                match result {
                    Ok(reply)
                        if reply.get("outcome").and_then(|v| v.as_str()) == Some("acquired") =>
                    {
                        let Some(lease_id) = reply.get("leaseId").and_then(|v| v.as_str()) else {
                            composer.failure =
                                Some("The chat host returned an invalid edit lease".into());
                            cx.notify();
                            return;
                        };
                        let Some(base_text_hash) =
                            reply.get("baseTextHash").and_then(|v| v.as_str())
                        else {
                            composer.failure =
                                Some("The chat host returned an invalid edit lease".into());
                            cx.notify();
                            return;
                        };
                        let raw_text = reply
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let attachments: Vec<String> = serde_json::from_value(reply["attachments"].clone()).unwrap_or_default();
                        let text = queue_visible_text(&raw_text, &attachments);
                        let text = if !attachments.is_empty() && text == crate::attachments::ATTACHMENT_ONLY_TEXT {
                            String::new()
                        } else { text };
                        let target_matches = composer.target.chat_id(composer.state.read(cx))
                            == Some(chat_id.as_str());
                        if !target_matches || !composer.can_edit_queue_in_composer() {
                            // Navigation or another composer action won acquisition. Release
                            // immediately; the expiry/review path is the backup.
                            let params = serde_json::json!({
                                "chatId": chat_id,
                                "id": id,
                                "leaseId": lease_id,
                                "action": "cancel",
                                "targetDeviceId": host_device_id,
                            });
                            let engine = engine.clone();
                            cx.spawn(async move |_, _| {
                                let _ = engine
                                    .client()
                                    .call(methods::FINISH_QUEUED_MESSAGE_EDIT, params)
                                    .await;
                            })
                            .detach();
                            return;
                        }
                        composer.editing_queued = Some(id.clone());
                        composer.queue_edit_lease_id = Some(lease_id.to_string());
                        composer.queue_edit_base_text_hash = Some(base_text_hash.to_string());
                        composer.queue_edit_chat_id = Some(chat_id.clone());
                        composer.queue_edit_host_device_id = Some(host_device_id.clone());
                        composer.queue_edit_draft = Some((
                            composer.input.read(cx).text().to_string(),
                            composer.attachments.remove(&composer.current_key).unwrap_or_default(),
                            composer.appshots.remove(&composer.current_key).unwrap_or_default(),
                        ));
                        composer.attachments.insert(composer.current_key.clone(), loaded_attachments);
                        composer.appshots.insert(composer.current_key.clone(), loaded_appshots);
                        composer.focus_pending = true;
                        composer.input.update(cx, |input, cx| input.set_text(text, cx));
                        composer.start_queue_edit_renewal(engine.clone(), cx);
                    }
                    Ok(reply)
                        if reply.get("outcome").and_then(|v| v.as_str()) == Some("locked") =>
                    {
                        composer.failure =
                            Some("That queued message is being edited on another device".into());
                    }
                    Ok(_) => {
                        composer.failure =
                            Some("That queued message is no longer available".into());
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "begin queue edit failed");
                        composer.failure =
                            Some("Connect to the chat host to edit this message".into());
                    }
                }
                cx.notify();
            })
            .ok();
        });
        self.queue_edit_task = Some(task);
    }

    fn queue_edit_controls_ready(&self) -> bool {
        self.queue_edit_pending_id.is_none()
            && !self.queue_edit_finishing
            && self.editing_queued.is_none()
            && self.can_edit_queue_in_composer()
    }

    /// Save the composer into the existing row, including its attachments.
    /// An entirely empty composer removes the row.
    pub(crate) fn commit_queue_edit(&mut self, cx: &mut Context<Self>) -> bool {
        if self.editing_queued.is_none() {
            return false;
        }
        if self
            .input
            .update(cx, |input, cx| input.finish_dictation(true, cx))
        {
            return true;
        }
        let text = self.input.read(cx).text().trim().to_string();
        if self.canonical_queue_edit.is_some() {
            self.finish_canonical_queue_edit(text, cx);
            return true;
        }
        if text.is_empty() && self.staged().is_empty() && self.staged_appshots().is_empty() {
            self.finish_queue_edit("discard", None, cx);
        } else {
            self.finish_queue_edit("commit", Some(text), cx);
        }
        true
    }

    /// Escape out of an edit, leaving the row as it was.
    pub(crate) fn cancel_queue_edit(&mut self, cx: &mut Context<Self>) -> bool {
        if self.editing_queued.is_none() {
            return false;
        }
        self.input.update(cx, |input, _| input.cancel_dictation());
        if self.canonical_queue_edit.is_some() {
            if !self.queue_edit_finishing {
                self.clear_queue_edit_local(cx);
            }
            return true;
        }
        self.finish_queue_edit("cancel", None, cx);
        true
    }

    pub(crate) fn clear_queue_edit(&mut self, cx: &mut Context<Self>) {
        self.release_queue_edit_best_effort(cx);
        self.clear_queue_edit_local(cx);
    }

    fn clear_queue_edit_local(&mut self, cx: &mut Context<Self>) {
        self.input.update(cx, |input, _| input.cancel_dictation());
        self.editing_queued = None;
        self.canonical_queue_edit = None;
        self.queue_edit_lease_id = None;
        self.queue_edit_base_text_hash = None;
        self.queue_edit_chat_id = None;
        self.queue_edit_host_device_id = None;
        self.queue_edit_pending_id = None;
        self.queue_edit_finishing = false;
        self.input.update(cx, |input, cx| {
            input.read_only = false;
            cx.notify();
        });
        self.queue_edit_task = None;
        self.queue_edit_renew_task = None;
        if let Some((text, attachments, appshots)) = self.queue_edit_draft.take() {
            self.appshots.insert(self.current_key.clone(), appshots);
            self.input.update(cx, |input, cx| input.set_text(text, cx));
            self.attachments
                .insert(self.current_key.clone(), attachments);
        }
        self.focus_pending = true;
        cx.notify();
    }

    fn finish_queue_edit(
        &mut self,
        action: &'static str,
        text: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.queue_edit_finishing {
            return;
        }
        let (Some(id), Some(lease_id), Some(chat_id), Some(host_device_id), Some(engine)) = (
            self.editing_queued.clone(),
            self.queue_edit_lease_id.clone(),
            self.queue_edit_chat_id.clone(),
            self.queue_edit_host_device_id.clone(),
            self.state.read(cx).engine().cloned(),
        ) else {
            self.failure = Some("The edit lease was lost; your text is still in the editor".into());
            cx.notify();
            return;
        };
        let expected = self.queue_edit_base_text_hash.clone();
        let mut staged = self.staged().to_vec();
        let staged_appshots = self.staged_appshots().to_vec();
        staged.extend(staged_appshots.iter().map(|shot| shot.screenshot.clone()));
        let mut params = serde_json::json!({
            "chatId": chat_id,
            "id": id,
            "leaseId": lease_id,
            "action": action,
            "text": text,
            "expectedTextHash": expected,
            "targetDeviceId": host_device_id,
        });
        self.queue_edit_finishing = true;
        self.input.update(cx, |input, cx| {
            input.cancel_dictation();
            input.read_only = true;
            cx.notify();
        });
        cx.notify();
        let task = cx.spawn(async move |this, cx| {
            let result = async {
                if action == "commit" {
                    let mut paths = Vec::new();
                    for attachment in &staged {
                        let path = crate::attachments::upload_attachment(
                            &engine, cx.background_executor(), Some(&host_device_id),
                            &uuid::Uuid::new_v4().to_string(), attachment, None,
                        ).await.map_err(|err| err.to_string())?;
                        paths.push(path);
                    }
                    let appshot_paths = staged.iter().zip(&paths)
                        .map(|(attachment, path)| (attachment.id.clone(), path.clone())).collect();
                    params["text"] = crate::appshots::with_appshots(
                        params["text"].as_str().unwrap_or_default(), &staged_appshots, &appshot_paths,
                    ).into();
                    if params["text"].as_str().is_some_and(|text| text.trim().is_empty()) && !paths.is_empty() {
                        params["text"] = crate::attachments::ATTACHMENT_ONLY_TEXT.into();
                    }
                    params["attachments"] = serde_json::json!(paths);
                }
                crate::attachments::call_with_timeout(
                    &engine, cx.background_executor(), methods::FINISH_QUEUED_MESSAGE_EDIT,
                    params, std::time::Duration::from_secs(30),
                ).await.map_err(|err| err.to_string())
            }.await;
            this.update(cx, |composer, cx| {
                composer.queue_edit_finishing = false;
                composer.input.update(cx, |input, cx| { input.read_only = false; cx.notify(); });
                match result {
                    Ok(reply) => match reply.get("outcome").and_then(|v| v.as_str()) {
                        Some("committed" | "cancelled" | "discarded" | "released") => {
                            composer.clear_queue_edit_local(cx);
                            return;
                        }
                        Some("conflict") => {
                            composer.failure = Some(
                                "This message changed on another device; your edit was kept locally".into(),
                            );
                        }
                        Some("missing") => {
                            composer.failure = Some(
                                "The queued message was removed; your edit was kept locally".into(),
                            );
                        }
                        _ => {
                            composer.failure = Some(
                                "The edit lease changed; your text is still in the editor".into(),
                            );
                        }
                    },
                    Err(err) => {
                        tracing::warn!(error = %err, "finish queue edit failed");
                        composer.failure = Some(
                            "Couldn't reach the chat host; your edit is still in the editor".into(),
                        );
                    }
                }
                cx.notify();
            }).ok();
        });
        self.queue_edit_task = Some(task);
    }

    fn start_queue_edit_renewal(
        &mut self,
        engine: crate::state::EngineHandle,
        cx: &mut Context<Self>,
    ) {
        let (Some(id), Some(lease_id), Some(chat_id), Some(host_device_id)) = (
            self.editing_queued.clone(),
            self.queue_edit_lease_id.clone(),
            self.queue_edit_chat_id.clone(),
            self.queue_edit_host_device_id.clone(),
        ) else {
            return;
        };
        self.queue_edit_renew_task = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(20))
                    .await;
                let params = serde_json::json!({
                    "chatId": chat_id,
                    "id": id,
                    "leaseId": lease_id,
                    "targetDeviceId": host_device_id,
                });
                match engine
                    .client()
                    .call(methods::RENEW_QUEUED_MESSAGE_EDIT, params)
                    .await
                {
                    Ok(reply)
                        if reply.get("outcome").and_then(|v| v.as_str()) == Some("renewed") => {}
                    Ok(_) => {
                        this.update(cx, |composer, cx| {
                            composer.failure = Some(
                                "Edit protection expired; review this message before sending"
                                    .into(),
                            );
                            cx.notify();
                        })
                        .ok();
                        break;
                    }
                    Err(err) => {
                        tracing::debug!(error = %err, "queue edit heartbeat failed");
                        // A transient miss is tolerated by the 60s lease. Keep
                        // trying; the host fails closed if all attempts miss.
                    }
                }
            }
        }));
    }

    fn release_queue_edit_best_effort(&self, cx: &mut Context<Self>) {
        let (Some(id), Some(lease_id), Some(chat_id), Some(host_device_id), Some(engine)) = (
            self.editing_queued.clone(),
            self.queue_edit_lease_id.clone(),
            self.queue_edit_chat_id.clone(),
            self.queue_edit_host_device_id.clone(),
            self.state.read(cx).engine().cloned(),
        ) else {
            return;
        };
        let params = serde_json::json!({
            "chatId": chat_id,
            "id": id,
            "leaseId": lease_id,
            "action": "cancel",
            "targetDeviceId": host_device_id,
        });
        cx.spawn(async move |_, _| {
            let _ = engine
                .client()
                .call(methods::FINISH_QUEUED_MESSAGE_EDIT, params)
                .await;
        })
        .detach();
    }

    fn begin_canonical_queue_edit(
        &mut self,
        id: String,
        entry: zeron_proto::QueueUiEntry,
        cx: &mut Context<Self>,
    ) {
        let state = self.state.read(cx);
        let Some(chat) = self.target.chat(state) else {
            return;
        };
        if !state.chat_host_supports(&chat.id, zeron_proto::capabilities::CANONICAL_QUEUE_V1) {
            self.failure = Some("Update the chat host to manage queued runs".into());
            cx.notify();
            return;
        }
        self.queue_edit_chat_id = Some(chat.id.clone());
        self.queue_edit_host_device_id = Some(chat.device_id.clone());
        self.editing_queued = Some(id);
        let paths = entry.attachment_paths.clone();
        let host_device_id = chat.device_id.clone();
        let run_id = entry.queued_run_id.clone();
        self.canonical_queue_edit = Some(CanonicalQueueEdit {
            run_id: run_id.clone(),
            base_text: entry.text.clone(),
            request: None,
            attachment_count: entry.attachments.len(),
            expected_attachments: entry.attachment_fingerprint(),
            loaded: Vec::new(),
            attachments_editable: paths.is_empty(),
        });
        self.queue_edit_draft = Some((
            self.input.read(cx).text().to_string(),
            self.attachments
                .remove(&self.current_key)
                .unwrap_or_default(),
            self.appshots.remove(&self.current_key).unwrap_or_default(),
        ));
        self.queue_drag = None;
        self.focus_pending = true;
        self.input
            .update(cx, |input, cx| input.set_text(entry.text, cx));
        if !paths.is_empty() {
            self.load_canonical_edit_attachments(run_id, host_device_id, paths, cx);
        }
        cx.notify();
    }

    /// Read the row's uploads back so the strip can show, drop and replace
    /// them. A failed read leaves the edit text-only: the host keeps them.
    fn load_canonical_edit_attachments(
        &mut self,
        run_id: String,
        host_device_id: String,
        paths: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let mut loaded = Vec::new();
            for path in &paths {
                let image = crate::attachments::read_attachment_image(
                    &engine,
                    cx.background_executor(),
                    Some(&host_device_id),
                    path,
                    None,
                )
                .await;
                let Some(image) = image else {
                    loaded.clear();
                    break;
                };
                loaded.push((
                    path.clone(),
                    crate::attachments::StagedAttachment {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: image.name,
                        image: image.image,
                    },
                ));
            }
            this.update(cx, |composer, cx| {
                // The edit may have ended or moved to another row meanwhile.
                let Some(edit) = composer
                    .canonical_queue_edit
                    .as_mut()
                    .filter(|edit| edit.run_id == run_id)
                else {
                    return;
                };
                if loaded.len() == paths.len() {
                    edit.loaded = loaded
                        .iter()
                        .map(|(path, staged)| (staged.id.clone(), path.clone()))
                        .collect();
                    edit.attachments_editable = true;
                    composer
                        .attachments
                        .entry(composer.current_key.clone())
                        .or_default()
                        .extend(loaded.into_iter().map(|(_, staged)| staged));
                } else {
                    composer.failure = Some(
                        "Couldn't load the queued attachments; this edit changes text only and keeps them"
                            .into(),
                    );
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn finish_canonical_queue_edit(&mut self, text: String, cx: &mut Context<Self>) {
        if self.queue_edit_finishing {
            return;
        }
        if text.trim().is_empty() {
            self.failure =
                Some("Enter a message, or cancel the edit and remove it from the queue".into());
            cx.notify();
            return;
        }
        if !self.staged_appshots().is_empty() {
            self.failure =
                Some("Appshots can't be added to a queued message; remove them to save".into());
            cx.notify();
            return;
        }
        let staged = self.staged().to_vec();
        let (Some(chat_id), Some(host), Some(edit), Some(engine)) = (
            self.queue_edit_chat_id.clone(),
            self.queue_edit_host_device_id.clone(),
            self.canonical_queue_edit.as_mut(),
            self.state.read(cx).engine().cloned(),
        ) else {
            return;
        };
        if !edit.attachments_editable {
            if !staged.is_empty() {
                self.failure = Some("The queued attachments are still loading".into());
                cx.notify();
                return;
            }
        }
        let attachments_changed =
            edit.attachments_editable && staged_changed(&edit.loaded, &staged);
        let identity = format!(
            "{text}\0{}",
            staged
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(",")
        );
        if edit
            .request
            .as_ref()
            .is_none_or(|(previous, _)| *previous != identity)
        {
            edit.request = Some((identity, uuid::Uuid::new_v4().to_string()));
        }
        let request_id = edit.request.as_ref().unwrap().1.clone();
        let (run_id, base_text, expected) = (
            edit.run_id.clone(),
            edit.base_text.clone(),
            edit.expected_attachments.clone(),
        );
        let loaded = edit.loaded.clone();
        self.queue_edit_finishing = true;
        self.input.update(cx, |input, cx| {
            input.read_only = true;
            cx.notify();
        });
        cx.notify();
        self.queue_edit_task = Some(cx.spawn(async move |this, cx| {
            let result = async {
                let attachments = if attachments_changed {
                    // Upload identities derive from the pinned request, so a
                    // retry recommits the same files instead of new ones.
                    let mut paths = Vec::new();
                    for (index, attachment) in staged.iter().enumerate() {
                        if let Some((_, path)) = loaded.iter().find(|(id, _)| *id == attachment.id) {
                            paths.push(path.clone());
                            continue;
                        }
                        let path = crate::attachments::upload_attachment(
                            &engine, cx.background_executor(), Some(&host),
                            &format!("{request_id}-{index}"), attachment, None,
                        ).await.map_err(|error| error.to_string())?;
                        paths.push(path);
                    }
                    Some(zeron_proto::QueueAttachmentEdit { expected, paths, remove_ids: Vec::new() })
                } else {
                    None
                };
                engine.mutate_queued_run(zeron_proto::MutateQueuedRunParams {
                    chat_id,
                    queued_run_id: run_id,
                    client_request_id: request_id,
                    target_device_id: Some(host),
                    action: zeron_proto::QueuedRunAction::Edit {
                        text,
                        expected_text: base_text,
                        attachments,
                    },
                }).await.map_err(|error| error.to_string())
            }.await;
            this.update(cx, |composer, cx| {
                composer.queue_edit_finishing = false;
                composer.input.update(cx, |input, cx| { input.read_only = false; cx.notify(); });
                match result {
                    Ok(reply) if reply.refusal.is_none() => {
                        composer.clear_queue_edit_local(cx);
                    }
                    Ok(reply) => {
                        composer.failure = Some(format!("{} Your edit is still in the composer.", reply.refusal.unwrap()).into());
                    }
                    Err(error) => {
                        tracing::warn!(%error, "canonical queue edit failed");
                        composer.failure = Some("Couldn't reach the chat host; your edit is still in the composer. Save again to retry.".into());
                    }
                }
                cx.notify();
            }).ok();
        }));
    }

    fn canonical_queue_action(
        &mut self,
        message_id: String,
        run_id: String,
        action: zeron_proto::QueuedRunAction,
        cx: &mut Context<Self>,
    ) {
        // Single flight per queue (T3's `busyRunId`): a second mutation would
        // carry `before_run_id`s from the same stale snapshot as the first.
        if !self.queue_removing.is_empty() {
            return;
        }
        let state = self.state.read(cx);
        let (Some(chat), Some(engine)) = (self.target.chat(state), state.engine().cloned()) else {
            return;
        };
        if !state.chat_host_supports(&chat.id, zeron_proto::capabilities::CANONICAL_QUEUE_V1) {
            self.failure = Some("Update the chat host to manage queued runs".into());
            cx.notify();
            return;
        }
        let chat_id = chat.id.clone();
        let request = zeron_proto::MutateQueuedRunParams {
            chat_id: chat_id.clone(),
            queued_run_id: run_id,
            client_request_id: uuid::Uuid::new_v4().to_string(),
            target_device_id: Some(chat.device_id.clone()),
            action,
        };
        self.queue_removing.insert(message_id.clone());
        self.queue_drag = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            // One safe retry of the exact identity covers after-commit response
            // loss. Never retarget or mint another steering command. A definite
            // answer from the host is final; only an unknown outcome is replayed.
            let result = match engine.mutate_queued_run(request.clone()).await {
                Err(error) if crate::details_data::rpc_message_outcome_unknown(&error) => {
                    engine.mutate_queued_run(request).await
                }
                result => result,
            };
            this.update(cx, |composer, cx| {
                composer.queue_removing.remove(&message_id);
                let failure = match result {
                    Ok(reply) => reply.refusal,
                    Err(error) => {
                        tracing::warn!(%error, "canonical queue action failed");
                        Some(
                            "Couldn't update the queue; reconnect to the chat host and try again"
                                .into(),
                        )
                    }
                };
                if let Some(failure) = failure
                    && composer.target.chat_id(composer.state.read(cx)) == Some(chat_id.as_str())
                {
                    composer.failure = Some(failure.into());
                }
                composer
                    .state
                    .update(cx, |state, cx| state.refresh_chat_queue(&chat_id, cx));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Fire one queue mutation at the chat's doc host.
    fn queue_rpc(
        &mut self,
        method: &'static str,
        params: serde_json::Value,
        failure: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let (chat_id, host_device_id, host_supports_action) = {
            let state = self.state.read(cx);
            let Some(chat_id) = self.target.chat_id(state).map(str::to_owned) else {
                return;
            };
            let host = queue_action_needs_host(method)
                .then(|| self.target.chat(state).map(|chat| chat.device_id.clone()))
                .flatten();
            let supported = !queue_action_needs_host(method)
                || state.chat_host_supports(
                    &chat_id,
                    zeron_proto::capabilities::MESSAGE_QUEUE_ACTIONS_V1,
                );
            (chat_id, host, supported)
        };
        if !host_supports_action {
            self.failure = Some("The chat host does not support queue actions".into());
            cx.notify();
            return;
        }
        let pending_message = matches!(
            method,
            methods::SEND_QUEUED_MESSAGE_NOW | methods::STEER_QUEUED_MESSAGE_NOW
        )
        .then(|| {
            params
                .get("id")
                .and_then(|id| id.as_str())
                .map(str::to_owned)
        })
        .flatten();
        if let Some(id) = &pending_message {
            self.state.update(cx, |state, cx| {
                state.begin_pending_send(&chat_id, id, chrono::Utc::now());
                cx.notify();
            });
        }
        let mut params = params;
        if let Some(object) = params.as_object_mut() {
            object.insert("chatId".into(), serde_json::Value::String(chat_id.clone()));
            if let Some(host) = host_device_id {
                object.insert("targetDeviceId".into(), serde_json::Value::String(host));
            }
        }
        // Detached, not held: these are independent one-shot mutations, and
        // parking them in a single slot meant the next arrow tap dropped — and
        // so cancelled — the move still in flight, leaving the optimistic list
        // showing an order the doc never got.
        cx.spawn(
            async move |this, cx| match engine.client().call(method, params).await {
                Ok(reply) if queue_mutation_acknowledged(method, &reply) => {
                    // The host has adopted this message. Clear even if the
                    // user switched chats and its transcript is no longer watched.
                    if let Some(id) = &pending_message {
                        this.update(cx, |composer, cx| {
                            composer.state.update(cx, |state, cx| {
                                state.end_pending_send(&chat_id, id);
                                cx.notify();
                            });
                        })
                        .ok();
                    }
                }
                Ok(reply) => {
                    tracing::debug!(
                        method,
                        ?reply,
                        "queue mutation was not applied; reconciling"
                    );
                    this.update(cx, |composer, cx| {
                        if let Some(id) = &pending_message {
                            composer.state.update(cx, |state, cx| {
                                state.end_pending_send(&chat_id, id);
                                cx.notify();
                            });
                        }
                        composer
                            .state
                            .update(cx, |state, cx| state.refresh_chat_queue(&chat_id, cx));
                    })
                    .ok();
                }
                Err(err) => {
                    tracing::warn!(method, error = %err, "queue mutation failed");
                    this.update(cx, |composer, cx| {
                        if let Some(id) = &pending_message {
                            composer.state.update(cx, |state, cx| {
                                state.end_pending_send(&chat_id, id);
                                cx.notify();
                            });
                        }
                        if composer.target.chat_id(composer.state.read(cx))
                            == Some(chat_id.as_str())
                        {
                            composer.failure = Some(failure.into());
                        }
                        composer
                            .state
                            .update(cx, |state, cx| state.refresh_chat_queue(&chat_id, cx));
                        cx.notify();
                    })
                    .ok();
                }
            },
        )
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use zeron_rpc::methods;

    use super::{
        PANEL_PAD_TOP, QueuePrimaryAction, ROW_SLOT, available_queue_primary_action,
        canonical_primary_action, latest_queued_message, one_line, primary_tooltip, queue_action_needs_host,
        queue_drag_offsets, queue_drop_index, queue_latest_shortcut_visible,
        queue_mutation_acknowledged, queue_visible_text, visible_queue_rows,
    };

    #[test]
    fn canonical_rows_are_ordered_deduplicated_and_never_resurrect_consumed_intents() {
        use zeron_doc::QueuedMessage;
        use zeron_proto::{QueueUiEntry, QueueUiState};
        let entry = |id: &str, document_backed, automatic| QueueUiEntry {
            queued_run_id: format!("run:{id}"),
            message_id: id.into(),
            text: format!("canonical {id}"),
            document_backed,
            automatic,
            ..Default::default()
        };
        let canonical = QueueUiState {
            schema_version: 1,
            queue: vec![
                entry("completion", false, true),
                entry("agent", false, false),
                entry("typed", true, false),
                entry("consumed", true, false),
            ],
            ..Default::default()
        };
        let doc = vec![
            QueuedMessage::new("typed", "new local edit", "device"),
            QueuedMessage::new("not-admitted-yet", "pending", "device"),
        ];
        let rows = super::queue_rows_for_display(&doc, Some(&canonical));
        assert_eq!(
            rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            ["agent", "typed", "not-admitted-yet"]
        );
        assert_eq!(rows[1].text, "new local edit");
        assert_eq!(doc.len(), 2, "synthetic rows never become intents");
        assert_eq!(
            super::queue_rows_for_display(&doc, None),
            doc,
            "old hosts retain the legacy path"
        );
    }

    #[test]
    fn queue_contains_agrees_with_the_display_rows() {
        use zeron_doc::QueuedMessage;
        use zeron_proto::{QueueUiEntry, QueueUiState};
        let entry = |id: &str, document_backed, automatic| QueueUiEntry {
            message_id: id.into(),
            document_backed,
            automatic,
            ..Default::default()
        };
        let canonical = QueueUiState {
            schema_version: 1,
            queue: vec![
                entry("automatic", false, true),
                entry("sql-only", false, false),
                entry("typed", true, false),
                entry("consumed", true, false),
                entry("auto-in-doc", true, true),
            ],
            ..Default::default()
        };
        let doc = vec![
            QueuedMessage::new("typed", "t", "device"),
            QueuedMessage::new("not-admitted-yet", "p", "device"),
            QueuedMessage::new("auto-in-doc", "a", "device"),
        ];
        for canonical in [Some(&canonical), None] {
            let shown: Vec<_> = super::queue_rows_for_display(&doc, canonical)
                .into_iter()
                .map(|row| row.id)
                .collect();
            for id in [
                "automatic",
                "sql-only",
                "typed",
                "consumed",
                "not-admitted-yet",
                "auto-in-doc",
                "missing",
            ] {
                assert_eq!(
                    super::queue_contains(&doc, canonical, id),
                    shown.iter().any(|shown| shown == id),
                    "{id} (canonical: {})",
                    canonical.is_some()
                );
            }
        }
    }

    #[test]
    fn display_moves_resolve_by_id_against_the_document_slice() {
        use zeron_doc::QueuedMessage;
        let row = |id: &str| QueuedMessage::new(id, id, "device");
        // Document order a, b, c; the canonical display order is c, a, b.
        let doc = vec![row("a"), row("b"), row("c")];
        let shown = vec![row("c"), row("a"), row("b")];
        // `a` (display 1) before `c` (display 0): the final document order
        // must be b, a, c, not a swap of document slots 1 and 0.
        assert_eq!(
            super::doc_move_indices(&shown, &doc, 1, 0),
            Some(("a".into(), 0, 1))
        );
        // `c` (display 0) to the end: document slot 2 stays last.
        assert_eq!(
            super::doc_move_indices(&shown, &doc, 0, 2),
            Some(("c".into(), 2, 2))
        );
        // `b` (display 2) to the front, before `c`.
        assert_eq!(
            super::doc_move_indices(&shown, &doc, 2, 0),
            Some(("b".into(), 1, 1))
        );
        // A display row the document lacks cannot be moved through the doc.
        assert_eq!(super::doc_move_indices(&shown, &doc[..1], 0, 1), None);
    }

    #[test]
    fn canonical_queue_snapshots_are_thread_scoped_and_sequence_fenced() {
        use zeron_proto::QueueUiState;
        let mut state = crate::state::AppState::new();
        let snapshot = |id: &str, version| QueueUiState {
            thread_id: id.into(),
            version,
            ..Default::default()
        };
        state.apply_canonical_queue("pane", Some(snapshot("pane", 9)));
        state.apply_canonical_queue("pane", Some(snapshot("pane", 3)));
        state.apply_canonical_queue("pane", Some(snapshot("other", 20)));
        state.apply_canonical_queue("selected", Some(snapshot("selected", 11)));
        assert_eq!(state.canonical_queues["pane"].version, 9);
        assert_eq!(state.canonical_queues["selected"].version, 11);
        state.apply_canonical_queue("pane", None);
        assert_eq!(state.canonical_queues["pane"].version, 9);
    }

    #[gpui::test]
    fn edit_buttons_use_the_same_admission_as_the_composer(cx: &mut gpui::TestAppContext) {
        use gpui::AppContext as _;
        let state = cx.new(|_| crate::state::AppState::new());
        let composer = cx.new(|cx| super::Composer::new(state, cx));
        composer.update(cx, |composer, _| {
            assert!(composer.queue_edit_controls_ready());
            composer.editing_queued = Some("editing".into());
            assert!(!composer.queue_edit_controls_ready());
            composer.editing_queued = None;
            composer.queue_edit_pending_id = Some("acquiring".into());
            assert!(!composer.queue_edit_controls_ready());
            composer.queue_edit_pending_id = None;
            composer.queue_edit_finishing = true;
            assert!(!composer.queue_edit_controls_ready());
        });
    }

    #[gpui::test]
    fn draining_a_canonical_edit_retains_both_drafts(cx: &mut gpui::TestAppContext) {
        use gpui::AppContext as _;
        let state = cx.new(|_| {
            let mut state = crate::state::AppState::new();
            state.selected_chat = Some("source".into());
            state.canonical_queues.insert(
                "source".into(),
                zeron_proto::QueueUiState {
                    schema_version: 1,
                    thread_id: "source".into(),
                    queue: vec![zeron_proto::QueueUiEntry {
                        message_id: "queued".into(),
                        queued_run_id: "queued-run".into(),
                        text: "original work".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            );
            state
        });
        let composer = cx.new(|cx| super::Composer::new(state.clone(), cx));
        composer.update(cx, |composer, cx| {
            composer.editing_queued = Some("queued".into());
            composer.canonical_queue_edit = Some(super::CanonicalQueueEdit {
                run_id: "queued-run".into(),
                base_text: "original work".into(),
                request: None,
                attachment_count: 0,
                expected_attachments: String::new(),
                loaded: Vec::new(),
                attachments_editable: true,
            });
            composer.queue_edit_draft = Some(("previous draft".into(), vec![], vec![]));
            composer
                .input
                .update(cx, |input, cx| input.set_text("unsaved edit", cx));
        });
        state.update(cx, |state, cx| {
            state
                .canonical_queues
                .get_mut("source")
                .unwrap()
                .queue
                .clear();
            cx.notify();
        });
        cx.run_until_parked();
        composer.read_with(cx, |composer, cx| {
            assert!(composer.editing_queued.is_none());
            assert!(composer.canonical_queue_edit.is_none());
            assert_eq!(
                composer.input.read(cx).text(),
                "previous draft\n\nunsaved edit"
            );
        });
    }

    #[test]
    fn queue_preview_work_follows_the_visible_rows() {
        assert_eq!(visible_queue_rows(0.0, ROW_SLOT * 3.0, 1000), 0..4);
        assert_eq!(
            visible_queue_rows(-ROW_SLOT * 20.0, ROW_SLOT * 3.0, 1000),
            20..24
        );
        assert_eq!(
            visible_queue_rows(-ROW_SLOT * 20.0, ROW_SLOT * 3.0, 0),
            0..0
        );
    }

    #[test]
    fn available_primary_action_obeys_row_and_host_gates() {
        assert_eq!(
            available_queue_primary_action(false, true),
            Some(QueuePrimaryAction::SendNow)
        );
        assert_eq!(available_queue_primary_action(true, true), None);
        assert_eq!(available_queue_primary_action(false, false), None);
    }

    #[test]
    fn canonical_primary_action_names_interrupting_delivery_and_retains_old_host_fallback() {
        use zeron_proto::{QueuePromotionMode, QueueUiState};
        let mut queue = QueueUiState::default();
        assert_eq!(canonical_primary_action(&queue), None);
        queue.can_promote_to_steer = true;
        assert_eq!(
            canonical_primary_action(&queue),
            Some(QueuePrimaryAction::Steer)
        );
        queue.promotion_mode = Some(QueuePromotionMode::InterruptRestart);
        assert_eq!(
            canonical_primary_action(&queue),
            Some(QueuePrimaryAction::Restart)
        );
        assert_eq!(
            QueuePrimaryAction::Restart.tooltip(),
            "Send now (interrupt and restart)"
        );
        queue.can_promote_to_steer = false;
        queue.promotion_mode = Some(QueuePromotionMode::ActiveSteering);
        assert_eq!(
            canonical_primary_action(&queue),
            Some(QueuePrimaryAction::Steer)
        );
        queue.promotion_mode = None;
        assert_eq!(canonical_primary_action(&queue), None);
    }

    #[test]
    fn primary_tooltip_names_the_selection_the_host_will_run_and_why_it_is_unavailable() {
        use zeron_proto::{QueuePromotionMode, QueueUiState};
        let mut queue = QueueUiState::default();
        queue.promotion_mode = Some(QueuePromotionMode::InterruptRestartWithHandoff);
        assert_eq!(
            canonical_primary_action(&queue),
            Some(QueuePrimaryAction::RestartWithHandoff)
        );
        queue.promotion_selection = serde_json::from_value(
            serde_json::json!({"instanceId":"claude","model":"claude-opus"}),
        )
        .ok();
        assert_eq!(
            primary_tooltip(QueuePrimaryAction::RestartWithHandoff, true, Some(&queue)),
            "Send now on claude-opus (restart with handoff)"
        );
        assert_eq!(
            primary_tooltip(QueuePrimaryAction::Restart, true, Some(&queue)),
            "Send now on claude-opus (interrupt and restart)"
        );
        queue.promotion_selection_deferred = true;
        assert_eq!(
            primary_tooltip(QueuePrimaryAction::Steer, true, Some(&queue)),
            "Steer active response (the new model applies next turn)"
        );
        assert_eq!(
            primary_tooltip(QueuePrimaryAction::Steer, true, None),
            "Steer active response"
        );
        assert_eq!(
            primary_tooltip(QueuePrimaryAction::Steer, false, Some(&queue)),
            "Waiting for provider capabilities"
        );
        queue.promotion_blocked = Some("Send it after the current run.".into());
        assert_eq!(
            primary_tooltip(QueuePrimaryAction::Steer, false, Some(&queue)),
            "Send it after the current run."
        );
    }

    #[test]
    fn queue_shortcut_only_appears_on_the_actionable_latest_row_when_revealed() {
        assert!(!queue_latest_shortcut_visible(0, 2, true, true));
        assert!(queue_latest_shortcut_visible(1, 2, true, true));
        assert!(!queue_latest_shortcut_visible(1, 2, false, true));
        assert!(!queue_latest_shortcut_visible(1, 2, true, false));
        assert!(!queue_latest_shortcut_visible(0, 0, true, true));
    }

    #[test]
    fn queue_shortcut_targets_the_most_recently_added_row() {
        let items = vec![
            zeron_doc::QueuedMessage::new("older", "first", "device"),
            zeron_doc::QueuedMessage::new("newer", "second", "device"),
        ];
        assert_eq!(latest_queued_message(&items).unwrap().id, "newer");
        assert!(latest_queued_message(&[]).is_none());
    }

    #[test]
    fn host_authoritative_queue_actions_route_to_the_host() {
        assert!(queue_action_needs_host(methods::SEND_QUEUED_MESSAGE_NOW));
        assert!(queue_action_needs_host(methods::STEER_QUEUED_MESSAGE_NOW));
        assert!(queue_action_needs_host(methods::REMOVE_QUEUED_MESSAGE));
        assert!(queue_action_needs_host(methods::BEGIN_QUEUED_MESSAGE_EDIT));
        assert!(queue_action_needs_host(methods::RENEW_QUEUED_MESSAGE_EDIT));
        assert!(queue_action_needs_host(methods::FINISH_QUEUED_MESSAGE_EDIT));
        assert!(!queue_action_needs_host(methods::QUEUE_MESSAGE));
        assert!(!queue_action_needs_host(methods::UPDATE_QUEUED_MESSAGE));
        assert!(!queue_action_needs_host(methods::MOVE_QUEUED_MESSAGE));
    }

    #[test]
    fn mutation_acknowledgements_detect_conflicts_and_malformed_replies() {
        assert!(queue_mutation_acknowledged(
            methods::MOVE_QUEUED_MESSAGE,
            &serde_json::json!({ "changed": true })
        ));
        assert!(!queue_mutation_acknowledged(
            methods::MOVE_QUEUED_MESSAGE,
            &serde_json::json!({ "changed": false })
        ));
        assert!(!queue_mutation_acknowledged(
            methods::REMOVE_QUEUED_MESSAGE,
            &serde_json::json!({})
        ));
        assert!(queue_mutation_acknowledged(
            methods::SEND_QUEUED_MESSAGE_NOW,
            &serde_json::json!({ "sent": true })
        ));
    }

    #[test]
    fn the_whole_panel_maps_to_a_clamped_queue_drop_slot() {
        assert_eq!(queue_drop_index(0.0, 2), 0, "top pad targets the head");
        assert_eq!(queue_drop_index(PANEL_PAD_TOP + ROW_SLOT - 0.1, 2), 0);
        assert_eq!(queue_drop_index(PANEL_PAD_TOP + ROW_SLOT, 2), 1);
        assert_eq!(queue_drop_index(10_000.0, 2), 1);
    }

    #[test]
    fn drag_offsets_move_the_real_row_and_open_its_destination() {
        assert_eq!(queue_drag_offsets(0, 0, 0, 2), (0.0, 2.0 * ROW_SLOT));
        assert_eq!(queue_drag_offsets(1, 0, 0, 2), (0.0, -ROW_SLOT));
        assert_eq!(queue_drag_offsets(2, 0, 0, 2), (0.0, -ROW_SLOT));

        // Moving the pointer back one slot restarts only the rows whose
        // visual destination actually changed.
        assert_eq!(queue_drag_offsets(0, 0, 2, 1), (2.0 * ROW_SLOT, ROW_SLOT));
        assert_eq!(queue_drag_offsets(1, 0, 2, 1), (-ROW_SLOT, -ROW_SLOT));
        assert_eq!(queue_drag_offsets(2, 0, 2, 1), (-ROW_SLOT, 0.0));
    }

    /// A row is one line tall, so a multi-line message has to read as one line
    /// — otherwise the panel's rows stop lining up.
    #[test]
    fn rows_flatten_multi_line_messages() {
        assert_eq!(
            one_line("fix the test\n\nthen ship it").as_ref(),
            "fix the test then ship it"
        );
        assert_eq!(one_line("  spaced   out  ").as_ref(), "spaced out");
    }

    #[test]
    fn context_references_render_as_labels_and_malformed_links_stay_typed() {
        let text = "See [build.log](t3-context://v1/terminal/ctx_1) and ![shot.png](t3-context://v1/image/img_2) now";
        assert_eq!(
            super::replace_context_references(text, false),
            "See build.log and shot.png now"
        );
        assert_eq!(
            super::replace_context_references(text, true),
            "See build.log and  now",
            "an image chip vanishes when the row lists its attachments"
        );
        for typed in [
            "[x](t3-context://v1/terminal)",
            "[x](t3-context://v1/terminal/a/b)",
            "[unclosed](t3-context://v1/terminal/a",
            "[two\nlines](t3-context://v1/skill/a)",
            "plain [link](https://example.com)",
            "](t3-context://v1/skill/a)",
        ] {
            assert_eq!(super::replace_context_references(typed, true), typed);
        }
        assert_eq!(
            super::replace_context_references(
                "[a](t3-context://v1/skill/s1)[b](t3-context://v1/mention/m1)",
                false
            ),
            "ab"
        );
    }

    #[test]
    fn retained_context_chips_name_every_kind_without_repeating_attachments() {
        let record = |kind: &str, label: &str, detail: &str| zeron_proto::QueueContextRef {
            kind: kind.into(),
            label: label.into(),
            detail: detail.into(),
            ..Default::default()
        };
        let chips = super::queue_context_chips(&[
            record("image", "shot.png", "shot.png"),
            record("file", "notes", "notes.txt"),
            record("terminal", "tail", "zsh L10-20"),
            record("element", "button", "button #save"),
            record("preview-annotation", "mark", "Header is cramped"),
            record("review-comment", "comment", "src/lib.rs lines 3-4"),
            record("mention", "lib", "src/lib.rs"),
            record("skill", "x", "frontend-design"),
            record("thread", "Parent", "Parent thread"),
            record("future-kind", "Opaque", ""),
            record("empty", "", ""),
        ]);
        assert_eq!(
            chips,
            [
                "terminal zsh L10-20",
                "element button #save",
                "preview-annotation Header is cramped",
                "review-comment src/lib.rs lines 3-4",
                "mention src/lib.rs",
                "skill frontend-design",
                "thread Parent thread",
                "future-kind Opaque",
                "empty",
            ]
        );
    }

    #[test]
    fn staged_attachment_changes_are_detected_by_identity_and_order() {
        use crate::attachments::StagedAttachment;
        use gpui::{Image, ImageFormat};
        let staged = |id: &str| StagedAttachment {
            id: id.into(),
            name: format!("{id}.png"),
            image: std::sync::Arc::new(Image::from_bytes(ImageFormat::Png, Vec::new())),
        };
        let loaded = vec![
            ("a".to_string(), "/p/a.png".to_string()),
            ("b".into(), "/p/b.png".into()),
        ];
        assert!(!super::staged_changed(&loaded, &[staged("a"), staged("b")]));
        assert!(
            super::staged_changed(&loaded, &[staged("b"), staged("a")]),
            "reorder"
        );
        assert!(super::staged_changed(&loaded, &[staged("a")]), "removal");
        assert!(
            super::staged_changed(&loaded, &[staged("a"), staged("b"), staged("new")]),
            "addition"
        );
        assert!(
            super::staged_changed(&loaded, &[staged("a"), staged("new")]),
            "replacement"
        );
        assert!(!super::staged_changed(&[], &[]));
    }

    #[gpui::test]
    fn canonical_edit_blocks_attaching_only_until_its_uploads_are_loaded(
        cx: &mut gpui::TestAppContext,
    ) {
        use gpui::AppContext as _;
        let state = cx.new(|_| crate::state::AppState::new());
        let composer = cx.new(|cx| super::Composer::new(state.clone(), cx));
        composer.update(cx, |composer, cx| {
            composer.canonical_queue_edit = Some(super::CanonicalQueueEdit {
                run_id: "run".into(),
                base_text: "work".into(),
                request: None,
                attachment_count: 0,
                expected_attachments: "claims=|paths=/p/a.png".into(),
                loaded: Vec::new(),
                attachments_editable: false,
            });
            composer.add_paths(vec![std::path::PathBuf::from("/nope/b.png")], cx);
            assert!(
                composer.failure.is_some(),
                "still loading: staging is refused with a notice"
            );
            composer.failure = None;
            composer
                .canonical_queue_edit
                .as_mut()
                .unwrap()
                .attachments_editable = true;
            assert!(
                composer
                    .canonical_queue_edit
                    .as_ref()
                    .unwrap()
                    .attachments_editable()
            );
            composer.add_paths(vec![std::path::PathBuf::from("/nope/b.png")], cx);
            assert!(
                composer.failure.is_none(),
                "an editable edit stages normally"
            );
        });
    }

    #[test]
    fn appshot_context_is_hidden_in_clean_and_legacy_queue_rows() {
        let shot = crate::appshots::tests::shot();
        let paths: Vec<String> = vec!["/host/image.png".into()];
        for user_text in ["inspect this", ""] {
            let body = crate::appshots::with_appshots(
                user_text,
                &[shot.clone()],
                &std::collections::HashMap::from([(shot.screenshot.id.clone(), paths[0].clone())]),
            );
            let expected = if user_text.is_empty() {
                crate::attachments::ATTACHMENT_ONLY_TEXT
            } else {
                user_text
            };
            assert_eq!(queue_visible_text(&body, &paths), expected);
            assert_eq!(
                queue_visible_text(&crate::attachments::with_attachments(&body, &paths), &paths),
                expected
            );
        }
    }

    #[test]
    fn attachment_labels_decode_app_names_and_preserve_ordinary_images() {
        let shot = crate::appshots::tests::shot();
        let paths = vec![
            "/tmp/shot & detail.png".to_owned(),
            "/tmp/reference.png".to_owned(),
        ];
        let mut shot = shot;
        shot.app_name = "Notes & Ideas".into();
        let body = crate::appshots::with_appshots(
            "look",
            &[shot.clone()],
            &[(shot.screenshot.id.clone(), paths[0].clone())]
                .into_iter()
                .collect(),
        );
        assert_eq!(
            super::queue_attachment_labels(&body, &paths),
            vec!["Notes & Ideas Appshot", "reference.png"]
        );
        assert_eq!(
            super::queue_attachment_labels(&body, &["/tmp/other.png".into()]),
            vec!["other.png"]
        );
        let malformed = format!("\n\n{}\n<appshot", crate::appshots::CONTEXT_MARKER);
        assert_eq!(
            super::queue_attachment_labels(&malformed, &paths),
            vec!["shot & detail.png", "reference.png"]
        );
    }

    #[test]
    fn queue_rows_show_file_labels_without_changing_delivery_text() {
        let link = "[queue.rs](zeron-file:src/queue.rs)";
        let text = format!("inspect\n{link}");
        assert_eq!(
            super::queue_row_text(&text, &[]).as_ref(),
            "inspect @queue.rs"
        );
        assert!(text.contains("zeron-file:"));
        let paths = vec!["/tmp/image.png".to_string()];
        let legacy = crate::attachments::with_attachments(&text, &paths);
        assert_eq!(
            super::queue_row_text(&legacy, &paths).as_ref(),
            "inspect @queue.rs"
        );
        assert_eq!(
            super::queue_row_text("plain  text", &[]).as_ref(),
            "plain text"
        );
    }

    #[test]
    fn legacy_attachment_trailers_are_hidden_from_queue_text() {
        let paths = vec!["/tmp/image.png".to_string()];
        let legacy = crate::attachments::with_attachments("inspect this", &paths);
        assert_eq!(queue_visible_text(&legacy, &paths), "inspect this");

        let image_only = crate::attachments::with_attachments("", &paths);
        assert_eq!(
            queue_visible_text(&image_only, &paths),
            crate::attachments::ATTACHMENT_ONLY_TEXT
        );
        assert_eq!(
            queue_visible_text("literal user text", &paths),
            "literal user text"
        );
    }
}

#[cfg(test)]
mod scroll_tests {
    use super::*;
    use gpui::{ScrollHandle, TestAppContext, point};

    struct QueueScrollTestView {
        queue: ScrollHandle,
        transcript: ScrollHandle,
        count: usize,
    }

    impl Render for QueueScrollTestView {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(
                    div()
                        .id("transcript-underlay")
                        .absolute()
                        .inset_0()
                        .overflow_y_scroll()
                        .track_scroll(&self.transcript)
                        .child(div().h(px(2000.0))),
                )
                .child(
                    div().absolute().bottom_0().w_full().child(
                        queue_panel_surface(Theme::of(cx)).child(queue_rows(
                            &self.queue,
                            px(180.0),
                            (0..self.count)
                                .map(|_| div().h(px(ROW_HEIGHT)).flex_none().into_any_element()),
                        )),
                    ),
                )
        }
    }

    #[gpui::test]
    fn queue_wheel_does_not_scroll_the_transcript_even_at_its_boundaries(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_global(Theme::default()));
        let (view, cx) = cx.add_window_view(|_, _| QueueScrollTestView {
            queue: ScrollHandle::new(),
            transcript: ScrollHandle::new(),
            count: 24,
        });
        cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
        cx.run_until_parked();
        let (queue, transcript) =
            view.read_with(cx, |view, _| (view.queue.clone(), view.transcript.clone()));
        for delta in [-80.0, -10_000.0, -80.0, 10_000.0, 80.0] {
            cx.simulate_event(gpui::ScrollWheelEvent {
                position: point(px(200.0), px(300.0)),
                delta: gpui::ScrollDelta::Pixels(point(px(0.0), px(delta))),
                ..Default::default()
            });
            cx.run_until_parked();
            assert_eq!(
                transcript.offset().y,
                px(0.0),
                "queue wheel leaked to transcript"
            );
            if delta == -80.0 {
                assert!(queue.offset().y < px(0.0), "the queue must still scroll");
            }
        }
        view.update(cx, |view, cx| {
            view.count = 2;
            cx.notify();
        });
        cx.run_until_parked();
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: point(px(200.0), px(370.0)),
            delta: gpui::ScrollDelta::Pixels(point(px(0.0), px(-80.0))),
            ..Default::default()
        });
        cx.run_until_parked();
        assert_eq!(transcript.offset().y, px(0.0));
        assert_eq!(queue.max_offset().y, px(0.0));
    }
}

#[cfg(test)]
mod appshot_edit_tests {
    use super::*;
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn restoring_displaced_draft_keeps_appshots_and_new_captures(cx: &mut TestAppContext) {
        let state = cx.new(|_| crate::state::AppState::new());
        let composer = cx.new(|cx| Composer::new(state, cx));
        composer.update(cx, |composer, cx| {
            let original = crate::appshots::tests::shot();
            let mut during_save = original.clone();
            during_save.id = "during-save".into();
            composer.queue_edit_draft = Some(("draft".into(), vec![], vec![original]));
            composer.editing_queued = Some("row".into());
            composer.queue_edit_finishing = true;
            composer.stage_appshot(during_save, cx);
            assert!(composer.staged_appshots().is_empty());
            composer.clear_queue_edit_local(cx);
            assert_eq!(composer.input.read(cx).text(), "draft");
            assert_eq!(composer.staged_appshots().len(), 2);
            assert_eq!(composer.staged_appshots()[1].id, "during-save");
            assert!(composer.editing_queued.is_none());
        });
    }
}
