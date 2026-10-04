# Transfer and file checkpoint API (wave 3)

Host-owned SQLite authority; replicas and read calls never execute providers,
consume a transfer, acknowledge delegated work, or restore files. MCP contracts
remain the generated T3 contracts. No gpui UI is implemented in this slice.

## Designer contract

Types live in `zeron_proto::transfer`. RPC method strings are in
`zeron_rpc::methods`. These names and camelCase fields are stable.

| RPC | Params | Result |
| --- | --- | --- |
| `GetThreadTransferState` | `{chatId}` | `ThreadTransferState` |
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
  "inheritedItems": []
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
  and `sourceItemId`. Their original thread/run/item identity is retained.
- `checkpoints`: `FileCheckpoint {checkpoint, scope, cwd, headSha, treeSha,
  indexTreeSha, phase}`. `checkpoint` and `scope` are generated V2 types; phases
  are `started`, `completed`, and `backup`.
- `GetOrchestrationState` also includes `forkedFrom`, `transfers`, and
  `checkpoints`, alongside its existing delegation state.

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
- `transfer::ensure_start_allowed(transfers, thread, queued)` must run before
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
changes use full/delta handoffs and never blindly resume another instance's ID.
Queued pending merges and competing forks retain T3's explicit refusals.

Native hooks implemented: Codex `thread/fork` at `lastTurnId`; OpenCode 1.x
`messageID` and 2.x `before` cut boundaries. Missing source/later cursors use
portable context instead of copying an unstable head. Mocked protocol tests are
not live installed-provider verification.

Remaining parity work, not claimed complete:

- Claude/Pi/negotiated ACP native fork hooks; Codex legacy paginated fork/revert
  fallback; lifecycle operations are not inferred from native resume support.
- Loaded-process native history injection (the optional lifecycle hook currently
  defaults to unsupported, so production delivery falls back to bounded inline).
- Live model-catalog context-window lookup. Accepted-root/native-identity usage
  reports are wired; unknown windows retain T3's 128k fallback.
- Cross-instance native-account resume compatibility, missing-native-input
  reconstruction, `/compact` handoff deferral, and conversation rewind.
- Exact project-root lookup for legacy null-worktree threads at the launch-slice
  seam: currently uncertainty conservatively refuses restore.
- Sparse checkout/submodule checkpoint parity: capture/restore explicitly refuse
  these checkouts until their private-index policy is ported. Live
  macOS/Windows/provider validation remains unverified.

Tests mirror `ContextHandoffBudget.test.ts`, `ContextHandoffDelivery.ts`,
`ThreadForkService`, and `CheckpointRestoreSafety` cases from the pinned T3
reference, covering portions of R3 C25/C26 rather than claiming every scenario.
