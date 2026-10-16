#!/usr/bin/env python3
"""Deterministic warm-session and permission peer; no external dependencies."""
import json
import sys


def emit(value):
    print(json.dumps(value), flush=True)


def reply(identifier, result):
    emit({"jsonrpc": "2.0", "id": identifier, "result": result})


def update(value):
    emit({"jsonrpc": "2.0", "method": "session/update", "params": {
        "sessionId": "warm-session", "update": value}})


pending = None
turns = 0
for line in sys.stdin:
    frame = json.loads(line)
    method = frame.get("method")
    identifier = frame.get("id")
    params = frame.get("params", {})
    if method == "initialize":
        reply(identifier, {"protocolVersion": 2, "agentCapabilities": {"loadSession": True}})
    elif method in ("session/new", "session/load"):
        reply(identifier, {"sessionId": "warm-session", "models": {
            "currentModelId": "fixture-model", "availableModels": [{"modelId": "fixture-model", "name": "Fixture"}]},
            "configOptions": [{"id": "mode", "category": "mode", "type": "select", "currentValue": "ask",
                               "options": [{"value": "ask", "name": "Ask"}, {"value": "read-only", "name": "Read only"}]}],
            "modes": {"currentModeId": "ask", "availableModes": [{"id": "ask", "name": "Ask"}, {"id": "read-only", "name": "Read only"}]}})
    elif method in ("session/set_config_option", "session/set_model"):
        reply(identifier, {})
    elif method == "session/prompt":
        turns += 1
        text = params["prompt"][0]["text"]
        update({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": "turn " + str(turns)}})
        update({"sessionUpdate": "plan", "entries": [{"content": "Inspect", "priority": "medium", "status": "completed"}]})
        update({"sessionUpdate": "tool_call", "toolCallId": "tool-" + str(turns), "title": "Read file", "kind": "read"})
        if text in ("permission", "wait"):
            pending = identifier
            emit({"jsonrpc": "2.0", "id": "permit", "method": "session/request_permission", "params": {
                "sessionId": "warm-session", "toolCall": {"toolCallId": "tool-" + str(turns), "title": "Edit file"},
                "options": [{"optionId": "allow", "name": "Allow once", "kind": "allow_once"},
                            {"optionId": "deny", "name": "Reject once", "kind": "reject_once"}]}})
        elif text == "crash":
            sys.exit(7)
        else:
            reply(identifier, {"stopReason": "end_turn"})
    elif identifier == "permit" and pending is not None:
        outcome = frame.get("result", {}).get("outcome", {})
        reply(pending, {"stopReason": "cancelled" if outcome.get("outcome") == "cancelled" else "end_turn"})
        pending = None
    elif method == "session/cancel" and pending is not None:
        reply(pending, {"stopReason": "cancelled"})
        pending = None
