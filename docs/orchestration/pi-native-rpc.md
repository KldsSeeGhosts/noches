# Native Pi (`pi --mode rpc`)

Noches drives Pi the way Pi's own RPC client and T3 Code's `PiAdapterV2` do:
`crates/harness/src/pi/` speaks Pi's LF-delimited JSON records directly. The
community `pi-acp` adapter (and the Noches wrapper, usage-file extension and ACP
steering workarounds around it) is retired. Pi 0.80.5 or newer is required
(`pi --version` is checked once per binary; older builds get an actionable
error).

Everything below the "Observed on Pi 1.0.4" heading was recorded from a live
run (`cpa/gemini-3.8-flash`, scratch cwd, temp session dir). The fake used by
the test suites (`crates/harness/tests/fixtures/fake-pi.py`) reproduces these
shapes.

## What Noches does with it

| Concern | Behaviour |
|---|---|
| Launch | `pi --mode rpc [--session <file>] <user launchArgs> -e noches-cua.ts -e noches-policy.ts [-e noches-mcp.ts]`, no `--no-*` flags: the user's extensions, skills, templates, AGENTS.md, settings and auth load as in the TUI. Own process group; stale `NOCHES_*` bindings are removed before this run's are set. |
| Native thread id | The **session file path** (`get_state.sessionFile`). `--session <path>` resumes it, so a Noches thread resumes in the Pi TUI and vice versa. Legacy ACP ids (Pi's session UUID) resolve through the session store; a vanished file starts a fresh session instead of being handed to `--session`, which would silently create a new session at that path. |
| Models | `get_available_models` -> exact `provider/id` rows after a leading `default` row ("Pi default" defers to settings). Ladders come from `thinkingLevelMap` (nulls removed, `xhigh`/`max` only when named). Selection at run start is `set_model` + `set_thinking_level` (clamped like pi-ai); an unknown model fails the run with Pi's own text. `selection_transition("pi")` is `ApplyOnNextTurn`. |
| Turn | `prompt` (with an id) -> events -> `agent_settled` -> an idle probe (`get_state`: not streaming/compacting, no pending messages) -> `Done`. `agent_end` alone never ends a turn (retries, compaction recovery and queued steers can follow). A `handled` prompt (extension command) starts no run, so it is settled by the same probe. |
| Receipts | Command responses are re-injected into the event stream *in Pi's own order* (`PiRpc::submit_ordered`), so a steer's `disposition` is seen relative to the user message it starts. `InputAcceptedFor{message_id}` comes only from `prompt` -> `queued`/`started`/`handled`; the `Steered` boundary is the later `message_start` of the injected user message. `confirms_steered_inputs()` is therefore honest. |
| Steer | `prompt` with `streamingBehavior: "steer"` (atomic: queues during a run, starts a run if the turn settled first; a bare `steer` while idle would queue forever). `/compact` as a steer is refused while working (compact aborts the agent) and runs as its own turn when idle. |
| Interrupt | `clear_queue` then `abort` (Pi's abort continues queued messages otherwise), escalating to SIGTERM/SIGKILL of the process group. Compaction is not cancelled by `abort` (as T3 reports; not re-verified here), so interrupting it terminates immediately. Pi's errored "operation was aborted" message is the requested outcome, not a failure. |
| Compact | `/compact [instructions]` maps to the `compact` command (Pi's RPC `get_commands` omits TUI built-ins, so the composer list is prefixed with it). |
| Fork | `--fork <source file>` in the destination cwd (RPC `fork`/`switch_session` would keep the source's cwd), then `fork {entryId}` before the *next* turn's user entry. Turn refs are the session-tree user entries (`NativeReference.turn_id`, from `get_entries since`). `can_fork_now` proves the boundary (a user entry of this file) before anything runs; head forks are the CLI copy alone. Pi names the CLI's copy the child's `parentSession`, so the copy stays (no dangling lineage). |
| Context | `get_session_stats` at every settle and after compaction: `ContextUsageSnapshot` with `compact_at = contextWindow - reserveTokens` read from Pi's settings (per-model overrides included). `tokens: null` after compaction is a real "waiting" state. `message_end.usage.totalTokens` drives the live meter. |
| Policy | `noches-policy.ts` hooks Pi's blocking `tool_call` event: Supervised confirms every non-read-only tool, Auto-accept waves through `edit`/`write`, host pre-approved MCP tools skip the prompt. Confirmations carry `{noches:"permission", tool, input}` and go through the host `PermissionGate` (exact per-input session grants). Auto and Plan are refused (no classifier / plan gate). |
| MCP | The existing `noches-mcp.ts` bridge registers `mcp__<server>__<tool>` and appends the orchestration instructions to Pi's system prompt in `before_agent_start` (never by wrapping the user text, which would stop `/commands` expanding). |
| Dialogs | Extension `select`/`input`/`editor`/`confirm` become content questions answered back as `extension_ui_response`; dialogs raised while Pi starts are answered while setup waits. Decoration (`setStatus`, `setWidget`, ...) is ignored. |

Not ported from T3 (deliberately): per-block turn items and subagent child
items (Noches renders text/reasoning/tool chips; the `subagent` extension tool is
a typed spawn chip), `get_messages`-based thread snapshots and conversation
rollback (Noches has no provider rollback), and one-shot text generation through
Pi (`supports_titles` stays Codex/Claude only).

## Observed on Pi 1.0.4

Start-up: a user's extensions emit `extension_ui_request` records
(`setStatus`, `setWidget`) before the first command; closing stdin exits Pi.

```jsonc
// get_state (abridged)
{"id":"a1","type":"response","command":"get_state","success":true,"data":{
  "model":{"id":"gemini-3.8-flash","provider":"cpa","reasoning":true,"contextWindow":1048576,"maxTokens":65535,
           "thinkingLevelMap":{"off":null,"minimal":null,"low":"low","medium":"medium","high":"high","xhigh":null,"max":null}},
  "thinkingLevel":"medium","isStreaming":false,"isCompacting":false,"steeringMode":"all","followUpMode":"one-at-a-time",
  "sessionFile":"/tmp/s/2026-10-07T13-39-59-438Z_01a11697-....jsonl","sessionId":"01a11697-...",
  "autoCompactionEnabled":true,"messageCount":0,"pendingMessageCount":0}}
// the session file path is reported before anything is written; an empty session is not persisted.
```

```jsonc
// prompt (idle): the response carries the id and a disposition
{"id":"p2","type":"response","command":"prompt","success":true,"data":{"disposition":"started"}}
// then: agent_start, turn_start, message_start/end (user), message_start (assistant),
//   message_update{assistantMessageEvent:{type:text_start|text_delta|text_end|thinking_*|toolcall_*,contentIndex,delta|content}},
//   message_end (assistant; usage.totalTokens only here for this provider), turn_end, agent_end, agent_settled
// prompt during a run, no streamingBehavior:
{"id":"s2","type":"response","command":"prompt","success":false,"error":"Agent is already processing. Specify streamingBehavior ('steer' or 'followUp') to queue the message."}
// prompt with streamingBehavior:"steer" during a tool:
{"type":"queue_update","steering":["Also, ..."],"followUp":[]}
{"id":"s1","type":"response","command":"prompt","success":true,"data":{"disposition":"queued"}}
// ... tool_execution_end, message_end(toolResult), turn_end, turn_start, queue_update{steering:[]},
//     message_start{role:"user",content:[{text:"Also, ..."}]}  <- the delivery boundary, after the tool turn
// the same prompt while idle: {"disposition":"started"} (atomic race handling)
// bare `steer` while idle: {"disposition":"queued"} and it stays queued (never use it for the race)
```

```jsonc
// tools (ids contain "|"): bash {command}, write {path,content}, edit {path,edits:[{oldText,newText}]}, read {path}, grep {pattern,path}
{"type":"tool_execution_start","toolCallId":"call_..|fc_..","toolName":"edit","args":{"path":"hello.txt","edits":[{"newText":"bye","oldText":"hi"}]}}
{"type":"tool_execution_end","toolCallId":"...","toolName":"edit","result":{"content":[{"type":"text","text":"Successfully replaced 1 block(s) in hello.txt."}],"details":{"diff":"-1 hi there\n+1 bye there","patch":"--- hello.txt\n+++ ..."}},"isError":false}
// abort mid-tool: tool_execution_end isError:true "Command aborted"; assistant message_end
//   {stopReason:"error",errorMessage:"This operation was aborted"}; turn_end; agent_settled; then {"command":"abort","success":true}
```

```jsonc
// auto-retry on a provider failure (observed when the CPA tunnel was down)
{"type":"agent_end","messages":[...],"willRetry":true}
{"type":"auto_retry_start","attempt":1,"maxAttempts":3,"delayMs":4000,"errorMessage":"Connection error."}
// ... another agent_start per attempt ...
{"type":"auto_retry_end","success":false,"attempt":3,"finalError":"Connection error."}
{"type":"agent_settled"}
```

```jsonc
// sessions
{"type":"get_entries"} -> {"entries":[{"type":"message","id":"451f4b51","parentId":"...","message":{"role":"user",...}},...],"leafId":"..."}
{"type":"get_entries","since":"<id>"} -> only entries after it; an unknown id is success:false
{"type":"fork","entryId":"<user entry>"} -> {"text":"<that prompt>","cancelled":false}   // new file; active branch ends BEFORE the entry
{"type":"fork","entryId":"<non-user or unknown>"} -> success:false "Invalid entry ID for forking"
{"type":"switch_session","sessionPath":"..."} -> {"cancelled":false}   // a missing path is an ENOENT error
{"type":"set_model","provider":"cpa","modelId":"does-not-exist"} -> success:false "Model not found: cpa/does-not-exist"
{"type":"set_thinking_level","level":"xhigh"}  // clamped: emits {"type":"thinking_level_changed","level":"high"}
{"type":"compact"} on a small session -> compaction_start/compaction_end{errorMessage} then success:false "Nothing to compact (session too small)"
```

CLI facts that shaped the driver:

- `--session <path>` resumes any session file, from any cwd; `--session <missing path>` silently creates a **new**
  session at that path. `--session <uuid>` from another project prompts interactively (`Fork this session into
  current directory? [y/N]`), which is why legacy ids are resolved to files first.
- `--fork <file>` writes the copy immediately and records the destination cwd in the header; RPC `fork` after
  `--session` keeps the *source's* cwd. A `fork` after `--fork` writes a second file whose `parentSession` is the
  first.
- `PI_CODING_AGENT_DIR` and `PI_CODING_AGENT_SESSION_DIR` relocate configuration and sessions; instance
  environments set them for private provider instances.

## Behaviours added after review

- **Baseline leaf and turn refs never list a whole session.** `get_entries` without `since` serialises every entry
  (hundreds of MB on a long chat) and can exceed the 8 MiB frame cap, so the baseline leaf is read from the tail of
  the session file (the last complete non-header entry, which is the leaf Pi itself loads), and each turn's ref comes
  from `get_entries since <leaf>`. A cursor Pi no longer recognises is resynced from the file; that turn simply has
  no ref.
- **A vanished resume target** is announced ("The earlier Pi session is no longer available on this device ...") and,
  when the engine plans the run, `SessionLifecycle::can_resume_now` makes the planner rebuild the conversation from
  portable context instead of starting a blank session. Resume and fork paths must sit under one of Pi's session
  roots; legacy pi-acp ids resolve through `~/.pi/pi-acp/session-map.json` first, then a store walk off the async
  workers. A fork with no destination cwd is refused (`--fork` would re-home the copy to the app's own cwd).
- **Terminal commands.** Pi's RPC `prompt` hands `/foo` to the model unless it is an extension command, skill or
  prompt template. `/compact` maps to `compact`; `/name`, `/session`, `/autocompact`, `/steering`, `/follow-up`
  and `/export` map to `set_session_name`, `get_session_stats`, `set_auto_compaction`, `set_steering_mode`,
  `set_follow_up_mode` and `export_html` (the mode and compaction toggles persist in Pi's settings, as they did under
  pi-acp), and are advertised in the composer. The other TUI built-ins (`/model`, `/login`, `/tree`, `/changelog`,
  ...) are refused with a message instead of being sent to the model. Anything else (paths, unknown names, commands
  Pi itself lists) goes to Pi untouched, as in T3. A mapped command sent mid-turn is refused, not queued.
- **Auto-accept edits** confirms any `edit`/`write` whose resolved path (after `~`, `@`, `file://` and symlink
  resolution) leaves the working directory or touches Pi's agent dir, `~/.pi` or a project's `.pi` directory (Pi
  loads extensions from those). The policy mode fails closed: a missing or unknown mode means approval-required. The
  `subagent` extension tool is gated as one call, and the child Pi processes it spawns do not load this policy.
- **Probes and visibility.** The settle probe, usage snapshot and turn-ref lookup give way to a Stop; a failed
  discovery is remembered for 20 s; the version gate re-checks a replaced executable and reads the last line of
  stdout, else stderr; `auto_retry_start` and a failed automatic compaction surface as errors without ending the turn.

## Verification

- `cargo test -p zeron-harness --test pi` (fake `pi`), `--lib pi::`, `--test pi_policy_extension`
  (node), `--test pi_mcp_extension` (node), engine `pi_native_tests` (3, real runner/store/transfer) and
  `pi_resume`.
- Live (`NOCHES_PI_LIVE=1 cargo test -p zeron-harness --test pi_live -- --ignored --test-threads=1`, model
  `cpa/gemini-3.8-flash`): turn + resume-by-file with real recall, steer receipt ordering, interrupt mid-tool,
  native fork (re-homed cwd, the fork answers with the pre-cut fact only), and the Supervised gate against the real
  blocking hook (a declined `bash` never runs).
- Not verified: Windows (the old `.cmd` wrapper is gone; `pi.cmd` spawning relies on the shared Windows process
  layer), Pi versions between 0.80.5 and 1.0.0, real (non-trivial) compaction, remote devices.
