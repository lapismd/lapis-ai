#!/usr/bin/env python3
"""Materialize the pinned original compute in this repo's ignored scratch tree."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

PIN = "01ea0741ff608bbe14daa8d929448e3ec0790afd"
parser = argparse.ArgumentParser()
parser.add_argument("--source", required=True, type=Path)
args = parser.parse_args()
root = Path(".local/legacy-compute")
assert not root.exists(), "legacy scratch already exists; keep its recorded source pin"
manifest = json.loads(Path("docs/imports/source-manifest.json").read_text())
records = [entry for entry in manifest["files"] if entry["destination"].startswith("crates/lapis-compute/")]
for entry in records:
    data = subprocess.check_output(["jj", "file", "show", "-r", PIN, entry["path"]], cwd=args.source)
    assert hashlib.sha256(data).hexdigest() == entry["sha256"], "source checksum mismatch"
    target = root / entry["path"].removeprefix("runtime/compute/")
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
lock = subprocess.check_output(["jj", "file", "show", "-r", PIN, "runtime/compute/Cargo.lock"], cwd=args.source)
expected = json.loads(Path("docs/imports/lock-inputs.json").read_text())
pin = next(entry for entry in expected if entry["input"] == "runtime_compute_Cargo.lock")
assert hashlib.sha256(lock).hexdigest() == pin["sha256"], "lock checksum mismatch"
(root / "Cargo.lock").write_bytes(lock)
(root / "source-pin.json").write_text(json.dumps({"revision": PIN, "files": records,
    "lock_sha256": hashlib.sha256(lock).hexdigest()}, indent=2)+"\n")
print(f"Materialized {len(records)} original files and pinned lock at {root}")
