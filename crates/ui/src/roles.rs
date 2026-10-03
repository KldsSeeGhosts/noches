//! Chrome colour roles that the theme model does not carry yet.
//!
//! D1 (theme foundation) adds these as real `ThemeColors` roles. Until that
//! lands each one is derived here from today's tokens, so chrome code asks for
//! the *role* and the swap is one line per helper (`// TODO(D1)` marks it).
//! Nothing outside this module should reach for the underlying token to mean
//! "the action colour", "the control hover fill", and so on.

use gpui::{BoxShadow, Hsla, hsla, point, px};

use crate::motion::mix;
use crate::theme::{Appearance, Theme, ink, wash};

/// Fill of the primary action (send, primary button, switch on).
pub(crate) fn action(theme: &Theme) -> Hsla {
    // TODO(D1): theme.action
    theme.solid
}

/// Label/icon colour on top of [`action`].
pub(crate) fn on_action(theme: &Theme) -> Hsla {
    // TODO(D1): theme.on_action
    theme.on_solid
}

/// Hover plate of the primary action.
pub(crate) fn action_hover(theme: &Theme) -> Hsla {
    // TODO(D1): theme.action_hover
    mix(action(theme), theme.bg, 0.14)
}

/// Hover fill for ghost/outline controls and highlighted menu rows.
pub(crate) fn control_hover(theme: &Theme) -> Hsla {
    // TODO(D1): theme.control_hover
    theme.element_hover
}

/// Pressed fill for ghost/outline controls: one step past hover.
pub(crate) fn control_pressed(theme: &Theme) -> Hsla {
    // TODO(D1): theme.control_hover darkened/lightened
    match theme.appearance {
        Appearance::Dark => wash(0.16),
        Appearance::Light => wash(0.10),
    }
}

/// Plate behind a keyboard-key chip (`kbd`) and quiet badges.
pub(crate) fn secondary_fill() -> Hsla {
    // TODO(D1): theme.secondary
    ink(0.06)
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
            assert!(focus_ring(&theme)[0].inset);
        }
    }

    #[test]
    fn action_hover_moves_off_the_action_plate() {
        for theme in [Theme::dark(), Theme::light()] {
            assert_ne!(action(&theme), action_hover(&theme));
        }
    }
}
