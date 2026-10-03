//! Elevation recipes and interaction fills the theme roles do not cover.
//!
//! Colour roles (action, control hover, sidebar tiers, placeholder, composer
//! outline, ...) live on [`Theme`]; this module holds the shadow stacks
//! (R5 §2.7), the focus ring and the pressed fill that chrome shares.

use gpui::{BoxShadow, Hsla, hsla, point, px};

use crate::motion::mix;
use crate::theme::{Appearance, Theme};

/// Pressed fill for ghost/outline controls: one step past the hover fill.
pub(crate) fn control_pressed(theme: &Theme) -> Hsla {
    let hover = theme.control_hover();
    if hover.a < 0.99 {
        Hsla {
            a: (hover.a * 1.6).min(0.30),
            ..hover
        }
    } else {
        mix(hover, theme.text, 0.06)
    }
}

/// Inset focus ring: an edge-only accent line, so it costs no layout and never
/// paints behind a translucent fill (drop shadows do; see
/// [`crate::theme::card_selected_shadows`]).
pub(crate) fn focus_ring(theme: &Theme) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: theme.accent,
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: px(2.0),
        inset: true,
    }]
}

/// One drop-shadow layer in black at `alpha`.
fn drop(offset_y: f32, blur: f32, spread: f32, alpha: f32) -> BoxShadow {
    BoxShadow {
        color: hsla(0.0, 0.0, 0.0, alpha),
        offset: point(px(0.0), px(offset_y)),
        blur_radius: px(blur),
        spread_radius: px(spread),
        inset: false,
    }
}

/// The composer plate's elevation (R5 §2.7). Light lifts with one long soft
/// drop; dark has no drop (it would vanish on the canvas) and takes a 1px
/// top highlight instead.
pub(crate) fn composer_shadow(theme: &Theme) -> Vec<BoxShadow> {
    match theme.appearance {
        Appearance::Light => vec![drop(12.0, 28.0, -18.0, 0.40)],
        Appearance::Dark => vec![BoxShadow {
            color: hsla(0.0, 0.0, 1.0, 0.06),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        }],
    }
}

/// The 1px top highlight filled plates (primary / destructive buttons) carry.
pub(crate) fn plate_highlight() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hsla(0.0, 0.0, 1.0, 0.16),
        offset: point(px(0.0), px(1.0)),
        blur_radius: px(0.0),
        spread_radius: px(0.0),
        inset: true,
    }]
}

/// The send/stop circle: a 1px top highlight plus a tight shadow tinted with
/// the plate colour so it sits on the surface instead of floating.
pub(crate) fn send_shadow(plate: Hsla) -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: plate.opacity(0.24),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(2.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: hsla(0.0, 0.0, 1.0, 0.16),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
            inset: true,
        },
    ]
}

/// Menu / popover elevation (R5 §2.7): `0 16px 40px -18px black/55`, deeper in
/// dark so the card still separates from a dark canvas.
pub(crate) fn menu_shadow(theme: &Theme) -> Vec<BoxShadow> {
    match theme.appearance {
        Appearance::Dark => vec![drop(18.0, 44.0, -18.0, 0.80)],
        Appearance::Light => vec![drop(16.0, 40.0, -18.0, 0.55)],
    }
}

/// Dialog elevation (R5 §2.7): a long soft drop plus, in dark, a 1px top
/// highlight so the plate reads as lit from above.
pub(crate) fn dialog_shadow(theme: &Theme) -> Vec<BoxShadow> {
    match theme.appearance {
        Appearance::Dark => vec![
            drop(24.0, 72.0, -20.0, 0.90),
            BoxShadow {
                color: hsla(0.0, 0.0, 1.0, 0.04),
                offset: point(px(0.0), px(1.0)),
                blur_radius: px(0.0),
                spread_radius: px(0.0),
                inset: true,
            },
        ],
        Appearance::Light => vec![drop(24.0, 64.0, -24.0, 0.65)],
    }
}

/// Tooltip elevation: one soft layer (T3 `shadow-md/5`).
pub(crate) fn tooltip_shadow(theme: &Theme) -> Vec<BoxShadow> {
    match theme.appearance {
        Appearance::Dark => vec![drop(6.0, 16.0, -6.0, 0.45)],
        Appearance::Light => vec![drop(6.0, 16.0, -6.0, 0.16)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_recipes_are_single_layers() {
        for theme in [Theme::dark(), Theme::light()] {
            assert_eq!(tooltip_shadow(&theme).len(), 1);
            assert_eq!(composer_shadow(&theme).len(), 1);
            assert_eq!(menu_shadow(&theme).len(), 1);
            assert!(!dialog_shadow(&theme).is_empty());
            assert!(focus_ring(&theme)[0].inset);
        }
    }

    #[test]
    fn pressed_is_heavier_than_hover() {
        for theme in [Theme::dark(), Theme::light()] {
            assert!(control_pressed(&theme).a > theme.control_hover().a);
        }
    }
}
