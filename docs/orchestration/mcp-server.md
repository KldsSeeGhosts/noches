# Engine-owned `t3-code` MCP and provider catalog (P2a/P2b)

Integration update (`orch/wave2`): real engine bootstrap now installs the
delegation service, scoped parent/child runner bindings, publisher and recovery.
Startup readiness includes native Claude API-key auth, and the canonical
catalog imports installed CPA discovery metadata read-only. See
[ui-api.md](ui-api.md) and the passing production [live-e2e.md](live-e2e.md).
Independent per-instance process/account lifecycles and the multi-instance
composer selector remain deferred as described below.

Baseline: T3 Code `v0.0.46-nightly.20261003.2632`, commit
`f391794a35c604d57e166a3ab48d56fc6e4e469a`, Effect `4.0.0-rc.115`.
Worktree/branch: `noches-wt/orch-mcp-server`, `orch/mcp-server`, based on
`orch/v2`. The program's local-commit-only workflow applies: no push, PR, or
merge is performed by this slice.

## Transport, credentials, and injection

`SessionsEngine` owns one lazy `McpServer`, bound exclusively to
`127.0.0.1:0`, with `/mcp` and protocol `2025-06-18`. Streamable HTTP uses
JSON request responses, accepts notifications with HTTP 202, negotiates a
session header on initialize, and supports DELETE. GET streaming is not
needed by this fixed, notification-free toolkit and returns 405. Connections
and request bodies are bounded; bearer authentication precedes discovery.
Non-loopback browser origins are rejected. This is not a remotely exposed
engine endpoint.

Credentials are 32 random bytes, base64url without padding. Only SHA-256
digests and trusted invocation scopes are held in the memory registry.
The 24-hour liveness window refreshes on authenticated traffic and provider
turns; session/thread/all revocation and shutdown cleanup are supported.
Invalid, expired, revoked, or wrong-session credentials receive T3's HTTP
401 `invalid_mcp_credential` body, `WWW-Authenticate: Bearer`, and
`Cache-Control: no-store`.

The scope binds environment, thread, run, project, workspace root, provider
instance, inherited selection/authority, and capability groups. An optional
narrower task scope refuses foreign task IDs before service dispatch.
Caller-supplied JSON cannot select an origin. Task direct-parent ownership,
live-run ownership, and project-scoped domain reads/writes remain the domain
service's responsibility; unavailable stubs perform no reads or mutations.
The minted MCP session UUID is not the kernel's process-session primary key.

Before a fresh legacy harness session starts, the engine calls the existing
`register_session_mcp` seam with the exact server name **`t3-code`**.
An explicitly registered V2 scope takes precedence. Warm runtimes retain
their binding; replacement/end cleanup compares registration ownership, so
old cleanup cannot revoke a successor. Tokens stay out of `RunRequest`,
SQL, journals, Loro, logs, argv, and portable payloads. The foundation's
private harness environment/config files and exact-value redaction remain
the transport mechanism, not persisted engine state.

The injected orchestration instruction text is verbatim upstream:
5,521 bytes, SHA-256
`23317b4e6443838eb57eae6302ab3808556522838cf5bba4d565b8899e6c3e5b`.
It is supplied both through the foundation instructions channel and MCP
initialize. Browser/device instructions are not added. The upstream text's
Electron/Node launcher example is intentionally unchanged; native ACP
adapters already provide their Rust launcher instructions and private
`acp-mcp-call` fallback.

The `zeron-mcp` package provides a `t3-code` stdio executable and exposes
`run_t3_code()`. The existing `zeron mcp` path selects this HTTP facade when
the private session binding environment is present; standalone legacy MCP
clients retain their old namespace. The facade reuses the foundation ACP
bridge, preserving negotiated headers, opaque results/errors, content/SSE
framing, and DELETE on close. No token or caller identity is accepted in argv.

## Toolkit boundary and integration

Live `tools/list` returns the exact **52 core descriptors** from
`crates/proto/tests/t3_oracle/fixtures/tools.json`: names, descriptions,
input/output schemas, and encoded annotations. The frozen 20 preview/device
descriptors are not advertised or callable in this capability-unavailable
slice. The complete core set fits one page; there is no `nextCursor`.
Harness preapproval is independent of discovery: plan/supervised sessions
use readonly annotations, excluding `task_status` and `t3_thread_read`.

`orchestrator_capabilities` returns the generated T3 shape:
parent/inherited selection/authority, `providers[]`, each provider's
`models[]` and optional `options[]`, `currentValue`/`isDefault`, readiness
`constraints[]`, and the pinned core `features{}`.

`delegate_task`, `task_status`, and `task_cancel` invoke the injected
`OrchestratorService`. `orchestration/service.rs` is byte-for-byte the
`orch/delegation` seam, including its generated input/result names,
`CallerScope`, and `ToolError`. The default implementation refuses with
`OrchestratorMcpFailure`, `orchestration_error`,
`The operation could not be completed.` It never emulates old task/session
RPC semantics.

The runner integrates the real implementation with
`sessions.mcp_server().set_service(Arc::new(service))`. Before dispatch,
it must call `McpServer::register` with the canonical kernel thread/run,
project/root, provider instance, modes, and selection. Do not derive these
from tool arguments or register after the harness is already warm.
Catalog/model/options and mode-escalation checks precede delegate service
dispatch on every retry. The service must still enforce authoritative
kernel ownership, permission, and receipt checks.

Other P3–P8 services are deliberately unavailable. Central toolkit refusals
use normal `isError:false` with structured content and JSON text;
worktree status/handoff use the separate `WorktreeMcpFailure` family and
upstream `Unable to read thread …` failure construction. PR tools preserve
their error-mode public messages as `isError:true` text, not a fabricated
central failure. Parameter validation uses AiError/Toolkit/
ToolParameterValidationError for return-mode tools and JSON-RPC -32602 for
error-mode tools. Rust stack traces are not exposed.

The JSON-schema decoder covers the pinned core vocabulary, trimming,
UTF-16 length bounds, strict target option shorthand, legacy model-selection
compatibility, structured/string schedules, metadata action/URL refinements,
and omission/null distinctions. Because published JSON Schema encodes
Undefined as null, executed AST-derived null refusals are retained separately.
`serde_json` preserves object insertion order for upstream JSON-text framing.
The executed parameter oracle locks 34 representative negative cases and
52 upstream-accepted invocations. This is not an exhaustive proof of every
Effect union diagnostic/refinement: multi-error ordering and raw unpaired
surrogate JSON remain unverified. Thread/queue slicing is not implemented
here and is not claimed as parity.

## Canonical per-host catalog

`HarnessRegistry::provider_instances` distinguishes opaque
`ProviderInstanceId` from open `ProviderDriverKind`; `HarnessId` remains the
legacy adapter/branding key. Built-in compatibility IDs retain driver names
(`claudeAgent` for Claude). Unknown/missing adapters remain constrained
shadow entries. Configured instance/model order is retained, removal does
not resurrect custom rows, and internal watch versions change only for
semantic inventory/readiness changes.

Both the composer `ListModels` RPC and MCP capabilities project this same
registry snapshot, not separate model lists. Discovered models are merged
with configured custom models; exact configured IDs are never shortened or
prefixed. Account RPC snapshots update legacy authentication state.
Select/boolean descriptors, their defaults, and arbitrary option IDs such as
`reasoningEffort`, `serviceTier`, `effort`, `thinking`, and `contextWindow`
are preserved. Legacy reasoning chips are projected as canonical descriptors;
boolean chips use existing on/off adapter conventions and are normalized
back to booleans in inherited MCP selections.

An optional `provider-instances.json` array in the engine data directory
configures the host inventory. It contains catalog/readiness metadata only,
not credentials. For example:

```json
[
  {
    "providerInstanceId": "codex_proxy",
    "driverKind": "codex",
    "harnessId": "codex",
    "displayName": "Proxy",
    "enabled": true,
    "installed": true,
    "authentication": "authenticated",
    "adapterRegistered": true,
    "models": [
      {
        "id": "exact-configured-proxy-model-id",
        "label": "Custom model",
        "isCustom": true,
        "options": [
          {
            "type": "select",
            "id": "reasoningEffort",
            "label": "Reasoning",
            "currentValue": "high",
            "options": [
              {"id": "high", "label": "High", "isDefault": true}
            ]
          }
        ]
      }
    ]
  }
]
```

Absent option descriptors differ from an explicitly empty descriptor array:
the former permits otherwise unknown options; the latter rejects them.
Duplicate selections always refuse. Explicit model membership is checked
only for nonempty inventories. Driver-only selection prefers an eligible
parent instance, then the first eligible matching instance. Options inherit
only for the same instance **and** model; cross-instance selections without
options omit the field rather than invent an empty array.

### Remaining catalog integration

The old composer is still HarnessId-based and projects the first matching
instance. It does not yet offer an independent multiple-instance selector
or independent per-instance driver process/config/account lifecycle; those
need the runner/settings/UI integration. Legacy auth starts Unknown until
account discovery, matching T3's non-blocking unknown state. Configured
instances can explicitly supply independent readiness/auth state, while
actual installed/enabled state is also constrained by the underlying
harness registry. Internal catalog watch is available, but no new mobile
or catalog-watch RPC is added here. These limitations must not be described
as completed multi-instance composer execution parity.

## Tests and validation

Tests cover live descriptor conformance, all core routes, executed
parameter/error serialization, instruction digest, invalid/revoked/expired
and wrong-session credentials, traffic/turn refresh, task/capability narrowing,
trusted scope, DI, escalation, readonly preapproval, cleanup races/redaction,
readiness and same/cross-instance matrices, descriptor omission, custom-model
removal, and composer/catalog agreement. An end-to-end stub harness receives
the real injected binding, initializes, calls tools/list/capabilities, and
scans the temporary engine files to ensure its token was not persisted.
A subprocess stdio test verifies negotiation, forwarding, refusal framing,
and close. No live provider/account is needed.

```sh
LINUX_TARGET=target-orch2 /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-mcp-server \
  test -p zeron-engine -p zeron-mcp -p zeron-proto --locked
LINUX_TARGET=target-orch2 /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-mcp-server \
  test -p zeron-ui --lib --locked --config profile.test.package.zeron-ui.opt-level=1
PATH=$HOME/.cargo/bin:$PATH \
  CARGO_TARGET_DIR=/Volumes/DevDrive/AiStack/noches-wt/target-orch2-mac \
  CARGO_BUILD_JOBS=6 cargo check -p zeron --locked
```

Final validation on 2026-10-03:

- Required Linux engine/MCP/proto command: passed, including the existing
  integration suites and their existing external-service ignores.
- Focused MCP filter: 18 passed (14 new MCP tests plus four existing matching
  tests); catalog filter: 4 passed. These cover 34 executed negative oracle
  cases and all 52 upstream-accepted core invocations.
- Injected-harness end-to-end test: 1 passed; stdio subprocess test: 1 passed.
- Proto: 39 unit tests and 14 orchestration oracle tests passed.
- Required Linux UI command: **1,425 passed, 0 failed**, no ignores.
- Required macOS app check: passed.
- `clippy -p zeron-engine -p zeron-mcp --no-deps --all-targets
  --message-format short --locked`: completed; no diagnostics in new
  MCP/catalog modules. Existing warnings elsewhere were not modified.
- Touched Rust files were formatted; `git diff --check` passed.

The Linux helper pipes cargo through `tail`; cargo result lines, not merely
the helper exit code, are the validation gate. The Mac app check passes with
the existing 132 UI warnings and existing future-incompatibility notices.
The Linux engine test build retains the existing `unused doc comment` on
the test-only `thread_local!` instrumentation in `sessions.rs`. Repository
CI does not require a strict-clippy gate; the earlier kernel's unrelated
strict failures remain documented in `kernel-validation.md`.
No UI chrome changed. Headed light/dark QA, real-harness injection on every
adapter, Windows, iOS, full Effect diagnostic/surrogate parity, and the real
delegation service integration are unverified here.

To regenerate the executed MCP cases, first run the proto oracle extractor
against the pinned read-only T3 reference with a separate temporary fixture
output directory. Then pass its retained isolated source/dependency directory
to `node crates/engine/tests/t3_mcp_oracle/extract.mjs <isolated-directory>`.
This evaluates actual pinned tool decoders/encoders, not a hand-authored
approximation, and never installs dependencies in the reference checkout.
