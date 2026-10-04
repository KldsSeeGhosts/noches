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
  "queue": [{
    "queuedRunId": "run:chat:2",
    "messageId": "queued-user-message",
    "text": "Continue after this turn",
    "attachments": [],
    "attachmentPaths": ["/uploads/image.png"],
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
objects; `attachmentPaths` retains Noches uploads without changing T3 schemas.
Pending questions exclude approvals; `responseType=not_resumable` remains
readable but `answerable=false`. Reads never acknowledge tasks, answer questions,
approve permissions, deliver messages, or execute on replicas.

## Merge seams and validation scope

- `QueuePullRequestLinks`: `TODO(merge-pr-watch)` replaces the metadata link
  storage adapter with that slice's stack-aware link authority.
- `QueueThreadDelivery`: retained `TODO(merge-threads)` after merging wave3.
  `ThreadService` has no existing-message strict delivery, question-answer, or
  provider detach primitive. `send` creates new ordinary message activity and
  permits restart/late-steer follow-up; substituting it would duplicate the
  promoted message or dispatch after the target run dies. Runner dispatch uses
  the durable command receipt to keep ordinary send and promotion separate.
  Provider selection-transition negotiation also belongs at that seam: cross-instance
  promotion currently refuses, rather than silently steering the old instance;
  session-restart promotion policies are not installed here.
- `QueueDomain::settle_for_host(thread, source, now)` exposes guarded `Auto`
  settlement for PR-watch/settings workers. Configured inactivity/PR sweeps are
  not installed by this slice; the PR-watch/settings integration must call it.
- Search matches finished user/assistant content, not titles (the T3 source
  implementation contradicts its tool description in exactly this way).
  Global ranking happens before project filtering; query content is never logged.
- Search's isolated UTF-16 surrogate snippet boundary uses Rust's replacement
  character; ordinary Unicode and all queue code-point truncation are covered.
  A lossless JS-string boundary codec is a remaining conformance seam.
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
