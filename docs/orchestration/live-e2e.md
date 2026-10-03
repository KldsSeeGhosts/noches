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
