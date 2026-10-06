# Application responsiveness audit — 2026-10-06

This audit found new application-level work that bypassed the existing renderer
and presentation caches: five continuously polling outbox workers, unindexed
queries over durable history, whole-backlog publication decoding, full-history
idle queue reconciliation, and token-by-token subagent inventory scans.

The changes remove those costs without changing animation clocks, blur,
appearance, pane geometry, transcript content, or durable execution guarantees.
They are not a claim of universal parity with the installed Zeron application.
The polling queries run through the shared synchronous SQLite authority used
by document storage (`DocsStore::with_connection`); avoiding them also removes
unnecessary competition for that connection. Input latency from this contention
was not measured directly.

## Measured results

Baseline: `dev` at `d0e08ba26f891e1101629a7ea0bdb838e5dfb752`.
Candidate: the application changes in this PR. Executables were copied before
other worktrees could overwrite the shared Cargo target directory.
[Measurements and executable hashes](performance/application-responsiveness.json).

Host: Apple M5 Max, Mac17,14, 36 GiB RAM, macOS 27.0.1 (26A434), arm64,
Rust 1.98.1. CPU percentages use **100% per core**.

### Optimized-test microbenchmarks

Four runs per build, sequentially ordered B/C/C/B/B/C/C/B. Both application
packages use test-profile `opt-level=2`; these are **not release or native
presentation measurements**. Only the identical ignored benchmark fixtures
were added to the baseline source. Each benchmark uses a fresh isolated fixture.
The runner rejected concurrent Cargo/compiler/linker activity; no such overlap
was observed during this retained comparison.

The table reports the median of four runs, for the entire stated workload.

| Controlled workload | Baseline | Candidate | Less elapsed time |
| --- | ---: | ---: | ---: |
| 1,000 empty effect claims, 50,000 terminal effects | 3,519.59 ms | 13.57 ms | 99.6% |
| 1,000 empty publication steps, 50,000 published batches | 1,176.09 ms | 2.10 ms | 99.8% |
| Drain 100 pending publication batches, 32 KiB payload each | 186.00 ms | 14.28 ms | 92.3% |
| 500 text appends, 10,000-entry history, no agents | 35.83 ms | 22.97 ms | 35.9% |
| Same append workload with live and finished agents | 36.82 ms | 22.73 ms | 38.3% |

Both append workloads contain 21.6 MB of text. Full subagent selector scans fell
from **500 to zero** in each run. This measures the AppState reducer, not the
background preparation pipeline, layout, GPU work, or input-to-presentation
latency.

An additional candidate-only benchmark compares the former queue repair order
against the equality-first order using **the same current Store and indexes**:
50 unchanged repairs with 10,000 retained projection records (21.6 MB of text)
took a median **1,792.44 ms versus 0.284 ms**. This is an in-process reference
comparison, not a historical-binary or full-daemon measurement. The fast path
does not decode that retained history; it reads the patch IDs and small intent
row. A regression test verifies two Store reads and zero writes for one unchanged
hosted queue, while changed queues still enter normal reconciliation.

### Release-engine idle comparison

Two retained runs per build, B/C/C/B, using immutable `cargo build --release
--locked -p zeron` executables. Each run starts a fresh headless engine, inserts
50,000 succeeded effects and 50,000 published batches, waits ten seconds after
setup, then samples approximately fifteen seconds. There is no UI, chat, or
model turn. Normal provider discovery remains enabled; startup is excluded.
Only the engine process is sampled, not discovery/provider child processes.

| Engine-only metric, 50,000 retained rows per outbox | Baseline range | Candidate range |
| --- | ---: | ---: |
| Idle CPU | 51.84–54.23% | 0.089–0.096% |
| Peak RSS during sampling | 103.97–105.55 MiB | 101.53–102.55 MiB |
| Peak macOS physical footprint during sampling | 71.24–72.83 MiB | 67.85–69.47 MiB |

Mean CPU fell approximately **99.8%** in this retained-history fixture. RSS and
physical-footprint differences are small and should not be treated as a general
memory improvement. The macOS idle-wakeup counter is not a count of Tokio timer
polls; the deterministic worker regression is the work-avoidance evidence.

An empty-outbox control also reduced engine idle CPU from 1.98% to
0.095–0.098%. At present this control has one retained baseline and two candidate
runs, so it is not presented as a matched four-run comparison.

Two retained-history baseline attempts and an empty-outbox baseline attempt
overlapped other worktree builds and were rejected, not included. Earlier
native-window resource samples that lost foreground were also discarded.
No UI CPU or native frame-time result is claimed here.

## Implemented changes and correctness boundaries

- **Park the four effect workers and publication worker.** Store clones share a
  commit notification. Workers subscribe and mark the current version seen
  *before* inspecting work, so commits between checking and sleeping cannot be
  lost. Only committed row changes notify; empty claims, no-row updates and
  rollbacks do not wake their own workers. A commit still notifies if the
  simulated after-commit response is lost.
- **Retain deadline and recovery behavior.** Pending retry deadlines and running
  lease expirations wake effect workers. The same query supplies claims and
  deadlines, preserving per-thread/lane FIFO and uncertainty barriers. Blocked
  followers cannot create a zero-delay loop. Independently opened diagnostic
  Store handles are repaired within thirty seconds; errors retain a cancellable
  100 ms backoff.
- **Index hot outbox state.** Partial indexes cover pending effect order, running
  lease expiry, and pending publication order. A covering thread/lane/state/order
  index replaces terminal-history scans in the FIFO barrier probe. Terminal
  history remains durable. The additional indexes cost storage and some
  insertion/update work, and must be built once on migration. There are no JSON
  expression indexes that would interfere with corrupt-projection recovery.
- **Decode one pending publication.** The worker reads `ORDER BY ordinal LIMIT 1`.
  The public all-pending diagnostic API remains. Publication order, serialization
  lane, replay fencing, and per-document acknowledgment remain unchanged. A bad
  later batch cannot poison a valid head, while a bad head still fails closed.
- **Compare queue intents before full projections.** Idle repair holds the queue
  lock and checks the small Loro/SQL intent views first. Changed queues still
  load the projection and enforce archive/deletion/authority rules. An unchanged
  empty legacy queue is not passively adopted.
- **Keep spawn presentations on pure text/reasoning deltas.** Primary and
  pane/subagent reducers reuse their existing shared inventories. Referencing
  parents still refresh child previews, including self-links, and delegation
  nudges remain. Structural/status changes still rebuild the inventory. This
  avoids rescanning a parent's unrelated spawn inventory per streamed token; it
  does not skip actual transcript application or child-preview updates.
- **Make the mock demo usable for verification.** `dev-demo.sh` honors
  `CARGO_TARGET_DIR` and idempotently registers the configured mock instance.
  The normal provider catalog intentionally does not enable mock. Without
  registration the first demo send persisted its user text but could not run.
  Production provider settings are not changed.

## Application-wide triage

The survey covered sidebar/split/composer state, transcript preparation and
rendering, subagents, provider discovery, orchestration/queue/scheduler/PR
workers, Git/file work, sync/RPC watchers, persistence, and existing renderer
performance reports. The following separates fixed problems from source-backed
candidates that still need attribution.

| Area | Finding and disposition |
| --- | --- |
| Durable orchestration | Fixed the five 25 ms polling loops, retained-history lookup scans, and whole-publication-backlog decoding described above. |
| Queue reconciliation | Fixed full-projection reads for unchanged queues. The 250 ms repair/snooze loop in `orchestration/assembly.rs` and lifecycle JSON scan still exist. Moving this to notifications needs Loro-side wake and deadline coverage. |
| Transcript state/subagents | Fixed redundant spawn inventory scans in both reducers. `doc/src/transcript_delta.rs` still finds append targets linearly from the front; the remaining reducer time is not attributed entirely to this lookup. |
| Background transcript preparation | `TranscriptPreparation::prepare` in `ui/src/transcript.rs` still visits retained entries, builds maps, accounts bytes, and captures a navigation baseline on each update, despite cached row parsing. Incremental preparation is a high-value next profiling target, not a shipped fix. |
| Sidebar/splits/composer | Existing scene/row caches, stable pane routing and virtualization remain. The sidebar mounts all visible-list cards when invalidated. Large-list attribution is still needed; naive cached-row reuse previously broke geometry and should not be reintroduced. |
| Models/providers | `ListProviderInstances` in `engine/src/rpc.rs` calls `refresh_all` on every request. Each installed instance's discovery has a ten-second timeout. Cached snapshot reads with explicit/stale refresh could improve picker responsiveness, but freshness/account semantics and real-provider timing need coverage before changing this. |
| Git polling | `ui/src/git_store.rs::ensure_pull_watch` keeps a 30-second polling task per watched space and notifies unconditionally. Narrowing watch lifetimes and suppressing unchanged results are candidates; active pull/status freshness must be retained. |
| PR monitoring | `orchestration/pull_requests/reactor.rs::threads` loads every full projection before filtering PR links every sixty seconds. Lightweight thread/link reads are a candidate; settlement and stack-watch semantics need dedicated tests. |
| Shell notifications | The session observer in `ui/src/shell.rs` finds chat titles linearly per session. A keyed lookup could reduce large-registry work; it was not established as a meaningful bottleneck in this fixture. |
| Persistence | Settings and layout writes are debounced/atomic but flush synchronously on the UI thread (`settings::flush_latest`, `Shell::flush_workspace_layout`). Background writes need ordered revision handling, multiwindow locking and shutdown flush guarantees; no unverified asynchronous rewrite was made. |
| Sync, RPC, presence, file/syntax work | Existing transcript deltas, bounded watches, idle presence, background syntax/file/Git tasks, and shared read models remain. No measured regression justified replacing these pipelines. |
| Rendering/Metal | Existing bounded highlight/Markdown/blur caches and scene reuse are retained. No dependency, shader, blur quality, animation clock or native presentation change was made. Software-Vulkan historical results are not used as native performance evidence. |

For prior renderer and native measurements, see
[render attribution](performance-render-caching.md),
[resource profiling](performance-resource-usage.md),
[native macOS profiling](performance-macos.md), and
[native stability](performance-macos-stability.md).

## Validation and limits

- Library suites: **637 engine + 1,498 UI tests passed**, zero failures; four
  engine and two UI tests are explicitly ignored, including opt-in benchmarks.
  The full UI suite includes pane/split/sidebar regressions.
- Targeted integration suites: **40 passed**, zero failures, two explicitly
  ignored. Targets: `message_queue`, `orchestration_bootstrap`,
  `orchestration_mcp`, `queue_lifecycle_rpc`, `queued_attachments`,
  `registry_adoption`, `scheduler_bootstrap`, `session_publication`.
- Release build passed. Existing Objective-C configuration/deprecation and
  unused-code warnings remain; no warning cleanup is included.
- New regression coverage includes idle worker parking, prompt commit wake,
  check-to-wait races, retry/FIFO/uncertainty deadlines, expired claims,
  rollback/lost-response notifications, first-batch isolation, queue authority
  behavior, and primary/pane/child/self-reference subagent invalidation.
- Native macOS functional QA used the candidate release app with isolated
  engine/UI profiles and a seeded two-pane mock workspace. Dark and light
  appearance, primary and secondary streamed Markdown/reasoning/tool/subagent
  completion, secondary draft editing during streaming, keyboard text selection,
  independent transcript scrolling, sidebar focus routing and draft restoration
  after navigation were observed. Both turns showed completed agents (2/2).
  This used Computer Use to operate the native fixture and did not change the
  installed application or its profile.
- Native picker interaction and divider dragging were attempted but not
  conclusively verified. Their automated UI regressions passed; that is not
  a substitute for complete native visual/interaction QA.
- Script syntax, task-scoped Rust formatting and diff checks pass. Unrelated
  existing formatting drift in `state.rs` is deliberately preserved.

These are short, isolated macOS measurements. Long mixed-use sessions, real
provider end-to-end timing, 120 Hz presentation/input latency, GPU attribution,
Windows/Linux runtime behavior and mobile are unverified. The idle script has
a Linux counter path, but it was not exercised. The remaining candidates above
are not solved merely because these benchmarks improve.

## Reproduction

### CI portability follow-up

The first [Windows engine job for PR #46](https://github.com/KldsSeeGhosts/noches/actions/runs/37539231148/job/112527850783)
failed eight tests. All eight failures, including their assertion values and
errors, were already present in the preceding
[dev Windows engine job](https://github.com/KldsSeeGhosts/noches/actions/runs/37259096658/job/111602305767).
The follow-up is limited to fixture portability and a checkout rule:

- Keep the exact-hash MCP instruction file LF on every checkout. The pinned
  length and SHA-256 assertions remain unchanged.
- Set `core.autocrlf=false` only in the temporary checkpoint/launch repositories,
  preserving exact restored-byte assertions without changing the user's Git
  configuration or production restore policy.
- Use local file URLs for clone fixture sources so canonical Windows paths are
  not mistaken for SSH hostnames, and compare canonical checkout identity
  instead of Git's path spelling. Detached checkouts remain excluded.
- Use `cmd.exe` on Windows and `/bin/sh` on Unix in the terminal-idleness fixture;
  retain the live-terminal pull refusal and explicit terminal close.

An isolated global `core.autocrlf=true` configuration reproduced all three
Git-induced CRLF failures with the pre-follow-up macOS test executable. A
checkout probe with the new attribute rule retained the exact 5,521-byte prompt
and original SHA-256. After the fixture fixes, the full macOS engine library
suite and selected auth/login integration targets passed under that same global
setting: **637 library + 19 integration tests**, zero failures, four existing
ignored library tests. All eight previously failing test names passed locally;
the Windows-only `codex_catalog` target has no macOS tests.

Native Windows path/ConPTY execution and the unrelated
Linux browser window-discovery failure still require hosted CI verification;
these fixture fixes are not additional performance measurements.

### Performance reproduction

Build both revisions before any timing run; do not compile or profile
concurrently. Use fresh output directories, isolated profiles, and immutable
binary copies. The microbenchmarks need the same fixture-only additions in the
baseline: `orchestration/performance_tests.rs` and its test-module declaration,
and `subagents::tests::profile_streaming_subagent_projection`. The queue
reference benchmark lives only in the candidate and is labeled separately.

```sh
cargo test --locked -p zeron-engine -p zeron-ui --lib \
  --config profile.test.package.zeron-ui.opt-level=2 \
  --config profile.test.package.zeron-engine.opt-level=2 \
  -- --test-threads=1

# Build the optimized-test executables without running (repeat in baseline).
cargo test --locked -p zeron-engine -p zeron-ui --lib --no-run \
  --config profile.test.package.zeron-ui.opt-level=2 \
  --config profile.test.package.zeron-engine.opt-level=2
# Copy the executable paths printed by Cargo to stable files, then:
node scripts/perf-responsiveness.mjs \
  /tmp/base-engine-tests /tmp/base-ui-tests \
  /tmp/new-engine-tests /tmp/new-ui-tests /tmp/noches-micro-comparison

cargo build --release --locked -p zeron
# Repeat with each immutable release binary in B/C/C/B order:
node scripts/perf-orchestration-idle.mjs \
  /tmp/new-zeron /tmp/noches-idle-new-1 50000 15
# Pass 0 instead of 50000 for an empty-outbox control.

cargo test --locked -p zeron-engine \
  --config profile.test.package.zeron-engine.opt-level=2 \
  --test orchestration_bootstrap --test orchestration_mcp \
  --test queue_lifecycle_rpc --test message_queue \
  --test session_publication --test registry_adoption \
  --test scheduler_bootstrap --test queued_attachments \
  -- --test-threads=1

# Headed functional follow-up, not a timing harness:
NOCHES_PERF_TRACE=1 scripts/dev-demo.sh --slow
```

The Node scripts require Node 22+; release idle profiling also needs SQLite and
Xcode Command Line Tools on macOS. They fail closed on detected concurrent
builds and retain executable hashes and counters. The render trace is opt-in;
`draw_us` is CPU scene construction, not GPU or native presentation time.
For split demo seeding, follow the schema and instructions in `AGENTS.md`.
