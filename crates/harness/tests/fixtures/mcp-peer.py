#!/usr/bin/env python3
"""Credential-free native wire/MCP fixture; diagnostics deliberately echo a token."""
import json
import os
import sys


def emit(value):
    print(json.dumps(value), flush=True)


def report(config):
    assert "stub" in config and "scope" in config, config
    assert config["stub"]["command"]
    assert config["scope"].get("headers", config["scope"].get("http_headers"))[
        "Authorization"
    ] == "Bearer opaque-session-secret"
    print("provider stderr opaque-session-secret", file=sys.stderr, flush=True)
    return "MCP_CONFIG_OK opaque-session-secret"


def answer(message, result):
    emit({"jsonrpc": "2.0", "id": message["id"], "result": result})


if "--mcp" in sys.argv:
    for line in sys.stdin:
        message = json.loads(line)
        method = message.get("method")
        if "--record" in sys.argv:
            with open(sys.argv[sys.argv.index("--record") + 1], "a") as record:
                record.write(method + "\n")
        if "id" not in message:
            continue
        if method == "initialize":
            answer(message, {"protocolVersion": "2025-06-18", "capabilities": {"tools": {}},
                             "serverInfo": {"name": "noches-stub", "version": "1"}})
        elif method == "tools/list":
            if "--empty" in sys.argv:
                answer(message, {"tools": []})
                continue
            if "--paginated" in sys.argv and "cursor" not in message.get("params", {}):
                answer(message, {"tools": [], "nextCursor": "page-two"})
                continue
            answer(message, {"tools": [{"name": "echo", "description": "Return a test marker.",
                                       "inputSchema": {"type": "object", "properties": {
                                           "value": {"type": "string"}}, "required": ["value"]}}]})
        elif method == "tools/call":
            answer(message, {"content": [{"type": "text", "text": "MCP_RESULT:" +
                                         message["params"]["arguments"]["value"]}],
                             "structuredContent": {"ok": True}})
        else:
            emit({"jsonrpc": "2.0", "id": message["id"],
                  "error": {"code": -32601, "message": "unknown method"}})
    sys.exit(0)

if "--input-format" in sys.argv:  # Claude CLI
    config_path = sys.argv[sys.argv.index("--mcp-config") + 1]
    config = json.load(open(config_path))["mcpServers"]
    text = report(config)
    assert "--strict-mcp-config" not in sys.argv
    assert sys.argv[sys.argv.index("--append-system-prompt") + 1] == "SESSION_INSTRUCTIONS"
    assert "mcp__scope__echo" in sys.argv[sys.argv.index("--allowedTools") + 1]
    assert config["scope"]["timeout"] == 3900000
    message = json.loads(sys.stdin.readline())
    emit({"type": "system", "subtype": "init", "model": "fixture", "tools": [],
          "cwd": os.getcwd(), "session_id": "mcp-claude"})
    emit({"type": "stream_event", "parent_tool_use_id": None, "event": {
        "type": "content_block_delta", "delta": {"type": "text_delta", "text": text}}})
    emit({"type": "result", "subtype": "success", "result": text,
          "usage": {"input_tokens": 1, "output_tokens": 1}, "session_id": "mcp-claude"})
    sys.exit(0)

config = None
text = None
for line in sys.stdin:
    message = json.loads(line)
    if message.get("op") == "run":  # Cursor shim
        text = report(message["mcpServers"])
        assert message["instructions"] == "SESSION_INSTRUCTIONS"
        emit({"ev": "ready", "agentId": "mcp-cursor", "model": "fixture"})
        emit({"ev": "text", "text": text})
        emit({"ev": "turn", "status": "finished"})
        break
    method = message.get("method")
    if method == "initialize":
        answer(message, {"protocolVersion": 1, "agentCapabilities": {
            "loadSession": True, "sessionCapabilities": {"resume": {}}}})
    elif method in ("thread/start", "thread/resume", "thread/fork"):
        params = message["params"]
        config = {key.removeprefix("mcp_servers."): value
                  for key, value in params["config"].items() if key.startswith("mcp_servers.")}
        text = report(config)
        assert params["developerInstructions"] == "SESSION_INSTRUCTIONS"
        answer(message, {"thread": {"id": "mcp-codex"}})
    elif method == "turn/start":
        answer(message, {"turn": {"id": "t-mcp"}})
        emit({"method": "item/agentMessage/delta", "params": {
            "threadId": "mcp-codex", "turnId": "t-mcp", "delta": text}})
        emit({"method": "turn/completed", "params": {
            "threadId": "mcp-codex", "turn": {"id": "t-mcp", "status": "completed"}}})
        break
    elif method in ("session/new", "session/load", "session/resume"):
        servers = {s["name"]: s for s in message["params"]["mcpServers"]}
        assert set(servers) == {"stub", "scope"}
        assert servers["stub"]["command"]
        assert servers["scope"]["args"] == ["acp-mcp-bridge"]
        env = {item["name"]: item["value"] for item in servers["scope"]["env"]}
        config = json.loads(env["NOCHES_SESSION_MCP_ENTRIES"])[0]["config"]
        assert config["headers"]["Authorization"] == "Bearer opaque-session-secret"
        assert os.environ["NOCHES_ACP_MCP_EXECUTABLE"]
        assert os.environ["NOCHES_SESSION_MCP_INSTRUCTIONS"] == "SESSION_INSTRUCTIONS"
        text = "MCP_CONFIG_OK opaque-session-secret"
        print(text, file=sys.stderr, flush=True)
        session_id = message["params"].get("sessionId", "mcp-acp")
        answer(message, {"sessionId": session_id})
    elif method == "session/prompt":
        assert "SESSION_INSTRUCTIONS" in json.dumps(message["params"]["prompt"])
        emit({"method": "session/update", "params": {"sessionId": session_id, "update": {
            "sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": text}}}})
        answer(message, {"stopReason": "end_turn"})
        break
    elif "id" in message:
        answer(message, {})
