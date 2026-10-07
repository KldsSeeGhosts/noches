# Wave 3 queue and lifecycle read contract

`Chat` is unchanged. `zeron_proto::ChatLifecycle` lives in
`crates/proto/src/thread_lifecycle.rs`, uses camelCase JSON, and defaults every
field when loading old peers/documents.

```json
{
  "pinnedAt": null,
  "snoozedUntil": null,
  "settledAt": null,
  "settledBy": null,
  "wokeAt": null
}
```

Rust timestamps are `Option<DateTime<Utc>>`. `SettleSource` serializes as `"User"`
or `"Auto"`. Sort pinned rows by `pinned_at` ascending. A snooze's host-clock
expiry clears its snooze and records `woke_at`; acknowledgement clears only the
wake marker. Active, queued, awaiting-input, failed, or background-working
threads never expose a snoozed/settled presentation. Their source lifecycle
fields remain durable, rather than being erased by presentation filtering.
The same guard applies to the merged thread list/read settlement fields.
Ordinary shared thread sends unpark the thread; `settledBy` is absent whenever
`settledAt` is absent, including after the re-engaged run completes.

## Synced map and desktop

The owning host publishes lifecycle inside its monotonic orchestration registry
summary, as a separate map keyed by chat ID. This reuses registry sync's existing
host epoch/version protection, persistence, reconnect, and watch notifications.
It adds no fields to `Chat` or to any old `Chat { .. }` literal.

- `RegistryDoc::thread_lifecycles()` and `WorkspaceHost::thread_lifecycles()`.
- `WatchThreadLifecycles {}` → `HashMap<String, ChatLifecycle>`, initial + live.
- `AppState.thread_lifecycles: HashMap<String, ChatLifecycle>`.
- `AppState::chat_lifecycle(&self, chat_id: &str) -> Option<&ChatLifecycle>`.
- `OrganizeThread {chatId, action, snoozedUntil?}` → `{sequence}`.
- `AcknowledgeThreadWoke {chatId}` → `{sequence}`.

Actions are `pin|unpin|snooze|unsnooze|settle|unsettle|archive|unarchive|mark_unread`.
Organization validates the pinned `t3_thread_organize` schema, then dispatches
under user/owner authority. The registry resolves the target device automatically;
the commands are also explicitly `targetDeviceId` forwardable. No live agent
caller is required for desktop user commands. Snooze requires a future time and
refuses queued work or pending requests. Settle refuses active/blocked work,
cancels message-capable async questions and automatic queued deliveries, and
clears pins. Pin clears snooze and explicit settlement.

`EngineHandle::{queue_state, organize_thread, acknowledge_thread_woke}` are typed
desktop client methods. The map is subscribed alongside chats and is cleared on
engine/workspace replacement. Rendering/order remains the designer's slice;
keep the rendered order and `sidebar_visible_order` in sync.

## Queue and questions

- `Store::queue_ui_state(&ThreadId) -> Result<QueueUiState>`.
- `GetQueueState {chatId}` → `QueueUiState | null` (passive host or replica).
- `GetOrchestrationState {chatId}` includes the same value as `queueState`.
- The same passive value is published in the thread document's `uiState`.

```json
{
  "threadId": "chat",
  "version": 42,
  "activeRunId": "run:chat:1",
  "backgroundRunId": null,
  "canPromoteToSteer": false,
  "promotionMode": "interrupt_restart",
  "promotionSelection": null,
  "promotionSelectionDeferred": false,
  "promotionBlocked": null,
  "queue": [{
    "queuedRunId": "run:chat:2",
    "messageId": "queued-user-message",
    "text": "Continue after this turn",
    "attachments": [],
    "attachmentPaths": ["/uploads/image.png"],
    "context": [{"contextId": "ctx_1", "kind": "terminal", "label": "build.log",
                 "detail": "zsh L10-20"}],
    "held": true,
    "deliveryGate": null,
    "automatic": false
  }],
  "pendingQuestions": [{
    "requestId": "question",
    "questions": [{
      "id": "choice", "header": "Choice", "question": "Which option?",
      "options": [{"label": "First", "description": ""}]
    }],
    "responseType": "live",
    "answerable": true
  }],
  "lifecycle": {
    "pinnedAt": null, "snoozedUntil": null, "settledAt": null,
    "settledBy": null, "wokeAt": null
  }
}
```

Queue entries are untruncated and in delivery order (automatic completion first,
then queue position/ordinal). Hide `automatic` entries in the user queue.
`deliveryGate` retains the existing Loro `editing|reviewRequired` JSON; its
presence blocks MCP mutations. `attachments` retains canonical T3 attachment
objects; `attachmentPaths` retains Noches uploads without changing T3 schemas
(for a SQL-only row, the uploads its last edit committed). `context` is a bounded
summary of the message's retained context records (every T3 kind; unknown kinds
keep their label): never the terminal text, diff or HTML bodies.
Pending questions exclude approvals; `responseType=not_resumable` remains
readable but `answerable=false`. Reads never acknowledge tasks, answer questions,
approve permissions, deliver messages, or execute on replicas.

`backgroundRunId` is an additive, nullable hint for the latest non-queued
finished root with pending native work. Older documents default it to null.
It is distinct from `activeRunId`: an empty composer offers **Stop**, but typing
a new message still uses normal new-turn delivery. The optional Escape shortcut
shares that availability. Background-only transitions participate in the queue
watch's equality check; unrelated sequence advances still do not repaint it.

The existing user `QueueCommand {chatId, command:{kind:"interrupt"}}` routes
through owning-host canonical admission. It needs no agent credential, holds
the queue, and pins the exact run/attempt/root/process. Background settlement
preserves completed output, ends native work through that run's ordinal, and
excludes independently owned delegated tasks, persistent monitors, rolled-back
work and later runs. No background-only effect may cancel a replacement process.

### Canonical queue promotion

`promotionMode` is an additive passive hint: `active_steering`,
`interrupt_restart`, `interrupt_restart_with_handoff`, or null. Older snapshots
default it to null. It is computed from the thread's complete saved selection
(instance, model and options) against the running run's: `promotionSelection`
echoes the saved selection a restart mode would move the run to (or that an
active steer leaves waiting, with `promotionSelectionDeferred` true), and
`promotionBlocked` explains a selection change that cannot be delivered into the
running turn.
`canPromoteToSteer` remains true only for non-interrupting steering, so older
clients never advertise an interrupt as **Steer**. The native canonical row
uses **Steer** for direct delivery and **Send now** with the tooltip
“Send now (interrupt and restart)” for replacement.

`MutateQueuedRun` retains the original queued run/message IDs and stable
`clientRequestId`. Its promotion actions are:

```json
{"type":"promoteToSteer","targetRunId":"run:chat:1","expectedSelection":null}
{"type":"promoteToRestart","targetRunId":"run:chat:1","handoff":false,"expectedSelection":null}
```

An edit is text-only unless it carries an `attachments` change:

```json
{"type":"edit","text":"...","expectedText":"...",
 "attachments":{"expected":"claims=a|paths=/u/1.png","paths":["/u/2.png"],"removeIds":["a"]}}
```

`paths` is the full set of host-owned uploads after the edit (kept plus files
the client committed with `UploadCommit` on the host); `removeIds` drops claimed
(agent) attachments. `expected` is `queue_attachment_fingerprint` of the entry
the editor saw (`QueueUiEntry::attachment_fingerprint`); a changed set is
refused without applying the text either. New paths must be files this host
committed and no other queued row names; at most 8 attachments remain. Files an
edit drops, and files of a cancelled queued message, are deleted by a durable
`queued-attachment.cleanup` effect only when no queue row references them
(claims no other message references use the existing `attachment.cleanup`).
The retry identity must be pinned to the staged set so a replay recommits the
same uploads. Document-backed rows keep their edit leases.

`handoff` names `interrupt_restart_with_handoff`; `expectedSelection` is the
`promotionSelection` the client displayed. The host rechecks both, plus the
observed delivery mode, before consuming the row: a stale Steer click cannot
become an interrupting restart, and a restart cannot retarget a selection other
than the one shown. A restart that moves the run to a changed selection must
carry `expectedSelection`. The pinned MCP
`t3_queue_promote_to_steer` operation follows the current capability policy.
Direct steering never falls back to a late send or restart.

Restart cancels the selected queued execution graph, supersedes the exact
active attempt, and creates a replacement attempt/root inside the same logical
run. It retains the selected message ID, attachments, context and provenance;
other queued work is unchanged. The durable control admission pins the old
physical runtime and replacement attempt. Actual acceptance binds the new turn,
not merely session readiness. Native resume requires the accepted immediate
predecessor, matching provider/native identity, model/options and checkout.
Document-owned image paths remain transportable after removal of the Loro intent.
The document queue also checks the canonical Starting state before draining:
the brief idle-runtime gap between attempts cannot send or prematurely display
another queued message.

Selection transitions follow the adapter's negotiated policy
(`task::selection_transition`). A change the live native session absorbs
restarts now on the new selection and keeps native resume; when the session
cannot interrupt/restart the message is actively steered and the saved selection
waits for the next turn. A different provider instance restarts into a new
provider generation with a bounded handoff built from the pre-restart
projection, including this run's partial output, under a per-generation
`provider-handoff:{run}:attempt:{n}` identity. The host validates the saved
selection through the live provider catalog and freezes it into the command, so
a replay never consults a changed catalog. Teardown stays fenced to the old
exact provider/process. A same-instance change the adapter cannot absorb (for
example a model change on `pi`, `grok` or an unlisted driver) takes the same
handoff restart as a cross-instance one: a new provider-thread generation, a
per-attempt `provider-handoff:{run}:attempt:{n}` identity, and a start the
sessions engine is told is **fresh** (`NativeIntent::Fresh`), so it never
resumes the native session the engine still remembers for the chat.
The thread's saved selection is the authority. The desktop composer's
`SetChatConfig` mirrors into it on the owning host
(`selection_sync::mirror_chat_config`): one catalog-validated
`thread.model-selection.set` / `provider.switch` command, none when the
selection is unchanged, and it never interrupts or restarts a running turn. A
config the live catalog rejects is not mirrored (the chat row stays LWW and the
next admission re-validates). A config written on another device reaches the
thread at its next admission; it is not mirrored while a turn runs.
The native typed-row **Send now** still uses its existing document command/edit
lease path; this is not a claim that every typed-row interaction uses canonical
promotion. Context records are retained on the message and shown in the row as
compact monospace chips (`context`); `[label](t3-context://v1/<kind>/<id>)`
references in the text display as their labels. Steering carries host-owned
image files natively where the adapter supports image input (Codex `localImage`,
Claude image blocks); every path also stays a text reference for adapters that
do not.

## Merge seams and validation scope

- `PullRequestLinks::update_metadata`: merged PR authority replaces the metadata link
  storage adapter with that slice's stack-aware link authority.
- `QueueThreadDelivery`: retained `TODO(merge-threads)` after merging wave3.
  `ThreadService` has no existing-message strict delivery, question-answer, or
  provider detach primitive. `send` creates new ordinary message activity and
  permits restart/late-steer follow-up; substituting it would duplicate the
  promoted message or dispatch after the target run dies. Runner dispatch uses
  the durable command receipt to keep ordinary send and promotion separate.
  Provider selection-transition negotiation is installed in the promotion planner
  (see the canonical queue promotion section) and shares the same restart/control
  executor.
- `QueueDomain::settle_for_host(thread, source, now)` exposes guarded `Auto`
  settlement for PR-watch/settings workers. Configured inactivity/PR sweeps are
  not installed by this slice; the PR-watch/settings integration must call it.
- Search matches finished user/assistant content, not titles (the T3 source
  implementation contradicts its tool description in exactly this way).
  Global ranking happens before project filtering; query content is never logged.
- Search reuses the merged threads slice's lossless UTF-16 MCP boundary codec,
  including isolated surrogate snippet boundaries. Private markers never enter
  SQL, lifecycle sync, or desktop RPC; queue truncation counts code points.
- The launch slice's canonical project-deletion authority must be joined into
  search at merge; this slice filters deleted/archived threads, but has no
  canonical project table to exclude deleted projects yet.
- Title regeneration uses the existing Noches title-model settings. T3's
  project-level text-generation setting and native multimodal title-context
  adapter need reconciliation when those shared settings/adapters land.
- Archive cancels queued work and detaches the provider; its terminal cleanup
  effect remains a shared thread/terminal integration seam.

Regression tests mirror R3 C03/C08/C19/C20/C21/C32 and the pinned schemas. They are
reference-derived Rust tests, not a completed two-application trace runner.

## Validation after wave3 merge

All commands below ran locally on macOS through the updated wrapper, with no
SSH or Linux build. Prefix:

```sh
LINUX_TARGET=target-w3-queue /Volumes/DevDrive/AiStack/noches-t3-program/linux-test.sh /Volumes/DevDrive/AiStack/noches-wt/w3-queue
```

- `test -p zeron-engine --lib`: 512 passed, 0 failed, 2 ignored at the merge
  boundary. Final `test -p zeron-engine --lib orchestration::`: 163 passed,
  0 failed, 1 ignored, including the subsequent read/codec/dispatch regressions.
- `test -p zeron-proto -p zeron-doc -p zeron-rpc -p zeron-mcp`: all suites
  passed, including the 14-test pinned schema oracle; 2 live RPC tests ignored.
- `test -p zeron-engine --test message_queue --test orchestration_bootstrap
  --test orchestration_mcp --test queue_lifecycle_rpc --test queued_attachments
  --test acp_lifecycle`: 31 passed, 0 failed.
- `test -p zeron-ui --lib`: 1464 passed, 0 failed, 1 native-font test ignored.
- Touched-file `rustfmt --check` and `git diff --check`: passed.

No full two-app trace runner, authenticated provider matrix, or headed visual
QA was performed. Full provider-transition parity and the documented shared
integration seams remain deferred, rather than silently using legacy send-now.
