# Host-local provider instances

The instance identity is T3's open slug `ProviderInstanceId`, not `HarnessId`.
Driver names are T3 names (`codex`, `claudeAgent`, `cursor`, etc.). Multiple
instances of one driver have independent runtimes, discovery caches and auth
readiness. Exact opaque model IDs and option descriptors are retained.

## Designer API

Types live in `zeron_proto::provider_settings`; typed methods are on
`zeron_rpc::RpcClient`. All operations accept optional `targetDeviceId` for
host routing. Credentials and launch configuration never enter CRDT documents.

| RPC | Input | Result |
| --- | --- | --- |
| `GetProviderInstanceSettings` | `{}` | redacted settings rows |
| `CreateProviderInstance` | `{instanceId, instance}` | all redacted settings rows |
| `UpdateProviderInstance` | `{instanceId, instance}` (full envelope) | all redacted settings rows |
| `DuplicateProviderInstance` | `{instanceId, newInstanceId, displayName?}` | all redacted settings rows |
| `DeleteProviderInstance` | `{instanceId}` | all redacted settings rows |
| `SetProviderInstanceEnabled` | `{instanceId, enabled}` | all redacted settings rows |
| `ListProviderInstances` | `{}` | refreshed catalog with models and readiness |
| `ListModels`, `ListCommands` | `{instanceId, harness?}` | that instance's models/commands |

Create/update input (`WriteProviderInstance`):

```json
{
  "instanceId": "codex_proxy",
  "instance": {
    "driver": "codex",
    "displayName": "Codex via CPA",
    "enabled": true,
    "environment": [
      {"name": "OPENAI_API_KEY", "value": "user-secret", "sensitive": true}
    ],
    "config": {
      "binaryPath": "codex",
      "homePath": "/private/codex-proxy",
      "launchArgs": "-c 'model_provider=\"cpa\"'",
      "customModels": ["opaque/exact-model"]
    }
  }
}
```

`ProviderInstanceSettings[]` response:

```json
[
  {
    "instanceId": "codex_proxy",
    "instance": {
      "driver": "codex",
      "displayName": "Codex via CPA",
      "enabled": true,
      "environment": [
        {"name": "OPENAI_API_KEY", "value": "", "sensitive": true, "valueRedacted": true}
      ],
      "config": {"customModels": ["opaque/exact-model"]}
    },
    "authentication": "authenticated",
    "modelCount": 1,
    "constraints": []
  }
]
```

`authentication` is `authenticated | unauthenticated | unknown`; unknown is
non-blocking. Constraints mirror the capability catalog, including
`Provider instance is disabled.`, `Provider executable is not installed.`,
`Provider is not authenticated.`, and invalid/unknown-driver shadow reasons.
`ListProviderInstances` uses `providerInstanceId` (existing catalog contract)
and exposes `driverKind`, `displayName`, `enabled`, `authentication`,
`modelCount`, `models`; constraints are also derived by `orchestrator_capabilities`.
`GetProviderInstanceSettings` is a passive read of current readiness; use
`ListProviderInstances`/`ListModels` to refresh discovery.

Sensitive env values are returned empty with `valueRedacted:true`. Preserve
that marker on an update to retain the last stored value of that name.
`apiKey`/`serverPassword` config fields use `[redacted]` with the same round-trip
semantics. A create cannot reuse another instance's redacted marker. Duplicate
preserves route/model/env settings but drops auth-home overrides.

For selection, carry `instanceId` in `ChatConfig` and `RunRequest`, or T3's
`ModelSelection {instanceId, model, options}` for orchestration. Missing IDs
select only the driver's canonical identity, never an arbitrary sibling.
`Pickers::select_provider_instance(harness, instance_id, cx)` is the nonvisual
composer seam; it invalidates old model slots and persists existing-chat picks.
Grouping and Settings UI are owned by the designer.

## Persistence and runtime semantics

`{dataDir}/provider-instances.json` is an atomic mode-0600 T3 config map keyed
by instance ID. Absent settings migrate one canonical entry per enabled
harness (`codex`, `claudeAgent`, etc.), preserving current auth homes and CPA
catalog import. Explicit maps, including `{}`, win on all later loads.
Old array inventories are converted; unknown driver/config payloads survive.
The legacy `SetHarnessEnabled` API controls only the canonical identity.

New Codex instances default to private `CODEX_HOME`; Claude instances use
private `CLAUDE_CONFIG_DIR` while preserving HOME/macOS keychain compatibility.
Other drivers receive private HOME/XDG/driver roots. Explicit home/env values
can intentionally share a root; no auth files are copied on duplicate.
Environment overlays are command-local and last-value-wins. Only CODEX_HOME
and CLAUDE_CONFIG_DIR env tilde prefixes are expanded. Launch args use T3's
quote tokenizer, never a shell. App-owned MCP credentials cannot be overridden.
Cursor's `binaryPath` is compatibility metadata; its SDK backend executable
override is `CURSOR_SDK_SHIM_EXECUTABLE`. OpenCode can attach via
`serverUrl`/`serverPassword`.

Structural no-op updates retain caches. Changed/deleted entries invalidate
only their own caches, interrupt their live sessions and revoke credentials;
discovery checks the config/runtime generation before publishing results.
Native resume references are scoped to instance plus cwd, with serde defaults
for old sessions/documents. Instance selection is sync-safe; secret settings
are device-local.

## Conformance and explicit gaps

Registry tests mirror T3 hydration, explicit-over-default maps, enabled-envelope
AND raw-config flags, structural runtime retention, unknown/invalid shadows,
last-value-wins environments, redacted secret restoration and CLI quote parsing.
C05/C06 catalog tests retain duplicate-driver identity, exact custom models,
availability/constraint ordering and selection/options inheritance.
`provider_instance_profiles` runs two fake Codex app servers and two Claude
auth contexts: distinct homes, models, readiness and native resume ownership.

This slice does not add provider login flows or the designer's Settings/grouping
UI. Drivers without an auth probe report unknown. Pi runs over its native RPC
transport: `binaryPath` selects the `pi` binary and `launchArgs` are validated
as T3 does (`--mode`, `--session*`, `--fork`, `--continue`, positional prompts
and a lone `--provider` are rejected when the instance is written). Codex `shadowHomePath`
auth-overlay symlinks are also explicitly constrained; private `homePath` works.
Explicit title-driver preferences remain harness-based; automatic titles follow
the session's instance. No live paid-provider or headed visual QA is claimed.
