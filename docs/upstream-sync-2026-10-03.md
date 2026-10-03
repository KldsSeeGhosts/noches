# Upstream review: Zeron v0.2.102 and `main`

Compared Noches `dev` at `0b82bf6d` with Zeron `main` at [9e1a1115](https://github.com/zeronsh/zeron/commit/9e1a1115) (October 3, 2026), which is [v0.2.102](https://github.com/zeronsh/zeron/releases/tag/v0.2.102) (`64ad6f6e`) plus 25 unreleased commits. The previous [review](upstream-sync-2026-10-02.md) covered v0.2.93–v0.2.101. Unlike that pass, this one targets upstream `main`, so it includes changes that have not been tagged yet.

This is a behavioral comparison and selective port, not a merge of upstream's UI. Noches keeps its control-plane design, pane-specific composers, split workspace, Swift companion, browser integration, credential inheritance, approval routing, dependency pins, and release/update channels.

The work was split into six workstreams ported in parallel worktrees and merged into one integration branch, `sync-20261003/integration`.

## Implemented

| Upstream source | Behavior brought over |
| --- | --- |
| [2a884777](https://github.com/zeronsh/zeron/commit/2a884777) (#757) | Shift+Backspace works in composer and palette inputs. |
| [c14d4579](https://github.com/zeronsh/zeron/commit/c14d4579) (#744) | The composer's session branch label uses free width and fades on overflow, using Noches' existing overflow fade. Leading checkout/branch grouping and the trailing PR slot are preserved. |
| [f9a4a18b](https://github.com/zeronsh/zeron/commit/f9a4a18b) (#739), partial | BMP clipboard images and attachments are re-encoded as PNG at staging, so agents can read Windows screenshots. A file that will not decode is refused; a pasted image that will not decode stays as pasted. Results stay with the draft that produced them. The wallpaper-rotation part is omitted because Noches has no such module. |
| [edac0d7d](https://github.com/zeronsh/zeron/commit/edac0d7d) (#727) | The project folder picker understands Windows drive paths: breadcrumbs, parent/child navigation, typed drive jumps and `path_under`. |
| [9782693b](https://github.com/zeronsh/zeron/commit/9782693b) (#740), partial | Attach-button hover fades are scoped per composer instance. Noches' composers shared that state. The new dictation button uses the same `composer_hover_key` helper. |
| [612df512](https://github.com/zeronsh/zeron/commit/612df512) (#763) | Preview discovery leaves authentication-callback listeners unprobed. Cherry-picked unchanged. |
| [27480d99](https://github.com/zeronsh/zeron/commit/27480d99) (#686), partial | Both `OPENCODE_PASSWORD` and `OPENCODE_SERVER_PASSWORD` carry Noches' generated password (OpenCode 2.x reads the former first). A harness that fails before streaming now writes the error as an assistant entry instead of a bare "Run failed". |
| [1f7b74a7](https://github.com/zeronsh/zeron/commit/1f7b74a7) (#707) | Dedicated Todo panel for the agent's checklist: structured, optional and leniently decoded status in the doc schema; per-harness normalization for Claude, Codex, Cursor, ACP, OpenCode and the mock harness; a pane-local panel scoped to each composer's chat target. |
| [c78bb1c1](https://github.com/zeronsh/zeron/commit/c78bb1c1), [46bedeca](https://github.com/zeronsh/zeron/commit/46bedeca) (#751) | Sidebar threads rename in place (active cards and compact archived rows): Enter/blur commit, Escape cancel, pane-aware focus return. |
| [e2a7706f](https://github.com/zeronsh/zeron/commit/e2a7706f) (#737) | The project-header new-chat button targets the explicit project, device and checkout before canvas routing. Project-header and Archive/Unarchive tooltips. |
| [9e1a1115](https://github.com/zeronsh/zeron/commit/9e1a1115) (#760) | Mermaid diagrams render in chat replies and Markdown previews. |
| [01832f2c](https://github.com/zeronsh/zeron/commit/01832f2c) (#706), partial | Opt-in `noches mcp` stdio server that creates standalone sessions on a selected device and project. |
| [56dc5ff3](https://github.com/zeronsh/zeron/commit/56dc5ff3) (#750) | Every external GitHub Action is pinned to a commit SHA, and weekly grouped Dependabot updates keep the pins current. |
| [80b946b1](https://github.com/zeronsh/zeron/commit/80b946b1) (#591), selective | Opt-in on-device dictation in pane composers. |
| #692, #693 (CI), adapted | Dev-seeded Rust caches are kept and cache-seeding push runs are no longer cancelled; the Windows suites are split with one dev cache saver. |

### Noches-specific adaptations

- **#686:** Noches has no `opencode --version` probe (it detects the protocol from the live server), so upstream's cold-probe retry does not apply and is not ported.
- **#706 authorization.** Noches had no session-MCP crate and no side-chat parent schema, so this is a new, separate, opt-in `noches mcp` subcommand backed by the new `zeron-mcp` crate. Selected devices must exist in the current profile; projects must belong to the selected device; remote catalog queries must succeed through the existing owner-authenticated relay before any write. There is no arbitrary `cwd`, no sandbox disabling and no automatic approval: runs use `autoApprove: false`, and questions and approvals keep their durable command route. Older callers without the additive routing capability fail closed. Side/parent requests are rejected. Browser MCP, agent injection policy and the document schema are unchanged. See [mcp.md](mcp.md).
- **#707 compatibility.** The new schema field is optional and leniently decoded, with `done` remaining authoritative, so old docs and old hosts still decode. The Swift companion's generic decoding accepts the field and shows `Todo · N/M done`; its production code is unchanged and a compatibility test was added. Upstream's `CONTRIBUTORS.md` change and screenshots are not imported. See [todo-panel.md](todo-panel.md).
- **#760 renderer.** Rendering uses the MIT-licensed `mermaid-rs-renderer` already in the lockfile, with its CLI and PNG features disabled: no JavaScript runtime, web view or network client. It runs in a serialized helper process (the hidden `mermaid-render` subcommand) with a three-second kill-and-reap deadline, bounded input and output, a cleared credential environment and a hard cache limit of 64 entries / 64 MiB. A pending, failed, timed-out or over-budget render keeps the original selectable code fence. Existing SVG sanitization blocks scripts, HTML and image resource resolution. The upstream Copy-button tooltip is omitted to preserve Noches' chrome. See [markdown-preview.md](markdown-preview.md).
- **#591 consent and scope.** The existing networked GPT-Live path in `crates/voice` is unchanged. The new local backend is `zeron-voice` in `crates/dictation` and uses NVIDIA Parakeet TDT 0.6B v3 (INT8) through `parakeet-rs` 0.3.8 and a CPU ONNX Runtime. It is strictly opt-in: while it is disabled there is no microphone access, no model download and no network request. Enabling it in Settings authorizes one 670,479,942-byte model download from `istupakov/parakeet-tdt-0.6b-v3-onnx` on Hugging Face, pinned to revision `8f23f0c0…` with per-file SHA-256 verification. Capturing requires a separate user action; audio stays in memory on the device and is never saved or uploaded. The default shortcut is Cmd/Ctrl+Alt+R so Cmd/Ctrl+D keeps splitting panes. CPAL stays at 0.15.3 because upstream's 0.17.3 conflicts with Noches' ALSA linkage; as a result input devices are selected by name and identical names cannot be told apart. See [reference/desktop-dictation.md](reference/desktop-dictation.md).
- **#750:** pins were resolved through `gh api`, including annotated-tag dereferencing. The release, TestFlight, deploy and SDK-updater workflows change only in their pins.
- **Merge resolutions.** `apps/zeron/src/main.rs` keeps both the hidden `MermaidRender` and the new `Mcp` subcommands. `crates/ui/src/composer.rs` keeps both the attach hover key and the dictation button, and has a single `composer_hover_key` helper (two branches each introduced it).

## Not imported

| Upstream change | Decision |
| --- | --- |
| [e96eccb1](https://github.com/zeronsh/zeron/commit/e96eccb1) (#754) | Production change skipped. Noches has one global reduced-motion boolean that makes activity loaders static; upstream's change keeps loaders pulsing under system reduced motion, which would break that contract. A regression test now pins the existing behavior. |
| Compact model picker follow-ups: #721, #749, #745 | They fix and extend the compact picker introduced by #471, which Noches deliberately did not adopt. Not applicable without it. |
| #694 (nextest/dev profile), #695 and its macOS commits (macOS workflow replacement) | They change test execution and coverage substantially, and the benefit for Noches' existing profiles and Swift companion is not established. |
| Version bump `64ad6f6e` | Noches keeps its own versioning. |
| Items still deferred from the previous review | Pi native RPC, agent update monitoring and Homebrew controls, durable application updates, the Rust-core iOS rewrite, upstream side-chat navigation, encoded and out-of-folder file links, the rich-composer command/skill protocol, wallpapers and marketing changes, and ambient MCP/plugin loading. Each replaces or widens something Noches has built differently and needs its own integration. |

## Dependencies and platform notes

- Mermaid adds no dependencies. `zeron-mcp` adds one internal workspace crate.
- Dictation adds one local crate and **51 registry packages**, with no existing version changed; the exact list and licenses are in `THIRD_PARTY_NOTICES.md`.
- The ONNX Runtime build script downloads a verified native archive from `cdn.pyke.io` **at compile time** (about 105 MB for Linux static). Builds therefore need that host reachable or a pre-populated cache. Release binary growth has not been measured.
- Intel macOS needs an independently supplied matching ONNX runtime, and Windows archives also link DirectML/DX12. Packaging and permission-prompt changes (macOS usage strings, Linux/Windows install scripts) are unverified.

## Validation

Run on Linux (x86_64) against the merged integration branch, with the rustup stable toolchain.

| Command | Result |
| --- | --- |
| `cargo check --workspace --locked --tests`, and `cargo check --workspace --locked` | **Passed.** The one warning (an unused doc comment in `crates/engine/src/sessions.rs`) is already on `dev`. |
| `cargo test -p zeron-ui --lib --locked -- --test-threads=1` | **1,424 passed**, 0 failed. |
| `cargo test -p zeron-ui --lib --locked` (parallel) | **1,424 passed**, 0 failed, in 20 of 20 repeated runs. See the selection-state fix below. |
| `cargo test -p zeron-proto -p zeron-doc -p zeron-rpc -p zeron-mcp -p zeron-voice -p noches-voice -p zeron-preview -p zeron --locked --no-fail-fast` | **281 passed**, 0 failed, 3 ignored. |
| `cargo test -p zeron-harness --features zeron-harness/native-fixture --locked --no-fail-fast` | **378 passed**, 0 failed, 11 ignored. |
| `cargo test -p zeron-engine --lib --test e2e --test workspace_files --test subagent_idle_reap --test m5c_accounts_uploads_titles --locked` | Library **355 passed** (2 ignored); e2e **24 passed** (2 ignored); workspace files **7 passed**; subagent idle reap **2 passed**; accounts/uploads/titles **17 passed**. |
| `cargo test -p zeron-engine --test device_routing mcp_standalone_session_executes_on_the_selected_device --locked` | **1 passed.** |
| `cargo test -p zeron-engine --test local_first --locked` | **Flaky, not clean.** See below. |
| `npm run test:unit` and `npm run typecheck` in `edge/` | **53 passed**; typecheck passed. |
| Per-agent runs | Each workstream also ran its own targeted suites; the engine-mcp run additionally passed a 10-request stdio smoke test, actionlint on all 11 workflows (79 pinned action references), and rustfmt on its changed files. |

### Test-state fix made while integrating

The parallel `zeron-ui` suite was already flaky on `dev` (7 of 12 runs failed) and failed every run (12 of 12) once these ports added more tests. The Markdown selection is process-global, but only tests that took `test_state_lock` were protected from each other, so tests that merely click or render Markdown could begin or clear a selection another test was asserting on. Under `cfg(test)` the selection state is now thread-local, and each test runs on its own thread. Production keeps its single process-wide state, and the production build is unchanged. This is a test-isolation fix, not a behavior change.

### Known failures that predate this work

- `local_first::online_runtime_shutdown_stops_edge_workers_and_retires_the_graph` fails intermittently, as the [previous review](upstream-sync-2026-10-02.md) also recorded. Measured over 10 whole-target runs, it failed once on untouched `dev` and once on this branch; run alone, it passed 10 of 10 on both, though one earlier single run on this branch did fail, so the rate is low but not zero. This pass does not change shutdown behavior and does not claim to fix it.
- The engine-mcp workstream measured, on untouched `dev`, the same stalls in `device_routing` (3 tests) and `m5_repos_diffs_terminals` (2 tests), and a failure in `project_action_commands`. Those targets were not run in full on the merged branch here.

### Not verified

- Native desktop visual QA in light and dark appearance for every UI change (sidebar rename, new-chat button, Todo panel, Mermaid diagrams, dictation controls, branch label fade).
- macOS, Windows, Linux ARM and iOS builds; Swift test execution; the Windows clipboard BMP path and folder-picker drive paths on a real Windows machine.
- Real microphone capture, the model download, and transcription quality; macOS microphone permission prompts, signing, and packaging scripts; release binary size.
- Real authenticated OpenCode and provider runs; real relay authorization enforcement for the MCP tools.
- Hosted GitHub workflow execution, including the pinned actions and the adapted caches.

Delivery follows `AGENTS.md`: the work is on a dedicated branch, pushed to Noches' `origin`, with a pull request against `dev`. Merging, release publication and branch/worktree cleanup are left until the PR lands.
