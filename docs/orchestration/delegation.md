# App-owned delegation, completion mailbox and runner bridge

Branch: `orch/delegation`, based on `orch/v2` (`1f56728a`). Upstream oracle:
T3 `v0.0.46-nightly.20261003.2632` / `f391794a35c604d57e166a3ab48d56fc6e4e469a`.
Scope: R3 P3a/P3b and the sessions runner bridge, using the foundation's
generated contracts and transactional kernel. No UI or public tool schemas
were added or changed.

## Shared service boundary

The first commit, `2016c1c3`, creates `orchestration/service.rs`. The three
`OrchestratorService` methods are exactly the Wave 2 agreement: authenticated
`CallerScope` plus generated `DelegateTaskInput`, `TaskStatusInput` or
`TaskCancelInput`, returning the corresponding generated result or `ToolError`.
`CallerScope` carries typed thread/run/project/provider-instance identities,
host-local session identity, resolved workspace root and inherited modes.
`ToolError::into_failure` preserves T3's orchestration error tag/code/message.
Scopes must be minted by the authenticated MCP host, never tool arguments.

`DelegationService` implements the trait. Additional concrete-service hooks:

- `create_threads`: 1–20 ordinary top-level conversations, shared caller
  checkout; sequential per-index receipts preserve partially accepted batches.
  These are not tasks and create no delegated ownership.
- `read_task(..., acknowledge)`: common delegate/status result reader.
- `acknowledge_child_read`: P4 calls only for a direct app-owned child's complete
  terminal result, untruncated and starting at offset zero.
- `wait_child_run`: task-oriented waiting helper; timeout does not interrupt or
  acknowledge. P4's full thread/run selector and pagination remain P4-owned.

## Transactions, identities and authority

`task.rs` plans host-owned commands. Creation atomically writes the parent
subagent, nonblocking node (`countsForRun=false`) and turn item, child app
thread/message/run/attempt/root/provider binding, spawn transfer, start effect
and multi-document publication. Parent and child locks are declared before
dispatch; the store refuses effects/events outside those participants.
No SQL connection/transaction remains held over harness, network or async
lock waits.

Task, backing app thread, logical run, attempt, provider thread/session/turn,
native task, transfer and delivery remain separate identities. IDs use the
foundation's JS `encodeURIComponent`-compatible allocator and
session/action/request command keys. A stable `clientRequestId` replays the
accepted command, not a new child. Preconditions and live catalog resolution
still run before receipt replay; a settled parent or unavailable instance may
refuse a retry. A new review round needs a new key/task, not another run in
the old child.

Role preparation is exactly `Act as the {role} sub-agent for this task.\n\n`;
general/omitted roles leave the task unchanged. No parent history is copied.
Children inherit project, branch/checkout, selected instance/model/options and
configuration, with lifecycle reset. A bound worktree wins; otherwise the
caller scope's resolved cwd is persisted before execution. Runtime and
interaction defaults inherit. Broader authority is refused using both the
authenticated caller ceiling and current parent policy.

`CatalogTargets` consumes `DelegationCatalog`, not a vendor-native model list.
It resolves exact instances, healthy driver-only fallback, configured/custom
model IDs, options and compatible same-selection option inheritance. The MCP
host must supply the same live catalog as the composer/capabilities endpoint.

## Result and cancellation semantics

Progress excludes monitor and rolled-back runs, distinguishes `working`,
`waiting_for_children` and `result_available`, and includes active nested
tasks, pending/claimed nested deliveries and provider background rosters.
Failures prefer error detail; otherwise the qualifying assistant result or
T3's explicit empty-result fallback is used.

The first published terminal task/result transfer is immutable. A later child
run does not reopen it. `task_status` independently exposes later pending runs
(ordinal greater than the delegated run) and `latestTerminal*`, excluding
monitor/rolled-back results and unstarted later runs. Legacy missing spawn
transfers fall back to the first run; an existing transfer with a missing/null
run binding does not. Result publication includes portable/manual-context
handoff provenance when both provider bindings exist.

A status read racing terminal observation first finalizes under parent/child
locks, then acknowledges delivery. Terminal delegate results take the same
path. Complete direct-child reads can acknowledge through the hook above;
partial/truncated reads and waits do not.

Terminal `task_cancel` preserves status/result and disposes mail; it never
interrupts a later child run. Active cancellation atomically disposes parent
mail, stops the backing run's own cohort, holds its queued work, retires its
process effects and requests interruption. The response is `cancel_requested`,
not terminal confirmation. Background-only work without an interruptible run
refuses `task_not_cancellable`. This is not blanket recursive descendant kill.

Wait mode uses settled-only wake, polls with a bounded timeout (default ten
minutes; clamped 1 ms–one hour), and never cancels the child. Timeout best-effort
upgrades to always-wake; `waitTimedOut` applies only to that invocation.

## Durable completion mailbox

`mailbox.rs` maintains per-parent-run disposition, generation and reserved
message membership. Task terminality, result availability, transport acceptance
and explicit observation are different states. Delivery retains
pending/claimed/delivered/acknowledged/disposed. A cancelled delivery run can
return its members to pending and reserve a successor; disposal is final.

Sibling results join an unstarted/queued batch. Once a message is in flight,
arrivals remain pending for a successor generation. Acknowledgement/disposal
removes membership. An emptied queued delivery is cancelled; an emptied active
delivery retains its generation fence until its receipt can reserve pending
successors. Stale generation/message/membership, stopped cohorts and
archived/deleted owners cannot wake. `continuation.rs` promotes only when
there is no blocking work; stale queued notifications are retired without
starving the next eligible entry.

Active routing is checked while holding the parent lock against the current
provider turn/session capability snapshot, not a newly resolved adapter:
all batch members must be always-wake, the run/turn actually running, the
session usable and active steering supported. `/compact` and `/logout`
maintenance inputs are excluded. The sessions runtime additionally checks its
actual bound run's StepBoundary capability and working state.

Claude acknowledges native input-buffer acceptance. Codex acknowledges only
successful `turn/steer` with its expected active turn; rejection/idle never
falls back to `turn/start` for completion notifications. These receipts are
host-only oneshot channels on `SteerMessage`, not replicated credentials or
wire fields. Other/no-longer-live harnesses use durable queued continuation
behind active work. No notification interrupts/restarts active parent work.

Delivery is **at least once**. Message identity remains stable across retries
and unreceipted-steer startup recovery. SQLite and external provider acceptance
cannot commit together; duplicate model consumption is possible in that
window, although transcript entries retain the same ID.

## Runner and integration assembly

`RunnerBridge` implements the foundation `EffectExecutor` using ordinary
`SessionsEngine` dispatch and transcript folding. It resolves the exact
provider instance to its actual `Arc<dyn Harness>`, compiles runtime policy,
materializes the child in the workspace/chat registry, injects fresh session
MCP, subscribes before dispatch, and returns effect acceptance without waiting
for task completion. Four workers therefore do not cap concurrent children.
Native provider subagent events remain observational and cannot create
app-owned tasks, receipts, spawn transfers or independent starts.

The ordinary sessions journal/transcript remains the child chat presentation;
V2 also records result/progress messages. Lagged observation replays through
the same acceptance/settlement path. Public session dispatch/steer refuses V2
subagent chats as user read-only; the runner's private path remains usable.
No composer/chrome rendering was changed. Interrupted/failed provider runs also
retire their native observations and background roster, without recursively
stopping separately owned app runtimes.

Assembly seams intentionally owned by the sibling/integration work:

1. Supply `DelegationCatalog` / `DelegationTargets` from the live configured
   instance catalog; mount `Arc<dyn OrchestratorService>` in the MCP dispatcher.
2. Supply `RunnerInstances` returning exact-instance harness configuration and
   an honest runtime capability snapshot (including active steering).
3. Supply `RunnerMcp::bind`: mint/register the exact run's scoped credential via
   `register_session_mcp`, install instructions and attach credential revocation.
   Binding failure fails the task before a provider start.
4. Assemble a bridge using `EngineCore.orchestration`, sessions/doc/workspace
   handles and execution-host device identity. Under the instance lock, call
   `recover_mailbox` before tools/workers are callable, then start workers with
   an engine-owned shutdown token.
5. Drain publications with the foundation's idempotent publisher/barrier.
   Parent/child task records participate in one barrier; live sessions,
   callbacks/runtime requests and tokens are filtered out.

`EngineCore` opens the profile kernel and installs the V2 recovery gate before
legacy journal recovery. It does **not** invent the sibling's production MCP
server/catalog or automatically start an incompletely configured runner.
The production end-to-end bootstrap must be completed when branches integrate.

Startup retires uncertain starts without re-launching providers, cancels stale
native observations/runless roots, records restart-cancelled background rosters,
holds ordinary queues, reconciles terminal child results and recovers
undelivered mailbox continuations with stable identities. Automatic
checkpoint/native-session restart continuation is P4/runtime-host work, not
silently provided by legacy auto-resume.

## Validation and remaining boundaries

The executable fixtures cover native/app-owned/top-level separation, spawn
rollback at every write boundary, session-key retries/preconditions, partial
batches, role/model/options/mode inheritance, escalation refusals, immutable
results across later runs, monitor/rollback status, nested/native/background
progress, timeout without cancellation, active/terminal/noncancellable cancel,
eight simultaneously accepted mock children with normal transcripts, parent
starting/running/waiting/idle/stopped/archived/deleted states, sibling arrivals
before/during delivery, acknowledgement/read gates, native steer acceptance,
cancelled delivery re-pending, stale generations, unreceipted acceptance crash
and startup recovery. Harness Codex fixtures separately verify receipt success
and rejected notification steering without a hidden follow-up turn.

These are Rust replay/behavior fixtures and mock harness end-to-end tests,
not relabelled as execution of the whole upstream C01–C33 scenario language.
`ZERON_HARNESS=mock` remains supported; tests instantiate the mock through the
existing registry/runtime rather than require a developer's live configuration.

An ignored opt-in test,
`codex_parent_delegates_to_installed_claude_child_via_injected_mcp`, runs a real
Codex parent and Claude child through injected scoped HTTP tools. It requires
both installed/authenticated CLIs and explicit environment values:
`NOCHES_LIVE_DELEGATION=1`, `NOCHES_LIVE_CODEX_MODEL`,
`NOCHES_LIVE_CLAUDE_MODEL`. Its minimal HTTP server is test transport, not
production MCP/auth conformance. **This live test has not been run.**

Known/unverified boundaries: production sibling assembly/publisher, full P4
thread selector/pagination/queue controls, automatic checkpoint-aware restart
continuations, remote-host MCP forwarding, Windows, all live provider/model
combinations and headed UI QA. Title budgets now count UTF-16 units, but when
T3 would split a surrogate pair, Rust truncates before the character: lossless
lone-surrogate wire transport is still the foundation/P4 boundary documented
in `contracts.md`. No stronger universal sandbox or exactly-once delivery claim
is made. No UI files changed, so the UI test/visual gate is not triggered.

All work is committed locally only; per `IMPL.md`, the orchestrator owns
push/PR/merge.

## Validation commands and development findings

```sh
LINUX_TARGET=target-orch3 /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-delegation \
  test -p zeron-engine -p zeron-harness -p zeron-proto --locked

LINUX_TARGET=target-orch3 /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-delegation \
  test -p zeron-engine --lib orchestration --locked

PATH=$HOME/.cargo/bin:$PATH \
  CARGO_TARGET_DIR=/Volumes/DevDrive/AiStack/noches-wt/target-orch3-mac \
  CARGO_BUILD_JOBS=6 cargo check -p zeron --locked

PATH=$HOME/.cargo/bin:$PATH \
  CARGO_TARGET_DIR=/Volumes/DevDrive/AiStack/noches-wt/target-orch3-mac \
  CARGO_BUILD_JOBS=6 cargo clippy -p zeron-engine --locked --no-deps \
  --lib --message-format short
```

The helper pipes through `tail`; success is judged by actual cargo/test result
lines and reaching the final suites, not its shell exit alone. The full Linux
suite passed again on final code commit `8f4c2016`, including engine/harness
integrations, all 39 proto unit tests, all 14 proto oracle tests and doctests.
The final focused orchestration run passed: **62 passed, 0 failed, 1 ignored,
360 filtered out** (0.25 s), including the active-batch acknowledgement fence.
Mac application check passed. Optional non-denying engine-only clippy completed
with 15 existing warnings, including the foundation's Copy-enum clones; no
warning remains in the new task/mailbox/runner code. Strict clippy was not a
requested/CI gate and was not claimed. Existing warnings are not suppressed.
Mac check retains 132 existing UI/Objective-C macro and unused-mut warnings,
plus dependency future-incompatibility notices. Linux tests retain the existing
`unused doc comment` on sessions' test instrumentation.

Development failures were corrected before handoff: host-only runtime requests
were initially included in a publication snapshot; a later-run fixture lacked
`SessionStarted`; two compact harness-example constructors missed the new
host-only receipt field; and a monitor fixture omitted required notification
fields. Representative full-suite diagnostics, preserved verbatim:

```text
error[E0063]: missing field `notification_acceptance` in initializer of `SteerMessage`
   --> crates/harness/examples/cursor_stability_probe.rs:213:29
error[E0063]: missing field `notification_acceptance` in initializer of `SteerMessage`
   --> crates/harness/examples/cursor_stability_probe.rs:442:21

---- orchestration::delegation_tests::status_monitor_rollback_ordinal_and_spawn_binding_match_t3 stdout ----
called `Result::unwrap()` on an `Err` value: Json(Error("missing field `outcome`", line: 0, column: 0))
test result: FAILED. 414 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 98.21s
```

The fixture errors do not remain. No UI test/visual run or billable live
Codex→Claude run was performed; Windows and remote execution remain unverified.
