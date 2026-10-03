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

/// Sidebar row fill under the pointer.
pub(crate) fn sidebar_hover(theme: &Theme) -> Hsla {
    // TODO(D1): theme.sidebar_hover
    theme.glass_hover()
}

/// Sidebar row on screen in a pane that is not the focused one. One step
/// heavier than hover so the two never read as the same fill (R5 §2.4).
pub(crate) fn sidebar_selected(theme: &Theme) -> Hsla {
    // TODO(D1): theme.sidebar_selected
    step_up(sidebar_hover(theme), 1.45, theme)
}

/// The focused (open) sidebar row: the heaviest tier.
pub(crate) fn sidebar_active(theme: &Theme) -> Hsla {
    // TODO(D1): theme.sidebar_active
    step_up(sidebar_hover(theme), 1.8, theme)
}

/// Scale a translucent wash by `factor` (capped so rows never turn into
/// plates); an opaque theme hover mixes toward the text colour instead.
fn step_up(base: Hsla, factor: f32, theme: &Theme) -> Hsla {
    if base.a < 0.99 {
        Hsla {
            a: (base.a * factor).min(0.30),
            ..base
        }
    } else {
        mix(base, theme.text, 0.05 * factor)
    }
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

/// Placeholder text. The shared `text_faint` is the AA-checked placeholder
/// tone today; the Claude family tunes it separately from icon-muted.
pub(crate) fn placeholder(theme: &Theme) -> Hsla {
    // TODO(D1): theme.placeholder
    theme.text_faint
}

/// Outline of the composer plate.
pub(crate) fn composer_outline(theme: &Theme) -> Hsla {
    // TODO(D1): theme.composer_outline
    theme.border
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
            assert!(focus_ring(&theme)[0].inset);
        }
    }

    #[test]
    fn sidebar_tiers_step_up_from_hover() {
        for theme in [Theme::dark(), Theme::light()] {
            let (h, s, a) = (
                sidebar_hover(&theme),
                sidebar_selected(&theme),
                sidebar_active(&theme),
            );
            assert!(h.a < s.a && s.a < a.a, "{h:?} {s:?} {a:?}");
        }
    }

    #[test]
    fn action_hover_moves_off_the_action_plate() {
        for theme in [Theme::dark(), Theme::light()] {
            assert_ne!(action(&theme), action_hover(&theme));
        }
    }
}
