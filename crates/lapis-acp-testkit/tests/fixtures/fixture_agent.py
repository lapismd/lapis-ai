"""Deterministic ACP peer for integration tests. No model access, no network.

Ported from the upstream buzz-acp task fixture. The relay URL, agent key and
Nostr git keyfile assertions are replaced by the harness-reserved variables this
repository actually injects, so the launch record proves injection rather than
identity.

Modes are selected with JJ_ACP_FIXTURE_MODE:
  normal             answer every method and stop with the configured reason
  startup-hang       never answer initialize
  session-hang       never answer session/new
  turn-hang          never answer session/prompt
  ignore-cancel      answer nothing and ignore session/cancel
  permission         ask for approval instead of finishing the turn
  error              fail the turn with a message that must never be echoed
  descendant         spawn a long-lived child at startup
  descendant-startup spawn a long-lived child, then never answer initialize
  descendant-turn    spawn a long-lived child, then never answer the turn
  crash-startup      exit abruptly instead of answering initialize
  crash-turn         exit abruptly instead of answering session/prompt
"""

import json
import os
import subprocess
import sys
import time

MODE = os.environ.get("JJ_ACP_FIXTURE_MODE", "normal")
PROTOCOL = int(os.environ.get("JJ_ACP_FIXTURE_PROTOCOL", "2"))
STOP_REASON = os.environ.get("JJ_ACP_FIXTURE_STOP_REASON", "end_turn")
HANG_SECONDS = 120

log = open(os.environ["JJ_ACP_FIXTURE_LOG"], "a", buffering=1)


def emit(frame):
    print(json.dumps(frame), flush=True)


def record(payload):
    log.write(json.dumps(payload) + "\n")


# The launch record is what a test reads to prove which environment the harness
# actually gave the agent. Reserved names must arrive; a credential must not.
record({
    "pid": os.getpid(),
    "mode": MODE,
    "cwd": os.getcwd(),
    "runner": os.environ.get("JJ_ACP_TASK_RUNNER"),
    "taskId": os.environ.get("JJ_ACP_TASK_ID"),
    "reserved": sorted(k for k in os.environ if k.startswith("JJ_ACP_")),
})

if MODE.startswith("descendant"):
    child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(%d)" % HANG_SECONDS])
    record({"descendant": child.pid})

OPTIONS = [
    {"id": "model", "name": "Model", "category": "model", "type": "select",
     "currentValue": "old",
     "options": [{"value": "old", "name": "Old"}, {"value": "chosen", "name": "Chosen"}]},
    {"id": "effort", "name": "Effort", "category": "thought_level", "type": "select",
     "currentValue": "low",
     "options": [{"value": "low", "name": "Low"}, {"value": "high", "name": "High"}]},
    {"id": "mode", "name": "Mode", "category": "mode", "type": "select",
     "currentValue": "default",
     "options": [{"value": "default", "name": "Default"},
                 {"value": "bypassPermissions", "name": "Bypass"},
                 {"value": "unlisted", "name": "Not in availableModes"}]},
    {"id": "collab", "name": "Collaboration", "category": "collaboration_mode",
     "type": "select", "currentValue": "default",
     "options": [{"value": "default", "name": "Default"},
                 {"value": "plan", "name": "Plan"}]},
]

session_id = None
prompt_id = None

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    message = json.loads(line)
    record(message)
    method = message.get("method")
    result = {}

    if method == "initialize":
        if MODE == "crash-startup":
            record({"crashed": "startup"})
            sys.exit(3)
        if MODE in ("startup-hang", "descendant-startup"):
            time.sleep(HANG_SECONDS)
        result = {"protocolVersion": PROTOCOL,
                  "agentInfo": {"name": "lapis-acp-fixture", "version": "1"},
                  "agentCapabilities": {}}
    elif method == "session/new":
        if MODE == "session-hang":
            time.sleep(HANG_SECONDS)
        session_id = "fixture-%d" % os.getpid()
        result = {"sessionId": session_id, "configOptions": OPTIONS,
                  "modes": {"availableModes": [{"id": "default"}, {"id": "bypassPermissions"}]}}
    elif method == "session/set_config_option":
        result = {"configOptions": OPTIONS}
    elif method == "session/set_model":
        result = {"models": {"currentModelId": message.get("params", {}).get("modelId"),
                             "availableModels": [{"modelId": "chosen"}]}}
    elif method == "session/prompt":
        prompt_id = message["id"]
        if MODE == "crash-turn":
            # Record first, so a test can prove the turn was dispatched exactly
            # once and never re-sent after the crash.
            record({"crashed": "turn", "promptId": prompt_id})
            sys.exit(4)
        if MODE in ("turn-hang", "descendant-turn", "ignore-cancel"):
            continue
        if MODE == "permission":
            emit({"jsonrpc": "2.0", "id": "permission",
                  "method": "session/request_permission",
                  "params": {"sessionId": session_id,
                             "toolCall": {"toolCallId": "tool-1", "title": "Test tool"},
                             "options": [{"optionId": "allow", "name": "Allow",
                                          "kind": "allow_once"},
                                         {"optionId": "deny", "name": "Deny",
                                          "kind": "reject_once"}]}})
            continue
        if MODE == "error":
            # The message is a marker: a test asserts it never reaches a record.
            emit({"jsonrpc": "2.0", "id": message["id"],
                  "error": {"code": -32000, "message": "SECRET_TASK_TEXT"}})
            continue
        emit({"jsonrpc": "2.0", "method": "session/update",
              "params": {"sessionId": session_id,
                         "update": {"sessionUpdate": "agent_message_chunk",
                                    "content": {"type": "text", "text": "fixture output"}}}})
        result = {"stopReason": STOP_REASON}
    elif method == "session/cancel":
        if MODE == "ignore-cancel":
            continue
        emit({"jsonrpc": "2.0", "id": prompt_id,
              "result": {"stopReason": "cancelled"}})
        continue
    elif message.get("id") == "permission":
        record({"permissionAnswer": message.get("result", {})})
        emit({"jsonrpc": "2.0", "id": prompt_id, "result": {"stopReason": STOP_REASON}})
        continue

    if "id" in message:
        emit({"jsonrpc": "2.0", "id": message["id"], "result": result})
