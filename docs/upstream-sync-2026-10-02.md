# Upstream review: Zeron v0.2.93–v0.2.101

Compared Noches `refine/sidebar-context-chrome` at `95cf3caf` with the latest published Zeron release, [v0.2.101](https://github.com/zeronsh/zeron/releases/tag/v0.2.101), released October 1, 2026, at `b42fc2b8`. The previous [review](upstream-sync-2026-09-26.md) covered v0.2.92. Upstream `main` contains additional, unreleased work and is not this pass's integration target.

This is a behavioral comparison and selective port, not a merge of upstream's UI or a claim of complete v0.2.101 parity. Noches keeps its control-plane design, pane-specific composers, split workspace, Swift companion, browser integration, credential inheritance, approval routing, dependency pins, and release/update channels.

## Implemented

| Upstream source | Behavior brought over |
| --- | --- |
| [c72c66d8](https://github.com/zeronsh/zeron/commit/c72c66d8) (#514) | File-tree context menu: Add to chat, Copy path, inline Rename/F2, confirmed permanent Delete, and tree-originated drag-to-move. Host-side mutation APIs support local and remote workspaces. |
| [4aceec16](https://github.com/zeronsh/zeron/commit/4aceec16) (#665) | File-tree edge fades follow the actual negative scroll offset and disappear at the appropriate scroll boundaries. |
| [4d106b82](https://github.com/zeronsh/zeron/commit/4d106b82) (#604) | Empty subagent folds clear their dirty flag; sink-only traffic arms the commit window rather than spinning the engine's flush loop. |
| [aaeede8b](https://github.com/zeronsh/zeron/commit/aaeede8b) (#637) | Idle reaping accounts for live background subagents and their last activity, with a bounded silence timeout. |
| [cc8b147a](https://github.com/zeronsh/zeron/commit/cc8b147a) (#676) | Claude task identity survives follow-up tool IDs and process resume. Restart recovery settles this device's abandoned running chips, including completed parent turns; sink-less completion updates an existing chip. |
| [a86f0587](https://github.com/zeronsh/zeron/commit/a86f0587) (#634) | OpenCode 2.x context occupancy comes from the latest step, not cumulative spend. Model identity supplies the correct context limit; compaction clears stale occupancy. Tool/model normalization state survives SSE reconnects. |
| [bf64e71f](https://github.com/zeronsh/zeron/commit/bf64e71f) (#605) | Read/access notifications do not kick checkout diff production. Actual writes and structural changes still do. |
| [366b3c9e](https://github.com/zeronsh/zeron/commit/366b3c9e) (#632) | Markdown selection is scoped to its painted transcript surface, so multiple panes and repeated fork-history row keys do not steal or leak selections. Closed surfaces leave the registry. |
| [92be0f26](https://github.com/zeronsh/zeron/commit/92be0f26) (#556) | Occluded/clipped transcript text cannot start a drag through a popup. Dialog inputs copy only their own text; message inputs retain transcript-copy fallback. |
| [6641095b](https://github.com/zeronsh/zeron/commit/6641095b) (#681) | Drag selection distinguishes adjacent table columns by horizontal distance within the nearest vertical band; includes wrapped cells, padding, reversed drags and cross-block selections. |
| [0fab030f](https://github.com/zeronsh/zeron/commit/0fab030f) (#669) | Question panels occlude the transcript and use Noches' composer tint and frost wrapper instead of an unblurred translucent fill. |
| [e5be4822](https://github.com/zeronsh/zeron/commit/e5be4822) (#615) | Terminal scrolling accumulates fractional movement instead of dropping sub-line trackpad events. |
| [b3d7f48b](https://github.com/zeronsh/zeron/commit/b3d7f48b) (#682), partial | Queue rows show file-reference chip labels rather than canonical Markdown transport links, without changing the queued delivery text or exposing attachment trailers. Command/skill invocation labels depend on the rich-composer protocol not present in Noches. |

### Noches-specific adaptations

- File-tree actions fit the existing browser/editor split layout; upstream's explorer, side-chat sections and shell layout are not substituted.
- References carry the originating surface, generation and workspace. A stale drag cannot attach to a newly selected session or the wrong composer's chat. Tree move targets and conversation attachment targets are tested together.
- Hidden file surfaces cancel transient menus, rename/delete prompts and drag feedback. Mutations coordinate all open editors in the affected workspace, preserve dirty buffers and rename tab/comment paths, and avoid duplicate reconciliation from RPC replies and semantic watch events.
- Mutations require a matching checkout identity and metadata revision. They serialize against saves, never overwrite an existing destination, reject traversal/symlinks/special files/protected paths, and do not retry ambiguous transport failures. Delete is explicitly confirmed; buffers are retained for recovery.
- New directory capabilities, metadata revisions and watcher operation IDs are optional wire fields. Old hosts remain browseable and do not enable unsupported mutations.
- Windows retains Noches' click-first row policy. Tree drag initiation stays disabled on Windows with the current zui pin; context-menu and keyboard actions remain available. Drag-interaction tests are scoped accordingly.
- Claude resume-history lookup uses the effective child environment, including provider/configuration values inherited from the host login shell. Noches' launch and steering behavior is otherwise unchanged.
- Upstream test-only MCP/message-duration fields are not imported into Noches' document schema.

## Release features deliberately not imported

| Release change | Decision |
| --- | --- |
| Compact model picker and effort controls (#471) | Keep Noches' existing harness/model/reasoning picker. Upstream couples the new UI to different zui/gpui-component pins, glass/haptics helpers and composer state. A wholesale pick would replace load-bearing Noches chrome. |
| Pi native RPC migration (#630) | Defer a dedicated adapter migration. Noches has ACP process/cancellation hardening, approval routing and a context-usage extension that must be reconciled rather than removed by cherry-picking the replacement. |
| Agent update monitoring and Homebrew update controls (#389, #661), Antigravity distribution changes (#617) | Defer the installer/update subsystem as a separate integration. It introduces new RPCs, runtime executable resolution, remote controls and platform-specific installation/signature policy; Noches retains its existing explicit installation and enablement policy. |
| Durable application updates (#595) | Do not replace Noches' epoch/channel feeds, prepared build identity or release-admission safeguards. |
| Rust-core/UIKit iOS rewrite and subsequent mobile changes (#570 and follow-ups) | Keep Noches' newer Swift companion and its host folder browsing, project/workspace tools, queue/edit leases, uploads and motion acceptance coverage. The upstream rewrite is a different client architecture, not an additive patch. |
| Right-pane focus/history, side-chat harness/completion and layout changes (#568, #571, #572, #567, #588, #590, #612, #620) | Do not substitute upstream side-chat navigation for Noches' pane/view focus routing and per-pane draft ownership. These need behavior-specific ports if a Noches regression is demonstrated. |
| Right-tab close placement (#587), faster archive shortcuts (#602), provider details redesign (#596) | Leave existing controls and keymaps intact in this pass; they are UX choices rather than missing engine capabilities. |
| Encoded/out-of-folder file links and unlabeled inline file links (#606, #633) | Defer the broader file-link port. It introduces read-only absolute host-file access and a separate Markdown crate; preserve Noches' current workspace-bound links and line navigation rather than widening host access incidentally. |
| Wallpaper shuffling/positioning/zoom (#598, #660), GitHub-star banner (#586), landing/footer changes | Do not import upstream presentation/marketing changes into Noches' design system or website. |
| Child-chat command-palette exclusion (#651), explorer subagent ordering (#638), iOS thinking Markdown (#636), reduced-motion UI (#642) | The exact patches target structures that differ or already have Noches equivalents: subagent docs are not `Chat` rows with upstream's `parentChatId`; Noches already has a shared subagent selector, Markdown thinking and reduced-motion support. |
| Linux launcher/allocator changes (#627, #635), version bumps, CI and TestFlight changes | Keep Noches' packaging and CI separate. Linux/Windows runtime changes require platform validation, not assumptions from this macOS pass. |
| Upstream MCP injection and Cursor ambient settings (#498, #616) | Keep Noches' explicit browser/tool integration and pinned Cursor SDK. Ambient MCP/plugin loading requires separate approval and executable-resolution review; do not silently import repo-provided commands. |

Earlier deferred rich-composer invocation/catalog/MCP work remains deferred; this pass does not claim to have added command/skill chip protocol support.

## Validation

- `cargo test -p zeron-ui --lib`: **1,291 passed** on macOS, including pane/workspace surfaces, drag exclusivity, inline rename/delete, dirty-buffer reconciliation, selection isolation, table geometry, queue labels and fractional terminal scrolling.
- `cargo check -p zeron --locked`: **passed**, with existing macOS Objective-C macro `cargo-clippy` cfg warnings.
- `cargo test -p zeron-engine -p zeron-harness -p zeron-proto -p zeron-rpc --no-fail-fast`: the final run completed every target. Engine library **325 passed, 1 ignored**; harness library **232 passed**; protocol library **31 passed**; RPC library **22 passed**. Integration targets passed except the single intermittent shutdown assertion below. Real authenticated/quota-consuming tests remain ignored; those behaviors are not claimed validated.
- Passing integration coverage includes engine e2e (**23 passed, 2 ignored**), device routing (**9 passed**), workspace files (**7 passed**), and background-subagent idle reaping (**2 passed**).
- The final broad core run exited **101** because the unchanged `local_first::online_runtime_shutdown_stops_edge_workers_and_retires_the_graph` test intermittently asserted `edge received requests after shutdown returned` (**9 observed, 8 expected**). It passed in the previous full run. A dedicated `cargo test -p zeron-engine --test local_first` recheck then passed **all 11 tests** without code changes. The broad run is not reported as clean, and shutdown behavior is not claimed fixed by this port.
- `git diff --check HEAD` and formatting checks for all changed Rust files: **passed**. All 13 selected upstream source commits were verified as ancestors of the published release commit.
- Core validation exposed three stale upload tests from Noches' earlier hardening (`93a8ccb6`). Assertions now use the authoritative hashed `pending_target` path and user-facing filenames (`red.png`, `photo.png`, `shot.png`), not the old raw upload-ID prefix. Production upload behavior is unchanged; the dedicated upload/accounts/title integration target passes all **17 tests**.
- Native desktop visual QA in light and dark appearance remains pending: computer-use tools are unavailable in this session. The GPUI test backend is not a substitute for native visual inspection. Linux, Windows and iOS builds are not claimed.
- Delivery follows `AGENTS.md`: commit the integration on a dedicated branch, push it to Noches' `origin`, and open a pull request. PR merging and release publication are outside this integration's scope.
