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

from native_agent_fixture import native_agent
from native_evidence import Evidence

def run(binary, root, model, native_runtime=None, agent_paths=None, diagnostics=None, stage="native"):
    evidence = Evidence(diagnostics, stage)
    evidence.helper("compute", binary)
    outcome = "failed"
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
        evidence.phase = action
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
            native_agent(*agent_paths, endpoint, token, model, evidence)
        child.stdin.write('{"action":"stop"}\n')
        child.stdin.flush()
        assert child.wait(timeout=20) == 0
        outcome = "passed"
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
        try:
            evidence.snapshot(root, native_runtime, model)
        except Exception:
            evidence.errors.append("fingerprint_unavailable")
        evidence.write(outcome)

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
    parser.add_argument("--diagnostics")
    parser.add_argument("--stage", default="native")
    args = parser.parse_args()
    if args.cold and Path(args.root).exists():
        parser.error("--cold requires a new isolated root")
    paths = [args.acp, args.agent, args.tools]
    if any(paths) and not all(paths):
        parser.error("--acp, --agent and --tools must be supplied together")
    run(str(Path(args.compute).resolve()), Path(args.root).resolve(), args.model,
        str(Path(args.native_runtime).resolve()) if args.native_runtime else None,
        [str(Path(path).resolve()) for path in paths] if all(paths) else None,
        args.diagnostics, args.stage)
