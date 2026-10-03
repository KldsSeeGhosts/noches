# Production orchestration assembly and passive UI API

`EngineCore::assemble*` assembles the profile kernel, real `DelegationService`,
catalog-backed instance resolver, sessions runner, scoped MCP issuer, recovery,
effect workers, and projection publisher. Recovery completes under the profile
instance lock before the real service becomes callable. Workers and catalog
discovery are engine-owned and stop before sessions are retired.

## Default and authority

Orchestration is **ON by default**, matching T3. Set `ZERON_ORCHESTRATION=0`
(also `off` or `false`) to retain the unavailable service for legacy-only rigs.
Turning it on alone launches no new user task. Startup disposes uncertain
process starts rather than rerunning them; durable completion mail can resume
an eligible parent via a queued continuation.

Ordinary sessions are admitted to the kernel before a provider can call a tool.
Admission records a logical run **without** scheduling a duplicate provider
start. Warm, parked sessions advance their credential's host-owned run scope
before the next prompt enters the native mailbox. Active steering retains the
current logical run. Delegated runs instead use the kernel's canonical scope
and receive their own newly minted token before startup.

Tokens remain only in the private harness context and in-memory digest registry,
never in SQL, journal, chat2, registry rows, RPC state, or portable projections.
Registration/end/replacement/shutdown revoke them. Task ownership and run
liveness are checked by the real service even after target validation.

## One production catalog

Composer `ListModels`, `ListProviderInstances`, `orchestrator_capabilities`,
MCP target validation, real delegation target validation, and runner instance
resolution all use `HarnessRegistry::provider_instances`. Explicit instance,
driver consistency, availability, exact model membership, duplicate option
IDs, boolean/select types, and choice membership are validated. Same-selection
model/option inheritance is revalidated against the current inventory and
descriptors; removed custom models cannot bypass membership checks by omission.

The optional profile `provider-instances.json` is the explicit inventory.
Without it, compatibility instances are derived from the installed adapters.
On enrolled CPA hosts, refresh additionally reads the installed discovery
paths from `CPA_SYNC_CONFIG` or `~/.config/cpa-catalog/sync.json`: Codex
`models[].slug`, reasoning/service-tier descriptors, and Claude
`modelPicker.options[].model`. IDs are not stripped, prefixed, or translated:
`opencode-go/deepseek-v4.1-flash`, `gpt-6.1-sol`, and
`cpa/claude-opus-5-5[1m]` remain distinct exact selections. This is a read-only
metadata compatibility import; it does not publish/sync the CPA catalog or
create inference routes. Explicit inventory disables this automatic import.
Removed installed metadata disappears on the next refresh.

Startup account discovery is bounded and not dependent on opening Settings.
Unknown authentication is non-blocking while discovery runs; detected signed
out providers are constrained. Explicit configured auth overrides discovery;
configured `unknown` follows the underlying adapter's discovered state.
Claude's native `auth status --json` readiness bit covers API-key logins as
well as OAuth: an absent OAuth slot alone does not mean unauthenticated.

Multiple entries can resolve to one installed adapter, but **independent
per-instance executable/env/account lifecycles and a multi-instance composer
selector are not implemented here**. An inventory row is not a new inference
route or independent credential store. Adapter capability snapshots are
conservative; only actual StepBoundary adapters advertise active steering.

## Read APIs

These are additive RPC method names in `zeron_rpc::methods`.
They are passive: no result acknowledgement, provider start, or cancellation.

- `GetOrchestrationState {chatId}` returns the authoritative local projection,
  or an already replicated chat2 projection when this host does not own SQL
  state. Unadopted replica chats return `null`.
- `ListOrchestrationThreads {}` returns registry discovery summaries, including
  `id`, `projectId`, `title`, `lineage`, archive/delete stamps, host/epoch,
  publication version and batch identity.
- `ListProviderInstances {}` refreshes and returns the canonical inventory,
  models/options/readiness. Existing `ListModels {harness}` is unchanged.

`GetOrchestrationState` shape:

```json
{
  "threadId": "parent",
  "version": 42,
  "lineage": {
    "parentThreadId": null,
    "relationshipToParent": null,
    "rootThreadId": "parent"
  },
  "workState": "waiting_for_children",
  "latestResult": "Parent assistant result or null",
  "tasks": [{
    "taskId": "node:delegated-task:...",
    "childThreadId": "thread:delegated-task:...",
    "title": "Task title or null",
    "providerInstanceId": "claudeAgent",
    "model": "claude-haiku-4-5",
    "status": "completed",
    "workState": "result_available",
    "result": "PONG",
    "latestResult": "PONG",
    "latestTerminalRunId": "run:...",
    "completionDelivery": {"state": "acknowledged"}
  }]
}
```

`completionDelivery` is the full upstream object; the example shows only its
state. `workState` is `working`, `waiting_for_children`, or `result_available`.
`status` and immutable `result` describe the original delegated task;
`latestResult`/`latestTerminalRunId` can describe a later qualifying child run.
Monitor/rolled-back/unstarted later runs are excluded. Only app-owned tasks
appear in this list; provider-native observations remain separate.

`childThreadId` is a regular workspace chat discovered through existing
`WatchChats`; read its transcript using `WatchDocMessages`. Children are
user-read-only through public sessions dispatch/steer. Do not use a backing
child as a new review round: call `delegate_task` with a new request ID.
The UI should call `GetOrchestrationState` on selection or after ordinary
workspace/transcript updates; no new high-frequency watch protocol is required.

## Replication and barriers

Each `SessionDoc` has an additive root Loro map `orchestration`:
`hostId`, `hostEpoch`, `version`, `batchId`, `barrier[]`, and `projection`.
`projection.uiState` is the same passive RPC shape. `SessionDoc::orchestration()`
reads it without changing task disposition. Ordinary chat2 incremental updates
and checkpoint persistence carry the map; it is not a transcript message.

The SQL outbox publishes parent and child documents with stable entity IDs,
then patches `orchestration` discovery metadata on their ordinary registry
chat rows. Epoch/version fencing rejects foreign owners and stale replay.
Registry patches preserve unrelated fields/threads. Document snapshots and
registry patches are persisted before SQL acknowledgement.

The document barrier lists required `docId`/version pairs. Registry publication
version is available on its per-chat summary; thread publication version is
available on the chat document map. Replicas may display partial state, but
must not infer actionable completeness before all referenced versions arrive.
RPC is read-only here: execution authority remains on the authenticated host.
No live session/request/credential records are replicated.

## Validation boundary

`tests/orchestration_bootstrap.rs` uses real `EngineCore` assembly, scoped
streamable HTTP tools, ordinary sessions dispatch, runner workers, mailbox,
steer receipts and queued continuations, explicit status acknowledgement, and
chat2 publication. The mock has two instance identities over one adapter.
Real cross-driver evidence and exact live failures are in `live-e2e.md`.
UI visuals, Windows/iOS, remote MCP forwarding and P4–P8 tool services remain
outside this production P3 assembly slice.
