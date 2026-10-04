# Wave 3 threads service and passive UI reads

`orchestration::thread_service::ThreadService` is the stable shared boundary:
`list`, `read`, `send`, `wait`, `interrupt`, `configuration`, `configure`, and
`create`, each taking authenticated `CallerScope` plus the corresponding
generated tool input. `KernelThreadService` implements it on the existing
transactional kernel, delegation catalog, runner, receipts and mailbox.

Launch, schedule, transfer and queue slices should call this service rather
than legacy chat RPCs. A thread send creates ordinary conversation activity,
not a new delegated task. A subsequent run on an app-owned child's backing
thread cannot reopen or replace the original task/result. The pinned T3
instructions and tool descriptions require `delegate_task` for each review
round; upstream does not implement a prompt-content detector in thread send.
Provider-native children refuse messages before lifecycle or input writes.

## Stable designer contract

Types live in `zeron_proto::orchestration_threads`, with typed
`RpcClient::thread_summaries` and `RpcClient::thread_timeline` methods. These
are owner SQL reads, not GPUI code. A host without orchestration state returns
an unavailable error; it never fabricates a local transcript for another owner.

`GetThreadSummaries`:

```json
{"projectId":"project"}
```

Returns `ThreadSummaries`:

```json
{"threads":[{"threadId":"parent","title":"Parent","createdBy":"user","creationSource":"web","status":"running","latestRunId":"run:parent:1","providerInstanceId":"codex","model":"gpt-6.1-sol","runtimeMode":"full-access","interactionMode":"default","linkedPullRequest":null,"settled":false,"settledAt":null,"parentThreadId":null,"relationshipToParent":null,"itemCount":2,"createdAt":"2026-10-03T00:00:00.000Z","updatedAt":"2026-10-03T00:01:00.000Z"}]}
```

Rows are `OrchestratorMcpThreadListItem`, ordered by updated time descending,
then ID descending. Deleted threads are omitted. Subagent rows are included
so the designer can apply the established hidden-child sidebar policy.
Activity status takes precedence over a newer queued/cancelled run.

`GetThreadTimeline` takes `ThreadTimelineRequest` (= `T3ThreadReadInput`):

```json
{"threadId":"parent","view":"activity","afterPosition":0,"limit":50,"runLimit":10,"maxCharsPerItem":20000}
```

Returns `ThreadTimeline {version, page: T3ThreadReadResult}`:

```json
{"version":42,"page":{"thread":{"threadId":"parent","projectId":"project","title":"Parent","createdBy":"user","creationSource":"web","status":"running","latestRunId":"run:parent:1","activeRunId":"run:parent:1","providerInstanceId":"codex","model":"gpt-6.1-sol","runtimeMode":"full-access","interactionMode":"default","linkedPullRequest":null,"titleRegeneration":null,"branch":"main","worktreePath":"/checkout","parentThreadId":null,"relationshipToParent":null,"runCount":1,"itemCount":2,"pendingRequestCount":0,"archived":false,"settled":false,"settledAt":null,"createdAt":"2026-10-03T00:00:00.000Z","updatedAt":"2026-10-03T00:01:00.000Z"},"recentRuns":[],"items":[{"position":1,"visibility":"local","sourceThreadId":"parent","itemId":"assistant-item","runId":"run:parent:1","messageId":"assistant-message","createdBy":"agent","creationSource":"provider","type":"assistant_message","status":"running","title":null,"text":"Working","textTruncated":false,"nextTextOffset":null,"updatedAt":"2026-10-03T00:01:00.000Z"}],"nextPosition":1,"hasMore":false}}
```

Both APIs are passive: opening a pane never acknowledges completion delivery,
starts a provider, or interrupts work. MCP `read` separately acknowledges only
a complete direct app-owned child's terminal result at text offset zero.
Thread wait is observational and cannot acknowledge or cancel.

Positions count all visible timeline items, including inherited/synthetic
items. `messages` selects user/assistant messages and proposed plans;
`activity` selects every visible item. Item-ID continuation ignores view and
afterPosition, with UTF-16 offsets. UI text replaces a lone surrogate for
display; `ThreadReadPage::wire_value` and the MCP HTTP encoder preserve its
exact `\ud83d`/`\ude00` wire escape. Private UTF-16 markers never enter SQL or RPC.

## Integration seams

No methods were added to `OrchestratorService`. Toolkit integration is a
separate optional service and one domain dispatch call. No generated schemas
were edited and no extra MCP parameters were added. Domain commands use
`Operation::Thread`; selection changes use `thread.model-selection.set` or
`provider.switch`, chosen by instance identity after live catalog validation.
Provider-switch portable/native handoff remains the transfer slice's seam:
the base selection command currently saves the next-turn choice only.

The runner adds ordinary steering/restart execution and timeline projection.
Claude/Pi retain T3's refusal of interrupt-restart; other adapter families can
use the engine's actual interrupt/start path. Queued follow-ups no longer
detach the active observer. Superseded attempt events cannot overwrite a new
attempt. A process-uncertain restart/steer is disposed on host recovery rather
than blindly repeated.

The existing title persistence uses Rust strings and cannot store a lone
surrogate produced by JS's 77-unit title truncation; thread *read text slicing*
is wire-exact at those boundaries. Truly unpaired-surrogate input/persistence
is a remaining kernel/codec parity limitation, not claimed as supported here.
