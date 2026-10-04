//! Paint-only presets, independent of the selected workbench theme.
//!
//! Pierre scopes were extracted from T3 Code Nightly's installed app.asar
//! (v0.0.46-nightly.20261003.2632), then hand-mapped to Noches' 31 kinds.
//! `embedded` intentionally shares punctuation; macro/label/markup use the
//! nearest Pierre scope rather than importing substring-based TextMate rules.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Appearance, Color};

pub const SYNTAX_CONTRAST_FLOOR: f32 = 3.0;

/// None in UI settings means automatic: Pierre for Claude, Theme elsewhere.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyntaxColors {
    #[default]
    Theme,
    Pierre,
}

impl SyntaxColors {
    pub const ALL: [Self; 2] = [Self::Theme, Self::Pierre];

    pub fn label(self) -> &'static str {
        match self {
            Self::Theme => "Theme",
            Self::Pierre => "Pierre",
        }
    }

    pub fn resolve(preference: Option<Self>, family_name: &str) -> Self {
        preference.unwrap_or_else(|| {
            if family_name.eq_ignore_ascii_case("claude") {
                Self::Pierre
            } else {
                Self::Theme
            }
        })
    }
}

/// Raw authored palette. Call [`pierre_on`] to enforce the floor against the
/// actual code/diff backgrounds (not an assumed editor backdrop).
pub fn pierre(appearance: Appearance) -> BTreeMap<String, Color> {
    let dark = appearance.is_dark();
    [
        ("comment", "#737373", "#737373"),
        ("keyword", "#d32a61", "#ff678d"),
        ("string", "#199f43", "#5ecc71"),
        ("stringSpecial", "#17a5af", "#64d1db"),
        ("escape", "#16a994", "#61d5c0"),
        ("number", "#1ca1c7", "#68cdf2"),
        ("boolean", "#1ca1c7", "#68cdf2"),
        ("type", "#a631be", "#d568ea"),
        ("typeBuiltin", "#a631be", "#d568ea"),
        ("constructor", "#a631be", "#d568ea"),
        ("function", "#693acf", "#9d6afb"),
        ("functionBuiltin", "#693acf", "#9d6afb"),
        ("macro", "#1a85d4", "#69b1ff"),
        ("property", "#d47628", "#ffa359"),
        ("constant", "#d5a910", "#ffd452"),
        ("variable", "#d47628", "#ffa359"),
        ("variableSpecial", "#d5901c", "#ffab16"),
        ("parameter", "#636363", "#a3a3a3"),
        ("operator", "#636363", "#636363"),
        ("punctuation", "#636363", "#636363"),
        ("embedded", "#636363", "#636363"),
        ("tag", "#d5512f", "#ff855e"),
        ("attribute", "#18a46c", "#60d199"),
        ("label", "#d5a910", "#ffd452"),
        ("markupHeading", "#d5512f", "#ff855e"),
        ("markupRaw", "#199f43", "#5ecc71"),
        ("markupLink", "#1a85d4", "#69b1ff"),
        ("markupReference", "#1a85d4", "#69b1ff"),
        ("markupEmphasis", "#d32a61", "#ff678d"),
        ("markupStrong", "#d5a910", "#ffd452"),
        ("invalid", "#d52c36", "#ff2e3f"),
    ]
    .into_iter()
    .map(|(key, light, dark_color)| {
        (
            key.into(),
            if dark { dark_color } else { light }
                .parse()
                .expect("Pierre hex"),
        )
    })
    .collect()
}

pub fn pierre_on(appearance: Appearance, backgrounds: &[Color]) -> BTreeMap<String, Color> {
    pierre(appearance)
        .into_iter()
        .map(|(key, color)| {
            // Select a candidate that clears *all* surfaces; sequential repair
            // alone could undo contrast on an earlier background.
            let safe = backgrounds.iter().fold(color, |color, background| {
                color.ensure_contrast(*background, SYNTAX_CONTRAST_FLOOR)
            });
            let safe = if backgrounds
                .iter()
                .all(|bg| safe.contrast(*bg) >= SYNTAX_CONTRAST_FLOOR)
            {
                safe
            } else {
                [Color::BLACK, Color::WHITE]
                    .into_iter()
                    .max_by(|a, b| {
                        let minimum = |color: Color| {
                            backgrounds
                                .iter()
                                .map(|bg| color.contrast(*bg))
                                .fold(f32::INFINITY, f32::min)
                        };
                        minimum(*a).total_cmp(&minimum(*b))
                    })
                    .unwrap_or(safe)
            };
            (key, safe)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kinds_and_contrast_floor() {
        for (appearance, background) in [
            (Appearance::Light, "#f5f4ed"),
            (Appearance::Dark, "#1f1e1d"),
        ] {
            let bg = background.parse().unwrap();
            let palette = pierre_on(appearance, &[bg]);
            assert_eq!(palette.len(), 31);
            for (key, color) in palette {
                assert!(
                    color.contrast(bg) >= SYNTAX_CONTRAST_FLOOR,
                    "{key}: {color}"
                );
            }
        }
    }

    #[test]
    fn automatic_default_and_explicit_override() {
        assert_eq!(SyntaxColors::resolve(None, "Claude"), SyntaxColors::Pierre);
        assert_eq!(SyntaxColors::resolve(None, "Zeron"), SyntaxColors::Theme);
        assert_eq!(
            SyntaxColors::resolve(Some(SyntaxColors::Theme), "Claude"),
            SyntaxColors::Theme
        );
    }
}
