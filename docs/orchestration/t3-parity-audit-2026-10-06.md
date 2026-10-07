# T3 orchestration parity audit

How Noches' Rust/GPUI orchestration maps onto T3 Code's orchestration V2, and
the safety decisions behind that adaptation. This is an original port, not a
replacement of Noches with T3's TypeScript/Electron stack.

Reference: [pingdotgg/t3code at `365aa879`](https://github.com/pingdotgg/t3code/tree/365aa87982a4d81cc8e0c085e8d1a40ca7daecdc)
(`Orchestrator`, `ProviderTurnStartService`, `ProviderTurnControlService`,
`EffectWorker`, `ContextHandoffDelivery`, `ThreadForkService`). Recheck upstream
before extending this work; verification evidence for the original port lives
in PR #47, and remaining work is tracked in issue #49.

T3's refinement comes from separating the app conversation, logical run,
provider attempt, native provider conversation, child task, completion mail,
context transfer and file checkpoint. Correctness depends on their ownership
and acceptance boundaries, not on a visually similar Agents panel.

## Full-stack map

The source links below are pinned implementation references. Upstream
`docs/orchestration-v2/feature-lifecycles.md` also describes target behavior;
it is not alone evidence that a feature works in either application.

| Boundary | T3 implementation | Noches implementation / finding |
| --- | --- | --- |
| Persistent authority | `EventStore`, `ProjectionStore`, `Orchestrator`, `EffectOutbox` | `orchestration/{store,command,projection,effects,outbox}`: transactional receipts, durable effects, owner/epoch fencing; existing. |
| Ordinary thread integration | `ThreadMessageIntake`, `RunExecutionService`, `ProviderEventIngestor` | `thread_service`, `threads/planner`, `sessions`, `runner`: legacy adoption, one logical run, no second provider start; existing and regression-tested. |
| Exact provider/model selection | `ProviderAdapterRegistry`, `ProviderSelectionTransition` | `provider_instances` catalog shared by composer, MCP and runner; custom IDs/options remain exact. Existing. |
| Return to a harness | [ProviderTurnStartService](https://github.com/pingdotgg/t3code/blob/fbe5df2d4b630d13adc8fe2d38cab354e6d66d67/apps/server/src/orchestration-v2/ProviderTurnStartService.ts), `ProviderSwitchService` | **Fixed:** independent native handles per instance/generation; A→B→A resumes A, bridges B's delta, or reconstructs fully when unsafe. Prior code overwrote one app-wide handle and cleared resume even for a delta. |
| Prompt acceptance and retry | `ProviderTurnStartService` missed-attempt history, `ContextHandoffDelivery` acceptance callback | **Fixed:** session readiness and local steering are not accepted input. Codex RPC/OpenCode POST acknowledgements or actual native response bind the exact root attempt; failed/interrupted untold inputs become bounded, restart-durable retry context. Settled deliveries and deliberate budget omissions are not repeatedly injected. |
| Portable context | [ContextHandoffService](https://github.com/pingdotgg/t3code/blob/fbe5df2d4b630d13adc8fe2d38cab354e6d66d67/apps/server/src/orchestration-v2/ContextHandoffService.ts), `ContextHandoffBudget`, `ContextHandoffDelivery` | `transfer/{context,delivery}`: bounded whole-item selection, occupancy/input/attachment budget, coverage and uncertain-delivery receipts. **Fixed:** imported runless history, checkout/native identity fences, restart metadata recovery. |
| Delegation and handoff | `DelegatedTaskService`, `NotificationMailbox`, `RunFinalizationService` | `task`, `mailbox`, `runner`, scoped MCP: task ID distinct from backing thread, nested children, cancellation, steering/queued completion and explicit acknowledgement. Existing. |
| Native vs app-owned agents | `SubagentProjection`, provider event ingest, client-runtime subagent selectors | `delegation`, `subagents`, `agents`, composer/sidebar: observational native children and app-owned tasks retain separate authority and share presentation. Existing. |
| Thread launch/workspace binding | `ThreadLaunchService`, `ThreadManagementService` | `launch`, `worktree`, `thread_service`: explicit root/existing/new-worktree preparation, durable setup admission; existing with adapter gaps below. |
| Lazy fork and merge-back | [ThreadForkService](https://github.com/pingdotgg/t3code/blob/fbe5df2d4b630d13adc8fe2d38cab354e6d66d67/apps/server/src/orchestration-v2/ThreadForkService.ts) | **Added:** owner-routed user RPCs, pinned source, stable request identity, immediate inherited chat config, no eager provider start. Direct-parent merge prepares context; it is not a Git merge. |
| Thread relationships | [ThreadRelationshipsControl](https://github.com/pingdotgg/t3code/blob/fbe5df2d4b630d13adc8fe2d38cab354e6d66d67/apps/web/src/components/chat/ThreadRelationshipsControl.tsx) | **Added:** Details lineage, parent/fork navigation, bounded scroll regions, fork/checkpoint actions, merge-back action and sidebar fork shortcut. Existing Agents panel continues to own delegated-task controls. Lineage rows in Details and the Agents panel are keyboard-reachable (Tab, up/down, Enter/Space), skip unavailable threads and show T3's hover arrow. Status icon/label per row and the hover card remain gaps. |
| Disconnect agent session | `ThreadRelationshipsControl.stopSession`, client-runtime `stopThreadSession`, `Orchestrator.dispatchProviderSessionDetach`, `ProviderSessionManager` | **Added:** passive attachment revisions, owner-routed user RPC, atomic detach plan and durable exact-run teardown. Conversation/native history and app-owned child threads are preserved. Reattachment, changed attempts/generations and active or idle replacement runtimes fence delayed effects. |
| Whole-thread and child Stop | `Orchestrator.dispatchRunInterrupt` (per run, `holdQueue`), client-runtime `interruptThreadTurn`, `DelegatedTaskService` cancel; T3 pins no bulk Stop at this commit | **Added (Noches-specific):** `StopThreadWork` (owner-routed, capability `thread-work-stop-v1`) freezes the thread's interruptible run/settled background work and every non-terminal app-owned child task, recursively, leaves first, under one stable request identity (`orchestration/stop_all.rs`). Each step is an ordinary kernel command (`TaskOperation::Cancel` / `ThreadOperation::Interrupt`) with derived identity, so it inherits exact run/attempt/root/process fences and control admission; a target that settled or was replaced is skipped, never retargeted, and a replay repeats only the first frozen set. Ownership is the parent's own task record, so forks, native children and other threads' tasks are never reached. Completed replies, persistent monitors and unrelated threads are untouched. The relationship-panel Stop (`CancelDelegatedTask`) was verified end to end against a live child runtime and fixed to be owner-routed. UI: Agents panel Active header **Stop all**; composer Stop/Escape are unchanged. |
| Forced session reconstruction | `ThreadRelationshipsControl` Disconnect only; T3 has no reset | **Added (Noches-specific):** Details **Reset agent session** (`ResetThreadSession`, capability `provider-session-reset-v1`) closes the provider conversations a started run used, tears down attached sessions as Disconnect does, and rebinds queued runs to one fresh generation. The next turn starts a new provider-thread generation with full bounded portable history and never rides the engine-remembered native session. Refused while a run is active, for a stale observed run or attachment set, and with nothing to reset; stable identity makes a retry repeat the same request. History, runs and the app conversation are kept (distinct from Disconnect, which keeps native history). |
| Inherited transcript | `threadHistoryPaging`, client-runtime conversation projection | **Added:** frozen text preview in the child transcript with unique source-qualified IDs and explicit boundary. No document duplication or historical live controls. Full inherited tool/media projection is still a gap. |
| Delivery visibility | V2 context transfers/handoffs and provider acceptance | **Added:** strategy, provider IDs, run coverage, omitted-item counts and Pending/Ready/Prepared/Delivered/Failed/Superseded statuses. Both source and target can see the redacted target acceptance receipt. Consumption alone is never displayed as delivery. |
| Queue, steering, questions | [threadWorkflows](https://github.com/pingdotgg/t3code/blob/fbe5df2d4b630d13adc8fe2d38cab354e6d66d67/packages/client-runtime/src/state/threadWorkflows.ts), [QueuedRunsControl](https://github.com/pingdotgg/t3code/blob/fbe5df2d4b630d13adc8fe2d38cab354e6d66d67/apps/web/src/components/chat/QueuedRunsControl.tsx), `ProviderTurnControlService`, `RuntimeRequestService` | **Added:** one native tray for typed intents and SQL-only agent/automation work, with composer text edit, cancellation, mixed reordering, capability-fenced active steering and same-selection interrupt/restart promotion. Automatic completions/notifications stay out of the user tray. **Fixed:** coherent queued run/attempt/root rebinding, exact admitted-run transfer preparation, adapter-confirmed canonical steering and durable per-input uncertainty/recovery. Exact completed-turn follow-up cannot override owner cancellation or target a replacement process. Provider/model promotion transitions (issue #49 §1) and queued merge-back consumption are implemented on branch i49/selection; same-instance handoff restarts, the composer-to-thread selection sync and the explicit fresh-native-start contract (the sessions engine never auto-resumes a generation the planner rebuilt from portable context) are implemented on branch i49/fresh. |
| Stop after foreground completion | Current `Orchestrator` background settlement and shared pending-work selector | **Added:** completed-root background Stop through user composer/Escape→durable command→canonical admission→exact runtime teardown→bounded settlement. Completed reply/attempt/timestamps survive; later work, app-owned tasks and persistent monitors are not terminated. Background-only queue-watch changes now reach the composer. |
| Scheduler | server scheduler and launch/intake dispatch | `scheduler`, Settings Automations: persistent claims, recurring/manual/webhook work, bound/unbound dispatch, restart/deduplication. Existing. |
| PR association and settlement | `PullRequestWatchReactor`, `PullRequestSyncReactor`, `ThreadSettlementService` | `pull_requests`, `git_actions`, Details/lifecycle: authenticated linking, stable watch receipts, wake/settlement fences. Some environment/project settlement policy UI remains incomplete. |
| Checkpoints | `CheckpointService`, `CheckpointRollbackService` | files-only checkpoint preview/checksum/HEAD/ownership/backup safety. No conversation rewind; sparse/submodule support is explicitly refused. |
| Remote devices/security | environment routing, scoped orchestrator MCP | owner-routed RPC, private MCP credentials, replicated Loro projections and barriers. Transport intentionally differs; replicas never execute provider effects. New transfer writes use user authority, not fabricated agent credentials. |
| Recovery/publication | effect reconciliation, projection recovery, thread streams | durable kernel recovery, publication outbox and ordinary chat/document discovery. **Fixed:** publishing a new fork no longer drops its inherited composer config. |

## End-to-end ownership

```text
User input / queued input / scheduler / delegated completion
  → owning-host intake and policy checks
  → one canonical logical run, active attempt and root node
  → exact catalog provider/model/options
  → that provider's native conversation OR bounded context reconstruction
  → harness execution and acceptance
  → fenced provider events, durable transfer receipts and completion mail
  → persistent projection/publication
  → native thread, composer, Agents and Details surfaces
```

An app thread is never itself the native provider conversation. The important
continuity regression is:

```text
A / native-a → B / native-b + full A context → A / native-a + missed B delta
```

The last step is allowed only with accepted native coverage, the same checkout
and a selection change the driver applies on the next turn
(`task::selection_transition`, a port of T3 `ProviderSelectionTransition`:
Claude, Codex, Cursor and OpenCode carry model/effort per turn; ACP agents only
options; Antigravity and unknown drivers hand off). Otherwise A gets a fresh generation and full portable
context. A queued message must retain its original logical run identity while
all execution bindings move together; preparing another queued run's history
is not an acceptable substitute.

## Interaction and safety decisions

- Honor Noches' `docs/design/control-plane.md`: native flush GPUI panes, neutral
  theme surfaces, monospace metadata and meaningful state color. T3 interaction
  semantics do not authorize reintroducing mascots or replacing this design.
- Fork a finished source without running an agent. Choose another harness on
  the idle child if desired; native compatibility is decided at first send.
- Pin the source before dispatch. Later parent runs cannot leak into a fork.
  Retry the exact request after an uncertain response; do not mint another
  child or retarget a previously accepted command.
- Merge-back retains the recorded fork base, supersedes the same fork's older
  pending transfer, and preserves queued/multiple-fork refusals. Acceptance
  never edits Git or starts/interrupts the parent.
- Inherited previews are prepared off the UI thread and bounded to the latest
  100 text messages/10,000 Unicode scalars each. Omission and shortening are
  explicit. Full source history remains in the parent. Private reasoning,
  approvals, tool calls and transport attachment fetches are not replayed.
- Queue snapshots are additive and capability/version gated. A bounded document
  accessor expands only `uiState.queueState`, not the full transcript/tool
  projection. Unrelated SQL sequence changes do not wake the queue renderer.
  Existing queue subscriptions, edit leases and split-pane routing are retained.
- Canonical rows are presentation-only, never synthetic Loro intents. A delayed
  document-backed projection cannot resurrect a consumed typed row. Automatic
  completion and notification rows are excluded, matching T3's user-queue selector.
- Canonical actions use the owning host's user authority and the existing kernel
  planner. Stable request reservations prevent a retry from changing its content
  or target. SQL-only edits preserve context and provenance and keep attachments unless the
  edit replaces them (host-validated uploads, fingerprint-fenced; dropped files
  are cleaned up once unreferenced);
  stale text or an already-started run is refused without overwriting it. An
  automatic drain retains both the edit and the composer's previous draft.
- Typed rows retain explicit interrupting **Send now** and their host edit leases.
  SQL-only rows advertise **Steer** only when the running provider attempt/turn
  supports active steering, or **Send now** for an explicitly interrupting,
  capability-gated restart. A passive UI hint and the actual mutation share the
  same fences; stale non-interrupting clicks cannot become restarts. Canonical
  restart supersedes the original attempt/root inside the same logical run,
  retains the queued message and metadata, and pins the old physical process
  plus its committed replacement attempt. Other rows remain queued.
  The new root has no provider turn until real acceptance. Native resume requires
  the exact accepted immediate predecessor, the same instance and checkout,
  and a turn-scoped selection change. Host-owned document images are transported, not merely displayed.
  Document draining respects a canonical Starting run even while its old
  physical runtime is retiring; another queued input cannot be taken and
  presented as sent during that gap.
  The typed-row command path remains distinct; full context-record rendering and
  provider/model-selection transitions are not claimed.
- Disconnect is distinct from archive, conversation reset and ordinary turn
  interruption. It uses the exact observed attachment set and attachment-local
  revisions, not changing token usage or a guessed provider generation. Stable
  request reservations preserve the target across response loss and projection
  rebuild. The worker additionally checks canonical attempt/provider identity
  and a private runtime run tag, independent of agent credential expiry, before
  signaling the exact handle under its map lock. Neutral feedback says
  **requested**, not that an asynchronous teardown has already finished.
- A native session ID proves attachment, not input acceptance. `SessionStarted`,
  local `Steered`, context telemetry and thread-only references cannot settle
  handoff receipts. Host-only `InputAccepted` is emitted after successful Codex
  turn-start/steer RPCs or the matching OpenCode prompt response, never on enqueue.
  OpenCode responses carry unique submission identities; a late ACK/failure
  from a finished input cannot bind or fail its successor.
- Inline context receipts retain the actual bounded selected/omitted IDs and
  prepared run/attempt/root/provider-thread fence in private SQLite state.
  Acceptance settles only that exact preparation/native identity inside the
  provider-event transaction. A repeated refused preparation preserves the
  original pending ACK fence. Session readiness cannot manufacture delivery.
- Retry context recovers earlier failed/interrupted untold root inputs on the
  same provider thread. Its stable saved snapshot survives process restart and
  projection rebuild; fork/switch context and accepted native coverage suppress
  duplicate items. Historical attachments/private reasoning/native tools are
  not re-executed. A successfully accepted failed turn is not replayed.
- Codex and OpenCode preserve the original mailbox message ID through native
  submission, queued boundary delivery and Codex's new-turn fallback. Their
  host-only `InputAcceptedFor` acknowledges that exact message only after its
  successful RPC/POST; local `Steered` cannot retire its recovery entry.
  Duplicate, unrelated and uncorrelated receipts cannot consume another input.
  A receipt from a replaced runtime is not published, and an older message's
  receipt cannot bind the next canonical root or settle its context. Ordinary
  provider failure redelivers only unacknowledged mailbox inputs with their
  original transcript identity. This legacy mailbox ledger is distinct from
  the canonical steering receipt contract below; equivalent native receipts
  for every legacy adapter are not claimed.
- Ordinary canonical steering and queue promotion now share an exact-target
  adapter: recorded provider session/thread/turn, message ownership, run/attempt/
  root, provider ordinal and the private physical process incarnation all
  participate. Admission captures the physical runtime before dispatch.
  A process replacement cannot receive or confirm the old effect even when
  it reuses the logical attempt. Queue promotion never starts a late follow-up.
- Strict canonical steering does not enter the legacy detached-redelivery
  mailbox. Successful adapter acknowledgement records a durable per-message
  receipt; explicit refusal fails the effect, while a dropped receipt channel
  or ten-second timeout leaves acceptance uncertain. Codex confirms after its
  native steering RPC. Claude's stdin writer answers the mailbox acknowledgement
  only after it successfully writes and flushes the user line (transport
  delivery, not the writer mailbox). The exact native receipt is separate: every
  Claude stdin line carries a host-chosen `uuid` and the CLI runs with
  `--replay-user-messages`, whose `isReplay` echo of that `uuid` (verified on CLI
  2.1.292) becomes `InputAcceptedFor`, so Claude retires pending input only by
  that echo. A duplicate or unknown echo accepts nothing; an echo that never
  arrives stays unaccepted for the host's recovery. Not proven live: echo timing
  for a steer queued past a turn boundary.
- Canonical receipts survive projection rebuild and process restart. An exact
  late `InputAcceptedFor` can record confirmation in the provider-event
  transaction and release only its own uncertainty barrier. Persisted proof
  wins over worker timeout/lease settlement. Startup retires unknown old
  process effects without resending input or permanently blocking their lane.
  An explicit next turn or owner cancellation can retire terminal-run steering
  uncertainty without pretending the input was rejected or accepted.
- Next-turn recovery selects only unconfirmed steering messages, not the
  accepted root's entire transcript. The saved bounded snapshot, settled
  coverage, omitted-item suppression and uncertain native-injection refusal
  remain intact. Recovery labels missing confirmation as uncertainty, not
  proof of rejection or completion. New-contract markers avoid speculatively
  replaying legacy effects with older acceptance semantics.
- A late follow-up is planned atomically against saved effect/current thread
  state under the owning kernel lock, with a stable command identity and the
  same original message. Attachments, context and automatic-mail/provenance
  metadata are preserved. Only normal target-turn completion can authorize it;
  owner-cancelled/settled effects, stopped runs, changed sessions/attempts and
  replaced physical processes cannot. An already-confirmed input is a
  receipted no-op, never another provider run.
- An engine question/approval callback may precede native-session metadata.
  Such a request is projected as not resumable, never as a live capability with
  a null/fabricated session ID. A later real attachment upgrades only pending
  requests owned by that exact run, preserves the waiting state, and cannot
  reopen an expired request. Without an attachment, interruption still expires
  the request and terminalizes the canonical run; the next send stays usable.
  Question/approval callbacks and session readiness do not establish root input
  acceptance. The former null capability killed the observer and stranded the
  run in Starting despite an aborted transcript entry.
- Imported ordinary user items keep their original ID and position while
  portable history attributes them to the canonical message's logical run.
  Untold-input retry filtering therefore cannot drop an adopted user message
  just because its transcript item was imported before admission. This does
  not manufacture provider acceptance: native turn binding is written only
  at a real acceptance boundary, and accepted interrupted inputs are not replayed.
- First-response acceptance and assistant/error materialization share a planned
  item-ordinal allocator. Newly emitted items in the same transaction contribute
  to the next ordinal, preserving user-before-assistant pagination and preventing
  premature acknowledgement of a partial child result.
- Queue repair acknowledges a saved command-generation watermark rather than
  requiring the entire live document to stay clean. Concurrent later transcript
  changes remain independently dirty; a real failed snapshot write still leaves
  the repair outbox unacknowledged. This fixes an RPC failure despite the queue
  patch already being durably saved, without serializing away concurrent tests.
- Ordinary Stop, managed child interruption and interrupt/restart now freeze
  their process target in a host-only outbox admission record. Restart pins both
  the interrupted attempt/root and its already-committed replacement attempt.
  Session/thread/turn and provider-ordinal checks precede exact runtime-map
  signalling. Missing legacy admission does not acquire authority over a new
  process. Owner-cancelled controls cannot run or synthesize terminal repair.
- Control waits release the kernel lane needed by provider ingestion. The
  executor waits for the signalled process to retire, gives ordinary terminal
  ingestion a bounded grace period, then performs any synthetic settlement in
  a transaction that rechecks the admission and physical-process fence.
  A late observer cannot terminalize the superseding restart attempt. These
  checks are also used by completed-root background Stop. Cleanup holds the
  runtime map empty across its synchronous transaction, excluding even an
  unbound ordinary-session replacement.
- Background waiting and Stop share one selector for provider rosters, native
  subagents and active command/dynamic-tool/native-subagent items. A queued run
  cannot steal the target; a new foreground run does. Settlement clears only
  work through the stopped ordinal after exact process retirement, including
  older dead-provider rosters. Independent app-owned tasks and persistent
  monitors retain their own authority. A separate background-stop result
  fences delayed native child observations without rewriting a completed reply.
- Ordinary composer Stop now uses host authority and its durable session-command
  identity rather than an unqualified legacy runtime lookup. It does not need
  an active agent credential. Early questions and actual output before session
  metadata retain exact-process cancellation without fabricated native session,
  turn or acceptance data. Queue pause survives terminality and edit completion;
  only an explicit successful send thaws it.
- `backgroundRunId` is an additive passive queue hint, not an active foreground
  run or a new replicated execution authority. The empty composer and opt-in
  Escape path offer Stop; a new draft still sends a new turn. The queue watch
  compares background changes while ignoring unrelated sequence-only churn.
- Delegated completion steering uses the live canonical run/attempt/root/process
  fence and adapter acknowledgement, not a chat-only mailbox lookup. The
  delegated mailbox retains its prior retry/recovery policy; complete durable
  per-notification acceptance/uncertainty retirement is not claimed. Its offline
  fixture now issues a real scoped MCP credential before dispatch rather than
  relying on an unbound ordinary runtime.

## Simultaneous provider processes

Noches keeps one physical provider runtime per **app thread**, keyed by thread
id in the sessions map; an app-owned delegated child is its own app thread, so
a parent, its children and their descendants already run concurrently as
separate processes (the delegation tests start several). Whole-thread Stop
therefore needs no multi-runtime-per-thread model: each target is stopped
through its own thread's exact runtime, and native subagents live inside their
provider process and are covered by that thread's own Stop. Changing the model
would only matter for several live provider processes *within one thread*
(for example a user-selected second harness alongside the first); nothing in
the delegation, queue or Stop paths needs that, so it is deliberately not
attempted here.

## Remaining gaps

Still open after #49: negotiated-ACP fork and exact input receipts (ACP
`session/fork` is unstable and head-only; those adapters keep boundary
retirement; native Pi now has both, see `pi-native-rpc.md`), loaded-process history injection, driver-authorized cross-account
and cross-checkout native continuation, conversation rollback, provider
clone/setup progress edges, project/environment PR-settlement settings,
sparse/submodule checkpoints, catalog context windows beyond Claude, and
live-provider, remote multi-device and non-Mac verification. Older builds stored false
acceptance at session initialization; those rows cannot retrospectively prove
provider submission, so no speculative migration or historical replay is done.

Legacy-row policy: steering effects admitted before the per-input receipt
contract have no `orchestration_steering_inputs` row. They are never joined into
unconfirmed-input recovery (not replayed as historical context), never gain an
`orchestration_steering_acceptances` row without an exact provider receipt, and
are never reinterpreted as rejected; acceptance of a run's root is likewise
derived only from a bound provider-turn record, never from session start.
Regression: `legacy_steering_rows_are_never_replayed_or_speculatively_accepted`.
