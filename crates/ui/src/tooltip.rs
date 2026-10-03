//! The one tooltip surface (R5 §3.7).
//!
//! Every hover note in the app paints through [`surface`]: 8px / 5px padding,
//! radius 8, a `border` hairline, the shared frosted popup tint, 12px `text`.
//! Icon buttons, chips and rows pick a content shape and never restyle the
//! card:
//!
//! - [`text`] / [`text_above`] / [`shortcut`] / [`mono`] — one line, the
//!   `.tooltip(...)` builders almost every call site wants;
//! - [`Lines`] — a bold title over muted detail lines (PR, project, history).
//!
//! Types that need data of their own (the PR badge card, the mention path
//! chip) build on [`surface`] directly and wrap with [`frost`].

use std::time::Duration;

use gpui::{
    AnyElement, AnyView, App, Context, Div, FontWeight, Hsla, IntoElement, Render, SharedString,
    Window, div, prelude::*, px,
};

use crate::controls;
use crate::theme::Theme;

/// Hover delay before a tooltip appears. Icon buttons set it explicitly so a
/// pointer crossing a toolbar does not strobe notes.
pub const SHOW_DELAY: Duration = Duration::from_millis(350);
pub const RADIUS: f32 = 8.0;
pub const MAX_WIDTH: f32 = 320.0;
/// Path and code notes run wider and mono (T3's `code` variant).
pub const MAX_WIDTH_MONO: f32 = 480.0;
const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 5.0;
const TEXT_SIZE: f32 = 12.0;
const MONO_SIZE: f32 = 11.0;
/// How far a [`text_above`] note lifts so it clears the row under the pointer.
const ABOVE_LIFT: f32 = 20.0;

/// The tooltip card without its blur: padding, radius, border, tint, text and
/// (on opaque surfaces) one soft shadow layer. Call [`frost`] on the finished
/// card so glass themes get the backdrop blur.
pub fn surface(theme: &Theme) -> Div {
    div()
        .max_w(px(MAX_WIDTH))
        .px(px(PAD_X))
        .py(px(PAD_Y))
        .rounded(px(RADIUS))
        .border_1()
        .border_color(theme.border)
        .bg(crate::popover::surface_bg(theme))
        .text_size(crate::typography::ui_rems(TEXT_SIZE))
        .text_color(theme.text)
        .when(!theme.is_frost(), |card| {
            card.shadow(crate::elevation::tooltip_shadow(theme))
        })
}

/// Wrap a finished [`surface`] in its own scene layer with the shared backdrop
/// blur. `frosted` is load-bearing beyond the blur: it keeps the card above
/// transcript text that shares the window layer.
pub fn frost(card: Div) -> AnyElement {
    crate::frost::frosted(RADIUS, crate::frost::MENU_BLUR, card).into_any_element()
}

/// A mono 11px line for paths, refs and other code-adjacent notes.
pub fn mono_line(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .min_w_0()
        .truncate()
        .whitespace_nowrap()
        .font_family(theme.font_mono.clone())
        .text_size(px(MONO_SIZE))
        .text_color(theme.text_muted)
        .child(text.into())
}

/// A bold-ish title over muted detail lines: the PR, project and device cards.
pub struct Lines {
    title: SharedString,
    title_color: Option<Hsla>,
    details: Vec<SharedString>,
}

impl Lines {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            title_color: None,
            details: Vec::new(),
        }
    }

    pub fn title_color(mut self, color: Hsla) -> Self {
        self.title_color = Some(color);
        self
    }

    pub fn detail(mut self, line: impl Into<SharedString>) -> Self {
        self.details.push(line.into());
        self
    }
}

impl Render for Lines {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &Theme::of(cx).for_popup();
        let card = surface(theme)
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .whitespace_nowrap()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(self.title_color.unwrap_or(theme.text))
                    .child(self.title.clone()),
            )
            .children(self.details.iter().cloned().map(|line| {
                div()
                    .min_w_0()
                    .truncate()
                    .whitespace_nowrap()
                    .text_color(theme.text_muted)
                    .child(line)
            }));
        frost(card)
    }
}

/// The one-line tooltip view behind the [`text`] family.
pub struct Tip {
    text: SharedString,
    shortcut: Option<SharedString>,
    mono: bool,
    above: bool,
}

impl Tip {
    fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            shortcut: None,
            mono: false,
            above: false,
        }
    }
}

impl Render for Tip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &Theme::of(cx).for_popup();
        let body = if self.mono {
            mono_line(theme, self.text.clone())
                .text_color(theme.text)
                .into_any_element()
        } else {
            div().min_w_0().child(self.text.clone()).into_any_element()
        };
        let mut card = surface(theme)
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .child(body);
        if self.mono {
            card = card.max_w(px(MAX_WIDTH_MONO));
        }
        if let Some(keys) = self.shortcut.clone() {
            card = card.child(controls::kbd(theme, keys));
        }
        let card = frost(card);
        if !self.above {
            return card;
        }
        // Overflow upward so the note doesn't cover the next sidebar row.
        div()
            .h(px(0.0))
            .flex()
            .flex_col()
            .justify_end()
            .child(div().relative().bottom(px(ABOVE_LIFT)).child(card))
            .into_any_element()
    }
}

fn builder(
    make: impl Fn() -> Tip + 'static,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_, cx| cx.new(|_| make()).into()
}

/// `.tooltip(tooltip::text("Archive session"))`.
pub fn text(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text: SharedString = text.into();
    builder(move || Tip::new(text.clone()))
}

/// [`text`], lifted above the pointer so it clears the row below (sidebar rows).
pub fn text_above(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text: SharedString = text.into();
    builder(move || {
        let mut tip = Tip::new(text.clone());
        tip.above = true;
        tip
    })
}

/// [`text`] with a trailing key chip: `tooltip::shortcut("Send", "↵")`.
pub fn shortcut(
    text: impl Into<SharedString>,
    keys: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text: SharedString = text.into();
    let keys: SharedString = keys.into();
    builder(move || {
        let mut tip = Tip::new(text.clone());
        tip.shortcut = Some(keys.clone());
        tip
    })
}

/// Mono one-liner for paths and other code-adjacent text.
pub fn mono(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text: SharedString = text.into();
    builder(move || {
        let mut tip = Tip::new(text.clone());
        tip.mono = true;
        tip
    })
}

/// A title-and-details card built per hover.
pub fn lines(
    make: impl Fn() -> Lines + 'static,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_, cx| cx.new(|_| make()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_surface_spec_is_the_documented_one() {
        assert_eq!((PAD_X, PAD_Y, RADIUS), (8.0, 5.0, 8.0));
        assert_eq!(TEXT_SIZE, 12.0);
        assert!(MAX_WIDTH_MONO > MAX_WIDTH);
    }
}
