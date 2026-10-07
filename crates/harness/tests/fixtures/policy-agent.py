#!/usr/bin/env python3
"""Deterministic native wire oracle; never executes the requested tool."""
import json
import sys


def send(value):
    print(json.dumps(value), flush=True)


def read():
    return json.loads(sys.stdin.readline())


def reply(request, result):
    send({"id": request["id"], "result": result})


args = sys.argv[1:]
if "--input-format" in args:
    read()  # first user message
    send({"type": "system", "subtype": "init", "session_id": "policy",
          "model": "fixture", "tools": ["Bash"], "cwd": "/tmp"})
    send({"type": "control_request", "request_id": "tool", "request": {
        "subtype": "can_use_tool", "tool_name": "Bash", "input": {"command": "echo safe"},
        "permission_suggestions": [{"type": "addRules", "destination": "projectSettings",
                                    "behavior": "allow", "rules": [{"toolName": "Bash", "ruleContent": "echo safe"}]}]}})
    approval = read()
    send({"type": "control_request", "request_id": "question", "request": {
        "subtype": "can_use_tool", "tool_name": "AskUserQuestion", "input": {
            "questions": [{"question": "Pick", "options": ["A", "B"]}]}}})
    question = read()
    plan_exit = None
    if "plan" in args:
        send({"type": "control_request", "request_id": "plan-exit", "request": {
            "subtype": "can_use_tool", "tool_name": "ExitPlanMode", "input": {}}})
        plan_exit = read()
    send({"type": "result", "subtype": "success", "session_id": "policy",
          "result": json.dumps({"args": args, "approval": approval, "question": question,
                                "planExit": plan_exit})})
elif "app-server" in args:
    init = read()
    reply(init, {})
    read()  # initialized notification
    start = read()
    reply(start, {"thread": {"id": "policy"}})
    turn = read()
    reply(turn, {"turn": {"id": "turn"}})
    send({"method": "turn/started", "params": {"threadId": "policy", "turn": {"id": "turn"}}})
    send({"id": 900, "method": "item/permissions/requestApproval", "params": {
        "threadId": "policy", "permissions": {"network": {"enabled": True}}}})
    approval = read()
    send({"id": 901, "method": "item/tool/requestUserInput", "params": {
        "threadId": "policy", "questions": [{"id": "q", "question": "Pick", "options": [{"label": "B"}]}]}})
    question = read()
    send({"method": "item/agentMessage/delta", "params": {"threadId": "policy",
          "delta": json.dumps({"start": start["params"], "turn": turn["params"],
                               "approval": approval, "question": question})}})
    send({"method": "turn/completed", "params": {"threadId": "policy", "turn": {"id": "turn", "status": "completed"}}})
else:
    init = read()
    reply(init, {"protocolVersion": 1, "agentCapabilities": {}})
    start = read()
    reply(start, {"sessionId": "policy",
                  "models": {"currentModelId": "fixture", "availableModels": [
                      {"modelId": "fixture", "name": "Fixture"}]},
                  "modes": {"availableModes": [
        {"id": name, "name": name} for name in ["default", "auto_edit", "yolo"]]}})
    turn = read()
    mode = None
    if turn["method"] == "session/set_mode":
        mode = turn["params"]["modeId"]
        reply(turn, {})
        turn = read()
    assert turn["method"] == "session/prompt", turn
    permission = {"sessionId": "policy", "toolCall": {
        "toolCallId": "exec", "title": "Run echo", "rawInput": {"command": "echo safe"}},
        "options": [{"optionId": "once", "kind": "allow_once", "name": "Allow once"},
                    {"optionId": "always", "kind": "allow_always", "name": "Persistent"},
                    {"optionId": "no", "kind": "reject_once", "name": "Deny"}]}
    send({"id": 900, "method": "session/request_permission", "params": permission})
    approval = read()
    # Unknown future permission kinds must not enter the content-question
    # bridge, even when their display label matches its scripted answer.
    send({"id": 903, "method": "session/request_permission", "params": {
        "sessionId": "policy", "toolCall": {"title": "Future approval"}, "options": [
            {"optionId": "future", "name": "B", "kind": "allow_future"},
            {"optionId": "future-deny", "name": "Deny", "kind": "reject_future"}]}})
    unknown_approval = read()
    send({"id": 901, "method": "session/request_permission", "params": {
        "sessionId": "policy", "toolCall": {"title": "Pick"}, "options": [
            {"optionId": "b", "name": "B"}]}})
    question = read()
    aliases = []
    if "agent" in args:
        for method in ["x.ai/ask_user_question", "_x.ai/ask_user_question"]:
            send({"id": 902, "method": method, "params": {"sessionId": "policy",
                  "questions": [{"id": "q", "question": "Pick", "options": [{"label": "B"}]}]}})
            aliases.append(read())
    send({"method": "session/update", "params": {"sessionId": "policy", "update": {
        "sessionUpdate": "agent_message_chunk", "content": {"type": "text",
        "text": json.dumps({"mode": mode, "args": args, "init": init["params"],
                            "approval": approval, "unknownApproval": unknown_approval,
                            "question": question, "aliases": aliases})}}}})
    reply(turn, {"stopReason": "end_turn"})
