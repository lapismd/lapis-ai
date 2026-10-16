#!/usr/bin/env python3
"""Cold TOML -> Rust client -> ACP -> native agent -> permissioned MCP edit."""
import argparse
import http.server
import json
import os
from pathlib import Path
import queue
import subprocess
import tempfile
import threading


def run(args):
    replies = queue.Queue()
    replies.put({"role": "assistant", "content": None, "tool_calls": [{
        "id": "write", "type": "function", "function": {"name": "workspace__write_file",
        "arguments": json.dumps({"path": "result.txt", "content": "approved\n"})}}]})
    replies.put({"role": "assistant", "content": "Done"})

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def answer(self, value):
            body = json.dumps(value).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            self.answer({"data": [{"id": "fixture-model", "object": "model"}]})

        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            assert 0 < length < 1_000_000
            json.loads(self.rfile.read(length))
            self.answer({"id": "fixture", "model": "fixture-model", "choices": [
                {"message": replies.get(timeout=5), "finish_reason": "stop"}]})

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        with tempfile.TemporaryDirectory(prefix="lapis-ai-client-") as directory:
            root = Path(directory).resolve()
            (root / "session").mkdir()
            q = json.dumps
            (root / "runners.toml").write_text(
                f'[[runners]]\nid="local"\nlabel="Lapis agent"\ncommand={q(args.agent)}\nargs=[]\n')
            (root / "tools.toml").write_text(
                f'[[tools]]\nname="workspace"\ncommand={q(args.tools)}\nargs=[]\nenv=[]\n')
            config = root / "lapis-ai.toml"
            config.write_text(f'''version=1
[runtime]
acp={q(args.acp)}
agent={q(args.agent)}
data_dir="state"
[acp]
runner="local"
control_version=3
cwd="session"
catalog="runners.toml"
tools="tools.toml"
[acp.environment]
HOME={q(str(root))}
[agent]
provider="openai"
model="fixture-model"
[agent.environment]
OPENAI_COMPAT_API="chat"
OPENAI_COMPAT_API_KEY={{env="LAPIS_ACCEPTANCE_TOKEN"}}
OPENAI_COMPAT_BASE_URL="http://127.0.0.1:{server.server_port}/v1"
BUZZ_AGENT_REQUIRE_REPLY="0"
''')
            environment = {k: os.environ[k] for k in ["PATH", "TMPDIR", "LANG"] if k in os.environ}
            environment.update({"HOME": str(root), "LAPIS_ACCEPTANCE_TOKEN": "fixture-only-token"})
            check = subprocess.run([args.acp, "--config", str(config), "--check-config"],
                                   env=environment, capture_output=True, text=True, timeout=10)
            assert check.returncode == 0 and check.stdout.strip() == "configuration valid", check.stderr
            # Explicit command-line values override the configured runner.
            override = subprocess.run([args.acp, "agents", "--config", str(config), "--runner", "local"],
                                      env=environment, capture_output=True, text=True, timeout=10)
            assert override.returncode == 0, override.stderr
            command = [args.client, str(config)] + (["--approve-tools"] if args.allow else [])
            if args.bundle:
                command += ["--bundle", str(args.bundle)]
            result = subprocess.run(command, env=environment, capture_output=True, text=True, timeout=90)
            assert result.returncode == 0, result.stderr
            record = json.loads(result.stdout.strip())
            assert record["status"] == "completed", record
            assert (root / "session/result.txt").exists() == args.allow, "permission boundary failed"
            if args.allow:
                assert (root / "session/result.txt").read_text() == "approved\n"
            assert "fixture-only-token" not in result.stdout + result.stderr
            if args.catalog_client:
                replies.put({"role":"assistant","content":json.dumps({"title":"Add notes command","summary":"Added the requested notes command. The command supports the user workflow.","keywords":["notes"],"goals":["Add notes command"]})})
                catalog=subprocess.run([str(args.catalog_client.resolve()),str(config)],env=environment,capture_output=True,text=True,timeout=30)
                assert catalog.returncode==0, catalog.stderr
                assert "agent Rust discovery and metadata: passed" in catalog.stdout
                print("Rust agent discovery/metadata: passed")
            print(f"TOML/Rust-client/native-agent {'allow' if args.allow else 'deny'}: passed")
    finally:
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    for name in ["acp", "agent", "tools", "client"]:
        parser.add_argument("--" + name, required=True, type=lambda value: str(Path(value).resolve()))
    parser.add_argument("--allow", action="store_true")
    parser.add_argument("--bundle", type=Path)
    parser.add_argument("--catalog-client", type=Path)
    run(parser.parse_args())
