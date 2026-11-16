#!/usr/bin/env python3
import json,sys
assert '--runner' in sys.argv
assert sys.argv[sys.argv.index('--control-version')+1]=='3'
print(json.dumps({'type':'ready','version':3}),flush=True)
for line in sys.stdin: print(line.strip(),flush=True)
