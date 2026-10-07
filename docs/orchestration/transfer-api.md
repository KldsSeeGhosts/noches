# Transfer and file checkpoint API (wave 3)

Host-owned SQLite authority; replicas and read calls never execute providers,
consume a transfer, acknowledge delegated work, or restore files. MCP contracts
remain the generated T3 contracts. The desktop exposes conversation fork,
merge-back, lineage navigation and delivery audit in Thread details, with a
fork shortcut in the sidebar context menu. See the
[2026-10-06 parity audit](t3-parity-audit-2026-10-06.md).

## Designer contract

Types live in `zeron_proto::transfer`. RPC method strings are in
`zeron_rpc::methods`. These names and camelCase fields are stable.
Typed desktop clients are `RpcClient::{thread_transfer_state,
fork_thread, merge_thread_back, preview_file_checkpoint_restore, restore_file_checkpoint}`, each requiring
the chat's owner device and sending `targetDeviceId`.

| RPC | Params | Result |
| --- | --- | --- |
| `GetThreadTransferState` | `{chatId}` | `ThreadTransferState` |
| `GetThreadInheritedHistory` | `{chatId, before?, limit?}` | `InheritedHistoryPage` |
| `ForkThread` | `{chatId, commandId, targetChatId, sourcePoint, title?}` | `ThreadTransferResult` |
| `MergeThreadBack` | `{chatId, commandId, targetChatId, sourcePoint}` | `ThreadTransferResult` |
| `PreviewFileCheckpointRestore` | `{chatId, checkpointId}` | `RestorePreview` |
| `RestoreFileCheckpoint` | `{chatId, checkpointId, expectedHeadSha, expectedChecksum}` | `RestoreResult` |

Example passive response (collections abbreviated):

```json
{
  "threadId": "child",
  "version": 42,
  "lineage": {
    "parentThreadId": "parent",
    "relationshipToParent": "fork",
    "rootThreadId": "parent"
  },
  "forkedFrom": {"type": "run", "threadId": "parent", "runId": "run:parent"},
  "transfers": [],
  "handoffs": [],
  "checkpoints": [],
  "inheritedItems": [],
  "latestForkableRunId": "run:child:1",
  "latestMergeableRunId": "run:child:1"
}
```

- `transfers`: full generated V2 context-transfer shape, including source/base
  identities, status, resolution, and error. MCP `t3_thread_transfers` deliberately
  returns only `id`, `sourceThreadId`, `targetThreadId`, and `status`.
- `handoffs`: sanitized `id`, `transferId`, `threadId`, `targetRunId`, `strategy`,
  `status`, `coveredRunOrdinals`, `coverage`, `omittedItems`, `omittedItemIds`,
  `deliveryStatus`, and empty `summaryText`. No private historical payload or
  native delivery record is published in a replica.
- `inheritedItems`: passive source items with `inherited: true`, `sourceThreadId`,
  and `sourceItemId`. Their original thread/run/item identity is retained. The
  Details payload carries only a capped text preview of these.
- `GetThreadInheritedHistory` (capability `thread-inherited-history-v1`): the full
  read-only inherited transcript, newest page first. `entries` are
  `SessionMessageEntry`-shaped (user prompt, or one agent entry per run with text,
  reasoning, tool calls/results, diffs and viewed-image refs; approvals, input
  requests and handoff/fork markers are omitted). Text, tool output and diffs are
  capped per part and a page stops at a byte budget (`shortened` counts cuts);
  `limit` is clamped to 1..=50. `nextBefore` is the id of the oldest entry returned
  and is the only cursor: the inherited list is frozen at the fork's source run, so
  new parent runs never move it. An unknown cursor is refused. Images are fetched
  by the client through the host's `ReadAttachmentChunk` transport, never inlined.
- `checkpoints`: `FileCheckpoint {checkpoint, scope, cwd, headSha, treeSha,
  indexTreeSha, phase}`. `checkpoint` and `scope` are generated V2 types; phases
  are `started`, `completed`, and `backup`.
- `GetOrchestrationState` also includes `forkedFrom`, `transfers`, and
  `checkpoints`, alongside its existing delegation state.

### User conversation transfers

`sourcePoint` is `{"type":"run","runId":"..."}`,
`{"type":"checkpoint","checkpointId":"..."}`, or `{"type":"latest_stable"}`.
The desktop pins the explicit eligible run before submitting. A fork is lazy:
it does not discover a provider, start a process, or copy transcript messages
into the child document. Its registry chat immediately inherits the project,
checkout, branch and model selection. The first input resolves native fork or
bounded portable history using the selected adapter's actual capabilities.

Use a client-minted `commandId` and `targetChatId`, and retain the entire request
until its outcome is known. The kernel scopes receipts by source/action/key;
an after-commit response loss must be retried with the same request. Acceptance
returns `{targetChatId, sequence, refusal:null, chat}`. A definite planner
rejection returns `refusal` with `chat:null`; it is not an uncertain transport
failure. Reusing an accepted request key with another target is refused.

Writes route to the source's owning host and use user/web provenance, not an
impersonated MCP session. Merge-back requires a live same-project direct
parent on the same host, refuses a delegated target, and only prepares context
for the next parent input. It neither starts nor interrupts an agent and never
merges Git branches or writes working files. Pending queued/multiple-fork
merge restrictions are preserved (a waiting merge-back no longer blocks queueing).

The Details audit distinguishes `Prepared` (logical consumption with no
confirmed provider acceptance) from `Delivered` (native fork or an inline/
injected receipt). It displays strategy, run coverage, omissions and provider
identities, not private handoff text. Fork transcripts prepend an immutable
text preview plus a visible fork boundary. Preview limits are 100 messages/
10,000 Unicode scalars per message with explicit omission/shortening notices;
historical tools, approvals and reasoning do not become live controls. Source
history remains pinned and accessible through parent navigation.

### Files-only restore

Preview then submit the exact observed HEAD/checksum, never infer either:

```json
{"chatId":"child","checkpointId":"checkpoint:run%3Achild:started"}
```

```json
{
  "checkpointId": "checkpoint:run%3Achild:started",
  "cwd": "/repo/worktrees/child",
  "headSha": "0123456789012345678901234567890123456789",
  "checksum": "opaque-sha256-observation",
  "paths": [{"path": "src/lib.rs", "kind": "M"}, {"path": "new.txt", "kind": "D"}],
  "restoresStaging": true,
  "allowed": true,
  "refusal": null
}
```

```json
{
  "chatId": "child",
  "checkpointId": "checkpoint:run%3Achild:started",
  "expectedHeadSha": "0123456789012345678901234567890123456789",
  "expectedChecksum": "opaque-sha256-observation"
}
```

Success: `{"restored":true,"backupCheckpointId":"checkpoint:backup:<uuid>"}`.
Paths are checkout-relative A/M/D/T without rename folding. HEAD can be null
for an unborn checkout. `restoresStaging` means the index is reset relative to
the **current HEAD**, matching T3; it does not reapply the checkpoint's staging.
The captured index tree remains retained in the immutable backup's Git objects.
No conversation, run status, or branch HEAD is rewound.

Disabled previews include a literal `refusal`. Restore rechecks ownership,
idle state, HEAD and the worktree/index/status checksum, retains a durable
hidden-ref/SQLite backup, rechecks again, and mutates only afterward. Main
checkouts, shared/nested/symlink-alias ownership, ignored-file collisions, and
another checkpoint operation's checkout lock refuse restoration.
On a partial restore failure the backup ID is included in the error; no broad
automatic destructive rollback is attempted. The lock is advisory: external
editors/Git processes do not participate.

## Engine seams

- `Store::{thread_transfers, transfer_ui_state, checkpoint_timeline,
  file_checkpoint}` are passive reads.
- Idempotent domain DDL is reconciled on open, so independently numbered
  pre-merge transfer and scheduler/git-actions databases acquire all domains
  without dropping existing metadata.
- `FileCheckpointService::{capture_turn, preview, restore}` owns immutable
  `refs/noches/checkpoints/...`, with separate-index `diff_sync` snapshots.
  Deterministic turn capture IDs and receipted publication recover a ref written
  before SQLite publication. Git objects/references are fsynced.
- `transfer::ensure_start_allowed(transfers, thread)` must run before
  admission in the threads/queue planners. It is installed transactionally in
  `ThreadService`'s send planner and task/queue-drain paths, and in ordinary
  admission.
- `transfer::delivery::TransferThreadAccess` is a local
  `TODO(merge-threads)` raw-history seam retained after merging `orch/wave3`.
  `ThreadService::read` is not a semantic replacement: it returns paginated,
  rendered/truncated activity text without raw tool/native fields or a pinned
  through-run bound, and may acknowledge a delegated result. Transfer reads
  must never acknowledge or import later source runs. The adapter reads the
  threads slice's canonical kernel turn-items plus legacy message fallback and
  inherited fork history. All actual thread read/send MCP tools now use the
  real `ThreadService`; transfer delivery does not send a new user message.

## Implemented semantics and remaining parity work

Fork creates an idle inherited-config thread; native/portable resolution waits
for first input. `latest_stable` requires a completed checkpointed run. Explicit
sources support completed/waiting/failed/interrupted/cancelled, with in-progress
and rolled-back refusal. Merge requires same-project direct fork ancestry and
the recorded base, supersedes a prior pending transfer from that fork, and
transfers only local fork context. It is **not a Git merge**.

Portable history uses upstream's 16k token/64k UTF-8 JSON-byte budget, current
input/attachment/occupancy reservation, coverage, whole-item omission, and
pending/injected/inline receipts. Failed/interrupted starts carry ready handoffs
forward without changing the original consumed transfer's target run. Pending
native delivery is uncertain; it cannot be blindly re-injected. Provider-instance
changes retain independent backing provider handles. Returning to a compatible
instance resumes its own accepted root-native identity and sends only the
off-provider delta, including after restart. Changed model/options, a changed
checkout or unavailable native coverage use full reconstruction. A stale
caller resume cannot override a known selected provider handle. Imported
runless history survives portable handoffs without duplicating the current
input.
Competing forks retain T3's explicit refusal. Unlike T3, a pending merge-back
does not block queueing: the first non-automatic run to actually start consumes
it exactly once (`prepare_run`); an automatic delivery leaves it pending, and a
cancelled queued run never consumed it.

Native hooks implemented: Codex `thread/fork` at `lastTurnId`; OpenCode 1.x
`messageID` and 2.x `before` cut boundaries; Claude's lazy fork (below). Missing
source/later cursors use portable context instead of copying an unstable head.

Claude has no standalone fork primitive, so `fork_thread` only mints a deferred
native id (`claude-fork:v1:<child>:<parent>:<assistant uuid>`) and starts no
process. The child's first run expands it to `--resume <parent> --fork-session
--session-id <child> --resume-session-at <uuid>`; the host-chosen child id makes
a start that dies before `init` retry the same fork, and once the child's
transcript exists the token resolves to a plain `--resume <child>`. Turn refs
are the last top-level assistant frame uuid of each successful turn. Before any
fork is recorded as in flight, `SessionLifecycle::can_fork_now` must confirm the
uuid is in the parent's local transcript recorded in the child's working
directory; otherwise (head with no ref, legacy turn, other directory, parent
gone) delivery uses portable context. Verified against fake-CLI fixtures and CLI
2.1.292 flag parsing only, not a live model turn. Mocked protocol tests are
not live installed-provider verification.

Remaining parity work, not claimed complete:

- Pi/negotiated ACP native fork hooks (ACP `session/fork` is head-only); lifecycle operations are not inferred from native
  resume support. Codex cursor-less (legacy) source turns do fork natively: head
  fork, then paginated `thread/revert` of the counted settled later turns,
  refused (portable fallback or uncertain fork) when a later turn is live or the
  fork uses legacy history.
- Loaded-process native history injection (the optional lifecycle hook currently
  defaults to unsupported, so production delivery falls back to bounded inline).
- Handoff budgets prefer the adapter-declared catalog window
  (`Harness::model_context_window`; Claude 200K/1M by the `contextWindow`
  option), then accepted-root usage reports; undeclared windows retain T3's 128k
  fallback. Codex/OpenCode/ACP catalogs do not declare one yet.
- Cross-instance native-account resume compatibility, missing-native-input
  reconstruction, and conversation rewind. A bare `/compact` turn defers inline
  handoff delivery (the next ordinary turn still owes it) and is refused on a
  thread with no conversation.
- Exact project-root lookup for legacy null-worktree threads at the launch-slice
  seam: currently uncertainty conservatively refuses restore.
- Sparse checkout/submodule checkpoint parity: capture/restore explicitly refuse
  these checkouts until their private-index policy is ported. Live
  provider/Windows validation and macOS visual QA remain unverified; the
  macOS unit, filesystem, and mocked-provider suites below have been exercised.

Tests mirror `ContextHandoffBudget.test.ts`, `ContextHandoffDelivery.ts`,
`ThreadForkService`, and `CheckpointRestoreSafety` cases from the pinned T3
reference, covering portions of R3 C25/C26 rather than claiming every scenario.

## Post-merge local validation

After merging `orch/wave3`, the requested `linux-test.sh` wrapper runs locally
on macOS, with `LINUX_TARGET=target-w3-transfer` (no SSH).

- Engine `--lib orchestration::`: 162 passed, 0 failed, 1 ignored. Includes
  transactional `ThreadService` merge refusals and both pre-merge SQLite
  upgrade histories, alongside transfer/checkpoint mirrors.
- Engine integrations: `diff_sync_churn` 3, `message_queue` 25,
  `orchestration_bootstrap` 2, `orchestration_mcp` 1, `restart_resume` 8 passed
  (1 live test ignored); no failures.
- Touched library suites: doc 117, harness 264, MCP 8, proto 42, RPC 22 passed;
  no failures.
- Proto `--test orchestration_oracle`: 14 passed; mocked Codex
  `native_fork_uses_stable_turn_and_returns_fresh_identity_without_a_turn`:
  1 passed; no failures.
- Explicit touched-file `rustfmt --check` and `git diff --check`: passed.

The initial merged compile exposed six missing provider-instance fields in
struct/event literals; these were fixed before the successful runs. The only
remaining compiler warning is the pre-existing test-only `thread_local!` doc
comment in `sessions.rs`. No desktop UI was changed in that historical wave-3
validation. Current desktop and RPC validation is recorded in the parity audit
linked above.
