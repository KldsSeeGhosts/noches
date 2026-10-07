# Production headless delegation E2E

Run on `kidsseeghosts`, 2026-10-03. This is a real installed/authenticated
Codex parent and Claude child, not the ignored fixture HTTP server in
`delegation_live_tests.rs`.

## Outcome (verbatim)

```text
LIVE E2E PASS: Codex/Claude child completed; parent received wake and acknowledged PONG.
```

Parent's reported result (verbatim):

```text
I’ll call the injected orchestrator, delegate once, and wait for the app’s completion wake without polling.

WAITING_FOR_CHILD

The completion wake arrived. I’ll read the child’s result.

LIVE_E2E_PASS: child completed; parent received wake; result=PONG
```

## Setup and transport

Linux binary built from `orch/wave2` using `cargo build -p zeron --locked`,
`CARGO_TARGET_DIR=~/AiStack/noches-agents/target-orch4`. Launched as `zeron
headless`. Parent: provider `codex`, exact CPA model `gpt-6.1-sol`. Child:
provider `claudeAgent`, model `claude-haiku-4-5`. Both readiness states were
`authenticated`.

Reproduction (billable, requires the already authenticated CLIs):

```sh
PATH="$HOME/.local/bin:$PATH" python3 scripts/orchestration-live-e2e.py \
  --binary /home/kidsseeghosts/AiStack/noches-agents/target-orch4/debug/zeron \
  --parent-model gpt-6.1-sol --child-model claude-haiku-4-5
```

The stdlib-only script creates its own scratch repository, data directory,
ephemeral localhost IPC port, engine log, and evidence files. Both
`NOCHES_DATA_DIR` and `ZERON_DATA_DIR` point to that scratch profile, preventing
an inherited Noches override from opening a real profile. Cloud sync is
disabled. No real Noches data directories on either host were opened or
changed; no CLI login/configuration was rewritten.

Successful scratch directory: `/tmp/noches-orch-live-_3inz9og` on Linux.
Retained evidence: `engine.log`, `parent-transcript.json`, `state.json`, and
scratch `data/orgs/dev-org/dev-user/docs.sqlite3`. The script shuts down only
its own headless subprocess. Scratch evidence is intentionally retained.
Credentials are excluded from the checked-in excerpts.

The parent prompt asks for `orchestrator_capabilities`, then exactly one
`delegate_task` in async mode, no native subagents, no shell/filesystem work,
no polling before the app's wake, followed by `task_status` acknowledgement.
This uses real `EngineCore` assembly, the production authenticated streamable
HTTP MCP transport, private harness injection, and real runner processes.

## Transcript and persisted-state excerpts

The parent's transcript has these three MCP tool calls, in order:

```json
{"kind":"mcp","server":"t3-code","tool":"orchestrator_capabilities"}
{"kind":"mcp","server":"t3-code","tool":"delegate_task"}
{"kind":"mcp","server":"t3-code","tool":"task_status"}
```

Passive state from the initial successful active-steer run, selected fields
from `state.json`:

```json
{
  "threadId": "headless-live-parent",
  "workState": "result_available",
  "tasks": [{
    "taskId": "node:delegated-task:command%3Amcp%3A91354c2b-9249-44bb-8b2a-4b991a8d1a63%3Adelegate-task%3Aheadless-live-round-1",
    "childThreadId": "thread:delegated-task:command%3Amcp%3A91354c2b-9249-44bb-8b2a-4b991a8d1a63%3Adelegate-task%3Aheadless-live-round-1",
    "providerInstanceId": "claudeAgent",
    "model": "claude-haiku-4-5",
    "status": "completed",
    "workState": "result_available",
    "result": "PONG",
    "latestResult": "PONG",
    "completionDelivery": {
      "observedByRunId": "run:headless-live-parent:1",
      "state": "acknowledged"
    }
  }]
}
```

SQL effect receipts:

```text
provider-turn.start|succeeded
delegated-completion.continue|succeeded
```

The mailbox command receipts include both `:steer` and `:accepted` for the
completion effect, followed by
`delegated_task.completion-delivery.acknowledge`. The parent consumed the
result in **run 1**, not a newly launched continuation run. Together with
the native acceptance receipt this verifies successful non-interrupting
Codex steering. Queued continuation and parked-process logical-run scope
rebinding are separately covered by the automated production-bootstrap E2Es.

## Fresh final-source live run: queued continuation

Rebuilt the Linux binary after exact-owning-session credential rebinding and
inherited model/option validation were finalized, then reran the same scratch
test. It passed again in `/tmp/noches-orch-live-t38i96h_`. In this run the
parent finished its first turn before the child completed, so the mailbox
queued a continuation rather than claiming active steering. The parent's
completion acknowledgement has `observedByRunId: run:headless-live-parent:2`.
This is the expected settled-parent fallback, not an interrupt/restart of an
active parent.

Verbatim final-source outcome and parent excerpt:

```text
LIVE E2E PASS: Codex/Claude child completed; parent received wake and acknowledged PONG.
I’ll read the child’s result to acknowledge the completion wake.


LIVE_E2E_PASS: child completed; parent received wake; result=PONG
```

Read-only inspection of its scratch SQL confirmed:

```text
delegated_task.completion-delivery.acknowledge|accepted
delegated_task.finalize|accepted
delegated_task.request|accepted
notification.delivery|accepted
provider-turn.start|succeeded
delegated-completion.continue|succeeded
provider-turn.start|succeeded
```

The two provider-start effects are the child and the parent's queued
continuation. The original successful run above independently retains the
active-turn `:steer`/`:accepted` receipts. Both runs use the same real
production bootstrap/HTTP server and already authenticated CPA routes.

## Initial failure, corrected before final validation

The first real attempt reached both injected discovery and delegation tools,
but the account-slot-only readiness join incorrectly refused API-key-backed
Claude:

```text
Delegation failed: `claudeAgent` is not authenticated. No child task or `taskId` was created.
```

The independent native probe showed:

```json
{"loggedIn":true,"authMethod":"api_key","apiKeySource":"ANTHROPIC_API_KEY"}
```

This was **not** a live inference authentication failure. The production
catalog now treats a missing Claude OAuth slot as Unknown and uses the
adapter's bounded `claude auth status --json` readiness bit. It never stores
or logs native probe output or credential values. The second live attempt
passed without changing credentials, logins, model routes, or configuration.
The first attempt's scratch directory is `/tmp/noches-orch-live-zhc6xpwg`.

## Automated evidence

`crates/engine/tests/orchestration_bootstrap.rs` runs through the same real
bootstrap and MCP server with an automated mock adapter. It covers parent
capabilities/delegate/status calls, a child with a distinct scoped credential,
completion, active steering and native acceptance, queued continuation for
non-steering adapters, a second delegation from a parked persistent process
with a newly admitted logical run, result acknowledgement, registry discovery,
RPC state, and durable Loro/chat2 publication.

Document tests verify epoch/version/owner fences, registry patch idempotence,
ordinary chat-field preservation, and orchestration metadata surviving thin
checkpoint rebuilds. Catalog tests retain exact CPA IDs/options and exclude
transport templates/instructions from imported metadata.

## Final validation and corrected development findings

Required commands, run against the final production source:

```sh
LINUX_TARGET=target-orch4 /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-wave2 \
  test -p zeron-proto -p zeron-harness -p zeron-engine -p zeron-doc -p zeron-mcp --locked

LINUX_TARGET=target-orch4 /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-wave2 \
  test -p zeron-ui --lib --locked --config profile.test.package.zeron-ui.opt-level=1

PATH=$HOME/.cargo/bin:$PATH \
  CARGO_TARGET_DIR=/Volumes/DevDrive/AiStack/noches-wt/target-orch4-mac \
  CARGO_BUILD_JOBS=6 cargo check -p zeron --locked
```

- Full Linux proto/harness/engine/doc/MCP suites: passed, including integration
  tests and final doctests. The helper buffers only the last 120 lines and
  does not propagate cargo's pipeline failure reliably; reaching the final
  successful doctests, not its shell exit alone, is the gate.
- Linux UI: **1,425 passed, 0 failed**, no ignores.
- Mac application check: passed. Existing 132 UI warnings and dependency
  future-incompatibility notices remain.
- Focused production-bootstrap E2Es: **2 passed**; catalog regressions:
  **7 passed**; MCP regressions: **15 passed**, including the exact-owning-session
  warm-scope fence and all four corrected fixture scenarios.
- Existing engine E2Es: **28 passed, 2 ignored**; restart/resume:
  **8 passed, 1 ignored**. Existing external-service/live-only ignores were
  not relabelled as passing live evidence.
- Touched Rust files were formatted; Python reproduction script parses;
  `git diff --check` passed. Repository CI has no strict-clippy gate.

During development an existing interrupt test observed the aborted transcript
stamp before the async executor had committed its durable command receipt.
It now waits for that receipt rather than racing it. The focused regression
and final full suites passed.

The stricter inherited-model validation also exposed four older MCP fixtures
whose scope used `mock`, although their adapter advertises `mock-1`.
The fixtures now use the advertised model; the catalog validation was not
weakened. Verbatim diagnostic excerpts from that corrected intermediate run:

```text
Model mock is not advertised by provider mock.

failures:
    mcp::tests::all_52_core_routes_accept_upstream_inputs_and_refuse_unimplemented_domains
    mcp::tests::delegation_escalation_is_refused_before_domain_dispatch
    mcp::tests::injected_domain_service_receives_only_trusted_caller_and_resolved_target
    mcp::tests::unavailable_and_validation_families_keep_t3_framing

test result: FAILED. 438 passed; 4 failed; 3 ignored; 0 measured; 0 filtered out; finished in 97.85s
```

No headed visual, Windows, iOS, reverse-direction Claude→Codex live run, or
remote-host MCP forwarding is claimed. The exact live failure and full
validation results are documented here and in `ui-api.md`; P4–P8 tools remain
the sibling workstreams' explicit unavailable routes.

# Wave 3 live scenarios (macOS, 2026-10-07)

`scripts/orchestration-live-scenarios.py` boots one headless `zeron` per
scenario on a scratch data directory (never a real Noches profile) and drives
the same RPC surface as the desktop: `Mutate` (`createChat`, `setChatConfig`),
`QueueCommand` (run), `QueueMessage` + `MutateQueuedRun` (promote to
steer/restart), `ForkThread`, `StopThreadWork`, `ResetThreadSession`,
`UploadChunk`/`UploadCommit`. Each scenario prints one `PASS|FAIL|UNSUPPORTED|SKIP`
line and writes `evidence/<n>.json`. The engine copies installed CLI logins into
`data/agent-accounts/` of its scratch profile; the driver deletes that directory
on shutdown, so retained evidence never holds credentials.

Authorized, cheap models only (all via the owner's CPA proxy): `claude-haiku-4-5`
(`claudeAgent`), `gpt-6-luna` and `gemini-3.8-flash` (`codex`, medium effort), and the
same models on the native Pi driver as exact `cpa/...` slugs (`pi` instance).
Provider turns are capped by `--max-turns` (default 60).

```sh
LINUX_TARGET=w3-live /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh <worktree> build -p zeron
python3 scripts/orchestration-live-scenarios.py \
  --binary /Volumes/DevDrive/AiStack/noches-wt/targets/w3-live/debug/zeron \
  [--only 1,2,...] [--stop-parent claude|codex|codex-gemini|pi] [--reset-pause SECONDS]
```

## Results

Evidence: `…/noches-t3-program/w3/live-scratch/<run>/evidence/<n>.json` (not committed).

| # | Scenario | Providers | Result | Run |
|---|----------|-----------|--------|-----|
| 1 | Steer mid-tool and right after the last tool result: one user message, one native receipt, one acceptance row, no redispatch | Haiku | PASS | run1b |
| 2 | Lazy native fork (turn-1 checkpoint vs head): child session id equals the host-minted `claude-fork:v1` id, parent session file byte-identical, turn-1 fork knows `X` only, head fork `X` and `Y` | Haiku | PASS¹ | run2a |
| 3 | Effort `medium→high` then model `gpt-6-luna→gemini-3.8-flash`: same native thread, no handoff/transfer, Codex's own `turn_context` records each model/effort | luna, gemini | PASS | run3a |
| 4 | Queue promotion to interrupt/restart after a model change: restart on the same native thread with the new model, second queued row untouched, `sleep` child released, one app-server | luna → gemini | PASS | run4b |
| 5 | A→B→A: B gets a portable `full_thread_summary`, A resumes its own session with a `delta_since_target_last_seen` handoff and recalls a fact told only to B | Haiku ↔ luna | PASS | run5d |
| 6 | Multimodal steer: solid-red PNG into a running turn, delivered natively once, answer `red` | luna; Haiku | PASS | run6b |
| 7 | Codex fork at a turn boundary (native `thread/fork`, child rollout names the parent); installed app-server uses `historyMode: paginated` and `thread/fork` + `thread/turns/list` + `thread/revert` leave exactly the first of two turns | luna | PASS | run7a |
| 8 | Stop all: parent + delegated Haiku child both stopped (`stoppedRuns: 2`), no `sleep` survives, queued message stays queued | Haiku parent, Haiku child | PASS² | run8c |
| 8 | Same, with a **Codex** parent and no Noches-side tool workaround: the CPA proxy now answers `tool_search`, the parent finds the deferred `delegate_task` itself (`stoppedRuns: 2`, no `sleep 90`, queue intact) | gpt-6-luna; gemini-3.8-flash | PASS | run8-luna, run8-gemini |
| 9 | Reset agent session: new native session without `--resume`, portable context in the new session's prompt | Haiku | PASS³ | run9b, run9c |

| 10 | Pi parent delegates to a Haiku child and receives the wake: one `delegate_task`, child on `claudeAgent`/`claude-haiku-4-5` completes `PONGHAIKU`, delivery `acknowledged` by the parent's run, parent reports it | Pi (`cpa/gemini-3.8-flash`), Haiku | PASS | run10a |
| 11 | Haiku parent delegates to a Pi child (`pi`, `cpa/gemini-3.8-flash`) and receives the wake: child session is a Pi session file, result `PONGPI` acknowledged | Haiku, Pi | PASS⁴ | run11b |
| 12 | Haiku → Pi → Haiku: Pi answers from a `full_thread_summary` and has a real session file; Haiku resumes its own session and gets the build number told only to Pi via `delta_since_target_last_seen` | Haiku, Pi | PASS | run12a |
| 13 | Pi lazy native fork through `ForkThread`: nothing starts at fork time; turn-1 child knows `X=1; Y=unknown`, head child `X=1; Y=2`; both resolve `native_fork` (no portable preamble), distinct Pi session files, parent file byte-identical | Pi | PASS | run13a |
| 14 | Pi queue promotion with a model change (`cpa/gemini-3.8-flash` → `cpa/gpt-6-luna`): Pi is not interrupt/restart-steerable (as in T3), so the promotion is a **deferred active steer**: turn 1 takes the steer on gemini, the next turn runs on luna in the SAME Pi session file; one receipt + one acceptance row, no handoff, queue drained | Pi | PASS⁵ | run14c |
| 15 | Pi steer receipts: mid-tool and right after the last tool result, each exactly one user message, one `inputAcceptedFor`, one acceptance row, both honored | Pi | PASS | run15a |
| 16 | Stop all with a Pi parent delegating a slow Haiku child: `stoppedRuns: 2`, no `sleep 90`, queued message stays queued | Pi, Haiku | PASS | run16a |

¹ Run2a predates the fork-token assertions added afterwards; they were replayed
read-only against run2a's own SQL/journal data (child id = token child id; token
turn = the parent's `nativeReference` for turn 1 / head) but not re-run live.
² At the time of run8c, `--stop-parent codex` (gpt-6-luna) was an environment limitation: luna called
`tool_search`, which the CPA route answered `unsupported call`, so it could never discover the deferred
`delegate_task` MCP tool (run8a). The proxy now speaks `tool_search`; the Codex-parent rows (run8-luna,
run8-gemini) pass.
³ Run9a (immediate send after Reset, before the fix) failed; see below.
⁴ Run11a passed every product check; its one FAIL was a driver bug (the "delegated exactly once" check
counted a `ToolSearch` call that merely mentioned `delegate_task`), fixed in the script and re-run live.
⁵ Run14a (written for a Codex-style interrupt/restart) failed because Pi reports
`supportsSteeringByInterruptRestart: false`; the scenario was rewritten for the deferred-steer semantics and
run14b passed except one driver assertion (it expected no `cancelled` run, but the promoted queue row is the
cancelled middle run, as for Claude in scenario 1, and the idle warm Pi process retired for the model change
journals one trailing `interrupted` done), corrected and re-run as run14c. Run14b's scratch directory was
accidentally reused for a rerun and is not evidence.

## Findings

* **Bug fixed: a message sent while a stopped/reset session was still exiting
  was accepted and then lost.** Reset (and Stop, and Disconnect) cancel a parked
  session handle that stays registered until its process exits; warm dispatch
  still routed the next prompt into that dying mailbox, so the user message
  appeared in the transcript and its run ended `interrupted` with no answer
  (run9a: Reset returned at 14:01:12.283, the next run was dispatched 100 ms
  later and was interrupted 2.2 s after). `SessionsEngine::dispatch` now treats a
  handle whose interrupt was requested as mid-teardown, waits for retirement and
  starts a fresh run. Regression:
  `restart_resume::a_send_during_a_requested_teardown_starts_a_fresh_run`
  (fails without the fix); live confirmation run9c (`--reset-pause 0`).
* **Observation (not changed): idle gap after a steer that lands after the final
  text.** A steer sent right after the last tool result can arrive after Claude
  has already written its answer; the CLI answers it as a trailing turn. The
  thread is delivered exactly once and answered (`GAMMA` then `GAMMA DELTA`), but
  between the first `done` and the trailing answer (≈0.9–1.1 s in
  scenario 1's state samples) `GetQueueState` reports no active run and
  `workState: result_available`.
* **Pi (wave 3 Q):** no product bug found by the seven Pi scenarios; the model change on the same Pi
  instance applies on the next turn via RPC `set_model` on the resumed session. Pi writes its own session
  files under the real `~/.pi/agent/sessions/--<scratch cwd>--/` (not under the scratch data directory).
  About 32 provider turns were spent (gemini-3.8-flash, gpt-6-luna and Haiku, all through the CPA proxy).
* **Codex parent (Stop all):** with the CPA proxy answering `tool_search` natively, gpt-6-luna and
  gemini-3.8-flash parents both discover and call the deferred `delegate_task` without any Noches eager-tools
  workaround (none exists on this branch); the earlier environment limitation no longer applies.
* **Environment, not product:** `codex` retried `Connection failed` against
  `127.0.0.1:8317` for minutes once (run5b); the same scenario passed on re-run.
  Driver races fixed along the way: `run()` now returns only after the host
  accepted the dispatch (a provider switch can take ~2.6 s), upload ids are
  `[A-Za-z0-9_-]`.
