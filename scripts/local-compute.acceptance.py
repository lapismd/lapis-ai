#!/usr/bin/env python3
"""Cold native-runtime/model acceptance in isolated private managed storage."""
import argparse
import json
import os
from pathlib import Path
import queue
import secrets
import subprocess
import tempfile
import threading
import time
import urllib.request

def native_agent(acp, agent, tools, endpoint, token, model):
    """Real ACP tool permission and edit against the running native model."""
    with tempfile.TemporaryDirectory(prefix="jj-store-native-agent-") as temporary:
        root = Path(temporary)
        session = root / "session"
        session.mkdir()
        catalog = root / "catalog.json"
        catalog.write_text(json.dumps({"runners": [{"id": "local", "label": "Local",
            "command": agent, "args": [], "env": {}}]}))
        declarations = root / "tools.json"
        declarations.write_text(json.dumps([{"name": "workspace", "command": tools,
            "args": [], "env": []}]))
        environment = {"HOME": str(root), "PATH": "/usr/bin:/bin",
            "BUZZ_AGENT_PROVIDER": "openai", "OPENAI_COMPAT_API": "chat",
            "OPENAI_COMPAT_MODEL": model, "OPENAI_COMPAT_API_KEY": token,
            "OPENAI_COMPAT_BASE_URL": endpoint, "BUZZ_AGENT_REQUIRE_REPLY": "0",
            "BUZZ_AGENT_MAX_CONTEXT_TOKENS": "8192", "BUZZ_AGENT_MAX_OUTPUT_TOKENS": "1024"}
        child = subprocess.Popen([acp, "serve", "--control-version", "3", "--runner", "local",
            "--catalog", str(catalog), "--tools", str(declarations), "--cwd", str(root)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            text=True, env=environment)
        frames = queue.Queue()
        threading.Thread(target=lambda: [frames.put(json.loads(line)) for line in child.stdout],
            daemon=True).start()
        def send(kind, request, **extra):
            child.stdin.write(json.dumps({"version": 3, "type": kind, "requestId": request,
                **extra}) + "\n")
            child.stdin.flush()
        def receive(kind, timeout=180):
            deadline = time.monotonic() + timeout
            while True:
                remaining = deadline - time.monotonic()
                assert remaining > 0, "native agent acceptance deadline"
                frame = frames.get(timeout=remaining)
                assert frame["type"] != "error", "native agent control error"
                if frame["type"] == kind:
                    return frame
                assert frame["type"] != "record" or kind == "record", "model did not request a tool"
        try:
            receive("ready", 30)
            send("warm", "warm")
            receive("warmed", 30)
            send("open", "open", sessionId="native", cwd=str(session))
            receive("opened", 30)
            send("configure", "configure", sessionId="native", mode="ask")
            assert receive("configured", 30).get("modeReady"), "ask mode unavailable"
            send("prompt", "turn", sessionId="native", maxDurationMs=180000,
                text="Call workspace__write_file with path native-ready.txt and content NATIVE_READY. Then reply DONE. Use no other tools.")
            permission = receive("permission")
            assert not (session / "native-ready.txt").exists(), "edit ran before permission"
            options = permission.get("options", permission.get("request", {}).get("options", []))
            choice = next(option["optionId"] for option in options if option["kind"] == "allow_once")
            send("permission", "decision", sessionId="native", permissionId=permission["permissionId"],
                optionId=choice)
            record = receive("record")
            assert record["record"]["status"] == "completed", f"native agent turn did not complete: {record['record']}"
            assert (session / "native-ready.txt").read_text().strip() == "NATIVE_READY"
            send("close", "close")
            receive("closed", 30)
            assert child.wait(timeout=20) == 0
            print("Native model ACP permission and coding edit: passed", flush=True)
        finally:
            child.stdin.close()
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            child.stdout.close()

def run(binary, root, model, native_runtime=None, agent_paths=None):
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    token = secrets.token_hex(32)
    environment = {"PATH": "/usr/bin:/bin", "HOME": str(root),
        "JJ_STORE_COMPUTE_IDENTITY": secrets.token_hex(32), "JJ_STORE_COMPUTE_TOKEN": token,
        "MESH_LLM_NATIVE_RUNTIME_CACHE_DIR": str(root / "native-runtime")}
    if native_runtime:
        environment["MESH_LLM_NATIVE_RUNTIME_BUNDLE_DIR"] = native_runtime
        environment["MESH_LLM_NATIVE_RUNTIME_CACHE_DIR"] = str(root / "bundled-runtime-empty-cache")
    child = subprocess.Popen([binary], cwd=root, env=environment, stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    frames = queue.Queue()
    def reader():
        for line in child.stdout:
            try:
                frames.put(json.loads(line))
            except ValueError:
                frames.put({"type": "invalid_frame"})
    threading.Thread(target=reader, daemon=True).start()
    def receive(request=None, timeout=360):
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            assert remaining > 0, "compute command exceeded its acceptance deadline"
            frame = frames.get(timeout=remaining)
            assert frame["type"] != "invalid_frame", "unstructured helper output"
            if request is None or frame.get("requestId") == request:
                if frame["type"] not in ["progress", "serving", "imported"]:
                    return frame
    def command(action, **extra):
        request = action
        child.stdin.write(json.dumps({"requestId": request, "action": action, **extra}) + "\n")
        child.stdin.flush()
        frame = receive(request)
        assert frame["type"] in ["completed", "configured"], frame
        print(f"Native compute {action}: passed", flush=True)
    try:
        ready = receive(timeout=15)
        assert ready["type"] == "ready", ready
        endpoint = ready["endpoint"]
        # Explicit configure/prepare/download/start: no hidden downloads at start.
        command("configure", config={"model": model, "memory_gb": 4, "sharing": False, "peers": {}, "relay_urls": []})
        command("prepare")
        if native_runtime:
            cache = Path(environment["MESH_LLM_NATIVE_RUNTIME_CACHE_DIR"])
            assert not cache.exists() or not any(cache.rglob("*")), "bundled preparation downloaded a runtime"
        command("download")
        command("start")
        body = json.dumps({"model": model, "messages": [{"role": "user", "content": "Reply with LOCAL_READY."}], "max_tokens": 64, "stream": False, "chat_template_kwargs": {"enable_thinking": False}}).encode()
        request = urllib.request.Request(endpoint + "/chat/completions", data=body,
            headers={"Content-Type": "application/json", "Authorization": "Bearer " + token})
        with urllib.request.urlopen(request, timeout=120) as response:
            value = json.load(response)
        assert value["choices"][0]["message"]["content"], "empty native inference"
        print("Native local inference: passed", flush=True)
        if agent_paths:
            native_agent(*agent_paths, endpoint, token, model)
        child.stdin.write('{"action":"stop"}\n')
        child.stdin.flush()
        assert child.wait(timeout=20) == 0
    finally:
        child.stdin.close()
        if child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=20)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
        child.stdout.close()

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--compute", required=True)
    parser.add_argument("--root", required=True)
    parser.add_argument("--cold", action="store_true")
    parser.add_argument("--model", default="unsloth/Qwen3-0.6B-GGUF:Q4_K_M")
    parser.add_argument("--native-runtime")
    parser.add_argument("--acp")
    parser.add_argument("--agent")
    parser.add_argument("--tools")
    args = parser.parse_args()
    if args.cold and Path(args.root).exists():
        parser.error("--cold requires a new isolated root")
    paths = [args.acp, args.agent, args.tools]
    if any(paths) and not all(paths):
        parser.error("--acp, --agent and --tools must be supplied together")
    run(str(Path(args.compute).resolve()), Path(args.root).resolve(), args.model,
        str(Path(args.native_runtime).resolve()) if args.native_runtime else None,
        [str(Path(path).resolve()) for path in paths] if all(paths) else None)
