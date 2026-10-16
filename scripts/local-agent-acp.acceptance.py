#!/usr/bin/env python3
"""Cold real ACP/agent/coding-helper exchange with a fixture inference service."""
import argparse
import http.server
import json
import os
import queue
import subprocess
import tempfile
import threading
from pathlib import Path


def run(acp, agent, tools, allow):
    requests = []
    replies = queue.Queue()
    replies.put({"role": "assistant", "content": None, "tool_calls": [{
        "id": "write-1", "type": "function", "function": {
            "name": "workspace__write_file",
            "arguments": json.dumps({"path": "result.txt", "content": "approved edit\n"}),
        },
    }]})
    replies.put({"role": "assistant", "content": "Done"})

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def send(self, value):
            body = json.dumps(value).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            self.send({"data": [{"id": "fixture-model", "object": "model"}]})

        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            assert length < 1_000_000
            requests.append(json.loads(self.rfile.read(length)))
            self.send({"id": "fixture", "object": "chat.completion", "model": "fixture-model",
                       "choices": [{"index": 0, "message": replies.get(timeout=2), "finish_reason": "stop"}]})

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    with tempfile.TemporaryDirectory(prefix="jj-store-acp-tools-") as temporary:
        root = Path(temporary)
        session = root / "session"
        session.mkdir()
        catalog = root / "catalog.json"
        catalog.write_text(json.dumps({"runners": [{"id": "local", "label": "Local", "command": agent, "args": [], "env": {}}]}))
        declarations = root / "tools.json"
        declarations.write_text(json.dumps([{"name": "workspace", "command": tools, "args": [], "env": []}]))
        environment = {k: os.environ[k] for k in ["PATH", "TMPDIR", "LANG"] if k in os.environ}
        environment.update({"HOME": str(root), "BUZZ_AGENT_PROVIDER": "openai", "OPENAI_COMPAT_API": "chat",
            "OPENAI_COMPAT_MODEL": "fixture-model", "OPENAI_COMPAT_API_KEY": "fixture-token-only",
            "OPENAI_COMPAT_BASE_URL": f"http://127.0.0.1:{server.server_port}/v1", "BUZZ_AGENT_REQUIRE_REPLY": "0"})
        child = subprocess.Popen([acp, "serve", "--control-version", "3", "--runner", "local",
            "--catalog", str(catalog), "--tools", str(declarations), "--cwd", str(root)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=environment)
        frames = queue.Queue()
        threading.Thread(target=lambda: [frames.put(json.loads(line)) for line in child.stdout], daemon=True).start()
        def send(kind, request, **extra):
            child.stdin.write(json.dumps({"version": 3, "type": kind, "requestId": request, **extra}) + "\n")
            child.stdin.flush()
        def receive(kind):
            while True:
                frame = frames.get(timeout=30)
                assert frame["type"] != "error", frame
                if frame["type"] == kind:
                    return frame
        try:
            receive("ready")
            send("warm", "warm")
            receive("warmed")
            assert not requests, "warm must not prompt"
            send("open", "open", sessionId="chat", cwd=str(session))
            receive("opened")
            send("configure", "configure", sessionId="chat", mode="ask")
            configuration = receive("configured")
            assert configuration.get("modeReady"), configuration
            send("prompt", "turn", sessionId="chat", text="Write result.txt", maxDurationMs=25000)
            permission = receive("permission")
            assert not (session / "result.txt").exists(), "edit ran before approval"
            options = permission.get("options", permission.get("request", {}).get("options", []))
            choice = next(option["optionId"] for option in options if option["kind"] == ("allow_once" if allow else "reject_once"))
            send("permission", "decision", sessionId="chat", permissionId=permission["permissionId"], optionId=choice)
            record = receive("record")
            assert record["record"]["status"] == "completed", record
            assert (session / "result.txt").exists() == allow
            if allow:
                assert (session / "result.txt").read_text() == "approved edit\n"
            assert len(requests) == 2
            send("close", "close")
            receive("closed")
            assert child.wait(timeout=10) == 0
        finally:
            child.stdin.close()
            if child.poll() is None:
                child.terminate()
                child.wait(timeout=10)
            server.shutdown()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--acp", required=True)
    parser.add_argument("--agent", required=True)
    parser.add_argument("--tools", required=True)
    args = parser.parse_args()
    for allow in [True, False]:
        run(str(Path(args.acp).resolve()), str(Path(args.agent).resolve()), str(Path(args.tools).resolve()), allow)
    print("Cold local-agent ACP coding acceptance: 2 passed (allow and deny)")
