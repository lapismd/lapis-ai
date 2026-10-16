#!/usr/bin/env python3
"""Pinned legacy <-> Lapis encrypted peer and real native inference acceptance."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import secrets
import subprocess
import threading
import time
import urllib.error
import urllib.request


class Helper:
    def __init__(self, binary, root, native):
        root.mkdir(parents=True, mode=0o700)
        env = {"PATH": "/usr/bin:/bin", "HOME": str(root),
               "JJ_STORE_COMPUTE_IDENTITY": secrets.token_hex(32),
               "JJ_STORE_COMPUTE_TOKEN": secrets.token_hex(32),
               "MESH_LLM_NATIVE_RUNTIME_BUNDLE_DIR": str(native),
               "MESH_LLM_NATIVE_RUNTIME_CACHE_DIR": str(root / "runtime-cache")}
        self.token = env["JJ_STORE_COMPUTE_TOKEN"]
        self.child = subprocess.Popen([str(binary)], cwd=root, env=env, text=True,
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.frames = queue.Queue()
        threading.Thread(target=lambda: [self.frames.put(json.loads(line)) for line in self.child.stdout],
                         daemon=True).start()
        self.ready = self.receive(timeout=15)
        assert self.ready["type"] == "ready", "helper readiness failed"

    def receive(self, request=None, timeout=360):
        end = time.monotonic() + timeout
        while True:
            frame = self.frames.get(timeout=max(.001, end-time.monotonic()))
            if request is None or (frame.get("requestId") == request and frame["type"] in ["completed", "configured", "error"]):
                return frame

    def command(self, action, **extra):
        self.child.stdin.write(json.dumps({"requestId": action, "action": action, **extra})+"\n")
        self.child.stdin.flush()
        result = self.receive(action)
        assert result["type"] != "error", f"{action} failed"
        return result

    def configure(self, model, sharing, peer=None):
        peers = {} if peer is None else {peer.ready["identity"]: peer.ready["invitation"]}
        self.command("configure", config={"model": model, "memory_gb": 4, "sharing": sharing,
                                          "peers": peers, "relay_urls": []})

    def request(self, path, data=None):
        request = urllib.request.Request(self.ready["endpoint"].removesuffix("/v1")+path,
                  data=json.dumps(data).encode() if data is not None else None,
                  headers={"Authorization": "Bearer "+self.token, "Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                return response.status, json.load(response)
        except urllib.error.HTTPError as error:
            return error.code, None

    def close(self):
        if self.child.poll() is None:
            self.child.stdin.write('{"action":"stop"}\n')
            self.child.stdin.flush()
        self.child.stdin.close()
        try:
            self.child.wait(timeout=25)
        except subprocess.TimeoutExpired:
            self.child.kill()
            self.child.wait()
        self.child.stdout.close()
        assert self.child.returncode == 0, "helper did not stop cleanly"


def direction(client_binary, host_binary, root, native, model):
    host = Helper(host_binary, root / "host", native)
    client = None
    try:
        client = Helper(client_binary, root / "client", native)
        imported = model
        host.configure(imported, False)
        host.command("download")
        host.command("prepare")
        host.command("start")
        client.configure("unused-client-model", False, host)
        route = "/peer/"+host.ready["identity"]+"/v1"
        assert client.request(route+"/models")[0] == 503, "unadmitted peer accepted"
        host.configure(imported, False, client)
        assert client.request(route+"/models")[0] == 503, "sharing-off peer accepted"
        host.configure(imported, True, client)
        status, models = client.request(route+"/models")
        assert status == 200 and models["data"][0]["id"] == imported
        status, result = client.request(route+"/chat/completions", {
            "model": imported, "messages": [{"role": "user", "content": "Reply READY."}],
            "stream": False, "max_tokens": 64, "chat_template_kwargs": {"enable_thinking": False}})
        assert status == 200 and result["choices"][0]["message"]["content"], "native peer inference failed"
        host.configure(imported, True)
        assert client.request(route+"/models")[0] == 503, "revoked peer accepted"
        status, local = host.request("/v1/chat/completions", {
            "model": imported, "messages": [{"role": "user", "content": "Reply READY."}],
            "stream": False, "max_tokens": 32, "chat_template_kwargs": {"enable_thinking": False}})
        assert status == 200 and local["choices"], "revocation broke local inference"
        print(f"{client_binary.name} -> {host_binary.name}: admission, models, native inference, revocation passed", flush=True)
    finally:
        if client:
            client.close()
        host.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    for name in ["lapis", "legacy", "native-runtime", "root"]:
        parser.add_argument("--"+name, required=True, type=lambda path: Path(path).resolve())
    parser.add_argument("--legacy-sha256", required=True)
    parser.add_argument("--model", default="unsloth/Qwen3-0.6B-GGUF:Q4_K_M")
    args = parser.parse_args()
    assert hashlib.file_digest(args.legacy.open("rb"), "sha256").hexdigest() == args.legacy_sha256, "legacy artifact checksum changed"
    assert not args.root.exists(), "choose a fresh isolated acceptance root"
    direction(args.lapis, args.legacy, args.root / "lapis-to-legacy", args.native_runtime, args.model)
    direction(args.legacy, args.lapis, args.root / "legacy-to-lapis", args.native_runtime, args.model)
