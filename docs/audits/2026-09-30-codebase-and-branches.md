# Noches codebase audit and branch consolidation

Audit date: 30 September 2026.

The committed iOS work is already in both `main` and `dev`. The current checkout is on an obsolete, fully merged branch, with unrelated uncommitted changes layered over it. The most important remaining engineering work is protecting offline data, isolating RPC backpressure, making update installation exclude new work, and enforcing release checks. This audit did not implement fixes, commit existing edits, push changes, delete branches, or deploy anything.

This is a broad, risk-based codebase analysis with executable checks, not a claim that every line or platform has been exhaustively proven correct. The tracked source inventory covers approximately 351,500 lines across the application, crates, edge, and scripts, including tests. The review follows the architecture and high-risk data paths.

## Branch state and the iOS changes

Remote refs were refreshed and GitHub branch metadata was read.

| Ref | Commit | Relationship to dev |
| --- | --- | --- |
| `origin/dev` | `7943b217` | Current integration branch |
| `origin/main` | `7943b217` | Identical to dev |
| Local `fix/ios-motion-display-clock` | `b07e6d1a` | Zero unique commits; five commits behind dev |
| Local `mobile/parity` | `5bd719ad` | Zero unique commits; twelve commits behind dev |
| `origin/automation/cursor-sdk-update` | `bad0d3f8` | One unique SDK pin update |

The remote iOS fix and mobile parity branches are already gone. The iOS fix branch survived locally because the working directory remained on it after integration. Its changes went through PR #26, merged at `afaf60d2`. The broader mobile work was integrated at `69b36dc7`. A further keyboard sampling correction, `5280d7e0`, is also in current dev and main.

The five newer commits on dev include updated release runners, that iOS test correction, and the epoch-one updater migration. They must not be replaced by the older versions of overlapping files in this checkout.

The primary worktree began with 20 modified tracked files, plus untracked Xcode user state and a `downloads/` directory. The tracked edits include:

- iOS signing team, file-sharing settings, fallback model catalog, and demo data.
- Pi adapter and reasoning discovery changes.
- Desktop model catalog changes.
- Product display version, updater UI, packaging, and release publishing changes.

`git switch dev` safely refused to switch because it would overwrite local changes in `crates/update/src/lib.rs`, `docs/desktop-updates.md`, `scripts/noches-release.py`, and `scripts/test-noches-release.py`. The attempt changed no files.

The separate `/Volumes/DevDrive/AiStack/noches-mobile` worktree also has uncommitted files. Its dashboard, wordmark, section-header, font binary, and brand implementation are already represented in current dev. Comparison against dev leaves a pairing-copy difference; the font license differs only in whitespace. Its untracked export settings were not imported. Removing that worktree without preserving its state would still be inappropriate.

GitHub currently has three branches: main, dev, and the SDK automation branch. The automation branch changes `@cursor/sdk@1.0.31` to `1.0.32`; it is not iOS work and had no open PR at inspection time. The scheduled workflow recreates this branch and opens PRs against **main**, bypassing dev as the intended integration destination.

Recommended consolidation:

1. Preserve both worktrees' uncommitted state with explicitly identified backups.
2. Reconcile selected local deltas onto current dev. Do not merge the obsolete branch to accomplish this; its committed work is already included.
3. Port display-version changes onto the new epoch-one publisher without reverting its migration and immutable-release protections.
4. Resolve the signing-team mismatch described below and validate Pi's new discovery path before committing those edits.
5. Verify and integrate, or explicitly abandon, the unique SDK update. Change its PR base to dev. Decide whether “only main and dev” allows temporary automation branches; otherwise the scheduled branch-creation workflow must change.
6. Move the primary checkout to dev, detach or repoint the preserved secondary worktree, and delete only the fully merged local branch refs. Keep main as a local tracking branch if both long-lived branches should appear locally.

No additional push of the already committed iOS feature work is necessary.

## Review coverage and evidence limits

| Area | Review focus | Validation |
| --- | --- | --- |
| Engine | Dispatch and steering, shutdown, update admission, journals, uploads, workspace files, profiles | Library suite and twelve integration targets |
| RPC | Multiplexing, cancellation, closure, gateway identity and authentication, relay retirement | Library/integration suites and isolated defect probes |
| Documents and sync | Command durability, snapshots/cursors, CRDT projection, admission and backfill | Library tests and source inspection |
| Harnesses | ACP/Codex/Claude/Cursor/OpenCode lifecycles, discovery, installation, ownership | Library suite and nine integration targets |
| Desktop | Sidebar ordering, panes, layout persistence, transcript/composer caches, update UI | All 1,255 UI library tests |
| iOS | Persistence, pruning, runtime retirement, transcript projection, direct companion transport | Full unit suite plus local companion/gateway integration |
| Edge | Authentication, registry validation, request budgets, checkpoint/backfill | TypeScript, unit tests, and real workerd tests |
| Workspace, syntax, theme, browser | Validation, persistence, cache boundaries, transport | Unit and integration tests |
| Voice | Tool protocol, cancellation, credential handling, resampling | Nine library tests; no live voice call |
| Release and packaging | Branch/channel policy, updater migration, CI dependencies, signing | Local script tests; current dev source and GitHub metadata |
| Chromium, native CUA, Windows/Linux platform code | Entry points and platform-specific limits | Inspection only for native execution; no new Chromium package or CUA installation |

Most source files are identical between the dirty checkout and current dev. The ten files changed by the five newer dev commits were inspected through `git show origin/dev:...` and the branch diff. Local tests ran against the **preserved dirty checkout based on `b07e6d1a`**, not a clean build of `7943b217`. In particular, local release-script tests do not certify the newer epoch-one publisher, and the local iOS run does not contain the later `5280d7e0` test-only change.

No physical iPhone, live Tailscale host, paid provider turn, production edge deployment, signed TestFlight upload, or installed-application update was exercised. Passing tests are evidence for their covered behavior, not evidence that the findings below are absent.

## Confirmed bugs and reliability findings

Priority P1 means address before relying on affected durability or release behavior. P2 means important correctness or hardening work with a narrower trigger. “Source-confirmed” identifies a code path, not a reproduced production incident.

### 1 Offline iOS commands can be deleted during startup pruning

**P1, source-confirmed.** [DocDisk.swift](../../apps/ios/Zeron/Sync/DocDisk.swift), `prune`, and [AppModel.swift](../../apps/ios/Zeron/App/AppModel.swift), `restore`.

`restore()` invokes `DocDisk.prune(keep: 80)` before loading the workspace or pending session stores. Pruning sorts all session snapshot files by modification date and deletes everything beyond the newest 80. It does not inspect pending-command metadata, even though `hasPendingCommands` and the snapshot header already expose it.

An older, offline-created command can therefore be deleted before `preloadSessions()` gets a chance to pin it. The in-memory residency policy correctly preserves pending stores, but that protection does not extend to startup disk pruning. Legacy and current snapshot files also share this count rather than being counted as distinct logical sessions.

Preserve any snapshot with pending work independently of the ordinary cache limit. Treat unreadable pending metadata conservatively. Add a test with more than 80 snapshots, the oldest containing an unsynchronized command; after restore, its command must remain deliverable.

### 2 iOS persistence errors are treated as successful saves

**P1, source-confirmed.** [DocDisk.swift](../../apps/ios/Zeron/Sync/DocDisk.swift), `saveChat2`, `DocSaver.flush`, and `RegistrySaver.flush`.

Snapshot export and writes use `try?` and return no success result. Both saver implementations clear `dirty` before attempting persistence. A full filesystem, failed export, or write error can thus leave an outbox only in memory while the saver considers it clean. Store eviction and process termination can then lose the edit.

Atomic replacement protects against a partially written file; it does not make a failed write successful. Return an error or durable success receipt, clear the dirty revision only after success, retain failed work through eviction, and surface a recoverable storage error. Inject a failing persistence backend and verify retry after recovery, pending-work pinning, and registry/session outbox survival.

### 3 One stalled RPC subscription blocks unrelated controls

**P1, reproduced.** [client.rs](../../crates/rpc/src/client.rs), `route_frame`.

The single connection reader awaits `tx.send(item)` into each stream's bounded queue. When one consumer stops reading and fills its 256-item queue, the reader cannot route subsequent responses for **any** request on that connection. A transcript watcher can consequently delay an unrelated stop, approval, or ordinary unary call.

The isolated probe filled a stream without consuming it. An unrelated Echo request timed out; dropping the stream immediately restored unary replies. Bounded memory is good, but connection-wide indefinite blocking is not acceptable isolation.

Introduce per-subscription overflow behavior: cancel and resync that subscription, or coalesce only snapshot streams whose semantics allow it. Delta streams cannot silently drop arbitrary items. Test one stalled stream alongside a prompt interrupt, unary calls, and another live stream.

### 4 RPC requests can hang after inbound closure

**P1, reproduced at the transport seam.** [client.rs](../../crates/rpc/src/client.rs), `RpcClient::new` and `call`.

The reader drains the pending map once when inbound closes, but records no shared closed state. A call registered after that drain can successfully enqueue while the outbound receiver is still retained, then await a response forever. Cancellation of a unary call also leaves its pending entry until a response or connection cleanup occurs.

A public-API probe closed inbound while retaining outbound, waited for reader cleanup, and made a new call. It timed out instead of returning `RpcError::Closed`.

Use a closed flag protected by the same pending-map synchronization as registration and reader cleanup. Give unary calls cancellation guards and appropriate deadlines. The harness JSON-RPC client already contains a related closed-state guard and can inform the design. Add tests for EOF racing registration, queued sends, request cancellation, and retained outbound transports.

### 5 Update installation does not exclude newly admitted work

**P1, source-confirmed.** [lib.rs](../../crates/update/src/lib.rs), `Updater::apply`; [lib.rs](../../crates/engine/src/lib.rs), the quiescence callback; [rpc.rs](../../crates/engine/src/rpc.rs), `CheckUpdateReady`.

Update application checks that no session or terminal is active, then awaits manifest fetch and staging before changing the installation and scheduling restart. Session dispatch and terminal creation do not acquire an admission barrier shared with that transition. Work can start after the check and before restart.

Desktop readiness is also a separate check from the eventual installation. Pre-staging reduces the time window but does not remove the race.

Introduce an engine lifecycle state or admission lease covering “idle check, stop admitting work, retire runtime, swap, restart.” Dispatch, queue drain, terminal open, and setup/project actions must all participate. Failure must release the barrier. Test attempted new work during a deliberately delayed staging operation.

### 6 Publication is not gated on the supported product test suite

**P1, source and repository configuration confirmed.** Current dev [.github/workflows/release.yml](../../.github/workflows/release.yml), inspected through `git show`, and GitHub branch/ruleset metadata.

The current publisher depends on prepare, Linux packaging, and macOS packaging. Those jobs run updater and Linux input-seat tests, but publication does not depend on the engine, sync, desktop UI, edge, or relevant compatibility suites. Those tests run in independent workflows. Both main and dev were reported unprotected, with no repository rulesets.

A successful build is not the same as a validated release. Gate publication on the actual candidate commit's required checks and enforce integration/promotion rules. Do not unnecessarily gate macOS/Linux shipping on unsupported Windows publication; define supported-platform checks explicitly.

At the inspected dev tip, the release and UI workflows succeeded while Windows UI tests failed. That is a separate Windows coverage finding, not proof that the supported macOS/Linux artifact itself is defective.

### 7 Duplicate RPC identifiers orphan server tasks

**P2, reproduced.** [server.rs](../../crates/rpc/src/server.rs), `serve_connection`.

Every request spawns a task and inserts its abort handle under the request ID. A duplicate ID replaces the old handle without aborting or rejecting the old task. Disconnect cleanup can then abort only the most recent task.

The probe sent two indefinitely waiting requests with ID 7. After disconnect, one request remained alive. The dispatcher also has no per-connection in-flight task limit; bounded frame channels do not bound spawned work.

Reject duplicate live IDs, or explicitly abort a superseded task if the protocol permits replacement. Add admission limits and tests for duplicates, disconnect cleanup, cancellation floods, and request saturation.

### 8 Attachment identities collide after eight characters

**P2, reproduced.** [uploads.rs](../../crates/engine/src/uploads.rs), `commit` and `pending_target`.

Committed attachments use `{first-eight-characters-of-upload-id}-{sanitized-name}`. Two distinct accepted IDs with the same prefix and filename resolve to the same file. The second commit overwrites the first attachment.

The probe committed `12345678-first` and `12345678-second` as `image.png`. Both returned the same path; the first attachment's contents became the second's.

Use the full validated ID, or a collision-resistant digest, for new filenames. Preserve old prefix-based references through an explicit compatibility lookup; do not break persisted `pending://` references while correcting the naming scheme. Test collisions, same-name attachments, and legacy resolution.

### 9 Ordinary attachment commits publish partial destination files

**P2, source-confirmed.** [uploads.rs](../../crates/engine/src/uploads.rs), `commit` and `resolve_pending`.

`commit()` writes directly to the final path with `std::fs::write`. That path can become visible as a regular file before its complete contents have landed. `resolve_pending()` treats file existence as readiness. A crash or write error can also leave an incomplete final file, or truncate a previous file during replacement.

Write to a unique sibling temporary file, validate and sync it, atomically rename, and only then make the upload available. Serialize commits for an upload identity. Tests should cover concurrent lookup during commit, injected write failure, retry after an ambiguous reply, and preservation of an existing completed file.

### 10 Registry wire validation throws on malformed JSON and undercounts bytes

**P2, reproduced.** [registry-core.ts](../../edge/src/registry-core.ts), `validateOp`, and [registry-room.ts](../../edge/src/registry-room.ts).

The TypeScript type annotation is not runtime validation. `validateOp(null)`, `set: null`, and `clocks: null` throw instead of producing a controlled rejection. The HTTP/WS handlers similarly cast decoded frames without first excluding `null`. Operation validation occurs outside the JSON parsing catch.

The operation budget uses `JSON.stringify(op).length`, which counts UTF-16 code units rather than UTF-8 bytes. A probe with 10,000 CJK characters was accepted at 10,097 code units despite encoding to 30,097 bytes, above the stated 16,384-byte budget. The iOS twin uses encoded `Data.count`, so these implementations disagree.

Accept `unknown`, validate object shapes and field types before accessing them, and count encoded bytes. Add shared malformed and Unicode vectors across TypeScript, Swift, and Rust. Invalid frames should return protocol errors without mutating rows.

### 11 Several HTTP limits are enforced only after buffering the request

**P2, source-confirmed.** [chat-room.ts](../../edge/src/chat-room.ts), checkpoint/sidecar/row uploads; [index.ts](../../edge/src/index.ts), tool blob PUT; [registry-room.ts](../../edge/src/registry-room.ts), HTTP push.

Several paths call `arrayBuffer()` or `json()` before enforcing their actual limit. Some check declared `Content-Length`, but that header alone does not enforce the streamed byte count. Oversized or chunked bodies can consume the worker's memory before being rejected.

Use one bounded body reader that stops as soon as the actual byte budget is exceeded, with a declared-length check as an early optimization. Test no Content-Length, lying lengths, chunked requests, and cancellation at the threshold. This is authenticated resource hardening; the audit did not establish a cross-account data exposure.

### 12 Desktop layout persistence has independent writers and a shared temporary path

**P2, source-confirmed concurrency risk.** [workspace_layout_store.rs](../../crates/ui/src/workspace_layout_store.rs).

Each Shell owns a separate layout store, while the store persists all project layouts to one device-level file. It uses the fixed sibling `workspace-layout.json.tmp`. Multiple windows can overwrite one another's independent snapshots; overlapping writers can also race on the same temporary filename.

The read path rejects files above 8 MiB, but flush does not enforce the matching serialized write limit. Enough stored layouts can therefore produce a file that the next launch treats as corrupt.

Share one revisioned device-level store or perform locked merge-on-write with a defined multiwindow ownership policy. Use unique temporary paths and the same read/write size contract. The standalone workspace persistence implementation already demonstrates unique temporary names and syncing. Add two-writer, different-project, failed-rename, and oversized-write tests.

## Uncommitted change hazards and unresolved test findings

### iOS signing configuration is inconsistent

The local Xcode edit changes both app configurations from team `5XY3M483YQ` to `78295MJ4K8`. Current dev's TestFlight export workflow still specifies `5XY3M483YQ`.

This is a **candidate integration defect**, not a claim about the unmodified dev project. Simulator tests use `CODE_SIGNING_ALLOWED=NO` and cannot validate it. Confirm the intended app/team ownership and use a single configured team for archive and export before publishing this local edit. Do not infer ownership from a locally selected Xcode team.

### New Pi discovery should not discard useful results

The uncommitted `refine_pi_ladders` switches through every discovered model serially inside the existing discovery deadline. One slow switch can consume the remaining budget and cause the entire successful catalog discovery to fail. `models()` then falls back instead of retaining useful discovered models.

It also keeps the old reasoning ladder whenever `thought_ladder` returns empty. A successfully advertised `off`-only model produces an empty supported ladder after filtering, which is different from missing or failed metadata. Keeping the old ladder can advertise reasoning levels that model does not support.

Use a refinement budget and per-switch deadlines, return the already discovered catalog on partial refinement failure, and distinguish missing options from an explicitly empty supported ladder. Test a large catalog, a hung switch, rejected switches, off-only options, and effort reapplication after model change. These are source-based candidate cases; no live Pi provider survey was performed.

### Relay sign-out failed once under parallel tests

The initial parallel RPC integration run failed `sign_out_closes_cached_peer_links` after its five-second deadline. The same test passed alone in 0.02 seconds, and all RPC integration tests passed serially.

Do not describe that as a reliably reproduced authorization bypass, and do not dismiss it solely because the rerun passed. Investigate process-wide wake/token notifications, transport retirement, and the closure/registration race in finding 4. The closure probe independently establishes a real generic RPC failure mode, but does not establish the cause of this specific sign-out test failure.

### Windows CI is not fully green

The dev-tip Windows run [36520191798](https://github.com/KldsSeeGhosts/noches/actions/runs/36520191798) reported successful harness/engine and app/packaging jobs, a failed UI-test job, and skipped native GUI validation. Its failed log identifies `settings::composer::tests::concurrent_projectless_saves_leave_a_complete_preference`: a concurrent reader observed `saved.no_project == false`. The suite reported 1,233 passed, one failed, and five ignored, with one filtered out.

[composer.rs](../../crates/ui/src/settings/composer.rs) uses unique temporary files and rename, but falls back to empty defaults on any read error. Investigate concurrent Windows replacement/read behavior rather than assuming a unique temporary name establishes the full read/write contract. The audit did not establish the precise OS error or whether the failure is a deterministic product defect or a timing-sensitive test issue. No native Windows rerun was performed here.

### TestFlight build allocation needs an explicit ordering contract

The workflow reads one page of at most 200 builds and chooses the maximum numeric version it sees. It does not follow pagination or specify ordering for that query. Once there are additional pages, it does not prove that the selected build number exceeds every existing build.

Prefer an explicit monotonic allocation scheme with a duplicate guard, or fetch the relevant complete history with an ordering/pagination policy. This is a source-based risk; App Store Connect history and credentials were not queried.

## Optimization opportunities

These identify avoidable work and proposed measurements. No percentage speedup is claimed.

| Priority | Opportunity | Source evidence | Proposed change and acceptance measurement |
| --- | --- | --- | --- |
| High | Journal reopening and replay | `RunJournal::scan_tail` reads the complete file and then decodes all lines. `last_event` and stale-session scans also materialize the entire history. Reaching 16 open files clears the whole cache. | Maintain a durable tail/sequence index, use bounded tail scans, and evict one least-recently-used descriptor. Benchmark restart, replay, and 17+ active chats against growing histories while preserving torn-write recovery. |
| High | Blocking upload work on async workers | `UploadChunk`, `UploadCommit`, and `ReadAttachmentChunk` call synchronous filesystem functions directly in the async RPC handler. | Move work to a bounded blocking worker/actor, with cancellation and upload ownership. Measure RPC interrupt/approval latency during large concurrent uploads and slow storage. |
| High | Upload staging rescans | Every append walks the staged chunks to recompute their size and sweeps all staging directories and files. With N chunks, repeated enumeration grows roughly quadratically. Commit retains the joined base64 and decoded payload simultaneously. | Track per-upload sizes and slots, rate-limit sweeping, and stream decoding into the atomic destination. Benchmark large uploads, concurrency, retries, and peak allocation. |
| High | Full mobile transcript projection | `SessionStore.project` coalesces background work, but `decodeEntries` still calls `getDeepValue` for the entire doc and rebuilds all messages/parts on updates. | Incremental projection keyed by changed messages/parts, retaining continuation and queue semantics. Compare CPU, allocations, and main-actor latency with large transcripts; do not remove generation/retirement guards. |
| Medium | Mobile markdown prefix scans | `IncrementalMarkdown.append` uses `source.count`, `dropFirst`, prefix checks, and a suffix search from the document start. Tail parsing is incremental, but finding the tail still traverses the prefix. | Retain safe append offsets and block-boundary indexes. Benchmark total work across increasing streamed document lengths; preserve Unicode and reference-link full-parse fallback. |
| Medium | Workspace image rereads | `read_image_blocking` reads and hashes the full image for every 384 KiB reply. An 8 MiB image requires 22 calls, approximately 176 MiB of application-level reads and hashing. This is derived work, not a measured disk-IO total. | Cache a bounded verified snapshot/descriptor keyed by checkout, path, and revision, or add a scoped image transfer. Preserve hash and changed-file checks. Compare hashes/bytes processed per preview. |
| Medium | Sidebar projection allocations | `sidebar_rows` clones and decorates all selected chats, sorts them, and builds every active row. Keyboard traversal recomputes the same projection. | Cache the shared projection against relevant data/settings/time revisions and virtualize large lists. Preserve exact rendered/shortcut ordering, semantic state colors, project badges, and flush pane layout. Measure at 100/1,000/10,000 sessions. |
| Medium | Discovery startup costs | New serial Pi refinement and existing short-lived discovery processes can make picker opening expensive. | Cache successful discovery with explicit invalidation and bounded refinement. Measure cold/warm picker latency without allowing stale configured models to disappear. |

Additional hardening work worth measuring or designing:

- Windows raw terminal output still uses an unbounded channel, unlike the bounded Unix reader. Its shutdown constraints need a Windows-specific solution, not a blind substitution.
- Unix ACP descendants now have owned process-group cleanup. Other harnesses still largely use direct-child ownership. Verify and define which detached servers should survive before broadening group termination.
- Update manifests use strict channel/product checks, HTTPS, mandatory hashes, and platform verification, but no independent manifest signature. Signing would reduce reliance on the feed/account as the sole authenticity authority; checksums alone do not authenticate a maliciously replaced manifest.
- Keep semantic model/catalog data generated or contract-tested across Rust and Swift. The current duplicated fallback lists make future parity drift easy.
- Add memory/latency acceptance budgets to existing workload rigs. Historical benchmark documents are useful baselines, not measurements of this dirty checkout.

## Existing safeguards worth preserving

The codebase already contains substantial hardening. A broad rewrite would risk losing it:

- Local and synced profile roots, attachment ownership, and runtime retirement are explicit.
- Document persistence tests exercise snapshot/cursor consistency and durable publication obligations.
- Command processing, queue edit leases, steering/recovery, and relay delivery have extensive regression coverage.
- Ordinary terminal replay/subscriber bounds and browser command-writer backpressure are implemented on Unix.
- Workspace file operations have path, symlink, content-hash, changed-checkout, and write-conflict checks.
- Archive installation checks digests and rejects traversal, links, special files, and excessive extraction.
- Split geometry, drag/drop, focus routing, sidebar order, theme contracts, and transcript anchors have large regression suites.
- Desktop sidebar rendering and jump shortcuts consume the same structural projection.
- Syntax caches are bounded and appearance-neutral; transcript/composer caching already avoids significant repeated layout work.

The current small preview re-pair probe passed: ten ordinary churn iterations and one re-pair retained 16 live tasks. This does **not** reproduce the older re-pair stall, so the September 20 finding should not be repeated as a current confirmed failure. It also does not prove long-duration reliability. The test's RSS helper reads Linux `/proc`; its zero on macOS is not a memory measurement.

## Validation results

| Check | Result |
| --- | --- |
| Core Rust libraries: proto, doc, workspace, sync, harness, update, browser, preview, theme, engine | Overall command succeeded |
| Desktop UI library, serialized | 1,255 passed |
| Selected engine integration targets | 88 passed; 5 ignored |
| Selected harness integration targets | 125 passed; 5 ignored |
| RPC library and integrations, serial | 22 library, 13 device-room, and 17 remote-connection tests passed; 2 integration tests ignored |
| Initial parallel RPC integrations | One sign-out test failed; isolated and serial reruns passed |
| Syntax | 18 library and 8 quality tests passed; 1 quality test ignored |
| Workspace | 4 library, 11 layout, and 6 import-format tests passed |
| Browser | 8 library tests passed |
| Theme | 25 library tests passed |
| Voice | 9 library tests passed |
| Edge TypeScript | Passed |
| Edge unit tests | 46 passed |
| Edge workerd tests | 20 passed |
| Local release-script tests | 5 passed; 1 skipped |
| iOS full unit suite | 186 passed; 7 skipped initially |
| iOS companion fixture rerun | 6 passed; only the live paired-host test skipped |
| Unique iOS tests across both runs | 192 passed; 1 live-host test unexercised |
| Preview churn and re-pair probe | Passed; 16 tasks before/after; no valid macOS RSS reading |
| Core Clippy including voice | Completed; 38 first-party warnings across nine crates; no automatic fixes applied |
| Isolated Rust defect probes | Four defects reproduced |
| Isolated edge validation probe | Null-shape exceptions and UTF-8 budget mismatch reproduced |

Representative commands:

```sh
cargo test --locked -p zeron-proto -p zeron-doc -p zeron-workspace \
  -p zeron-sync -p zeron-harness -p zeron-update -p zeron-browser \
  -p zeron-preview -p zeron-theme -p zeron-engine --lib
cargo test --locked -p zeron-ui --lib -- --test-threads=1
cargo test --locked -p zeron-rpc --tests -- --test-threads=1
PREVIEW_LEAK_ITERATIONS=10 PREVIEW_LEAK_CHURN=1 \
  cargo test --locked -p zeron-preview --test leak -- --nocapture
npm --prefix edge run typecheck
npm --prefix edge test
python3 scripts/test-noches-release.py
```

iOS used Xcode 27.0 and the available iPhone 18 Pro simulator, with signing disabled. Companion integration used local synthetic hosts on ports 28777/28778 and the repository's Rust gateway on 28779. These fixtures started no real agent and were stopped afterward.

Local logs, iOS result bundles, and isolated probe sources are retained under `/tmp/noches-codebase-audit.VYnxyS/`. The Rust probe uses the repository crates' public APIs from a temporary Cargo package; its own dependency resolution differs from the workspace lock. Its assertions exercise RPC routing/task ownership and upload filename behavior, not a CRDT-version comparison. Normal repository test runs used `--locked`.

## Recommended work order

First reconcile the working tree onto current dev without reverting the epoch migration. Then fix offline pruning and save-failure handling, RPC backpressure/closure/task ownership, and attachment integrity. Introduce update admission exclusion and supported-platform release gates before further updater rollout.

Next harden registry/body validation and shared desktop persistence, settle the signing/team and Pi candidate cases, and investigate the parallel sign-out failure. Only then pursue measured performance changes: journal indexing, upload actors/streaming, incremental mobile projection, and verified image caching.

Keep main as the promotion branch and dev as the integration branch. Temporary branches should carry genuinely isolated work, not become alternative long-lived copies of the iOS app. This audit added only this report to the repository; all pre-existing edits were preserved.
