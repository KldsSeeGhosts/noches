

## T3 Code orchestration

The `t3-code` MCP server provides app-owned orchestration. Treat these concepts distinctly:

- A delegated task/subagent is child work owned by the current thread. Use `orchestrator_capabilities` to discover the current provider/model IDs from the same live catalog as the composer, including configured custom models. Do not treat a native tool's model list as the full list of available subagent models. Prefer native subagent tools for same-provider work only when they support the chosen model. Use `delegate_task` with that provider instance and model when native tools cannot, including for same-provider work. Also use `delegate_task` for cross-provider or explicitly T3-owned child tasks. Retain each returned `taskId`, and use `task_status` or `task_cancel` to manage it. The returned `childThreadId` is backing storage for the subagent, not the target for starting another delegated review round.
- `t3_thread_launch` and `create_threads` create ordinary top-level T3 conversations. Use them only when the user explicitly asks for separate/new/top-level threads or conversations. Never use them merely because the user said "subagent" or requested parallel delegated work.
- For every T3 delegated review round, call `delegate_task` again. Include the original brief, prior findings, responses, and unresolved objections in each new task prompt. Track each round by its own `taskId`. Use a distinct `clientRequestId` per round, stable across retries of that round. Do not use `t3_thread_send` on `childThreadId` to continue a delegated review.
- `schedule_task` creates persistent recurring work in the app scheduler. Pass `schedule` as a structured object, never as JSON text: `{"type":"interval","everyMs":3600000}` for an interval, or `{"type":"fixed_time","timeOfDay":"09:00","weekdays":[1,2,3,4,5]}` for a wall-clock schedule. By default runs return to the current thread; set `bindToCurrentThread=false` only when the user wants a fresh thread for every run. After scheduling, report the returned cadence and next run time.

### Choose the workspace before starting a new thread

For independent implementation or a PR stack in its own worktree, use `t3_thread_launch` with an explicit `workspaceStrategy`. It creates or selects the workspace, binds the new thread to it, and prepares it before the agent starts. Put the task in `message`, not `prompt`:

- New worktree: `{"title":"UI cleanup","workspaceStrategy":{"type":"worktree","baseRef":"feature/base","branch":"feature/ui-cleanup","startFromOrigin":false},"message":"Implement the cleanup and open a PR against feature/base."}`
- Existing worktree: `{"title":"Continue cleanup","workspaceStrategy":{"type":"existing_worktree","worktreePath":"/absolute/path/to/worktree","branch":"feature/ui-cleanup"},"message":"Continue the cleanup."}`
- Project's main checkout: `workspaceStrategy:{"type":"root"}`. Omitting workspaceStrategy also selects root; it does not inherit the caller's worktree.

For stacked work, set `baseRef` to the intended parent branch and `startFromOrigin:false` to use its local commits. Use `startFromOrigin:true` when you intend to fetch and start from origin. Uncommitted edits are not copied. Use `t3_worktree_list` to discover existing checkout paths. Project, model selection, and modes inherit unless supplied; launch requires a full-access/default caller.

`t3_thread_launch` is the single-thread launch tool. Use `create_threads` only for a batch of threads intentionally sharing the caller's checkout: it always inherits the caller's project, branch, and worktree and has no workspace override. Asking an agent to run `git worktree add` or `cd` in its prompt does not update T3's thread binding. Select the workspace in the launch call instead. `t3_worktree_handoff` moves the calling thread, not another thread, and cannot move a thread already attached to a worktree.

`t3_thread_launch` has no idempotency key. Retain its returned threadId and inspect it with `t3_thread_read` / `t3_thread_wait`; preparation can still be running after acceptance. If a launch fails or its response is lost, inspect `t3_thread_list` before retrying, since a thread may already exist.

Tool names may include a harness-normalized MCP prefix, such as `mcp__t3_code__delegate_task`; the semantics are the same. Some harnesses attach optional MCP servers lazily: if an initial tool-catalog scan does not show T3 tools, do not conclude that cross-provider delegation is unavailable. Make one bounded direct attempt using the known T3 tool name on the next tool step. In Codex code mode, for example, call `tools.mcp__t3_code__orchestrator_capabilities({})` before reporting that the capability is absent. Keep polling/wait loops bounded, do not duplicate active work, and use stable `clientRequestId` values when retrying tools that accept them.

ACP fallback: some ACP agents accept the injected MCP server but fail to expose its tools. When the T3 tools are absent and `T3_ACP_MCP_NODE` is present, call the same tools through the terminal: `ELECTRON_RUN_AS_NODE=1 "$T3_ACP_MCP_NODE" ${T3_ACP_MCP_ENTRYPOINT:+"$T3_ACP_MCP_ENTRYPOINT"} acp-mcp-call orchestrator_capabilities '{}'` (`T3_ACP_MCP_ENTRYPOINT` is unset when T3 runs as a standalone executable). Delegate with `acp-mcp-call delegate_task '{"task":"...","target":{"providerInstanceId":"...","model":"..."},"mode":"async","clientRequestId":"..."}'`. This is the supported T3 transport fallback, not an ordinary shell-based substitute for delegation.
