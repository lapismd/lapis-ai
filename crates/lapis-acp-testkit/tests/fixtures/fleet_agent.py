#!/usr/bin/env python3
"""Independent sessions in one initialized process, recording protocol methods."""
import json
import os
import sys

sessions = 0
previous_session = None
for line in sys.stdin:
    frame = json.loads(line)
    method = frame.get("method")
    params = frame.get("params", {})
    with open(os.environ["FLEET_LOG"], "a", encoding="utf-8") as log:
        log.write(str(method) + "\n")
    result = {}
    if method == "session/new" and os.environ.get("FLEET_TOOLS_LOG"):
        with open(os.environ["FLEET_TOOLS_LOG"], "a", encoding="utf-8") as tools:
            tools.write(json.dumps(params.get("mcpServers")) + "\n")
    if method == "initialize":
        result = {"protocolVersion": 2, "agentCapabilities": {"loadSession": True, "promptCapabilities": {"embeddedContext": True}}}
    elif method == "session/new":
        sessions += 1
        result = {"sessionId": "adapter-" + str(sessions), "configOptions": [{"id":"mode","name":"Mode","category":"mode","type":"select","currentValue":"ask","options":[{"value":"ask","name":"Ask"}]}], "modes": {
            "currentModeId": "ask", "availableModes": [{"id": "ask", "name": "Ask"}]}}
    elif method == "session/prompt":
        text = params["prompt"][-1]["text"]
        with open(os.environ["FLEET_LOG"], "a", encoding="utf-8") as log:
            log.write("prompt-blocks:" + json.dumps(params["prompt"]) + "\n")
        if previous_session and previous_session != params["sessionId"]:
            print(json.dumps({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":previous_session,"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"stale inactive update"}}}}), flush=True)
        previous_session = params["sessionId"]
        if text == "permission":
            print(json.dumps({"jsonrpc":"2.0","id":"permission-rpc","method":"session/request_permission","params":{"sessionId":params["sessionId"],"toolCall":{"toolCallId":"tool","title":"Read"},"options":[{"optionId":"once","name":"Allow once","kind":"allow_once"}]}}), flush=True)
            answer = json.loads(sys.stdin.readline())
            with open(os.environ["FLEET_LOG"], "a", encoding="utf-8") as log:
                log.write("decision:" + answer["result"]["outcome"]["optionId"] + "\n")
        print(json.dumps({"jsonrpc": "2.0", "method": "session/update", "params": {
            "sessionId": "foreign" if text == "foreign" else params["sessionId"], "update": {"sessionUpdate": "agent_message_chunk",
            "content": {"type": "text", "text": params["prompt"][-1]["text"]}}}}), flush=True)
        result = {"stopReason": "end_turn"}
    if "id" in frame:
        print(json.dumps({"jsonrpc": "2.0", "id": frame["id"], "result": result}), flush=True)
