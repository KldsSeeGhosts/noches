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
| Thread relationships | [ThreadRelationshipsControl](https://github.com/pingdotgg/t3code/blob/fbe5df2d4b630d13adc8fe2d38cab354e6d66d67/apps/web/src/components/chat/ThreadRelationshipsControl.tsx) | **Added:** Details lineage, parent/fork navigation, bounded scroll regions, fork/checkpoint actions, merge-back action and sidebar fork shortcut. Existing Agents panel continues to own delegated-task controls. |
| Disconnect agent session | `ThreadRelationshipsControl.stopSession`, client-runtime `stopThreadSession`, `Orchestrator.dispatchProviderSessionDetach`, `ProviderSessionManager` | **Added:** passive attachment revisions, owner-routed user RPC, atomic detach plan and durable exact-run teardown. Conversation/native history and app-owned child threads are preserved. Reattachment, changed attempts/generations and active or idle replacement runtimes fence delayed effects. |
| Inherited transcript | `threadHistoryPaging`, client-runtime conversation projection | **Added:** frozen text preview in the child transcript with unique source-qualified IDs and explicit boundary. No document duplication or historical live controls. Full inherited tool/media projection is still a gap. |
| Delivery visibility | V2 context transfers/handoffs and provider acceptance | **Added:** strategy, provider IDs, run coverage, omitted-item counts and Pending/Ready/Prepared/Delivered/Failed/Superseded statuses. Both source and target can see the redacted target acceptance receipt. Consumption alone is never displayed as delivery. |
| Queue, steering, questions | [threadWorkflows](https://github.com/pingdotgg/t3code/blob/fbe5df2d4b630d13adc8fe2d38cab354e6d66d67/packages/client-runtime/src/state/threadWorkflows.ts), [QueuedRunsControl](https://github.com/pingdotgg/t3code/blob/fbe5df2d4b630d13adc8fe2d38cab354e6d66d67/apps/web/src/components/chat/QueuedRunsControl.tsx), `ProviderTurnControlService`, `RuntimeRequestService` | **Added:** one native tray for typed intents and SQL-only agent/automation work, with composer text edit, cancellation, mixed reordering, capability-fenced active steering and same-selection interrupt/restart promotion. Automatic completions/notifications stay out of the user tray. **Fixed:** coherent queued run/attempt/root rebinding, exact admitted-run transfer preparation, adapter-confirmed canonical steering and durable per-input uncertainty/recovery. Exact completed-turn follow-up cannot override owner cancellation or target a replacement process. Provider/model promotion transitions and queued merge-back remain gaps. |
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
  or target. SQL-only text edits preserve attachments, context and provenance;
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
  native steering RPC. Claude confirms only after its stdin writer successfully
  writes and flushes the user line, not when the writer mailbox accepts it.
  This Claude boundary proves adapter transport delivery, not CLI prompt echo
  or completed provider work.
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

## Remaining gaps

Tracked in issue #49 (provider/model transitions during queue promotion,
queued merge-back consumption, native Claude/Pi/ACP fork and receipts, full
inherited tool/media history, whole-thread Stop, context-window/compact policy,
and live-provider and non-Mac verification). Older builds stored false
acceptance at session initialization; those rows cannot retrospectively prove
provider submission, so no speculative migration or historical replay is done.
