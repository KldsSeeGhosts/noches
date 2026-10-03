# Orchestration V2 kernel (P1a/P1b)

This opt-in engine module is based on T3
`v0.0.46-nightly.20261003.2632` / `f391794a35c604d57e166a3ab48d56fc6e4e469a`.
It consumes the generated contracts already on `orch/v2`. It does not register
MCP tools, start providers, change legacy session behavior, or modify chat2.

## Authority and transactions

`Kernel::open(Arc<DocsStore>, host_id)` extends the existing **profile**
`docs.sqlite3` connection. `DocsStore::with_connection` uses the existing
blocking-runtime/mutex discipline. The engine's `InstanceLock` remains the
process ownership boundary. A different host cannot open this kernel for writes.
The additional schema uses an independently namespaced, ordered,
append-only `orchestration_schema_migrations` ledger, with each migration in an
immediate transaction, following `sync::store` conventions. Existing sync
migration versions and tables are unchanged.

An accepted command commits a receipt reservation, V2 event envelopes,
queryable per-entity projections and frontier, cancellations/effects, optional
adoption record, publication batch, and final receipt in one immediate
transaction. Rejected planner commands persist a rejected receipt without
events or effects. Both statuses replay by command identity, **without payload
comparison** or a global `clientRequestId` namespace. `mcp_command_id` uses
T3's session/action/request scope and JS `encodeURIComponent` escaping.
Tool authorization/catalog preconditions still run before receipted dispatch
in future MCP callers; replay is not an authorization bypass.

`mark_intent_processed` refuses an unreceipted V2 intent. Owner-routed V2
commands must dispatch first and mark the old transport intent processed
second. A crash between these calls replays the receipt. The unrelated legacy
executor is intentionally not changed before runner adoption.

The source is `orchestration_events` (application event version **2**), not a
legacy V2 event table or native journal. Envelopes retain command, stream
version, sequence, typed event payload, and provider provenance when present.
Thread/run/attempt/node projections are separate SQL tables, with immutable
ownership columns and run/attempt ordinal uniqueness. Auxiliary kernel records
(provider threads/sessions/turns, requests, messages) remain separate indexed
rows, not one large thread/task JSON authority.
The same reducer runs on commit and rebuild; rebuilding never re-enqueues an
effect, a publication, a receipt, or a provider call.

`append_provider_events` is a separate unreceipted EventSink path: stale
asynchronous ownership commits nothing, while exact event-batch replay is
deduplicated. Its envelopes have a nullable command identity, as in T3.
Internal normalized provider commands can separately use receipted dispatch.

`ThreadLocks` acquires sorted, deduplicated keys and uses weak registry entries.
SQL receipt/ownership checks also protect independently opened kernel handles.
No SQL transaction or connection mutex crosses an async lock wait, executor,
publication callback, process, Git, or network operation.

## Implemented command boundary

- Wire `thread.create`, native-thread import without execution,
  `thread.unarchive`, runtime/interaction mode setters, and model selection.
- `CreateExecution`: trusted primitive for later message/queue planners;
  atomically creates the provider binding, run, first attempt and root node.
  Guards thread state, ordinal, initial statuses, graph and instance ownership,
  and blocking-run exclusivity. A starting run enqueues a start effect only.
- Guarded provider execution batches: run/attempt/node transitions, with
  run/active-attempt/provider-thread/session/turn/last-run-ordinal ownership.
  Superseded/terminal owners and backward transitions are refused durably.
- `ReplaceAttempt`: supersedes the old attempt/root, changes run ownership,
  cancels old process-bound effects, and enqueues the new start atomically.
- Explicit legacy adoption: records a chat-to-thread mapping and native
  provenance, with no invented delegated-task lineage and no provider work.
- Host-only runtime reconciliation.

Modes are opaque contract values here; runtime escalation and adapter policy
remain the sibling runtime-policy slice. Archive/delete, message delivery,
queue controls, mailbox/task/cohort, context, and policy-dependent thread
commands are **refused**, not implemented with partial semantics.
These primitives are internal owner APIs, not new MCP wire tool schemas.

## Effects and external acceptance

Effects have stable IDs, enqueue ordinals, per-thread provider/title lanes,
leases, claim generations, attempt counts, retry times, cancellation and
diagnostics. A pending retry blocks every later effect in its lane; title
work and other threads remain independent. Defaults mirror T3: four worker
slots, 30-second leases, five attempts, 100ms exponential backoff capped at 30s.
`EffectWorker::step` performs a single bounded claim; the later daemon owns
scheduling. The executor returns on **acceptance**, not turn completion, so
four start workers do not cap live provider sessions.

`EffectExecutor` is the only external execution seam. A fake implements it in
tests; no sessions/harness implementation exists here. Leases are fenced by
worker ID **and claim generation**. Dispatch begins durably before entering
the executor. Cancellation checks durable state before execution and races a
process-local cancellation signal during execution.

Process-bound start/interrupt/steer/restart/respond work is not replayed after
process loss. Pending or claimed-but-not-started work is retired; started work
is held in `uncertain`, an internal diagnostic/outbox state, not a new public
run/task status. An expired started process effect cannot be stolen.
`resolve_uncertain` releases its lane only after later provider-specific
reconciliation proves acceptance or disposal. Replay-safe effects are requeued
with the **same identity**. External provider consumption cannot be made
exactly-once with SQLite; the kernel never claims that guarantee.

## Publication and replica barrier

Acceptance enqueues immutable projection snapshots and registry patches in
the source transaction. Each batch contains schema version, host identity,
host epoch, source frontier, stable batch ID, and every participating
document/version. Deletions, when later implemented, are represented by the
thread's tombstone fields. Effects, sessions, live runtime-request answers,
bearer credentials and process resources are not publication payloads.

`ProjectionPublisher` is the host-only Loro/chat2 hook. It must apply stable
IDs idempotently, enforce owner/epoch/version, carry the batch's complete
barrier, and treat registry entries as patches. Only after its callback
returns does `PublicationWorker` persist a per-document ack. A crash after
publication but before ack repeats the same document/version, not another
transcript entry. Batches are drained FIFO; partial acks survive restart.

`ReplicaBarrier` exposes only observation/read readiness, never execution.
It blocks actionable reads until all batch participants arrive, rejects
foreign owners and older epochs, and resets observed versions for a new
host epoch. A chat2 cursor alone is not an execution/batch frontier.
This foundation currently emits thread + registry documents; parent/child
multi-thread planners will extend the same batch participant set in P3.

## Recovery and integration obligations

Call `Kernel::recover` under the owning process lock, **before** starting V2
effect/publication workers. It verifies schema/frontier and decodability,
rebuilds damaged projections, advances the host epoch with effect retirement,
holds queued work, cancels stale runs/attempts/nodes/turns, stops sessions,
expires non-message requests, and preserves message-answerable requests.
All runtime changes become receipted events/publications and survive another
restart. Recovery never executes a harness or drains queued work.

`is_v2_managed` is the durable recovery-ownership gate for adopted legacy
chats. The later `sessions.rs` runner **must** use it to exclude V2-owned chats
from native journal auto-resume before enabling adoption in production.
Kernel assembly/adoption is deliberately not enabled in `EngineCore` yet.
Subagent/background rosters, terminal task results/mailbox reconciliation,
checkpoint-aware continuation eligibility, and restart continuation creation
are the later runner/task slices, not claimed as implemented recovery parity.

## Validation

The unit suite exercises replay/rebuild, accepted/rejected receipt replay
(including changed payload), individual write-boundary rollback, concurrent
commands/independent handles, stale provider ownership, superseded attempts,
adoption, receipt/intent crash ordering, leases/generation fencing, ambiguous
acceptance, safe identity-preserving replay, cancellation, FIFO retry lanes,
eight starts through four workers, publication/ack crashes, epoch/read
barriers, projection repair, queue holds, and deterministic keyed locking.
Structural sample rows come from the pinned contracts oracle; they are
explicitly not relabelled as executed T3 scenario traces.

Linux results and verbatim pre-existing strict-clippy failures are recorded in
[kernel-validation.md](kernel-validation.md). No UI files were changed;
macOS/Windows builds, headed visual QA, real harness execution and a live
Loro/chat2 publisher are unverified in this slice.

```sh
LINUX_TARGET=target-orch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-kernel test -p zeron-engine --locked
LINUX_TARGET=target-orch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-kernel clippy -p zeron-engine --locked --all-targets -- -D warnings
```

Reference paths: T3 `apps/server/src/orchestration-v2/{Orchestrator,EventSink,
EventStore,CommandReceiptStore,ProjectionStore,ProjectionMaintenance,
EffectOutbox,EffectWorker,ProviderRuntimeRecoveryService}.ts` and
`apps/server/src/persistence/{Layers/OrchestrationEventStore,
Layers/OrchestrationCommandReceipts,Migrations/OrchestrationV2/ApplicationEventSource,
Migrations/OrchestrationV2/Foundation,Migrations/OrchestrationV2/EffectCancellation}.ts`.
