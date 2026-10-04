# Orchestration UI: delegated tasks and child threads (R3 P9a)

Branch `orch/ui`. Native gpui surfaces for T3's subagent/task UX, built on the D1 theme
roles and D3 chrome primitives. No change to `crates/engine` orchestration internals.

## Model

| Concept | Noches shape |
|---|---|
| Delegated task | A **real chat** (the child) owned by a parent chat. `DelegatedTask` in `crates/ui/src/delegation.rs`: task id, parent/child chat ids, provider (harness), model, effort, `status`, `workState`, start/finish, result/progress, `cancellable`. |
| Native vendor subagent | Unchanged and observational: derived from the parent transcript in `subagents.rs`, never stoppable, never a chat. |
| Relationship | `ThreadLink { chat_id, parent_chat_id, kind: subagent \| fork }`. Subagent links derive from tasks; fork links come from the snapshot. |
| Read model | `DelegationSnapshot { tasks, links }`, indexed on update into `AppState.delegation` (`DelegationIndex`). |

`workState` is not the run status: a completed child still waiting on nested tasks is
`waiting_for_children` and stays active (`DelegatedTask::phase`). Phases map to the one
`SubagentPhase` every surface keys on: Running (equalizer), Waiting (static ring), Started
(neutral dot), Done, Failed, Stopped (cancelled/interrupted, neutral stop mark). Hues come only
from `SessionState`; only Running pulses.

## Surfaces

- **Child banner** (`delegated_child_banner`): replaces the composer in the child's pane (dock and
  split panes alike, from `Composer::render`). Provider mark, model (medium), effort (muted),
  status glyph + workState word + self-ticking elapsed, "Runs on its own", Stop, "Open parent".
  The chat is read-only to the user: no input renders, and the engine already refuses public
  dispatch on app-owned children. A pending permission request still renders first. The child's
  own agents tray (nested delegation) stacks above the banner.
- **Agents tray**: delegated tasks join native pills with a provider mark, the shared hover card
  and a hover-revealed Stop slot (space is reserved, so nothing shifts). Fitting budgets the extras.
- **Agents panel**: *Lineage* (parent/forks, T3's rows h36), *Active · N running*, collapsible
  *Previous agents · N [· M failed]*. Groups page 6 then +12. Delegated rows show provider,
  workState + latest result, `model · effort`, elapsed, and a Stop button.
- **Sidebar**: delegated children are never ordinary rows (`visible_chats`, archived shelf, the
  projection fingerprint). They nest under their parent card (running/waiting only, any card, not
  only the open one). Nested child rows open the child chat and drag to a split through the same
  `SidebarSessionPointer` path as chat cards (`Shell::press_sidebar_session`); nothing creates a
  task or run.
- **Click** opens the child chat itself (not a right-pane tab); drag-to-split works from the
  tray-panel row and the sidebar row.
- **Stop**: `Shell::stop_delegated_task` -> `DelegationApi::cancel_task` (engine `task_cancel`).
  Acceptance marks the row *Stopping…* (and removes the Stop affordances) until a snapshot reports
  a terminal status; acceptance is never shown as terminal.

## Performance

- One update-owned `SubagentPresentation` per chat; delegated summaries are appended in
  `prepare_subagents`, so tray, panel, sidebar and banner read the same cache.
- `apply_delegation_snapshot` rebuilds only parents whose tasks/links/cancel set changed (test:
  `applying_a_snapshot_rebuilds_only_changed_parents_and_never_scans_histories`) and returns
  whether to notify, so an unchanged poll is free.
- Render only does index lookups; the panel's relationship rows are O(parent + forks).
- Elapsed labels are their own views (`elapsed_label.rs`): a live label wakes only itself at the
  instant its text changes, a settled one holds no task. No surface re-renders to tick.
- Hover uses the owner-scoped drivers (`controls::button` / `icon_button` with the surface's
  entity id); tooltips are the shared `tooltip::lines` cards, built per hover.

## Data source

The UI reads the passive ui-api (`docs/orchestration/ui-api.md`) and nothing else; it depends on
the `DelegationApi` trait:

```rust
trait DelegationApi {
    fn snapshot(&self, always: Vec<String>, cache: SnapshotCache)
        -> BoxFuture<'static, Result<(DelegationSnapshot, SnapshotCache), String>>;
    fn cancel_task(&self, parent_chat_id: String, task_id: String) -> BoxFuture<'static, Result<(), String>>;
}
```

- `EngineDelegationApi` composes the snapshot (`compose_snapshot`) from two passive RPCs:
  `ListOrchestrationThreads` gives child → parent links (`lineage.relationshipToParent` =
  `subagent` | `fork`) and per-thread publication `version`; `GetOrchestrationState {chatId}`
  gives each parent's task list (`taskId`, `childThreadId`, `status`, `workState`, `result` /
  `latestResult`, `startedAt`, `completedAt`). A parent is re-read only when its own or a child's
  version moved, or it is selected / owns live work (`SnapshotCache`).
- Stop is `CancelDelegatedTask {chatId, taskId}`: the engine's user-authority `task_cancel` under
  the parent chat (`DelegationService::cancel_for_user`). It resolves on acceptance
  (`cancel_requested`); the terminal state arrives with the next read. It never acknowledges a
  result.
- `NullDelegationApi`: before the engine connects. `FixtureDelegationApi`: a snapshot JSON from
  `NOCHES_DELEGATION_FIXTURE` for headed QA without a run.

There is no task watch stream (the ui-api asks readers to follow ordinary workspace/transcript
updates). `Shell::spawn_delegation_sync` therefore wakes on `AppState::nudge_delegation`, sent from
chat-row frames (a new child chat, a parent's publication), selection, session changes and a
delegated parent/child transcript update, debounced 150 ms. A 2 s heartbeat runs only while a
task is live (progress is read-only state); an idle app does no delegation reads.

The child chat is a regular workspace chat. Its transcript is the ordinary chat transcript
(`WatchDocMessages`); the index marks it read-only and swaps the composer for the child banner.

## Headed real-data QA

`ZERON_HARNESS=mock` plus a prompt starting `QA-DELEGATE <task>` makes the mock harness call the
engine's real `delegate_task` (async) over its scoped `t3-code` MCP server, so a genuine app-owned
child, publication and completion wake appear in the headed app. Set `ZERON_MOCK_DELAY_MS` (e.g.
`6000`) to hold the child live long enough to open it, tray, Stop and all.

## Not done

- iOS parity (R3 §5.2).
- Merge-back / detach actions and context-transfer rows of T3's relationships panel (the P5/P6
  fork lifecycle owns them); fork rows render once the snapshot supplies fork links.
- A relationships entry for chats that have lineage but no agents (no tray, hence no Agents tab
  entry point yet); the child banner's "Open parent" covers the child side.
- The tray carries Stop per pill; there is no tray-level "stop all".
