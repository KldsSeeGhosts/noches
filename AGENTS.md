# Noches Desktop: UI design system

The desktop UI follows **docs/design/control-plane.md**: a control plane for coding agents, with BB-style compact title-first threads in the sidebar and Cursor-style meaningful color. Read it before changing any sidebar, pane-header, or composer chrome. It supersedes the earlier open-design "buddy bot" sidebar plan. Buddy avatars were removed on purpose; do not bring them back.

## Rules in one breath

- Color encodes state or identity, never decoration.
- Status hues come only from `crate::status_palette::SessionState` (sky working, indigo awaiting input, emerald completed-unseen, danger failed).
- Projects show a repo favicon or a colored monogram (`Shell::render_project_icon`). Harness marks keep their brand tint.
- Panes are flush on the same shell backdrop as a lone session, with 1px hairline dividers, not rounded islands or darker opaque fills.
- Metadata (branch, device, elapsed time, model) is monospace 11px.

## Where things live

- Sidebar cards: `render_chat_row` in `crates/ui/src/shell.rs`.
- State sections and visible order: `render_active_rows` / `sidebar_visible_order` in `crates/ui/src/shell/spaces.rs`. Keep both in sync, because jump shortcuts use the visible order.
- Project badges: `crates/ui/src/shell/project_icon.rs` (favicon discovery for local and remote projects, with a monogram fallback).
- Pane chrome: `crates/ui/src/pane/chrome.rs` (`pane_header`) and `crates/ui/src/pane/render.rs` (flush layout, dividers).
- Drop geometry: `crates/ui/src/pane/hit_test.rs`. Its tolerances are explicit constants; do not re-derive them from layout padding.
- Model chip and placeholder: `crates/ui/src/pickers.rs` (`trigger_chip`, `chip_model_label`) and `crates/ui/src/composer.rs`.

## Split views are load-bearing

Split panes, view splits, header drag, sidebar-row drag-to-split, drop previews, divider drag and equalize, and focus routing are covered by `crates/ui/src/shell/pane_surface_regressions.rs`, `workspace_regressions.rs`, and the `pane::` tests. Run `cargo test -p zeron-ui --lib` after any pane or sidebar change.

## Delivery workflow

- Large changes, including multi-file features, substantial refactors, and upstream integrations, must be committed on a dedicated branch, pushed to `origin`, and opened as a pull request in `KldsSeeGhosts/noches` against the appropriate integration branch.
- Do not finish a large change with only local, uncommitted edits unless the user explicitly requests that. Inspect existing PRs and branch ancestry before choosing the PR base; always specify the Noches repository explicitly in `gh` commands so the fork's upstream is not selected accidentally.
- Commit only task-related files; preserve unrelated changes and untracked files. Include implemented scope, validation results, known failures, and unverified platforms or visual QA in the PR description.
- Do not push directly to an integration branch, merge the PR, or publish a release unless explicitly requested.

## Visual QA

`scripts/dev-demo.sh` boots a seeded mock engine and the headed app (it runs under macOS bash 3.2). For a split layout, seed `{data_dir}/workspace-layout.json` with a `split` root for the selected space; `SplitNode` is serde-tagged `{"type":"split","horizontal":..,"ratio":..,"first":..,"second":..}`. Check light and dark appearance.
