# Pinned orchestration contracts

Baseline: T3 Code `v0.0.46-nightly.20261003.2632`, commit
`f391794a35c604d57e166a3ab48d56fc6e4e469a`. T3's root MIT notice is
copied in `THIRD_PARTY_NOTICES.md`. No live userdata, provider, Git workflow,
engine scheduler, or production MCP server is used by this slice.

## Reproduce and review

From this worktree, with Node >=24.13.1, npm, git, tar, and rustfmt available:

```sh
node crates/proto/tests/t3_oracle/extract.mjs /Volumes/DevDrive/AiStack/t3code-ref
node crates/proto/tests/t3_oracle/generate.mjs
LINUX_TARGET=target-orch /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh \
  /Volumes/DevDrive/AiStack/noches-wt/orch-contracts test -p zeron-proto
```

The extractor uses `git archive` of the tag's resolved commit, not the
reference checkout's current files. It copies sources to a fresh `/tmp`
directory and installs only pinned Effect there, with lifecycle scripts
disabled. The checked-in extraction lockfile pins package integrity.
Temporary copies are retained and their paths printed for inspection.
Nothing is installed or written in `t3code-ref`.

An optional second argument is a different fixture output directory. Extract
to another temporary directory and compare it with the committed fixtures
before accepting an upstream change. Regeneration is not an automatic
approval of schema drift.

`tools.json` evaluates the ten actual toolkit modules using their actual
`Tool.make`, schema, description, annotation, and toolkit expressions.
Only service-token imports are replaced with inert tokens in the temporary
copy; schemas and registrations are not rewritten. The attachment schema
import is redirected to the same isolated contracts package. There are
exactly **72 names, 52 core and 20 later**; no `preview_screenshot`, device
tap/type tools, or optional retry keys are added.

`contracts.json` records the complete exported orchestration V2,
orchestrator MCP, provider/model, scheduled-task, PR, and application-project
event type graphs and their transitive schema dependencies. Branded IDs
remain distinct strings; `TaskId` is intentionally an alias of `NodeId`.
The full two requested TS contract sources and the MCP framing source are
also retained, since transformation functions cannot be losslessly described
by JSON Schema. `provenance.json` hashes the original sources and license.
`refusal-sources.json` retains the complete toolkit handlers and MCP services/
access helper for all refusal branches and exact message construction,
without attempting to guess runtime-dependent messages from a static schema.
`scenario-sources.json` freezes four real upstream normalized-provider replay
traces (nested wake, before-queued-prompt, after-root, interrupt), their
input/expected-output fixtures, and the task/queue/mailbox projection sources.
They are replay seeds for subsequent kernel work, not a claim that the Rust
engine already executes those scenarios.
The shared `conformance-protocol.schema.json` reserves transport-neutral
codec/tool/provider-event/clock/barrier/crash steps. Tool-call names resolve
only against the pinned inventory; bootstrap and assertion payloads are
target-adapter data. Codec cases execute now. Provider/reducer/crash adapters
are deliberately not implemented in this contracts-only branch.

`codec-cases.json` is an executed upstream decode/encode oracle, including
rejections. `serde-cases.json` is mechanically synthesized structural
coverage from the extracted graph, with every top-level union arm, all
required fields, and both omitted/present optional-field cases. These are
different kinds of evidence: structural samples are not provider execution
traces. Source digests plus the schema/descriptor and generated-artifact
digests lock additions/removals, fields, defaults, annotations, tags, and
the Rust translation. Existing legacy Chat/CRDT types are unchanged.

## Rust ownership and wire representation

- `orchestration.rs`: complete V2 commands, domain events, stored events,
  projections/shell/stream/history records, thread/run/attempt/node/task
  separation, provider sessions/threads/turns, context/checkpoints,
  cohorts/delivery, schedules and PR state. Queues are queued runs.
- `orchestration_mcp.rs`: all 72 input/result/error types, named upstream
  contracts, typed invocation dispatch, pinned metadata, and distinct error
  families. Opaque upstream `Unknown`/JSON/Defect payloads remain JSON values,
  not guessed domain structs.
- `provider_instance.rs`: open driver and instance slugs, opaque instance
  configuration, model selections, catalog entries and option descriptors.

Tagged unions retain the actual tag key (`type`, `kind`, `_tag`, `strategy`,
or `operation`) and spelling. Unions with an optional discriminator (notably
image upload) must remain untagged. Numeric/boolean literals are validated
literal types, not string enums. Dates use the JSON ISO-string
representation rather than importing Effect's in-memory DateTime objects.
`JsonNumber` retains Effect's named non-finite-number representation.

`Optional<T>` preserves omission; `Optional<Option<T>>` preserves omitted
versus explicit null versus value. Required nullable fields have an explicit
deserializer so serde cannot silently fill a missing field with null.
Only upstream decoding defaults are inserted. For example, `enabled` and
`bindToCurrentThread` remain optional in schedule input: their documented
true defaults are service decisions, not extra codec fields.

Runtime/interaction/approval ownership belongs to `orch/runtime-policy`.
`runtime_policy.rs` now owns the authoritative enums. The generated contracts
and generator re-export `RuntimeMode`, `InteractionMode` (also under the
upstream `ProviderInteractionMode` name), and `PermissionDecision` (under
`ProviderApprovalDecision`) with identical T3 JSON. The generated
`ProviderApprovalOption` retains its exact wire fields; the engine's live
`PermissionOption` additionally owns callback IDs and scopes.

## Upstream ambiguities and easily missed boundaries

1. **Published versus decoded input schema.** Effect's tools/list schema
   describes the encoded JSON boundary, not all post-transformation checks.
   It can advertise optional null/non-finite encodings and lose checks behind
   trimming transformations. Both `inputSchema` and `decodedInputSchema`
   are recorded. Neither is silently substituted for the actual descriptor.
   Engine handlers must use the real decoding/validation pipeline, not infer
   permission or length checks from permissive wire structs.
   There is also a pinned title-annotation quirk: Effect's standard MCP
   registration spreads a title string, whose character keys are removed on
   encoding. Standard tools therefore omit the title hint despite
   `Tool.Title` annotations; T3's two hand-registered image tools retain it.
   `descriptor`/`annotations` capture the encoded wire object, while
   `sourceAnnotations` preserves the toolkit author's annotation.
2. **Model options have two different compatibility policies.** Persisted
   `ProviderOptionSelections` accepts legacy records and drops unsupported
   scalar values/empty entries. `OrchestratorMcpTargetOptions` rejects values
   other than nonempty strings/booleans. ModelSelection accepts historical
   `provider` only when `instanceId` is absent and emits canonical instance
   selection. Focused compatibility codecs are tested against T3 execution.
3. **Notification kinds are not identity encodings.** Decoded `command`
   re-encodes as `background_command`; decoded `subagent` re-encodes as
   `background_task` plus `work:"subagent"`. Unknown/missing notification or
   background-task kinds fall back, but malformed known kinds do not.
4. **Schedules have separate read/write models.** Positive legacy sub-minute
   schedules remain readable; mutation schedules require >=60000 ms.
   Fixed-time single-digit hours remain accepted. Structured schedule input
   is documented, while a JSON-string compatibility arm still decodes.
5. **Toolkit success is not always the MCP transport result.**
   `preview_snapshot` and `device_screenshot` are hand-registered: screenshot
   data goes into image content, not structured metadata. Snapshot save/text
   branches and refusal envelopes are retained in `mcp-framing.ts` and tool
   transport metadata. Device errors intentionally omit remote messages.
   They must not be flattened into `OrchestratorMcpFailure`.
   The declared failure schema is also not the complete toolkit failure
   union: Effect adds AiError (including parameter-validation errors) and
   execution-denied/interrupted results. Both schemas and their tags are
   frozen. For `failureMode:"return"`, toolkit refusals use the normal
   `isError:false` framing; error-mode validation uses JSON-RPC InvalidParams.
6. **Task terminality is not delivery or thread terminality.** Original
   result, later child runs, task workState, cohort generations and mailbox
   delivery state are all separate fields/types. The contract does not grant
   engine cancellation, recursive process stop, or exactly-once consumption.
7. **Strings/offsets remain a later behavioral boundary.** Thread read uses
   JS UTF-16 code units, including possible lone-surrogate slices; queue
   truncation uses code points. Rust byte/scalar slicing is not parity.
   This contracts slice does not implement slicing or a lossless raw-JSON
   surrogate transport. P4 must address that explicitly rather than claim
   that ordinary Rust strings solve it.

## Validation scope

This slice supplies wire shapes, defaults, compatibility codecs, and frozen
validation schemas. It does not implement all Effect refinements in every
Rust field, dynamic provider/catalog checks, runtime permission ranks,
HTTP authentication, planner guards, reducers, SQL receipts/effects, or MCP
handler refusal timing. Those belong to subsequent engine/runtime slices.
Schema parity is a foundation, not a claim that the 52 tools are callable.

The schema-only oracle is executable in isolated source/dependency copies.
Full R3 C01–C33 provider/reducer/crash/UI traces still require the later
service test harness; no production API or test-only engine bootstrap is
added here. No macOS-only or UI code changed, so no visual QA is required.
