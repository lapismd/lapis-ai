#!/usr/bin/env python3
import os,json,sys
assert sys.argv[1]=='--models'
assert os.environ['HOST_ONLY']=='retained'
assert 'JJ_STORE_TEST_SERVER_CREDENTIAL' not in os.environ
sys.stdout.write(json.dumps({'status':'ready','extra':'x'*100}))
