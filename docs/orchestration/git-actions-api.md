# F1 Git actions, default pull and CLI history API (v1)

Engine-only service: `orchestration::git_actions::GitActionsService`; passive read
API: `orchestration::ui_git_actions` (`Store` methods). Shared serde-defaulted
types: `zeron_proto::git_actions`. Typed client methods:
`zeron_rpc::git_actions` (on `RpcClient`). Production engines advertise
`git-actions-v1`. No gpui chrome changes are included.

All RPC names and JSON fields below are stable. `targetDeviceId` can be added to
any raw RPC request to route to the owning host; an unavailable owner never falls
back to executing on the viewer. History paths and secrets are not client inputs.
The engine chooses Claude/Codex homes from `AgentAccountsConfig` (including
`CLAUDE_CONFIG_DIR`/`CODEX_HOME`), scoped to the current engine profile.

## Git status, preview and separate authorization

| RPC | Input | Result |
| --- | --- | --- |
| `GetGitActionStatus` | `{cwd}` (known local checkout) | `GitCheckout` |
| `GetSourceControlSettings` | `{}` | `SourceControlSettings` |
| `SetSourceControlSettings` | `SourceControlSettings` | `null` |
| `PreviewGitAction` | `GitPreviewRequest` | `GitMessagePreview` |
| `StartGitAction` | `GitActionRequest` | `GitActionState` (acceptance) |
| `GetGitAction` | `{actionId}` | `GitActionState` |
| `WatchGitAction` | `{actionId}` | stream of `GitActionState` snapshots |

Settings example:

```json
{"style":"repo_conventions","customInstructions":"","providerInstanceId":"codex","model":"configured-writer-model"}
```

Styles are `repo_conventions`, `conventional_commits`, `custom`. Repository
conventions include up to 20 recent subjects and bounded root `AGENTS.md`;
Claude writers also get bounded root `CLAUDE.md`. Symlink instruction files are
not followed. The configured model runs in a scratch directory using the
restricted `Harness::run_source_control`, permission-refusing text-generation
adapter, with source-control-specific (not session-title) system instructions,
no tools, resume
or MCP context. Generation failure does not silently substitute a commit.
Commit context is the staged diff; PR context includes the explicit local base's
existing commit/diff range plus staged changes. Missing local base fails preview
generation, instead of guessing a remote or fetching without consent.
Custom configured provider instances fail explicitly until
`TODO(merge-provider-instances)` resolves their adapter; they never use another
account's legacy adapter. Supplying all three message fields bypasses generation.

```json
{
  "threadId":"thread-id","cwd":"/host/project/worktree","baseBranch":"dev",
  "commitMessage":"feat: add Git actions",
  "prTitle":"Add Git actions","prBody":"Summary and verified tests."
}
```

Preview returns `previewId`, `threadId`, `checkout`, the three approved messages,
`baseBranch`, and `settings`. `checkout` includes `checkoutId`, canonical `cwd`,
`branch`, `head`, `stagedTree`, `stagedPaths`, `dirty`, nullable `upstream`, and
`remotes:[{name,fetchUrl,pushUrl}]`. Unborn HEAD is `""`; detached HEAD, unfinished
merge/rebase/cherry-pick, unmerged index, oversized index and non-UTF8 Git paths
fail closed. Multi-push-URL remotes are not offered.

The UI must show the messages, exact staged files and checkout identity, then
send the entire returned checkout as `confirmedCheckout`:

```json
{
  "requestId":"new-user-generated-id",
  "previewId":"returned-preview-id",
  "confirmedCheckout":"REPLACE WITH THE EXACT checkout OBJECT",
  "authorizeCommit":true,"authorizePush":true,"authorizeCreatePr":true,
  "pushRemote":{"name":"origin","fetchUrl":"git@github.com:KldsSeeGhosts/noches.git","pushUrl":"git@github.com:KldsSeeGhosts/noches.git"},
  "prRepository":"https://github.com/KldsSeeGhosts/noches"
}
```

`authorizeCommit` is required. A commit never authorizes a push; a push never
authorizes PR creation. PR creation additionally requires authorized push and
an explicit canonical HTTPS repository URL among the inspected remotes. A fork
PR may deliberately target another inspected remote on the same host; the UI
must offer that as an explicit user choice. No `origin`/upstream inference.
Repeated `requestId` plus identical payload returns the original state;
different payload refuses. Never replace a lost request ID blindly.

The chain revalidates checkout/HEAD/staged tree/remote URLs before effects.
Host-wide idle admission excludes live/warm harnesses, open PTYs, file writers,
other Git actions and updates; it does not stop or retire them. An index lock
protects the real staged scope. Inspection uses a disposable index copy.
Commit uses `commit-tree` and `update-ref` with expected old HEAD; push addresses
the chosen URL and exact new SHA, never an implicitly selected remote or force.
PR creation uses `gh pr create --repo <explicit URL> --head owner:branch`.
No staging, stashing, reset, force push, branch creation or merge is performed.

**Known limitation:** commit/signing hooks are disabled by immutable-tree
plumbing. Progress explicitly says so. Hook/signing configuration parity,
line-by-line subprocess/hook output and hook cancellation are deferred.
Repos requiring `commit.gpgsign=true` refuse before commit rather than silently
creating an unsigned commit. Other hooks do not execute on this v1 path.
Host
subprocesses have 60-second deadlines and bounded stdout/stderr; diagnostics
never expose raw Git transport stderr. External noncooperating host processes
cannot be frozen; CAS/immutable refs and Git locking fail safely where possible.

Action state example:

```json
{
  "actionId":"request-id","threadId":"thread-id","status":"completed",
  "commit":"sha","prUrl":"https://github.com/KldsSeeGhosts/noches/pull/42",
  "prLinked":false,"error":null,
  "progress":[
    {"sequence":1,"phase":"commit","kind":"phase_started","text":"Committing confirmed staged tree (hooks disabled)"},
    {"sequence":7,"phase":"link","kind":"link_pending","text":"PR created; PR-watch link service not merged"}
  ]
}
```

Streams replay current durable state, then changed snapshots; phases are
`commit`, `push`, `pr`, `link`, `action`, with monotonic sequences. The retained
tail is capped at 256 events. Streams close at non-running status. Dropping a
Git stream cancels only observation, **not the mutation**. Restarted/failed
mutations become `uncertain` and never automatically replay. Inspect actual
Git/forge state before making a fresh request. `prUrl` is durable before linking.

`PullRequestLinker::link(thread_id,url)` is the `TODO(merge-pr-watch)` seam.
Assembly must attach the PR-watch user-authority adapter with
`set_pr_linker`; absent adapter yields `link_pending`, never `prLinked:true`.
Link failure leaves the created URL visible and marks the operation uncertain.

## Safe default-branch pull

| RPC | Input | Result |
| --- | --- | --- |
| `GetDefaultBranchPull` | `{spaceId}` | `PullState` |
| `SetDefaultBranchPull` | `PullPolicy` | `PullState` |
| `RetryDefaultBranchPull` | `{spaceId}` | `PullState` |

Policy example:

```json
{"spaceId":"project","enabled":true,"remote":{"name":"origin","fetchUrl":"git@github.com:KldsSeeGhosts/noches.git","pushUrl":"git@github.com:KldsSeeGhosts/noches.git"},"defaultBranch":"dev","cadenceSeconds":300}
```

Disabled by default, host/project-local and durable. Cadence clamps to
60..3600 seconds plus deterministic 0..20% project jitter; a 15-second daemon
tick checks due projects serially. Worker does not catch up repeatedly. Manual
retry respects every safety gate and does not enable the policy. Policy changes
and retries serialize on a policy lane. Main checkout only (not linked worktree).
Require verified `remote/HEAD`, matching branch/upstream and unchanged remote
URLs, no modified/staged/untracked files, zero local commits, idle admission and
checkout/index locks. Execute only `pull --ff-only --no-rebase` from the selected
fetch URL/branch, then CAS-refresh that remote-tracking ref. Never stash,
reset, merge, force, rebase or retry an uncertain mutation.

`PullState` includes `policy`, `lastCheckedAt`, `nextCheckAt` (epoch millis),
`lastSkipReason`, `lastError`, `lastResult`. Skip reasons:
`disabled`, `checkout_not_idle`, `not_main_checkout`, `checkout_locked`,
`not_default_branch`, `remote_changed`, `upstream_mismatch`,
`default_branch_unverified`, `changed_or_untracked_files`, `local_commits`.
Result is `pulled` or `skipped_up_to_date`. Command/inspection failures populate
`lastError`; no stash/reset recovery is attempted.
An error suspends automatic attempts until manual retry or policy replacement.

## Read-only CLI history, selected import, explicit continuation

| RPC | Input | Result |
| --- | --- | --- |
| `ScanCliHistory` | `{spaceId,scanId?:returned-id}` | `HistoryScanState` |
| `GetCliHistoryScan` / `WatchCliHistoryScan` | `{scanId}` | state / state stream |
| `PreviewCliHistory` | `{candidateId}` | `HistoryPreview` |
| `ImportCliHistory` | `{spaceId,candidateIds:[selected-ids]}` | `HistoryImportState` |
| `GetCliHistoryImport` / `WatchCliHistoryImport` | `{importId}` | state / state stream |
| `CancelCliHistory` | `{operationId:scanId-or-importId}` | `null` |
| `ContinueCliHistory` | `{spaceId,candidateId,chatId}` | `null` (identity binding only) |

Scan state:

```json
{"scanId":"scan-id","spaceId":"project","status":"paused","scannedFiles":256,"totalFiles":400,"bytesRead":1000000,"truncated":false,"candidateIds":["sha256-candidate"],"error":null}
```

Resume paused/cancelled scans using the same `scanId`: persisted discovery
snapshot and file cursor, no rescanning already-completed files. Restart changes
running scans/imports to `paused`. Scan cancellation preserves file cursor;
import cancellation stops between atomic histories. Resume import by selecting
remaining candidates in a new request; native-ID receipts dedupe completed ones.
Watch drop cancels observation only; use `CancelCliHistory` to cancel work.

Preview includes `candidateId`, title, selected text messages
`[{role,text,createdAt}]`, nullable `alreadyImportedChatId`, and provenance:

```json
{"source":"codex","nativeSessionId":"uuid","filePath":"/configured/codex/sessions/file.jsonl","sha256":"content-hash","projectRoot":"/canonical/project","truncated":false,"unsupportedContent":true}
```

Sources: `codex`, `claude_code`. Require exact canonical project cwd, native
session identity and at least one visible user message. Parse Claude text blocks
without sidechain/meta/compaction records. Codex canonical user events supersede
matching response copies and generated same-turn context; response-only markup
is preserved. Fork metadata takes the first native ID. No filename-resume guess.

Bounds: 5000 files, 20,000 discovery operations, directory depth 6, 8 MiB/file,
1 MiB/record, 100,000 records/file, 200 messages/2 MiB text per candidate;
256 files/64 MiB per resumable batch, 256 MiB per scan. Larger files are skipped
explicitly through `truncated`; malformed files are best-effort skips.
History I/O uses four host-wide disposable filesystem lanes with 15-second
operation deadlines; cancellation does not wait for a wedged home mount.
Timed-out reads keep their lane until exiting, preventing unbounded thread leaks.
Source symlinks are not followed. Selection/import is 1..100 candidates. Tests pass
only fixture roots and never scan real homes. This is deliberately more
conservative than T3's streaming multi-GiB scanner; huge-tool-output selective
streaming and newest-first/30-day filtering are deferred.

Import revalidates project, content hash and native identity. Durable reservation
binds native ID to the selected content snapshot across crashes; canonical chat2
SessionDoc snapshot precedes registry publication. Stable
`cli-history-<source/native hash>` chat IDs dedupe across paths/retries. Existing
conversations are never overwritten. Imported messages use canonical text parts
and deterministic IDs; omitted tools/images/truncation get a system notice.
No CLI files are changed. Provenance stays in host-local durable SQL.

Import does **not** bind `harnessSessionId`, start a provider, send a prompt or
grant execution. `ContinueCliHistory` is a separate confirmed action, requiring
UUID native identity, unchanged transcript/project, no active turn and no
non-imported activity; it binds the matching installed legacy harness and native
session identity with approval-required policy. The user's next send starts the
provider. Custom-account continuation waits for the provider-instances seam.

## Validation

Rust fixture suite `orchestration::git_actions::tests` mirrors source rules from
T3 `GitManager.ts`, `GitVcsDriverCore.ts`, `VcsStatusBroadcaster.ts`,
`AgentSessionScanner.ts` and `AgentSessionImporter.ts`, plus selected scenarios
from their tests (canonical Codex events, copied fork metadata, native-ID
dedupe, no import execution, configured style, ff-only/dirty/upstream gates).
These are structural/source-derived oracle cases, **not** a claim that live T3
or GitHub conformance tests were run. New F1 RPCs are not additional T3 MCP tools
and do not modify the frozen 72-tool catalog.

Recorded Linux validation (2026-10-04, `target-w3-git-actions`):

```sh
LINUX_TARGET=target-w3-git-actions /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh /Volumes/DevDrive/AiStack/noches-wt/w3-git-actions test -p zeron-engine -p zeron-proto -p zeron-rpc -p zeron-mcp -p zeron-harness -j 4
LINUX_TARGET=target-w3-git-actions /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh /Volumes/DevDrive/AiStack/noches-wt/w3-git-actions test -p zeron-engine -p zeron-proto -p zeron-rpc -p zeron-mcp -p zeron-harness --lib -j 4
LINUX_TARGET=target-w3-git-actions /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh /Volumes/DevDrive/AiStack/noches-wt/w3-git-actions test -p zeron-engine --lib -j 4
LINUX_TARGET=target-w3-git-actions /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh /Volumes/DevDrive/AiStack/noches-wt/w3-git-actions test -p zeron-engine --lib orchestration::git_actions -j 4
```

All passed. Final engine library suite: 474 passed, 3 ignored; harness library:
261 passed; proto library: 41 passed; RPC library: 22 passed; MCP library:
8 passed. Thirty F1 fixture tests plus relay-routing/proto/restricted-adapter
tests cover the new workflows. Formatting and `git diff --check` passed for
task files. The base has an inherited `unused_doc_comments` warning in
`sessions.rs`. CI does not configure a Clippy gate; Clippy was not run.
macOS/Windows execution, live writer/forge credentials, actual remote Git
mutation and headed visual QA were not performed. No UI files were changed.
