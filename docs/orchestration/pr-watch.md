# Thread PR links and watches (wave 3)

The five PR MCP tools retain the pinned T3 descriptors/input/output schemas.
Their handlers use the authenticated caller's thread (no caller-supplied
thread target). PR failures remain the separate tagged family internally and
are delivered as MCP `isError: true` text, matching T3's `failureMode: error`.
Capability, missing-thread, target, and known-nonopen checks precede mutation.
`watch_pull_request` links first atomically; `unwatch_pull_request` retains
the link. Duplicate link/unlink/watch calls report the upstream boolean flags.

## Authority and durability

Links, snapshots, native stacks, tombstones, and watch baselines are the
contract's existing `OrchestrationV2AppThread.pullRequests` field. The PR
planner goes through kernel dispatch: receipts, thread locks, event log,
projection updates, effect outbox, and publication commit together. There
is deliberately no parallel PR database/table whose contents could diverge
from a rebuilt thread projection. Old single-link projections are read as
manual links; an explicit empty array wins over that legacy field.

The engine starts the reactor on assembly, runs one pass immediately and
then waits one minute after each completed sweep. Host reads share a
host/repository/number identity across threads and run at concurrency four.
Open/unsettled links sync each pass; closed or settled links use a
15-minute slow cadence; merged links stop host reads. Closed links can
therefore discover a reopen. Stack read failure retains the old snapshot
and retries, including terminal reads. Missing stack siblings are linked
in the same transaction before the terminal snapshot is recorded.
Stack-dismissed tombstones count as present and never auto-reappear.

Watch baselines persist head SHA, failed check names, passed state, remarks
watermark/boundary IDs, conflict state, and consecutive comment-only wakes.
Required checks gate passing news when any exist; otherwise all checks do.
Failures/cancellations/action-required checks report immediately, before
passing news. Viewer comments are excluded case-insensitively (author is the
fallback only when viewer is missing). Unknown mergeability preserves prior
conflict knowledge. Head changes reset check baselines and the comment budget.
Ten consecutive comment-only wakes stop a watch; other progress resets it.
Fifteen consecutive failed reads stop with a durable failure wake. That read
counter is process-local, as in T3; restart delays the cutoff but never loses
the durable watch baseline or an accepted wake.

Watch progress uses a started-at compare-and-set. A read overtaken by
unwatch, restart, or settlement cannot resurrect the old watch or start a
stale turn. PR news becomes a monitor-source user message and logical run,
queued after active work. The existing continuation drain and provider
effect runner deliver it; no toast or separate PR provider-start loop is
involved. Monitor runs do not reopen a delegated task's original result.

Settlement only examines the thread's own visible links after sync. Any
unsynced/open layer blocks settlement. The selector covers explicit
overrides, pin/disable/archive, runtime requests, live/background work,
recent queued user turns, snooze wake rules, terminal-vs-user timestamps,
merge setting and inactivity setting. A sequence fence and repeated selector
check protect the final command. Reused branches with open PR work do not
settle. An unrelated PR merge supplies no authority to settle another thread.

## Public integration seams

`orchestration::pull_requests::PullRequestLinks` is the small host-owned store
boundary exposed by `OrchestrationHost.pull_requests`:

- `links(&ThreadId) -> Result<Vec<ThreadPullRequestLink>>`
- `link(&ThreadId, Identity, ThreadPullRequestLinkSource) -> Result<bool>`
  returns `alreadyLinked`.
- `unlink(&ThreadId, Identity) -> Result<bool>` returns `wasLinked`.
- `set_watching(&ThreadId, Identity, bool) -> Result<(bool, bool)>`
  returns `(watching, wasWatching)`.

Queue's metadata action uses `Manual`; Git creation auto-link uses `Created`;
MCP uses `Agent`; sync-discovered native siblings use `Stack`. Targets use
`identity::resolve(Target, Option<&ProjectHost>)` and `chains::identity` for
host-level canonical comparison. These methods do not modify the central
`OrchestratorService` trait.

`PullRequestHost` is the injectable host boundary; production GitHub reads
reuse `source_control::GitHubCli::read_json`, its login PATH, bounded timeout/
output, noninteractive environment and sanitized errors. GraphQL checks
ask `isRequired` on github.com only; Enterprise omits that unsupported field.
Check reruns are deduplicated by workflow/name and newest timestamp. Stack
404 is distinct from transient failure. Sync uses the ordered stack listing
(`includeDetails: false` in T3), not an extra detail request. Author avatars
survive the snapshot, and deleted/missing authors remain null. Every host
request has an explicit host/repository, never an inferred fork upstream.

`reactor::PrThreadContext` supplies checkout and settlement settings.
`TODO(merge-threads/queue)`: route its settings method to the merged
project/environment settings authority. Defaults currently match T3:
merge enabled, inactivity three days. The PR reactor handles linked threads;
queue's general lifecycle/inactivity service remains responsible for threads
without links. The production context resolves existing worktree first,
then the chat's recorded repository root/cwd.

## Stable UI read API

RPC: `GetThreadPullRequests {"chatId":"thread"}`.
Typed UI client: `RpcClient::thread_pull_requests(&str)`.
Owner read: `Store::ui_pull_requests(&ThreadId)`.
Wire type: `zeron_proto::pull_requests::ThreadPullRequestsUi`.

The same model is published as
`SessionDoc.orchestration().projection.uiState.pullRequests`. Remote RPC
uses this passive replica when it does not own the SQL thread. Empty/unadopted
threads return an empty model, not executable work. Reads never acknowledge
delegated task results or request host refresh.

```json
{
  "threadId": "thread",
  "version": 42,
  "pullRequests": [{
    "host": "github.com",
    "repository": "owner/repo",
    "number": 124,
    "url": "https://github.com/owner/repo/pull/124",
    "source": "agent",
    "watching": true,
    "state": "open",
    "title": "Second layer",
    "headBranch": "layer-two",
    "baseBranch": "layer-one",
    "isDraft": false,
    "checksState": "pending",
    "stack": {"kind": "derived", "position": 2, "size": 2}
  }],
  "chains": [{"kind": "derived", "numbers": [123, 124]}]
}
```

Entries preserve visible link insertion order, matching MCP. Chains are
bottom-to-top; native chains win over branch-derived chains; stack positions
are one-based. Reused/ambiguous branch names and cycles do not invent an
ordering. Before first host sync, state/title/branches/draft/checks are null.
`source` is nullable only for decoding older UI payloads; current host entries
always supply it. Every new UI field has a serde default.

## Validation scope and remaining parity work

Source-oracle ports cover R3 C01/C29/C30 and the PR portions of C15/C21:
`handlers.test.ts`, `pullRequestWatch.test.ts`, `PullRequestSyncReactor.test.ts`,
`ThreadSettlementService.ts`, and GitHub check/stack decoder tests. These
are deterministic Rust tests, not claims of live T3 trace conformance.
Tests also exercise real Toolkit framing/schema refusals, SQL rebuild/reopen,
transaction rollback, competing link calls, publication, stale watch CAS,
and shared continuation queue/drain.

Remaining integration/verification:

- Queue must call the public store for legacy metadata link/unlink; Git actions
  must call it after PR creation. No unrelated slice's implementation was edited.
- Project-host defaults currently use the existing checkout resolver with
  provider-name inference. Merge with launch's repository identity authority
  for arbitrary self-hosted provider kinds/configured Forgejo web origins.
- Event-driven force-sync hooks (`requestSync` after link/terminal host
  events and PR-closing shell commands) remain integration work. The minute
  and 15-minute periodic sync cadences are implemented.
- Production host state reads are GitHub/Enterprise only. Recognized GitLab,
  Forgejo, Bitbucket and Azure URLs normalize/link correctly but their state
  backends are not implemented here.
- GitHub activity follows T3's `gh pr view` comments/reviews plus bounded
  GraphQL review-thread walk (ten pages, first ten replies per thread).
  Degraded reads defer remarks; empty review containers/pending reviews do
  not wake, while verdicts and dismissed-review reasons do.
- T3 closes only idle thread-owned terminals on settlement. Noches terminals
  currently expose neither thread ownership nor idle-shell detection; do not
  close arbitrary user terminals to approximate that behavior.
- Headed light/dark chips, live authenticated `gh`, Windows and iOS
  verification were not performed. No gpui chrome was changed.

## Validation record

Cargo runs used
`LINUX_TARGET=target-w3-pr-watch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh /Volumes/DevDrive/AiStack/noches-wt/w3-pr-watch`.
Initial runs were on Linux. At the user's request the wrapper switched to
local macOS execution with two cargo slots; no SSH was used after that
instruction. The original remote wrapper masked cargo's exit status, so
Linux results below were checked from cargo's output, not the shell status.

- `test -p zeron-engine -p zeron-proto -p zeron-rpc -p zeron-mcp`: passed
  all four crates' unit, integration and doc tests (live-host tests ignored).
- Expanded `test -p zeron-engine --lib` (before the last legacy Forgejo
  fixture): `478 passed; 0 failed; 3 ignored`.
- Final `test -p zeron-engine --lib orchestration::pull_requests`:
  `32 passed; 0 failed`; includes Azure URL construction, legacy Forgejo
  stack tombstones, GraphQL degradation, review-container exclusion and
  bounded thread replies.
- `test -p zeron-proto -p zeron-rpc -p zeron-mcp`: passed, including the
  new typed RPC roundtrip test.
- Serial Linux integration run:
  `test -p zeron-engine --test orchestration_bootstrap --test orchestration_mcp --test local_first --test message_queue -- --test-threads=1`
  passed all 39 tests (2 bootstrap, 1 MCP, 11 local-first, 25 queue).
- Final local macOS checks after the user freed the Linux host:
  `test -p zeron-engine --lib orchestration::pull_requests`: `32 passed; 0 failed`.
  `test -p zeron-engine --lib source_control::tests::orchestration_`: `5 passed; 0 failed`.
  `test -p zeron-rpc --test pull_requests`: `1 passed; 0 failed`.
  `test -p zeron-proto --lib pull_requests::tests`: `2 passed; 0 failed`.
  The latter reran the Linux build the orchestrator killed (`SIGTERM`) and
  covers exact host routing, bounds/errors, GraphQL degradation/pagination,
  summary-only stack reads, avatars and deleted authors.
- The first broad run failed in the existing message queue test:

  ```text
  ---- queued_text_waits_for_a_steerable_turn_even_with_legacy_policy stdout ----
  thread 'queued_text_waits_for_a_steerable_turn_even_with_legacy_policy' panicked at crates/engine/tests/message_queue.rs:656:29:
  called `Result::unwrap()` on an `Err` value: SendError(())
  test result: FAILED. 24 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.15s
  ```

  The isolated rerun passed (`1 passed; 0 failed`), and the subsequent full
  four-crate run passed. No queue-test or queue implementation was changed.
  The final normal parallel `test -p zeron-engine` hit this test again:

  ```text
  thread 'queued_text_waits_for_a_steerable_turn_even_with_legacy_policy' (205531) panicked at crates/engine/tests/message_queue.rs:656:29:
  called `Result::unwrap()` on an `Err` value: SendError(())
  thread 'tokio-rt-worker' (205577) panicked at /home/kidsseeghosts/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.53.1/src/runtime/time/entry.rs:539:9:
  A Tokio 1.x context was found, but it is being shutdown.
  test result: FAILED. 24 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.02s
  error: test failed, to rerun pass `-p zeron-engine --test message_queue`
  ```

  The unchanged test waits for a user transcript entry, not for the fake
  harness to subscribe to its broadcast finish signal. Thus it can send
  at line 656 while there is no receiver. Do not claim the final normal
  parallel engine suite is green.
- Another broad rerun failed in the unchanged shutdown test:

  ```text
  ---- online_runtime_shutdown_stops_edge_workers_and_retires_the_graph stdout ----
  thread 'online_runtime_shutdown_stops_edge_workers_and_retires_the_graph' (154678) panicked at crates/engine/tests/local_first.rs:700:5:
  assertion `left == right` failed: edge received requests after shutdown returned
    left: 11
   right: 9
  test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.89s
  error: test failed, to rerun pass `-p zeron-engine --test local_first`
  ```

  Its isolated rerun passed (`1 passed; 0 failed`). Treat both intermittent
  failures as follow-up if they recur in integration CI; they were not
  suppressed and no unrelated test was modified.
- Targeted `rustfmt --check` and `git diff --check`: passed.
- Strict Clippy remains blocked by pre-existing warnings. Normal Clippy
  first stops in `zeron-update` at `src/lib.rs:560` and `:620`
  (`collapsible_if`). RPC-only/no-deps also finds the same warning in
  `remote/client.rs:182`, `remote/server.rs:299` and `server.rs:64`.
  New PR-domain warnings found during review were corrected; unrelated
  warnings were not suppressed or edited.

Final engine-only strict-Clippy output (`--lib --no-deps --message-format=short -- -D warnings`):

```text
crates/engine/src/agent_accounts.rs:3778:9: error: this `if` statement can be collapsed
crates/engine/src/diff_sync.rs:14:5: error: doc list item without indentation
crates/engine/src/diff_sync.rs:15:5: error: doc list item without indentation
crates/engine/src/diff_sync.rs:16:5: error: doc list item without indentation
crates/engine/src/doc_host.rs:3725:5: error: this function has too many arguments (8/7)
crates/engine/src/orchestration/command.rs:510:43: error: using `clone` on type `RuntimeMode` which implements the `Copy` trait: help: try removing the `clone` call: `set.runtime_mode`
crates/engine/src/orchestration/command.rs:514:47: error: using `clone` on type `ProviderInteractionMode` which implements the `Copy` trait: help: try removing the `clone` call: `set.interaction_mode`
crates/engine/src/repos.rs:1777:13: error: this method chain can be written more clearly with `if .. else ..`: help: try: `if empty_query { path_a.split('/').count().cmp(&path_b.split('/').count()) } else { std::cmp::Ordering::Equal }`
crates/engine/src/repos.rs:1782:13: error: this method chain can be written more clearly with `if .. else ..`: help: try: `if empty_query { dir_a.cmp(dir_b) } else { dir_b.cmp(dir_a) }`
crates/engine/src/run_journal.rs:114:13: error: this `if` statement can be collapsed
crates/engine/src/sessions.rs:1816:9: error: this `if` statement can be collapsed
crates/engine/src/sessions.rs:2229:9: error: this `if` statement can be collapsed
crates/engine/src/sessions.rs:2483:40: error: redundant guard
crates/engine/src/terminals.rs:482:24: error: this expression creates a reference which is immediately dereferenced by the compiler: help: change this to: `bytes`
crates/engine/src/workspace_files.rs:1980:10: error: using `chunks_exact` with a constant chunk size: help: consider using `as_chunks` instead: `as_chunks::<2>().0.iter()`
error: could not compile `zeron-engine` (lib) due to 15 previous errors
```

The seeded incremental cache also produced corrupt-artifact warnings and
one `error[E0463]: can't find crate for zeron_doc` during an earlier Clippy
attempt. Final runs disable `profile.dev.incremental` and
`profile.test.incremental`; no build directory was deleted.
