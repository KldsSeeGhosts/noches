//! Offline T3 v1 role-file adapter. No T3 runtime or user-data writes.
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::vscode::{
    CompileOptions, DetectedThemeSource, ImportAdjustment, ImportMapping, ImportReport,
    SourceCompilation, ensure_contrast_across, read_bounded,
};
use crate::{
    AccentRoles, Appearance, Color, SurfaceTreatment, ThemeFamily, ThemeRegistry, ThemeSource,
    ThemeVariant, syntax_presets,
};

#[derive(Deserialize)]
struct ThemeFile {
    version: u32,
    name: String,
    appearance: Appearance,
    colors: BTreeMap<String, Value>,
    #[serde(default)]
    variants: BTreeMap<String, BTreeMap<String, Value>>,
}

pub(crate) fn is_t3(value: &Value) -> bool {
    value.get("version").is_some() && value.pointer("/colors/canvas").is_some()
}

pub fn import_file(path: &Path, mut options: CompileOptions) -> Result<SourceCompilation> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("could not resolve {}", path.display()))?;
    // Stable local provenance across macOS /var -> /private/var and linked
    // reloads. Authored non-path URLs remain untouched.
    if options.source_url == path.display().to_string()
        || Path::new(&options.source_url).canonicalize().ok().as_ref() == Some(&canonical)
    {
        options.source_url = canonical.display().to_string();
    }
    let source = read_bounded(&canonical, "T3 theme source")?;
    import_str(&source, canonical, options)
}

pub(crate) fn import_str(
    source: &str,
    path: PathBuf,
    options: CompileOptions,
) -> Result<SourceCompilation> {
    let file: ThemeFile = serde_json::from_str(source).context("could not parse T3 theme v1")?;
    if file.version != 1 {
        bail!("unsupported T3 theme version {}; expected 1", file.version);
    }
    if file.name.trim().is_empty() || !file.colors.contains_key("canvas") {
        bail!("T3 themes require a name and colors.canvas");
    }
    let hash = format!("sha256:{:x}", Sha256::digest(source.as_bytes()));
    let mut variants = Vec::new();
    let mut reports = BTreeMap::new();
    for appearance in Appearance::ALL {
        let key = if appearance.is_dark() {
            "dark"
        } else {
            "light"
        };
        if appearance != file.appearance && !file.variants.contains_key(key) {
            continue;
        }
        let mut colors = file.colors.clone();
        if let Some(overrides) = file.variants.get(key) {
            colors.extend(overrides.clone());
        }
        let mut report = ImportReport {
            source_files: vec![path.display().to_string()],
            source_hash: hash.clone(),
            ..Default::default()
        };
        let variant = convert(&colors, appearance, &file.name, &options, &mut report)?;
        reports.insert(variant.id.clone(), report);
        variants.push(variant);
    }
    Ok(SourceCompilation {
        path,
        source_kind: DetectedThemeSource::File,
        family: ThemeFamily {
            id: options.family_id,
            name: file.name,
            variants,
        },
        reports,
        failures: Vec::new(),
    })
}

fn convert(
    values: &BTreeMap<String, Value>,
    appearance: Appearance,
    name: &str,
    options: &CompileOptions,
    report: &mut ImportReport,
) -> Result<ThemeVariant> {
    // Avoid calling the registry here: the Claude builtin uses this adapter
    // while that registry's OnceLock is still being initialized.
    let mut output = crate::builtins::fallback_variant(appearance);
    output.id = format!(
        "{}-{}",
        options.family_id,
        if appearance.is_dark() {
            "dark"
        } else {
            "light"
        }
    );
    output.family_id.clone_from(&options.family_id);
    output.name = format!(
        "{name} {}",
        if appearance.is_dark() {
            "Dark"
        } else {
            "Light"
        }
    );
    output.recommended_surface_treatment = SurfaceTreatment::Opaque;
    let mut used = HashSet::new();
    macro_rules! map {
        ($role:literal, $key:literal, $target:expr) => {{
            used.insert($key);
            match values.get($key).and_then(Value::as_str) {
                Some(value) => match value.parse::<Color>() {
                    Ok(color) => {
                        $target = color;
                        report.mappings.push(ImportMapping {
                            zeron_role: $role.into(),
                            vscode_key: $key.into(),
                            value: color.to_string(),
                        });
                    }
                    Err(_) => report.warnings.push(format!(
                        "invalid T3 color {}: {value}; retained fallback",
                        $key
                    )),
                },
                None => report
                    .fallbacks
                    .push(format!("{}: absent {}; retained fallback", $role, $key)),
            }
        }};
    }
    macro_rules! optional {
        ($field:ident, $key:literal) => {{
            let mut color = None;
            used.insert($key);
            if let Some(value) = values.get($key) {
                if let Some(parsed) = value.as_str().and_then(|value| value.parse::<Color>().ok()) {
                    color = Some(parsed);
                    report.mappings.push(ImportMapping {
                        zeron_role: stringify!($field).into(),
                        vscode_key: $key.into(),
                        value: parsed.to_string(),
                    });
                } else {
                    report.warnings.push(format!(
                        "invalid T3 color {}; retained derived fallback",
                        $key
                    ));
                }
            } else {
                report.fallbacks.push(format!(
                    "{}: absent {}; retained derived fallback",
                    stringify!($field),
                    $key
                ));
            }
            output.colors.$field = color;
        }};
    }
    map!("background", "canvas", output.colors.background);
    map!("shell", "chrome", output.colors.shell);
    map!("raised", "messageSurface", output.colors.raised);
    map!("card", "surface", output.colors.card);
    map!("dialog", "surfaceOverlay", output.colors.dialog);
    map!("overlay", "surfaceOverlay", output.colors.overlay);
    map!("hover", "sidebarRowHover", output.colors.hover);
    map!("active", "sidebarRowActive", output.colors.active);
    map!("border", "border", output.colors.border);
    map!("borderStrong", "input", output.colors.border_strong);
    map!("text", "text", output.colors.text);
    map!("textMuted", "textMuted", output.colors.text_muted);
    map!("textFaint", "placeholder", output.colors.text_faint);
    map!("solid", "messageAction", output.colors.solid);
    map!("onSolid", "messageActionForeground", output.colors.on_solid);
    map!("danger", "error", output.colors.danger);
    map!("dangerMuted", "errorForeground", output.colors.danger_muted);
    map!("warning", "warning", output.colors.warning);
    map!(
        "warningMuted",
        "warningForeground",
        output.colors.warning_muted
    );
    map!("input", "surfaceRaised", output.colors.input);
    map!("cursor", "terminalCursor", output.colors.cursor);
    output.colors.diff_delete = output.colors.danger;
    map!("diffHunk", "updateSurface", output.colors.diff_hunk);
    optional!(action, "messageAction");
    optional!(on_action, "messageActionForeground");
    optional!(action_hover, "messageActionHover");
    optional!(control_hover, "accentSurface");
    optional!(sidebar_hover, "sidebarRowHover");
    optional!(sidebar_selected, "sidebarRowSelected");
    optional!(sidebar_active, "sidebarRowActive");
    optional!(placeholder, "placeholder");
    optional!(composer_outline, "toolbarBorder");
    optional!(message_surface, "messageSurface");
    optional!(message_foreground, "messageForeground");
    optional!(code_background, "codeBackground");
    optional!(code_foreground, "codeForeground");
    optional!(icon_muted, "iconMuted");
    optional!(accent_surface, "accentSurface");
    optional!(danger_surface, "errorSurface");
    optional!(warning_surface, "warningSurface");
    optional!(link, "updateForeground");
    optional!(muted, "muted");
    let fallback_background = crate::builtins::fallback_variant(appearance)
        .colors
        .background;
    output.colors.background = flatten(
        "background",
        output.colors.background,
        fallback_background,
        report,
    );
    let background = output.colors.background;
    for (role, color) in [
        ("shell", &mut output.colors.shell),
        ("raised", &mut output.colors.raised),
        ("card", &mut output.colors.card),
        ("dialog", &mut output.colors.dialog),
        ("overlay", &mut output.colors.overlay),
        ("input", &mut output.colors.input),
        ("solid", &mut output.colors.solid),
    ] {
        *color = flatten(role, *color, background, report);
    }
    if appearance.is_dark() && values.contains_key("input") {
        let input = output.colors.border_strong;
        let mix = |front: u8, back: u8| {
            (f64::from(front) * 0.3 + f64::from(back) * 0.7).round_ties_even() as u8
        };
        let derived = Color::rgb(
            mix(input.r, background.r),
            mix(input.g, background.g),
            mix(input.b, background.b),
        );
        adjust(
            report,
            "composerOutline",
            output
                .colors
                .composer_outline
                .unwrap_or(output.colors.border),
            derived,
            "T3 dark composer: input 30% over canvas",
        );
        output.colors.composer_outline = Some(derived);
    }
    let surfaces = [
        background,
        output.colors.shell,
        output.colors.raised,
        output.colors.card,
        output.colors.dialog,
        output.colors.overlay,
        output.colors.input,
    ];
    for (role, color, floor, backgrounds) in [
        ("text", &mut output.colors.text, 4.5, surfaces.as_slice()),
        (
            "textMuted",
            &mut output.colors.text_muted,
            4.5,
            surfaces.as_slice(),
        ),
        (
            "textFaint",
            &mut output.colors.text_faint,
            3.0,
            surfaces.as_slice(),
        ),
        (
            "onSolid",
            &mut output.colors.on_solid,
            4.5,
            std::slice::from_ref(&output.colors.solid),
        ),
    ] {
        let original = *color;
        *color = ensure_contrast_across(original, backgrounds, floor, None);
        adjust(
            report,
            role,
            original,
            *color,
            &format!("raised to {floor}:1 on mapped surfaces"),
        );
    }
    let mut primary = output.accent.primary;
    map!("accent.primary", "accent", primary);
    output.accent = AccentRoles::derive(primary, appearance, background);
    map!("accent.on", "accentForeground", output.accent.on);
    output.accent.on = output.accent.on.ensure_contrast(output.accent.strong, 4.5);
    map!(
        "accent.selection",
        "terminalSelection",
        output.accent.selection
    );
    map!(
        "terminal.background",
        "terminalBackground",
        output.terminal.background
    );
    map!(
        "terminal.foreground",
        "terminalForeground",
        output.terminal.foreground
    );
    map!(
        "terminal.selection",
        "terminalSelection",
        output.terminal.selection
    );
    output.terminal.background = flatten(
        "terminal.background",
        output.terminal.background,
        background,
        report,
    );
    let original = output.terminal.foreground;
    output.terminal.foreground = original.ensure_contrast(output.terminal.background, 4.5);
    adjust(
        report,
        "terminal.foreground",
        original,
        output.terminal.foreground,
        "raised to 4.5:1",
    );
    harden_optional(&mut output, report);
    let code_bg = output
        .colors
        .code_background
        .unwrap_or(background)
        .blend_over(background);
    let raw = syntax_presets::pierre(appearance);
    let add_bg = output
        .colors
        .diff_add
        .with_alpha(0.055)
        .blend_over(background);
    let delete_bg = output
        .colors
        .diff_delete
        .with_alpha(0.055)
        .blend_over(background);
    output.syntax =
        syntax_presets::pierre_on(appearance, &[code_bg, background, add_bg, delete_bg]);
    for (key, safe) in &output.syntax {
        adjust(
            report,
            &format!("syntax.{key}"),
            raw[key],
            *safe,
            "Pierre 3:1 contrast floor on code background",
        );
    }
    report
        .fallbacks
        .push("syntax: Pierre (T3 role files contain no syntax palette)".into());
    report
        .fallbacks
        .push("terminal ANSI16: retained Zeron fallback (T3 v1 has no ANSI roles)".into());
    report
        .fallbacks
        .push("surface treatment: opaque recommended by T3 role adapter".into());
    for key in values.keys().filter(|key| !used.contains(key.as_str())) {
        report
            .dropped
            .push(format!("T3 role {key}: no Noches semantic equivalent"));
    }
    output.source = ThemeSource {
        format: "t3-theme-v1".into(),
        url: options.source_url.clone(),
        revision: options.revision.clone(),
        license: options.license.clone(),
        asset_hash: String::new(),
    };
    output.source.asset_hash = format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(&output)?));
    report.validation = ThemeRegistry {
        families: vec![ThemeFamily {
            id: output.family_id.clone(),
            name: name.into(),
            variants: vec![output.clone()],
        }],
    }
    .validate();
    Ok(output)
}

fn harden_optional(output: &mut ThemeVariant, report: &mut ImportReport) {
    let c = &mut output.colors;
    let background = c.background;
    for (role, color) in [
        ("action", &mut c.action),
        ("actionHover", &mut c.action_hover),
        ("messageSurface", &mut c.message_surface),
        ("codeBackground", &mut c.code_background),
    ] {
        if let Some(value) = color {
            *value = flatten(role, *value, background, report);
        }
    }
    let action = c.action.unwrap_or(c.solid);
    let message = c.message_surface.unwrap_or(c.raised);
    let code = c.code_background.unwrap_or(background);
    for (role, value, backgrounds, floor) in [
        ("onAction", &mut c.on_action, vec![action], 4.5),
        (
            "messageForeground",
            &mut c.message_foreground,
            vec![message],
            4.5,
        ),
        ("codeForeground", &mut c.code_foreground, vec![code], 4.5),
        ("placeholder", &mut c.placeholder, vec![c.input], 3.0),
        (
            "iconMuted",
            &mut c.icon_muted,
            vec![background, c.shell],
            3.0,
        ),
        ("link", &mut c.link, vec![background], 4.5),
    ] {
        if let Some(color) = value {
            let original = *color;
            *color = ensure_contrast_across(original, &backgrounds, floor, None);
            adjust(
                report,
                role,
                original,
                *color,
                &format!("raised to {floor}:1 on its semantic surface"),
            );
        }
    }
}

fn adjust(report: &mut ImportReport, role: &str, original: Color, resolved: Color, reason: &str) {
    if original != resolved {
        report.adjustments.push(ImportAdjustment {
            zeron_role: role.into(),
            original: original.to_string(),
            resolved: resolved.to_string(),
            reason: reason.into(),
        });
    }
}

fn flatten(role: &str, original: Color, background: Color, report: &mut ImportReport) -> Color {
    let resolved = original.blend_over(background);
    adjust(
        report,
        role,
        original,
        resolved,
        "flattened translucent foundation over its mapped backdrop",
    );
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;
    const CLAUDE: &str = include_str!("../tests/fixtures/claude.json");

    fn options() -> CompileOptions {
        CompileOptions {
            family_id: "t3-claude".into(),
            family_name: "Claude".into(),
            source_url: "fixture:claude".into(),
            revision: "local".into(),
            license: "User supplied".into(),
        }
    }

    #[test]
    fn fixture_maps_one_family_and_both_appearances() {
        let imported = import_str(CLAUDE, PathBuf::from("claude.json"), options()).unwrap();
        assert_eq!(imported.family.variants.len(), 2);
        let light = &imported.family.variants[0];
        let dark = &imported.family.variants[1];
        assert_eq!(light.colors.background.to_string(), "#faf9f5");
        assert_eq!(dark.colors.code_background.unwrap().to_string(), "#1f1e1d");
        assert_eq!(dark.colors.composer_outline.unwrap().to_string(), "#31302d");
        assert_eq!(light.colors.on_action, Some(Color::WHITE));
        assert_eq!(dark.colors.action_hover.unwrap().to_string(), "#e58a6b");
        for variant in &imported.family.variants {
            assert_eq!(
                variant.recommended_surface_treatment,
                SurfaceTreatment::Opaque
            );
            assert_eq!(variant.source.format, "t3-theme-v1");
            assert_eq!(variant.syntax.len(), 31);
            let bg = variant.colors.code_background.unwrap();
            assert!(
                variant
                    .syntax
                    .values()
                    .all(|color| color.contrast(bg) >= 3.0)
            );
            assert!(
                imported.reports[&variant.id]
                    .validation
                    .iter()
                    .all(|issue| issue.severity != crate::ValidationSeverity::Error)
            );
        }
        assert!(
            imported.reports[&light.id]
                .adjustments
                .iter()
                .any(|entry| entry.zeron_role == "syntax.constant")
        );
    }

    #[test]
    fn css_colors_partial_variants_and_unknown_roles() {
        let imported = import_str(r##"{"version":1,"name":"Example","appearance":"light","colors":{"canvas":"rgb(250 250 250)","text":"oklch(.2 0 0)","future":"anything"},"variants":{"dark":{"canvas":"rgba(20,20,20,1)","text":"#eee"}}}"##, PathBuf::from("example.json"), options()).unwrap();
        assert_eq!(
            imported.family.variants[1].colors.background,
            Color::rgb(20, 20, 20)
        );
        assert!(
            imported
                .reports
                .values()
                .all(|report| report.dropped.iter().any(|role| role.contains("future")))
        );
    }

    #[test]
    fn malformed_version_and_missing_foundations() {
        for source in [
            r##"{"version":2,"name":"Bad","appearance":"light","colors":{"canvas":"#fff"}}"##,
            r#"{"version":1,"name":"Bad","appearance":"light","colors":{}}"#,
        ] {
            assert!(import_str(source, PathBuf::from("bad.json"), options()).is_err());
        }
    }
}
