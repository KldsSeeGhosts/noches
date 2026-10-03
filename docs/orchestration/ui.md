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

## Data source and the O-G seam

`docs/orchestration/ui-api.md` (sibling agent O-G, `orch/wave2`) did not exist when this was
written. The UI therefore depends only on the `DelegationApi` trait:

```rust
trait DelegationApi {
    fn snapshot(&self) -> BoxFuture<'static, Result<DelegationSnapshot, String>>;
    fn cancel_task(&self, task_id: String) -> BoxFuture<'static, Result<(), String>>;
}
```

- `NullDelegationApi`: default. Native subagents only.
- `EngineDelegationApi`: calls placeholder RPC names (`OrchestrationUiSnapshot`,
  `OrchestrationTaskCancel`) marked `TODO(O-G)`. Until the engine serves them the sync loop
  backs off to 60 s and the UI behaves like Null.
- `FixtureDelegationApi`: reads a snapshot JSON from `NOCHES_DELEGATION_FIXTURE` for headed QA
  (`/Volumes/DevDrive/AiStack/noches-t3-program/qa-orch-ui.sh`).

The merge is a small swap: point `EngineDelegationApi` at the real read/watch RPC and `task_cancel`,
replace the poll in `Shell::spawn_delegation_sync` with the watch stream, and map the real
projection into `DelegationSnapshot`. `DelegatedStatus`/`DelegatedWorkState` already convert from
`OrchestratorMcpDelegatedTaskStatus` / `…WorkState`. Nothing above the trait changes.

## Not done

- iOS parity (R3 §5.2).
- Merge-back / detach actions and context-transfer rows of T3's relationships panel (the P5/P6
  fork lifecycle owns them); fork rows render once the snapshot supplies fork links.
- A relationships entry for chats that have lineage but no agents (no tray, hence no Agents tab
  entry point yet); the child banner's "Open parent" covers the child side.
- The tray carries Stop per pill; there is no tray-level "stop all".
