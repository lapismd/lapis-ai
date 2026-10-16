#!/usr/bin/env python3
"""Cold native inference through the standalone Rust compute client."""
import argparse
import json
import os
from pathlib import Path
import secrets
import subprocess

parser = argparse.ArgumentParser()
for name in ["client", "compute", "native-runtime", "root"]:
    parser.add_argument("--" + name, required=True, type=lambda value: Path(value).resolve())
parser.add_argument("--bundle", type=lambda value: Path(value).resolve())
args = parser.parse_args()
assert not args.root.exists(), "select a fresh private root"
args.root.mkdir(parents=True, mode=0o700)
q = json.dumps
config = args.root / "lapis-ai.toml"
config.write_text(f'''version=1
[runtime]
compute={q(str(args.compute))}
data_dir={q(str(args.root))}
[compute]
model="unsloth/Qwen3-0.6B-GGUF:Q4_K_M"
memory_gb=4
sharing=false
identity={{env="ACCEPTANCE_IDENTITY"}}
token={{env="ACCEPTANCE_TOKEN"}}
native_runtime={q(str(args.native_runtime))}
[compute.environment]
HOME={q(str(args.root))}
''')
env = {"PATH": os.environ["PATH"], "HOME": str(args.root),
       "ACCEPTANCE_IDENTITY": secrets.token_hex(32), "ACCEPTANCE_TOKEN": secrets.token_hex(32)}
command = [str(args.client), str(config)]
if args.bundle:
    command += ["--bundle", str(args.bundle)]
subprocess.run(command, env=env, check=True, timeout=720)
