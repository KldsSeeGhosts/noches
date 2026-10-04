# Wave-3 launch read API

Engine owner routes these methods; `targetDeviceId` is supported. Nothing in
the read API acknowledges a task, sends a message or starts a provider.
Typed clients are `RpcClient::{launch_projects,launch_state,control_worktree_setup}`.
Types live in `zeron_proto::launch`; new fields are serde-defaulted.

Desktop handoff uses `HandoffThreadWorktree {chatId, targetDeviceId, input}`,
where `input` is the generated `T3WorktreeHandoffInput`.
`RpcClient::handoff_thread_worktree` enters the same handoff workflow under
owner-user authority: checkout locks, binding CAS, setup gate, continuation,
and provider detach are shared with MCP. Blank base refs and continuation
prompts are omitted, not submitted as null or as an empty continuation.

- `ListLaunchProjects {}` → `LaunchProjects { projects: Project[] }`.
  Projects retain the exact T3 nested settings/scripts shape. Existing owned
  Noches spaces are adopted with their original space id, so navigation does
  not need a parallel project registry.
- `GetLaunchState {chatId}` → `LaunchUiState | null`.
  Includes `threadId`, `projectId`, `status`, `stage`, `branch`,
  `worktreePath`, `projectWorkspaceRoot`, `error`, `setup`.
  A root binding has `worktreePath:null`; never substitute the caller's cwd.
  Status is `preparing | ready | blocked | failed | cancelled`.
  Stages are `fetch | checkout | bind | setup-script | agent | failed`.
- `ControlWorktreeSetup {chatId,runId,action}` → `{accepted:true}`.
  Actions: `cancel`, `retry`, `continue`. The exact setup run id is a CAS
  guard; a late completion cannot overwrite a newer run or explicit continue.
  This is an owner-user action, not an agent MCP preference.

```json
{
  "threadId":"command:mcp:session:uuid",
  "projectId":"space-id",
  "status":"blocked",
  "stage":"setup-script",
  "branch":"feature/new",
  "worktreePath":"/worktrees/repo/feature-new",
  "projectWorkspaceRoot":"/repos/repo",
  "error":null,
  "setup":{
    "runId":"setup-uuid",
    "terminalId":"terminal-uuid",
    "status":"failed",
    "exitCode":1,
    "blocking":true,
    "timeoutMs":300000,
    "scriptName":"Setup",
    "detail":null
  }
}
```

Saved project scripts opt into blocking with `async:false`; omitted/true stays
nonblocking. Execution uses the existing host-local trusted project Actions
and PTY service. A repository config read never grants execution trust.
UI should show the setup terminal and offer retry/continue-on-failure. Cancel
and timeout close only that setup terminal. Restart does not silently re-run
an uncertain setup process; it requires retry or continue.

`LaunchThreadIntake` now takes the host-authorized `ThreadSendRequest`; its
production adapter calls `ThreadService::send_to_thread`, including native-child
refusal, auto steering, attachments and sender provenance. Attachment permission
guards run before upload claims, and uncertain accepted sends retain their files.
`HostThreadLaunchRequest` shares launch preparation with MCP launch for scheduled
runs, carrying stable command/message IDs, schedule ID and actor/source.
`LaunchOperation` is a single kernel-planner routing arm.
Launch tables and workflows are independent of other slices' migrations.
