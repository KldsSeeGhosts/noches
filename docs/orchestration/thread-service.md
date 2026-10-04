# Wave 3 threads service and passive UI reads

`orchestration::thread_service::ThreadService` is the stable shared boundary:
`list`, `read`, `send`, `wait`, `interrupt`, `configuration`, `configure`, and
`create`, each taking authenticated `CallerScope` plus the corresponding
generated tool input. `KernelThreadService` implements it on the existing
transactional kernel, delegation catalog, runner, receipts and mailbox.

Host integrations use `send_to_thread(ThreadSendRequest)` rather than inventing
an authenticated MCP caller. This request carries trusted project scope,
stable command/message IDs, optional schedule/sender IDs, attachments,
selection, mode and actor/source. MCP `send` applies caller permission checks
before entering the same intake. Schedules use `auto`, read the current
thread binding, and accept the ordinary command queue without waiting for a
provider turn to complete.

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
The ACP HTTP-to-stdio bridge and terminal fallback forward response payloads
as opaque JSON, so surrogate-split reads survive both JSON and SSE framing.
Only insignificant JSON whitespace is removed for single-line stdio frames.

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

## Conformance coverage and remaining boundaries

`threads/tests.rs` mirrors `ThreadManagementService.test.ts`,
`OrchestratorMcpService.test.ts`, `OrchestratorMcpService.activity.test.ts`,
and the shared timeline visibility rules from T3 `20261003.2632`.
Covered scenario subsets: C03 (scopes/context), C07 (top-level/native/app-owned),
C08 (receipt retries/partial batch), C10/C13 (immutable result/acknowledgement),
C14 (queue hold/interrupt), C16 (superseded attempts/late steering),
C17 (views/inherited positions/UTF-16), C18 (send/wait/interrupt),
and C22 (shared checkout batch).
These are Rust mirrors of upstream assertions, not a claim that the shared
two-application conformance runner or live provider matrix has been executed.

The generic persisted JSON codec canonicalizes object-key order. Controlled
activity text objects keep T3's field order and omit absent optional fields,
but arbitrary nested tool JSON may have canonicalized keys. Full portable/native
provider handoff remains an integration boundary with the transfer slice.
macOS/live providers/visual QA are unverified.

Execution IDs still use the existing kernel's allocator (`run:<thread>:<ordinal>`,
`run-attempt:<run>:<ordinal>`, etc.), not T3's labelled `IdAllocator` components.
Changing/migrating those shared identities is a kernel-level integration seam;
the MCP session-scoped thread/message/command request identities match T3.

## Validation (Linux)

All commands used `LINUX_TARGET=target-w3-threads` with
`noches-t3-program/linux-test.sh /Volumes/DevDrive/AiStack/noches-wt/w3-threads`.

- `test -p zeron-engine --lib`: 466 passed, 0 failed, 3 ignored (23 thread tests).
- `test -p zeron-proto -p zeron-rpc -p zeron-mcp`: all suites passed, including
  the 14-test pinned schema oracle and RPC integration tests; 2 live RPC tests ignored.
- `test -p zeron-harness --lib`: 261 passed, 0 failed.
- `test -p zeron-harness --test mcp_bridge`: 4 passed, 0 failed.
- Explicit touched-file `rustfmt --check --edition 2024 --config skip_children=true`
  and `git diff --check`: passed.

Strict `clippy -p zeron-engine -p zeron-proto -p zeron-rpc -p zeron-mcp -- -D warnings`
is blocked by existing `collapsible_if` errors at `update/src/lib.rs:560,620`.
`--no-deps` reaches existing `collapsible_if` errors in `rpc/src/remote/client.rs:182`,
`rpc/src/remote/server.rs:299`, and `rpc/src/server.rs:64`.
Adding `zeron-harness` with those baseline lints allowed reports existing
`type_complexity` (`harness/src/lib.rs:255`), `too_many_arguments`
(`harness/src/acp/mod.rs:2817`), `unnecessary_get_then_check`
(`harness/src/opencode/mod.rs:3154`), and `manual_pattern_char_comparison`
(`harness/src/redact.rs:375`). Those unrelated files were not modified.

Verbatim engine diagnostic output for
`clippy --no-deps --message-format=short -p zeron-engine -- -D warnings -A clippy::collapsible_if -A unused_doc_comments`:

```text
crates/engine/src/diff_sync.rs:14:5: error: doc list item without indentation
crates/engine/src/diff_sync.rs:15:5: error: doc list item without indentation
crates/engine/src/diff_sync.rs:16:5: error: doc list item without indentation
crates/engine/src/doc_host.rs:3725:5: error: this function has too many arguments (8/7)
crates/engine/src/orchestration/command.rs:510:43: error: using `clone` on type `RuntimeMode` which implements the `Copy` trait: help: try removing the `clone` call: `set.runtime_mode`
crates/engine/src/orchestration/command.rs:514:47: error: using `clone` on type `ProviderInteractionMode` which implements the `Copy` trait: help: try removing the `clone` call: `set.interaction_mode`
crates/engine/src/repos.rs:1777:13: error: this method chain can be written more clearly with `if .. else ..`: help: try: `if empty_query { path_a.split('/').count().cmp(&path_b.split('/').count()) } else { std::cmp::Ordering::Equal }`
crates/engine/src/repos.rs:1782:13: error: this method chain can be written more clearly with `if .. else ..`: help: try: `if empty_query { dir_a.cmp(dir_b) } else { dir_b.cmp(dir_a) }`
crates/engine/src/sessions.rs:2483:40: error: redundant guard
crates/engine/src/terminals.rs:482:24: error: this expression creates a reference which is immediately dereferenced by the compiler: help: change this to: `bytes`
crates/engine/src/workspace_files.rs:1980:10: error: using `chunks_exact` with a constant chunk size: help: consider using `as_chunks` instead: `as_chunks::<2>().0.iter()`
error: could not compile `zeron-engine` (lib) due to 11 previous errors
```
