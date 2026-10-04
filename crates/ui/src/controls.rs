//! Shared button and key-chip primitives (R5 §3.7).
//!
//! One vocabulary for chrome controls: `Xs / Sm / Md` sizes crossed with
//! `Ghost / Outline / Primary / Destructive` variants, text or icon-only.
//! Hover rides [`motion::hover_blend_owned`] keyed by the render owner's
//! entity and the control's id. Callers initialize and drive that owner with
//! [`motion::init_hover_owner`] / [`motion::drive_hover_owner`], pass ids that
//! are unique and stable within it, and must NOT attach another `.on_hover`
//! (gpui keeps one listener per element).
//!
//! Colour comes from [`Theme`]'s role accessors so the primary action takes the theme's
//! `action` role once D1 lands and stays monochrome until then.

use gpui::{
    Div, EntityId, FontWeight, Hsla, SharedString, Stateful, div, hsla, point, prelude::*, px,
};

use crate::elevation;
use crate::icons::{self, icon};
use crate::motion;
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// Quiet: muted text, a `control_hover` plate on hover. Toolbars, headers.
    Ghost,
    /// Bordered: `border_strong` outline, otherwise ghost behaviour.
    Outline,
    /// The one filled action in a surface: `action` plate.
    Primary,
    /// Destructive confirmation: the danger plate.
    Destructive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    /// 20px: tab-strip adders and other in-chip affordances.
    Micro,
    /// 24px: pane headers, dense toolbars.
    Xs,
    /// 28px: composer rows, inline actions.
    Sm,
    /// 32px: dialog and settings buttons.
    Md,
}

impl Size {
    pub const fn height(self) -> f32 {
        match self {
            Self::Micro => 20.0,
            Self::Xs => 24.0,
            Self::Sm => 28.0,
            Self::Md => 32.0,
        }
    }

    const fn pad_x(self) -> f32 {
        match self {
            Self::Micro => 6.0,
            Self::Xs => 8.0,
            Self::Sm => 10.0,
            Self::Md => 12.0,
        }
    }

    const fn text(self) -> f32 {
        match self {
            Self::Micro | Self::Xs => 12.0,
            Self::Sm | Self::Md => 13.0,
        }
    }

    const fn icon(self) -> f32 {
        match self {
            Self::Micro | Self::Xs => 14.0,
            Self::Sm | Self::Md => 16.0,
        }
    }

    pub const fn radius(self) -> f32 {
        match self {
            Self::Micro => 5.0,
            Self::Xs => 6.0,
            Self::Sm | Self::Md => 8.0,
        }
    }
}

/// Disabled controls share one dimming (T3 uses .64; .5 reads clearer on the
/// warm palettes where muted text is already soft).
pub const DISABLED_OPACITY: f32 = 0.5;

struct Palette {
    rest_bg: Hsla,
    hover_bg: Hsla,
    rest_fg: Hsla,
    hover_fg: Hsla,
    pressed_bg: Hsla,
    border: Option<Hsla>,
}

fn palette(theme: &Theme, variant: Variant) -> Palette {
    let clear = crate::theme::wash(0.0);
    match variant {
        Variant::Ghost => Palette {
            rest_bg: clear,
            hover_bg: theme.control_hover(),
            rest_fg: theme.text_muted,
            hover_fg: theme.text,
            pressed_bg: elevation::control_pressed(theme),
            border: None,
        },
        Variant::Outline => Palette {
            rest_bg: clear,
            hover_bg: theme.control_hover(),
            rest_fg: theme.text,
            hover_fg: theme.text,
            pressed_bg: elevation::control_pressed(theme),
            border: Some(theme.border_strong),
        },
        Variant::Primary => Palette {
            rest_bg: theme.action(),
            hover_bg: theme.action_hover(),
            rest_fg: theme.on_action(),
            hover_fg: theme.on_action(),
            pressed_bg: theme.action_hover(),
            border: None,
        },
        Variant::Destructive => Palette {
            rest_bg: theme.danger_strong,
            hover_bg: theme.danger_strong.opacity(0.9),
            rest_fg: gpui::white(),
            hover_fg: gpui::white(),
            pressed_bg: theme.danger_strong.opacity(0.8),
            border: None,
        },
    }
}

/// Filled variants carry a 1px top highlight so the plate reads as proud.
fn plate_shadows(variant: Variant) -> Vec<gpui::BoxShadow> {
    match variant {
        Variant::Primary | Variant::Destructive => vec![gpui::BoxShadow {
            color: hsla(0.0, 0.0, 1.0, 0.16),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        }],
        _ => Vec::new(),
    }
}

fn frame(
    id: SharedString,
    owner: EntityId,
    theme: &Theme,
    variant: Variant,
    size: Size,
) -> (Stateful<Div>, Palette) {
    let p = palette(theme, variant);
    let mut el = div()
        .id(id.clone())
        .role(gpui::Role::Button)
        .flex_none()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .rounded(px(size.radius()))
        .cursor_pointer()
        .bg(motion::hover_blend_owned(owner, &id, p.rest_bg, p.hover_bg))
        .text_color(motion::hover_blend_owned(owner, &id, p.rest_fg, p.hover_fg))
        .active(|s| s.bg(p.pressed_bg))
        .focus_visible(|s| s.shadow(elevation::focus_ring(theme)));
    let shadows = plate_shadows(variant);
    if !shadows.is_empty() {
        el = el.shadow(shadows);
    }
    if let Some(border) = p.border {
        el = el.border_1().border_color(border);
    }
    el.interactivity()
        .on_hover(motion::hover_listener_owned(owner, id));
    (el, p)
}

/// A text button. Chain `.on_click(..)` on the result; add a leading glyph with
/// [`with_icon`].
pub fn button(
    id: impl Into<SharedString>,
    owner: EntityId,
    theme: &Theme,
    variant: Variant,
    size: Size,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    let (el, _) = frame(id.into(), owner, theme, variant, size);
    el.h(px(size.height()))
        .px(px(size.pad_x()))
        .gap(px(6.0))
        .text_size(crate::typography::ui_rems(size.text()))
        .font_weight(match variant {
            Variant::Primary | Variant::Destructive => FontWeight::MEDIUM,
            _ => FontWeight::NORMAL,
        })
        .child(label.into())
}

/// A square icon-only button. `label` is the accessible name; pair it with
/// [`crate::tooltip::text`] at the call site when the glyph is not obvious.
pub fn icon_button(
    id: impl Into<SharedString>,
    owner: EntityId,
    theme: &Theme,
    variant: Variant,
    size: Size,
    glyph: &'static str,
    label: &'static str,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let (el, p) = frame(id.clone(), owner, theme, variant, size);
    let tint = motion::hover_blend_owned(owner, &id, p.rest_fg, p.hover_fg);
    el.size(px(size.height()))
        .aria_label(label)
        .child(icon(glyph).size(px(size.icon())).text_color(tint))
}

/// Keyboard-key chip (`kbd`): 20px tall, 4px radius, 4px padding, 11px mono,
/// the shared secondary plate. Menu shortcuts and tooltips use this; the
/// palette footer's larger `popover::key_cap` stays its own cloth.
pub fn kbd(theme: &Theme, label: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .h(px(20.0))
        .min_w(px(20.0))
        .px(px(4.0))
        .rounded(px(4.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(theme.muted())
        .font_family(theme.font_mono.clone())
        .text_size(px(11.0))
        .text_color(theme.text_muted)
        .child(label.into())
}

/// Close affordance for dialogs and banners: a ghost `Xs` icon button.
pub fn close_button(id: impl Into<SharedString>, owner: EntityId, theme: &Theme) -> Stateful<Div> {
    icon_button(
        id,
        owner,
        theme,
        Variant::Ghost,
        Size::Xs,
        icons::CLOSE,
        "Close",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn chrome_controls_notify_only_their_hover_owner(cx: &mut gpui::TestAppContext) {
        struct Owner;
        impl gpui::Render for Owner {
            fn render(
                &mut self,
                window: &mut gpui::Window,
                cx: &mut gpui::Context<Self>,
            ) -> impl IntoElement {
                let theme = Theme::dark();
                let owner = cx.entity_id();
                let content = div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_start()
                    .child(button(
                        "text-control",
                        owner,
                        &theme,
                        Variant::Ghost,
                        Size::Md,
                        "Parent",
                    ))
                    .child(icon_button(
                        "icon-control",
                        owner,
                        &theme,
                        Variant::Ghost,
                        Size::Md,
                        icons::PLUS,
                        "Add",
                    ))
                    .child(close_button("close-control", owner, &theme));
                motion::drive_hover_owner(owner, window);
                content
            }
        }
        cx.update(|cx| motion::set_reduced_motion(cx, true));
        let first = cx.add_window(|_, cx| {
            motion::init_hover_owner(cx);
            Owner
        });
        let second = cx.add_window(|_, cx| {
            motion::init_hover_owner(cx);
            Owner
        });
        let first_id = cx.update(|cx| first.entity(cx).unwrap().entity_id());
        let second_id = cx.update(|cx| second.entity(cx).unwrap().entity_id());
        let second_notifications = std::rc::Rc::new(std::cell::Cell::new(0));
        let observed = second_notifications.clone();
        let _subscription = cx.update(|cx| {
            cx.observe(&second.entity(cx).unwrap(), move |_, _| {
                observed.set(observed.get() + 1);
            })
        });
        for (key, y) in [
            ("text-control", 8.0),
            ("icon-control", 40.0),
            ("close-control", 72.0),
        ] {
            cx.update_window(first.into(), |_, window, cx| {
                window.draw(cx).clear();
                window.dispatch_event(
                    gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                        position: point(px(8.0), px(y)),
                        ..Default::default()
                    }),
                    cx,
                );
            })
            .unwrap();
            cx.run_until_parked();
            assert_eq!(motion::hover_t_owned(first_id, key), 1.0, "{key}");
            assert_eq!(motion::hover_t_owned(second_id, key), 0.0, "{key}");
            assert_eq!(
                motion::hover_t(key),
                0.0,
                "no window-wide hover state for {key}"
            );
            assert_eq!(second_notifications.get(), 0, "{key}");
        }
    }

    #[test]
    fn sizes_follow_the_documented_ladder() {
        assert_eq!(Size::Micro.height(), 20.0);
        assert_eq!(Size::Xs.height(), 24.0);
        assert_eq!(Size::Sm.height(), 28.0);
        assert_eq!(Size::Md.height(), 32.0);
        // Icons never outgrow their button.
        for size in [Size::Micro, Size::Xs, Size::Sm, Size::Md] {
            assert!(size.icon() < size.height());
        }
    }

    #[test]
    fn filled_variants_carry_a_highlight_and_ghosts_do_not() {
        assert_eq!(plate_shadows(Variant::Primary).len(), 1);
        assert_eq!(plate_shadows(Variant::Destructive).len(), 1);
        assert!(plate_shadows(Variant::Ghost).is_empty());
        assert!(plate_shadows(Variant::Outline).is_empty());
    }

    #[test]
    fn only_outline_has_a_border() {
        let theme = Theme::dark();
        assert!(palette(&theme, Variant::Outline).border.is_some());
        assert!(palette(&theme, Variant::Ghost).border.is_none());
        assert!(palette(&theme, Variant::Primary).border.is_none());
    }
}
