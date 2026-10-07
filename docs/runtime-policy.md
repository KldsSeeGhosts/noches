# Runtime authority and requests

Noches uses T3's exact `RuntimeMode` JSON:
`approval-required`, `auto-accept-edits`, `auto`, `full-access`.
`InteractionMode` is independently `default` or `plan`.
Runtime authority ranks in that order; plan is narrower than default.
The shared `permits` helpers are the inheritance boundary for the later
orchestration implementation. This work does not implement delegation.

## Migration and defaults

**Every legacy session/request missing the new fields loads as
`full-access` / `default`, regardless of `sandbox` or `autoApprove`.**
This deliberately preserves the effective legacy behavior: ordinary Codex
forced danger-full-access/never, and ordinary Claude/ACP callbacks allowed
tools. The old fields remain readable but no longer compile runtime authority.
There is no attempt to infer supervision from an old checkbox that did not
actually enforce it.

New desktop sessions also default to `full-access` / `default`, matching the
pinned T3 nightly's `packages/contracts/src/providerPolicy.ts`
(`DEFAULT_RUNTIME_MODE`, `DEFAULT_PROVIDER_INTERACTION_MODE`) and
`packages/contracts/src/settings.ts` (`defaultRuntimeMode` decoding default).
Users can select Supervised, Auto-accept, Auto, or Full access in the composer.
Mode changes replace warm runtimes; native session resume receives the new
policy explicitly. Authority is journaled so crash revival does not widen it.
Explicit steering cannot retain broader authority after a saved mode change.
Its new-turn fallback and orphan-question continuations inherit the last
journaled authority, narrowed by the selected modes. Escalation needs an
explicit Run request, never an implicit question answer or steer fallback.
Restricted/Plan runs require the explicit `runtime-policy-v1` capability from
both the local engine and execution host. Old or unknown hosts refuse before
forwarding or writing a durable command; semver is not evidence of support.
The older `create_chat` MCP tool, which explicitly promises constrained
creation, now requests Supervised rather than inheriting the desktop default.

## Native mapping and limitations

Source oracle: `t3code-ref/apps/server/src/orchestration-v2/Adapters/`
and `apps/server/src/provider/acp/`, pinned by the program brief.
These are provider/client enforcement boundaries, not universal OS sandboxes.

| Adapter | Supervised | Auto-accept | Auto | Full access | Plan |
|---|---|---|---|---|---|
| Claude CLI | `default` + prompts | `acceptEdits` + remaining prompts | `auto` + remaining prompts | `bypassPermissions` | native `plan`; ExitPlanMode denied as in T3 |
| Codex app-server | `untrusted`, readOnly, user reviewer | `on-request`, workspaceWrite, user | `on-request`, workspaceWrite, auto_review | `never`, dangerFullAccess, user | native collaborationMode (explicit model required) |
| Cursor SDK | autoReview=true, sandbox=true | autoReview=false, sandbox=true | same flags as Auto-accept | autoReview=false, sandbox=false | native SDK `plan` |
| Grok ACP | launch `--permission-mode default` | refused | launch `--permission-mode auto` | `agent --always-approve` | refused until native gate supported |
| Antigravity ACP | native `default` | `auto_edit` | native `default` (T3's intentional mapping) | `yolo` | refused |
| Generic ACP (Devin/Hermes) | advertised default/ask mode required | advertised acceptEdits/auto_edit mode required | refused: no classifier | advertised bypass if present | refused |
| OpenCode 1/2 | T3 ask/read rules | T3 edit-allow rules | T3 ask rules, no invented classifier | explicit wildcard allow | refused until plan agent/path rules implemented |
| Pi native RPC | `noches-policy.ts` confirms every non-read-only tool | same, minus `edit`/`write` | refused: no classifier | no hook | refused: no plan gate |

ACP missing/rejected required mode selection fails before `session/prompt`.
Grok retains Noches's `--no-auto-update` and `--no-leader` safeguards and sets
initialize `_meta.clientType=extension`. Model options cannot overwrite a
permission mode. OpenCode installs native rules on create/resume and children;
a failed child-policy install aborts that child. Unsupported provider versions
must reject policy-critical RPCs, never silently continue with defaults.
Pi enforces Supervised and Auto-accept through its public blocking `tool_call`
extension hook (`crates/harness/src/pi/noches-policy.ts`): a refused tool never
runs (`pi_live::live_supervised_mode_blocks_tools_until_the_gate_allows_them`
verifies this against the real binary). The hook raises an ordinary `confirm`
dialog whose message is the structured request; the host routes it through its
permission gate, so Allow-for-session grants are exact per tool input. Host
pre-approved MCP tools skip the prompt. This is a client boundary, not an OS
sandbox. Plan and Auto stay refused until Pi has a native plan gate/classifier.
Cursor follows T3's SDK flag mapping but has no public approval/question-answer
channel; it does not offer Noches approval-modal parity.

## Live approvals, separate from content questions

`PermissionRequest` has stable option IDs, decision/scope, and
pending/resolved/expired state. Only the owning engine registers the callback.
Provider-stream events cannot mint permission requests or forge resolutions.
`RespondPermission` travels through the existing `QueueCommand` RPC/durable
command plane; it accepts an exact request ID and option ID, not a label.
Unknown IDs, duplicate answers, unknown options, and dead callbacks refuse.
Approval callbacks expire on receiver drop, interrupt, turn end, runtime exit,
and restart. **An expired permission never becomes a resumed prompt.**

Allow once grants only this callback. Codex uses native session grants.
Claude applies scoped native rule suggestions with destination=session only;
it refuses mode-switch suggestions and never fabricates wildcard tool rules.
Without a scoped native suggestion it caches the exact request for this runtime.
ACP/OpenCode use runtime-local exact request fingerprints and return only
native one-time grants; they never turn session consent into `allow_always`
or OpenCode's project-persistent `always`. Where the request lacks sufficient
scope information, Allow for session is disabled. Grants disappear on runtime
replacement/restart. Unknown requests and missing UI answers default to denial.

Content questions use the separate `UserInputQuestion` / `UserInputRequest`
contract and `RespondInput` bridge; questions never auto-approve, including in
Full access. Grok's two native ask_user_question aliases return native answers.
Legacy ACP kind-less content choices remain compatible; unknown, null, or
mixed permission option kinds cannot route through the question bridge.
Existing orphan content-question recovery remains a conversation continuation,
not an approval mechanism. Persistent consent (`acceptAlways`) is represented
in proto but is not exposed by this minimal UI.

The gpui approval surface reuses AwaitingInput (the shared indigo state palette)
and the composer area, with Allow once / Allow for session / Deny. Its styling
is intentionally minimal; light/dark/split headed QA remains a delivery gate
for the designer's final styling.
