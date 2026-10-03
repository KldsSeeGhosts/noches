# Theme system

Zeron themes are complete, source-neutral `ThemeVariant` values owned by the
`zeron-theme` crate. Runtime components consume only Zeron semantic roles. VS
Code workbench ids, T3 role names, and TextMate selectors stop at the source compiler.

## Runtime model

- `ThemeFamily` groups related variants.
- `ThemeVariant` is one completely resolved light or dark palette.
- `ThemeSelection` stores independent light and dark variant ids.
- `AccentSelection::ThemeDefault` preserves the variant's authored accent.
- `AccentSelection::Preset` derives a contrast-checked interaction overlay.
- Every variant records a recommended `SurfaceTreatment`: Zeron recommends
  frost, while VS Code-derived themes recommend opaque surfaces because that is
  what their authors targeted.
- `SurfacePreference` is a separate device-local choice: `Theme default`,
  `Frosted`, or `Opaque`. It does not change appearance, theme, or accent
  selection.
- Terminal background, foreground, selection and ANSI16 colors belong to the
  variant instead of the terminal renderer.

Accent overlays affect controls, focus, selections, caret, activity and the
three-tone glyph. They do not recolor syntax, terminal ANSI, status, or diff
semantics.

Surface preference affects only surface composition. Theme default preserves
the variant's recommendation; either override remains active while users move
between built-in, imported, and linked themes. Forced frost derives window,
floating, input, card, and hover tints from the variant's mapped shell roles
instead of fixed Zeron greys. Its window tint becomes denser when necessary to
keep primary text at 4.5:1 and muted text at 3:1 against the adverse desktop
luminance. Floating overlays, settings cards, and inputs run the same
composited-background check independently; a delicate palette can therefore
receive a thicker material on one surface without disabling frost everywhere.

macOS and Windows can frost the main window (native vibrancy and Acrylic,
respectively). Linux keeps the main window opaque because compositor blur is
not guaranteed. Supported floating surfaces can frost on macOS, Linux, and
Windows using their in-app renderers. Windows uses the bounded Direct3D
`BackdropBlur` implementation; native window Acrylic remains independent.
The preference remains portable even where a particular surface cannot honor blur.

The built-in registry contains 32 variants across 20 families:

- Zeron Light and Dark
- VS Code Light+ and Dark+
- Catppuccin Latte and Mocha
- Tokyo Night Light and Tokyo Night
- Dracula
- GitHub Light and Dark
- Ayu Light, Dark, and Mirage
- Gruvbox Light and Dark
- Rosé Pine Dawn and Moon
- Nord
- One Dark Pro
- Atom One Dark
- Night Owl and Night Owl Light
- Winter is Coming Dark Blue and Light
- Palenight
- SynthWave '84
- Shades of Purple
- Cobalt2
- Andromeda
- Claude Light and Dark (user-authored warm palette; opaque recommended)

Every bundled variant records source URL, exact upstream revision, license, and
a SHA-256 hash of the resolved curated definition.

Appearance settings keep light and dark choices in ordinary settings rows.
Each row opens a palette-preview menu, which allows the catalog to grow without
turning the page into a grid of bespoke buttons. Accent remains a compact
right-aligned swatch control; its three-tone first swatch means “Theme default.”
Glass is an adjacent conventional settings row with a compact
`Theme default` / `Frosted` / `Opaque` selector.

## Optional semantic roles

`ThemeColors` accepts serde-defaulted optional `action`, `onAction`,
`actionHover`, `controlHover`, `sidebarHover`, `sidebarSelected`,
`sidebarActive`, `placeholder`, `composerOutline`, `messageSurface`
(`message` is also accepted), `messageForeground`, `codeBackground`,
`codeForeground`, `iconMuted`, `accentSurface`, `dangerSurface`,
`warningSurface`, `link`, and `muted` overrides. Absent roles are omitted
when saving, so existing native libraries still load and round-trip unchanged.

The UI exposes snake-case accessors on `Theme`: `action()`, `on_action()`,
`action_hover()`, `control_hover()`, `sidebar_hover()`, `sidebar_selected()`,
`sidebar_active()`, `placeholder()`, `composer_outline()`, `message_surface()`,
`message_foreground()`, `code_background()`, `code_foreground()`, `icon_muted()`,
`accent_surface()`, `danger_surface()`, `warning_surface()`, `link()`, and
`muted()`. Without overrides these preserve the legacy solid/on-solid,
element-hover, faint-text, border, user-bubble wash, body/code text, accent,
notice-tint, and raised-surface derivations. They do not change existing
transcript or chrome call sites; those migrate separately.

The intentional exception is the row-state wash retune: hover is 4.5% light /
5% dark, selected 6.5% / 7.5%, and frost-active 8% / 9%. Opaque themes can
author all three sidebar tiers; forced frost uses the three translucent tiers.
`wash(alpha)` still honors its explicit alpha. Floating-card selection and
user-bubble recipes remain unchanged.

Claude preserves the palette from `crates/theme/tests/fixtures/claude.json`
(a read-only-source copy, not a link into live T3 data). The adapter maps
`surfaceRaised` to the composer input, `messageSurface` to raised/message
surfaces, `updateForeground` to links, terminal selection to accent selection,
and the dark composer outline to input 30% over canvas (`#31302d`).
The builtin adds a curated warm/desaturated ANSI16 palette; imported T3 files
retain Zeron ANSI16 because T3 v1 does not define it.

## T3 role-file import

Appearance's existing import/link dialog also detects T3 JSON:
`{version: 1, name, appearance, colors, variants: {dark: {...}}}`.
One file produces one family with both light and dark variants. Partial
variants override the base color record; unknown roles and invalid colors are
reported without hiding usable roles. Unsupported versions are rejected.
Snapshots and linked-file reloads use the same library path as VS Code imports.
No source files are rewritten, and no T3 runtime dependency is introduced.

`Color` supports CSS hex (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`),
legacy comma and modern space/slash `rgb()` / `rgba()`, percentages,
and `oklch()` with percent lightness/chroma, alpha, and hue units
(`deg`, `rad`, `grad`, `turn`). Nonfinite and malformed values are rejected.
Resolved colors serialize as canonical hex.

T3 imports flatten alpha foundations, harden foregrounds for their mapped
surfaces, and report every repair. Syntax is absent from T3 role files, so
they inherit the hand-mapped Pierre palette with a 3:1 floor on code/diff
surfaces. Source provenance uses `format: "t3-theme-v1"`.

```bash
cargo run -p zeron-theme --bin zeron-theme-import -- \
  --format t3 --input /path/to/claude.json \
  --output /tmp/claude.zeron.json --report /tmp/claude.report.json \
  --family-id t3-claude
```

T3 mode emits a complete `ThemeFamily` plus per-variant reports; VS Code mode
continues emitting one variant. Source URL/revision/license can be supplied
explicitly; T3 mode defaults to the local path, `local`, and `User supplied`.

## Syntax and installed typography

Settings → Appearance → **Syntax colours: Theme | Pierre** is independent of
workbench theme, accent, and fonts. The absent (`null`) preference chooses
Pierre for the Claude family and Theme elsewhere; an explicit choice persists
across theme switches. Changing it invalidates paint/style caches, not parser
results or text geometry. See [syntax-highlighting.md](syntax-highlighting.md).

Anthropic Sans Variable is an installed-only choice. It is never bundled,
downloaded, or selected when absent. On the Mac used for this implementation,
both the native GPUI catalog and Latin-metrics filter accepted the family.
The pinned GPUI/CoreText backend resolved NORMAL (400), MEDIUM (500), and
SEMIBOLD (600) to distinct native faces (FontIds 0, 8, 12), with increasing
`m` advances (1741.97, 1761.99, 1787.99 font units). No static-weight map is
needed on that configuration. This is not a Linux/Windows variable-font claim.
Repeat the isolated native probe after changing GPUI or font installations:

```bash
cargo test -p zeron-ui --lib native_anthropic_variable_weights -- \
  --ignored --test-threads=1 --nocapture
```

## VS Code import

The importer supports JSONC, trailing commas,
`include` inheritance, inline or external token rules, TextMate plist files,
workbench colors, semantic token colors, and terminal ANSI colors.

Appearance settings accept a single theme file, an extension `package.json`, or
an extension folder. Packages are detected from `contributes.themes`; variants
are classified by `uiTheme`, an explicit theme `type`, or the resolved editor
background. Each package variant compiles independently so a broken variant is
reported without hiding valid siblings. Users can select all or individual
variants, preview representative workbench/code/terminal/diff roles, and open
the optional mapping review.

The importer records `Opaque` as the variant's recommended surface treatment
and reports that inference. This preserves the source palette by default while
still allowing the independent Zeron surface preference to force frost.

After source mapping, a deterministic hardening pass checks Zeron's shared
roles across every solid surface where they are painted. It keeps VS Code's
semantic distinctions (`foreground`, `descriptionForeground`,
`editor.foreground`, and component-specific foregrounds), promotes a stronger
related source role when the preferred one cannot carry the Zeron role, and
only then adjusts the original color toward a contrast-safe anchor. Primary,
muted, button, terminal, focus/status, and interaction roles are covered.
Foundational surfaces with alpha are flattened against the resolved theme
background so an opaque import cannot accidentally expose the desktop. Syntax,
terminal ANSI, diff, warning, error, and success identity remain theme-owned;
their unresolved quality findings stay visible in the report.

The custom library has four source forms:

- `ImportedSnapshot` persists a self-contained compiled copy.
- `LinkedFile` follows one source theme file.
- `LinkedPackage` follows a VS Code extension package.
- `EditableFile` follows a native resolved-family JSON file created by
  “Duplicate as editable”.

Linked and editable sources reload explicitly. A failed reload or validation
stores a quiet warning and continues using the last successfully compiled
family. All four forms resolve to the same `ThemeFamily` model as built-ins, so
runtime components remain unaware of VS Code tokens. Library mutations are
activated only after the updated library is persisted successfully. The library is stored in
`{data_dir}/theme-library.json` and is loaded before the first palette is
installed.

The `zeron-theme-import` development tool exposes the same single-file adapter
for built-in curation:

Example:

```bash
cargo run -p zeron-theme --bin zeron-theme-import -- \
  --input /path/to/theme.json \
  --output /tmp/theme.zeron.json \
  --report /tmp/theme.report.json \
  --id example-dark \
  --family-id example \
  --name "Example Dark" \
  --appearance dark \
  --source-url https://github.com/example/theme \
  --revision 0123456789abcdef \
  --license MIT
```

The report records every source mapping, hardening adjustment, fallback,
unsupported font style, invalid color, validation result, and accent candidate
without truncating the advanced mapping review. Structural errors such as
duplicate ids or incomplete provenance block installation. Contrast findings
remain reviewable because compiled imports and runtime native-theme resolution
apply deterministic safeguards rather than rejecting an otherwise valid
source. Zeron does not fetch or execute VSIX packages at runtime; custom
sources are local files and folders compiled into resolved data.

## Acceptance and visual QA

`ThemeRegistry::validate` checks unique ids, provenance, text contrast,
interaction contrast, on-accent contrast, and terminal foreground contrast.
Supplied semantic foregrounds also check on-action/message/code at 4.5:1,
placeholder/icon at 3:1, and links at 4.5:1. Absent overrides do not introduce
new gates for older palettes.
Issues are classified as structural or contrast so callers cannot accidentally
treat a repairable quality finding as corrupt data. Tests also exercise every
preset against both appearances and adverse frosted backdrops.

Before adding or updating a bundled variant, review both appearances where
available across every `VisualFixture` scene:

1. Sidebar
2. Transcript Markdown
3. Transcript code
4. Composer
5. Picker/popover
6. Appearance settings
7. Diff
8. Terminal
9. Empty state
10. Dialog

Review normal, hover, active, focused, selected, disabled, working, warning,
error, and success states. Importer reports and automated contrast checks are
gates, not substitutes for this visual pass.
