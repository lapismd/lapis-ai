#!/usr/bin/env python3
import json,sys,os
assert os.getcwd()==os.path.realpath(os.environ['HOME'])
print(json.dumps({'type':'ready','endpoint':'http://127.0.0.1:12345/v1','identity':'retained'}),flush=True)
for line in sys.stdin:
 frame=json.loads(line)
 if frame['action']=='stop': break
 print(json.dumps({'type':'progress','requestId':frame['requestId'],'progress':42}),flush=True)
