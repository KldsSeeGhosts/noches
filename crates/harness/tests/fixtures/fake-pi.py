#!/usr/bin/env python3
"""A fake `pi --mode rpc` for harness and engine tests.

Record shapes follow a live Pi 1.0.4 (see docs/orchestration/pi-native-rpc.md):
every command is answered with its own `id`; a prompt answers
`{disposition: started|queued|handled}`; `agent_settled` closes a run; session
files are JSONL entry trees. Behaviour is selected by the prompt text:

  hello / anything   -> text reply "reply:<prompt>"
  tool               -> a bash tool call, then the reply
  slow               -> a long-running tool that ends on abort or when a steer
                        is queued (steers land at the step boundary after it)
  confirm            -> a runtime-policy confirmation dialog; replies with the answer
  select / input     -> extension dialogs
  error              -> assistant message with stopReason "error"
  retry-fail         -> auto-retry events ending in a final failure
  crash              -> exits 23 mid-turn
  /handled           -> disposition "handled": no run starts
  /compact           -> the compact RPC (via the harness)

Every command, the argv and selected environment variables are appended to
$FAKE_PI_LOG (one JSON object per line) so tests can assert exactly what the
harness sent. Sessions live under $FAKE_PI_SESSIONS.
"""
import json
import os
import select
import sys
import time
import uuid

LOG = os.environ.get("FAKE_PI_LOG")
SESSIONS = os.environ.get("FAKE_PI_SESSIONS") or os.path.join(os.getcwd(), ".fake-pi-sessions")
VERSION = os.environ.get("FAKE_PI_VERSION", "1.0.4")


def log(kind, **fields):
    if LOG:
        with open(LOG, "a") as handle:
            handle.write(json.dumps({"kind": kind, **fields}) + "\n")


def emit(record):
    sys.stdout.write(json.dumps(record) + "\n")
    sys.stdout.flush()


argv = sys.argv[1:]
if "--version" in argv or "-v" in argv:
    version = VERSION
    if os.environ.get("FAKE_PI_VERSION_FILE"):
        version = open(os.environ["FAKE_PI_VERSION_FILE"]).read().strip()
    if os.environ.get("FAKE_PI_VERSION_BANNER"):
        print("node v20.1.0 shim")  # a wrapper's banner before the version
    # Older Pi printed the version on stderr only.
    print(version, file=sys.stderr if os.environ.get("FAKE_PI_VERSION_STDERR") else sys.stdout)
    sys.exit(0)

args = {"extensions": []}
i = 0
while i < len(argv):
    a = argv[i]
    if a in ("-e", "--extension"):
        args["extensions"].append(argv[i + 1])
        i += 1
    elif a in ("--session", "--fork", "--session-dir", "--model", "--thinking", "--mode", "--provider"):
        args[a.lstrip("-")] = argv[i + 1]
        i += 1
    else:
        args[a.lstrip("-")] = True
    i += 1
log(
    "start",
    argv=argv,
    cwd=os.getcwd(),
    env={k: os.environ.get(k) for k in (
        "NOCHES_PI_RUNTIME_MODE", "NOCHES_SESSION_MCP_ENTRIES", "NOCHES_SESSION_MCP_INSTRUCTIONS",
        "NOCHES_SESSION_MCP_ALLOWED_TOOLS", "NOCHES_ACP_MCP_EXECUTABLE", "NOCHES_CUA_SOCKET",
        "PI_CODING_AGENT_DIR", "FAKE_PI_SENTINEL")},
    extensions_exist=[os.path.isfile(p) for p in args["extensions"]],
)

if args.get("mode") != "rpc":
    print("fake pi: only --mode rpc", file=sys.stderr)
    sys.exit(2)
if os.environ.get("FAKE_PI_STARTUP_EXIT"):
    print("pi fatal: startup failed (fixture)", file=sys.stderr)
    sys.exit(int(os.environ["FAKE_PI_STARTUP_EXIT"]))

MODELS = json.loads(os.environ.get("FAKE_PI_MODELS") or json.dumps([
    {"provider": "cpa", "id": "gemini-3.8-flash", "name": "Gemini 3.8 Flash",
     "reasoning": True, "contextWindow": 1048576,
     "thinkingLevelMap": {"off": None, "minimal": None, "low": "low", "medium": "medium",
                          "high": "high", "xhigh": None, "max": None}},
    {"provider": "cpa", "id": "devin/swe-2", "name": "SWE 2", "reasoning": False, "contextWindow": 262144},
]))


def model_obj(slug):
    for model in MODELS:
        if f"{model['provider']}/{model['id']}" == slug:
            return model
    return None


def default_model():
    settings = os.path.join(os.environ.get("PI_CODING_AGENT_DIR", ""), "settings.json")
    try:
        data = json.load(open(settings))
        slug = f"{data['defaultProvider']}/{data['defaultModel']}"
        if model_obj(slug):
            return slug
    except Exception:
        pass
    return f"{MODELS[0]['provider']}/{MODELS[0]['id']}" if MODELS else ""


state = {
    "model": args.get("model") or default_model(),
    "thinking": args.get("thinking") or "medium",
    "name": None,
    "steering": "all",
    "follow_up": "one-at-a-time",
    "auto_compaction": True,
}
entries = []  # session tree, append order
header = None
session_file = None
counter = [0]


def new_id():
    counter[0] += 1
    return f"e{counter[0]:03d}"


def now_iso():
    return time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime())


def new_session_path(parent=None):
    os.makedirs(SESSIONS, exist_ok=True)
    sid = str(uuid.uuid4())
    stamp = time.strftime("%Y-%m-%dT%H-%M-%S", time.gmtime()) + f"-{counter[0]:03d}Z"
    return os.path.join(SESSIONS, f"{stamp}_{sid}.jsonl"), sid


def load(path):
    global header, entries, session_file
    session_file = path
    entries = []
    header = None
    with open(path) as handle:
        for line in handle:
            record = json.loads(line)
            if record.get("type") == "session":
                header = record
            else:
                entries.append(record)
    for entry in entries:
        try:
            counter[0] = max(counter[0], int(entry["id"][1:]))
        except Exception:
            pass


def persist():
    if session_file is None:
        return
    os.makedirs(os.path.dirname(session_file), exist_ok=True)
    with open(session_file, "w") as handle:
        handle.write(json.dumps(header) + "\n")
        for entry in entries:
            handle.write(json.dumps(entry) + "\n")


def start_fresh(parent=None):
    global header, entries, session_file
    path, sid = new_session_path()
    session_file = path
    entries = []
    header = {"type": "session", "version": 3, "id": sid, "timestamp": now_iso(), "cwd": os.getcwd()}
    if parent:
        header["parentSession"] = parent
    return path


def leaf():
    return entries[-1]["id"] if entries else None


def append(entry_type, **fields):
    entry = {"type": entry_type, "id": new_id(), "parentId": leaf(), "timestamp": now_iso(), **fields}
    entries.append(entry)
    return entry


if args.get("session"):
    path = args["session"]
    if os.path.isfile(path):
        load(path)
    else:  # real Pi silently creates a NEW session at a missing path
        session_file = path
        header = {"type": "session", "version": 3, "id": str(uuid.uuid4()),
                  "timestamp": now_iso(), "cwd": os.getcwd()}
elif args.get("fork"):
    source = args["fork"]
    load(source)
    parent = source
    path, sid = new_session_path()
    header = {**header, "id": sid, "cwd": os.getcwd(), "parentSession": parent}
    session_file = path
    persist()  # `--fork` writes the copy immediately
else:
    start_fresh()
log("session", file=session_file)

buffer = b""
run = None  # active run generator state
steer_queue = []
aborted = [False]
pending_dialogs = {}
dialog_answers = {}
exit_when_idle = [False]


def text_of(message):
    content = message.get("content")
    if isinstance(content, list):
        return "".join(b.get("text", "") for b in content if b.get("type") == "text")
    return content or ""


def usage(total):
    return {"input": total, "output": 1, "cacheRead": 0, "cacheWrite": 0, "totalTokens": total + 1,
            "cost": {"total": 0}}


def assistant_message(text, stop="stop", error=None, total=11259):
    message = {"role": "assistant", "content": [{"type": "text", "text": text}] if text else [],
               "provider": state["model"].split("/")[0], "model": state["model"].split("/", 1)[1],
               "usage": usage(total), "stopReason": stop, "timestamp": int(time.time() * 1000)}
    if error:
        message["errorMessage"] = error
    return message


def user_message(text):
    return {"role": "user", "content": [{"type": "text", "text": text}], "timestamp": int(time.time() * 1000)}


def user_events(text):
    message = user_message(text)
    append("message", message=message)
    persist()
    return [{"type": "message_start", "message": message}, {"type": "message_end", "message": message}]


def assistant_events(text, stop="stop", error=None, total=11259, stream=True):
    out = [{"type": "message_start", "message": {"role": "assistant", "content": [], "stopReason": "pending"}}]
    if text and stream:
        out.append({"type": "message_update", "assistantMessageEvent": {"type": "text_start", "contentIndex": 0}})
        half = max(1, len(text) // 2)
        for part in (text[:half], text[half:]):
            if part:
                out.append({"type": "message_update", "usage": usage(0),
                            "assistantMessageEvent": {"type": "text_delta", "contentIndex": 0, "delta": part}})
        out.append({"type": "message_update", "assistantMessageEvent": {"type": "text_end", "contentIndex": 0, "content": text}})
    elif text:
        out.append({"type": "message_update", "assistantMessageEvent": {"type": "text_start", "contentIndex": 0}})
        out.append({"type": "message_update", "assistantMessageEvent": {"type": "text_end", "contentIndex": 0, "content": text}})
    message = assistant_message(text, stop, error, total)
    append("message", message=message)
    persist()
    out.append({"type": "message_end", "message": message})
    return out


def tool_events(call_id, name, tool_args, result_text, is_error=False, details=None):
    return [
        {"type": "tool_execution_start", "toolCallId": call_id, "toolName": name, "args": tool_args},
        {"type": "tool_execution_end", "toolCallId": call_id, "toolName": name,
         "result": {"content": [{"type": "text", "text": result_text}], "details": details or {}},
         "isError": is_error},
    ]


def deliver_steers():
    """Pi injects queued steers after a turn's tool calls, before the next model call."""
    out = []
    while steer_queue:
        text = steer_queue.pop(0)
        emit({"type": "queue_update", "steering": list(steer_queue), "followUp": []})
        out += [{"type": "turn_start"}] + user_events(text) + assistant_events(f"steered:{text}")
    return out


def agent(prompt):
    """The scripted run: yields (events, delay) steps."""
    yield [{"type": "agent_start"}, {"type": "turn_start"}] + user_events(prompt), 0
    if prompt == "crash":
        print("pi fatal: crashed mid-turn (fixture)", file=sys.stderr, flush=True)
        os._exit(23)
    if prompt == "slow":
        yield assistant_events("working", total=100), 0
        yield [{"type": "tool_execution_start", "toolCallId": "call|slow", "toolName": "bash",
                "args": {"command": "sleep 30"}}], 0
        deadline = time.time() + 30
        while not aborted[0] and not steer_queue and time.time() < deadline:
            yield [], 0.02
        if aborted[0]:
            yield [{"type": "tool_execution_end", "toolCallId": "call|slow", "toolName": "bash",
                    "result": {"content": [{"type": "text", "text": "Command aborted"}], "details": {}},
                    "isError": True}] + assistant_events("", "error", "This operation was aborted", total=0), 0
            yield [{"type": "turn_end"}, {"type": "agent_end", "messages": [], "willRetry": False}], 0
            return
        yield tool_events("call|slow", "bash", {"command": "sleep 30"}, "(no output)")[1:], 0
        yield [{"type": "turn_end"}], 0
        steered = deliver_steers()
        if steered:
            yield steered + [{"type": "turn_end"}], 0
        yield [{"type": "agent_end", "messages": [], "willRetry": False}], 0
        return
    if prompt == "require-resume" and not args.get("session"):
        yield assistant_events("NOT-RESUMED"), 0
        yield [{"type": "turn_end"}, {"type": "agent_end", "messages": [], "willRetry": False}], 0
        return
    if prompt == "idle-crash":
        # Settle normally, then die while idle (an adapter/process crash
        # between turns); the next dispatch must resume the stored session.
        exit_when_idle[0] = True
    if prompt == "tool":
        yield tool_events("call|1", "bash", {"command": "echo hi"}, "hi\n"), 0
        yield tool_events("call|2", "write", {"path": "a.txt", "content": "A"}, "Successfully wrote to a.txt",
                          details={"_type": "new", "lines": 1, "filePath": "a.txt"}), 0
        yield tool_events("call|3", "edit", {"path": "a.txt", "edits": [{"oldText": "A", "newText": "B"}]},
                          "Successfully replaced 1 block(s) in a.txt."), 0
        yield tool_events("call|4", "mcp__t3-code__task_status", {"id": "x"}, "ok"), 0
    if prompt == "confirm":
        dialog = {"type": "extension_ui_request", "id": "dlg-1", "method": "confirm", "title": "Allow bash?",
                  "message": json.dumps({"noches": "permission", "tool": "bash", "input": {"command": "rm -rf x"}})}
        pending_dialogs["dlg-1"] = "confirm"
        yield [dialog], 0
        while "dlg-1" not in dialog_answers and not aborted[0]:
            yield [], 0.02
        answer = dialog_answers.get("dlg-1", {})
        if answer.get("confirmed"):
            yield tool_events("call|c", "bash", {"command": "rm -rf x"}, "done"), 0
        else:
            yield tool_events("call|c", "bash", {"command": "rm -rf x"},
                              "bash was declined in Noches.", is_error=True), 0
        prompt = "confirm:" + json.dumps(answer, sort_keys=True)
    if prompt in ("select", "input"):
        dialog = {"type": "extension_ui_request", "id": "dlg-2", "method": prompt, "title": "Pick one",
                  "options": ["alpha", "beta"], "message": "Which?", "placeholder": "type"}
        pending_dialogs["dlg-2"] = prompt
        yield [dialog], 0
        while "dlg-2" not in dialog_answers and not aborted[0]:
            yield [], 0.02
        prompt = "dialog:" + json.dumps(dialog_answers.get("dlg-2", {}), sort_keys=True)
    if prompt == "error":
        yield assistant_events("", "error", "model exploded", total=0), 0
        yield [{"type": "turn_end"}, {"type": "agent_end", "messages": [], "willRetry": False}], 0
        return
    if prompt == "retry-fail":
        for attempt in (1, 2):
            yield assistant_events("", "error", "Connection error.", total=0) + [
                {"type": "agent_end", "messages": [], "willRetry": True},
                {"type": "auto_retry_start", "attempt": attempt, "maxAttempts": 2, "delayMs": 1,
                 "errorMessage": "Connection error."}], 0.01
            yield [{"type": "agent_start"}, {"type": "turn_start"}], 0
        yield assistant_events("", "error", "Connection error.", total=0) + [
            {"type": "agent_end", "messages": [], "willRetry": False},
            {"type": "auto_retry_end", "success": False, "attempt": 2, "finalError": "Connection error."}], 0
        return
    if prompt == "retry-ok":
        yield assistant_events("", "error", "Connection error.", total=0) + [
            {"type": "agent_end", "messages": [], "willRetry": True},
            {"type": "auto_retry_start", "attempt": 1, "maxAttempts": 3, "delayMs": 1,
             "errorMessage": "Connection error."}], 0.01
        yield [{"type": "agent_start"}, {"type": "turn_start"}], 0
        yield assistant_events("recovered") + [{"type": "auto_retry_end", "success": True, "attempt": 2}], 0
        yield [{"type": "turn_end"}, {"type": "agent_end", "messages": [], "willRetry": False}], 0
        return
    # Two assistant messages in one turn: paragraphs must not run together.
    if prompt == "two-messages":
        yield assistant_events("first"), 0
        yield tool_events("call|t", "bash", {"command": "true"}, ""), 0
        yield assistant_events("second"), 0
        yield [{"type": "turn_end"}], 0
        yield [{"type": "agent_end", "messages": [], "willRetry": False}], 0
        return
    yield assistant_events(f"reply:{prompt}"), 0
    yield [{"type": "turn_end"}], 0
    steered = deliver_steers()
    if steered:
        yield steered, 0
    yield [{"type": "agent_end", "messages": [], "willRetry": False}], 0


def step_run():
    """Advance the active run by one step; returns the next wake-up time or None."""
    global run
    try:
        events, delay = next(run["gen"])
    except StopIteration:
        run = None
        emit({"type": "agent_settled"})
        if exit_when_idle[0]:
            sys.exit(0)
        return None
    for event in events:
        emit(event)
    return time.time() + delay


def respond(command_id, command, success=True, data=None, error=None):
    record = {"type": "response", "command": command, "success": success}
    if command_id is not None:
        record["id"] = command_id
    if data is not None:
        record["data"] = data
    if error is not None:
        record["error"] = error
    emit(record)


def model_state():
    m = model_obj(state["model"])
    return {k: v for k, v in m.items()} if m else None


def handle(cmd):
    global run, header, entries, session_file
    kind = cmd.get("type")
    cid = cmd.get("id")
    log("command", **{k: v for k, v in cmd.items() if k not in ("images",)},
        has_images=bool(cmd.get("images")))
    if kind == "get_state":
        respond(cid, kind, data={
            "model": model_state(), "thinkingLevel": state["thinking"],
            "isStreaming": run is not None, "isCompacting": False,
            "steeringMode": state["steering"], "followUpMode": state["follow_up"],
            "sessionFile": session_file, "sessionId": header["id"],
            "autoCompactionEnabled": state["auto_compaction"],
            "messageCount": len(entries), "pendingMessageCount": len(steer_queue)})
    elif kind == "get_commands":
        respond(cid, kind, data={"commands": [
            {"name": "skill:review", "description": "Review code", "source": "skill"},
            {"name": "ext-cmd", "description": "An extension command", "source": "extension"}]})
    elif kind == "get_available_models":
        respond(cid, kind, data={"models": MODELS})
    elif kind == "set_model":
        slug = f"{cmd['provider']}/{cmd['modelId']}"
        if not model_obj(slug):
            respond(cid, kind, False, error=f"Model not found: {slug}")
        else:
            state["model"] = slug
            append("model_change", provider=cmd["provider"], modelId=cmd["modelId"])
            respond(cid, kind, data=model_obj(slug))
    elif kind == "set_thinking_level":
        levels = ["off", "minimal", "low", "medium", "high", "xhigh", "max"]
        wanted = cmd["level"]
        m = model_obj(state["model"]) or {}
        offered = [l for l in levels if (m.get("thinkingLevelMap") or {}).get(l, "x") is not None] if m.get("reasoning") else ["off"]
        state["thinking"] = wanted if wanted in offered else (offered[-1] if levels.index(wanted) > levels.index(offered[-1]) else offered[0])
        emit({"type": "thinking_level_changed", "level": state["thinking"]})
        respond(cid, kind)
    elif kind == "set_session_name":
        state["name"] = cmd["name"]
        emit({"type": "session_info_changed", "name": cmd["name"]})
        respond(cid, kind)
    elif kind == "set_auto_compaction":
        state["auto_compaction"] = bool(cmd["enabled"])
        respond(cid, kind)
    elif kind == "set_steering_mode":
        state["steering"] = cmd["mode"]
        respond(cid, kind)
    elif kind == "set_follow_up_mode":
        state["follow_up"] = cmd["mode"]
        respond(cid, kind)
    elif kind == "export_html":
        respond(cid, kind, data={"path": cmd.get("outputPath") or os.path.join(SESSIONS, "export.html")})
    elif kind == "get_entries":
        since = cmd.get("since")
        if since is None and os.environ.get("FAKE_PI_NO_FULL_ENTRIES"):
            return  # a session too big to serialise: the reply never arrives
        window = entries
        if since is not None:
            ids = [e["id"] for e in entries]
            if since not in ids:
                respond(cid, kind, False, error=f"Entry not found: {since}")
                return
            window = entries[ids.index(since) + 1:]
        respond(cid, kind, data={"entries": window, "leafId": leaf()})
    elif kind == "get_session_stats":
        total = 11260 if entries else 0
        respond(cid, kind, data={
            "sessionFile": session_file, "sessionId": header["id"], "userMessages": 1,
            "assistantMessages": 1, "toolCalls": 0, "toolResults": 0, "totalMessages": len(entries),
            "tokens": {"input": 11259, "output": 2, "cacheRead": 5, "cacheWrite": 0, "total": 11266},
            "cost": 0.01,
            "contextUsage": {"tokens": None if os.environ.get("FAKE_PI_NULL_TOKENS") else total,
                             "contextWindow": (model_obj(state["model"]) or {}).get("contextWindow", 0),
                             "percent": 1.0}})
    elif kind == "prompt":
        text = cmd.get("message", "")
        if text.startswith("/handled"):
            respond(cid, kind, data={"disposition": "handled"})
            return
        if run is not None:
            if cmd.get("streamingBehavior") in ("steer", "followUp"):
                steer_queue.append(text)
                emit({"type": "queue_update", "steering": list(steer_queue), "followUp": []})
                respond(cid, kind, data={"disposition": "queued"})
            else:
                respond(cid, kind, False,
                        error="Agent is already processing. Specify streamingBehavior ('steer' or 'followUp') to queue the message.")
            return
        if text == "reject":
            respond(cid, kind, False, error="Fixture rejected this prompt")
            return
        aborted[0] = False
        respond(cid, kind, data={"disposition": "started"})
        run = {"gen": agent(text), "wake": time.time()}
    elif kind == "steer":
        steer_queue.append(cmd.get("message", ""))
        respond(cid, kind, data={"disposition": "queued"})
    elif kind == "clear_queue":
        cleared = list(steer_queue)
        steer_queue.clear()
        respond(cid, kind, data={"steering": cleared, "followUp": []})
    elif kind == "abort":
        aborted[0] = True
        # Real Pi answers once idle: finish the run first.
        while run is not None:
            wake = step_run()
            if wake is None:
                break
        respond(cid, kind)
    elif kind == "compact":
        if len(entries) < 4 and not os.environ.get("FAKE_PI_ALWAYS_COMPACT"):
            emit({"type": "compaction_start", "reason": "manual"})
            emit({"type": "compaction_end", "reason": "manual", "aborted": False, "willRetry": False,
                  "errorMessage": "Compaction failed: Nothing to compact (session too small)"})
            respond(cid, kind, False, error="Nothing to compact (session too small)")
        else:
            result = {"summary": "## Goal\nfixture", "firstKeptEntryId": leaf(),
                      "tokensBefore": 150000, "estimatedTokensAfter": 32000}
            emit({"type": "compaction_start", "reason": "manual"})
            emit({"type": "compaction_end", "reason": "manual", "result": result, "aborted": False, "willRetry": False})
            respond(cid, kind, data=result)
    elif kind == "get_fork_messages":
        respond(cid, kind, data={"messages": [
            {"entryId": e["id"], "text": text_of(e["message"])} for e in entries
            if e["type"] == "message" and e["message"]["role"] == "user"]})
    elif kind == "fork":
        target = next((e for e in entries if e["id"] == cmd.get("entryId")), None)
        if not target or target["type"] != "message" or target["message"]["role"] != "user":
            respond(cid, kind, False, error="Invalid entry ID for forking")
            return
        index = entries.index(target)
        parent = session_file
        path, sid = new_session_path()
        header = {**header, "id": sid, "parentSession": parent}
        entries = entries[:index]
        session_file = path
        persist()
        respond(cid, kind, data={"text": text_of(target["message"]), "cancelled": False})
    elif kind == "switch_session":
        path = cmd.get("sessionPath")
        if not path or not os.path.isfile(path):
            respond(cid, kind, False, error=f"ENOENT: no such file or directory, open '{path}'")
        else:
            load(path)
            respond(cid, kind, data={"cancelled": False})
    elif kind == "new_session":
        start_fresh()
        respond(cid, kind, data={"cancelled": False})
    elif kind == "extension_ui_response":
        dialog_answers[cmd["id"]] = {k: v for k, v in cmd.items() if k not in ("type", "id")}
    else:
        respond(cid, kind, False, error=f"Unknown command: {kind}")


def read_commands():
    global buffer
    data = os.read(sys.stdin.fileno(), 65536)
    if not data:
        return False
    buffer += data
    while b"\n" in buffer:
        line, buffer = buffer.split(b"\n", 1)
        line = line.rstrip(b"\r")
        if not line:
            continue
        try:
            handle(json.loads(line))
        except json.JSONDecodeError:
            respond(None, "parse", False, error="Failed to parse command")
    return True


while True:
    timeout = None
    if run is not None:
        timeout = max(0.0, run["wake"] - time.time())
    ready, _, _ = select.select([sys.stdin], [], [], timeout)
    if ready:
        if not read_commands():
            # stdin closed: orderly shutdown after the active run, like Pi.
            if run is None:
                sys.exit(0)
            exit_when_idle[0] = True
            sys.stdin = open(os.devnull)
    if run is not None and time.time() >= run["wake"]:
        wake = step_run()
        if wake is not None and run is not None:
            run["wake"] = wake
