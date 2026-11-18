#!/usr/bin/env python3
"""Real isolated protocol processes; no model, provider or user state."""
import importlib.util
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
spec = importlib.util.spec_from_file_location("cold_acceptance", Path(__file__).with_name("local-compute.acceptance.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

FAKE = r'''
import json,os,select,sys
from pathlib import Path
root=Path(sys.argv[-1]);scenario=Path(__file__).stem
count=0;expected=2 if scenario in ['repeat','read_then_write','search_then_write','duplicate'] else (17 if scenario=='many' else 1)
sequence=0
def emit(kind,**extra):
 global sequence
 sequence+=1
 print(json.dumps({'version':3,'type':kind,'requestId':'turn','sessionId':'native','sequence':sequence,**extra}),flush=True)
def record(status='completed',**extra):emit('record',record={'status':status,**extra})
def permission():
 n=count+1
 payload={'path':'native-ready.txt','content':'NATIVE_READY'}
 title='workspace__write_file'
 if count==0 and scenario=='read_then_write':title='workspace__read_file';payload={'path':'native-ready.txt'}
 if count==0 and scenario=='search_then_write':title='workspace__search';payload={'query':'NATIVE_READY'}
 if scenario=='unsafe':payload['path']='outside.txt'
 if scenario=='wrong_tool':title='workspace__delete_file'
 tool={'title':title,'toolCallId':'tool-'+str(n),'rawInput':payload}
 emit('permission',permissionId='permission-'+str(1 if scenario=='duplicate' else n),request={'subject':{'type':'tool_call','toolCall':tool},'options':[{'optionId':'vendor:allow/'+str(n),'kind':'allow_once'},{'optionId':'vendor:deny/'+str(n),'kind':'reject_once'}]})
emit('ready')
for line in sys.stdin:
 f=json.loads(line);kind=f['type']
 if kind=='warm':emit('warmed')
 elif kind=='open':emit('opened')
 elif kind=='configure':emit('configured',modeReady=True)
 elif kind=='prompt':
  assert f['maxDurationMs']==300000
  if scenario=='early':record('failed',failureKind='provider')
  else:permission()
 elif kind=='permission':
  if f.get('optionId') is None:break
  count+=1
  assert f['optionId']=='vendor:allow/'+str(count)
  if not (count==1 and scenario in ['read_then_write','search_then_write']):(root/'session'/'native-ready.txt').write_text('NATIVE_READY')
  if scenario=='unknown':record('outcome_unknown',timeoutKind='Absolute')
  elif count<expected:
   permission()
   if not select.select([sys.stdin],[],[],2)[0]:record('outcome_unknown',timeoutKind='Absolute');break
  else:record()
 elif kind=='close':emit('closed');break
'''

class NativeFixtureTests(unittest.TestCase):
    def fixture(self, scenario):
        temporary = tempfile.TemporaryDirectory(prefix="native-protocol-test-")
        self.addCleanup(temporary.cleanup)
        path = Path(temporary.name) / scenario
        path.write_text('#!' + sys.executable + '\n' + FAKE)
        path.chmod(0o755)
        return str(path)

    def run_fixture(self, scenario):
        binary = self.fixture(scenario)
        return module.native_agent(binary, binary, binary, 'http://127.0.0.1/fixture', 'fixture-token', 'fixture-model')

    def test_single_permission_control(self):
        self.run_fixture('single')

    def test_repeated_legal_permissions_complete(self):
        self.run_fixture('repeat')

    def test_early_terminal_retains_status(self):
        with self.assertRaisesRegex(AssertionError, 'terminal.*failed|failed.*terminal'):
            self.run_fixture('early')

    def test_unsafe_path_is_not_approved(self):
        with self.assertRaisesRegex(AssertionError, 'permission_scope'):
            self.run_fixture('unsafe')

    def test_unexpected_tool_is_not_approved(self):
        with self.assertRaisesRegex(AssertionError, 'permission_scope'):
            self.run_fixture('wrong_tool')

    def test_read_then_write_completes(self):
        self.run_fixture('read_then_write')

    def test_search_then_write_completes(self):
        self.run_fixture('search_then_write')

    def test_duplicate_permission_is_cancelled(self):
        with self.assertRaisesRegex(AssertionError, 'permission_identity'):
            self.run_fixture('duplicate')

    def test_permission_count_is_bounded(self):
        with self.assertRaisesRegex(AssertionError, 'permission_bound'):
            self.run_fixture('many')

    def test_cleanup_failure_cannot_leave_a_passed_report(self):
        from native_evidence import Evidence
        with tempfile.TemporaryDirectory() as temporary:
            evidence = Evidence(Path(temporary))
            evidence.errors.append('fixture_cleanup_failure')
            binary = self.fixture('single')
            with self.assertRaisesRegex(AssertionError, 'cleanup_diagnostics_failed'):
                module.native_agent(binary, binary, binary, 'http://127.0.0.1/fixture', 'fixture-token', 'fixture-model', evidence)
            self.assertEqual(json.loads((Path(temporary) / 'native.json').read_text())['outcome'], 'failed')

    def test_unknown_outcome_stays_failure(self):
        with self.assertRaisesRegex(AssertionError, 'outcome_unknown'):
            self.run_fixture('unknown')

class EvidenceTests(unittest.TestCase):
    def test_report_contains_hashes_not_payloads_or_environment(self):
        from native_evidence import Evidence
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            model = root / 'model.gguf'
            model.write_bytes(b'GGUF-MODEL-SECRET-BYTES')
            alias = root / 'alias.gguf'
            os.link(model, alias)
            (root / 'installed-models.json').write_text(json.dumps({'fixture': {'primary': str(model), 'files': [str(model), str(alias)]}}))
            (root / '.env').write_text('PRIVATE_KEY=never-upload')
            runtime = root / 'native-runtime'
            runtime.mkdir()
            (runtime / 'manifest.json').write_text('native-public-manifest')
            evidence = Evidence(root / 'reports')
            evidence.helper('fixture-helper', model)
            evidence.snapshot(root, model='fixture')
            for _ in range(1000):
                evidence.frame({'type': 'activity', 'text': 'private-token', 'env': {'KEY': 'never-upload'}})
            evidence.stderr('Authorization: private-token')
            evidence.frame({'type': 'record', 'record': {'status': 'outcome_unknown', 'timeoutKind': 'Absolute', 'secret': 'private-token'}})
            report = evidence.write('failed')
            self.assertEqual(len(report['events']), 64)
            self.assertEqual(len(report['fingerprints']['models']), 1)
            self.assertEqual(report['fingerprints']['models'][0]['sha256'], hashlib.sha256(model.read_bytes()).hexdigest())
            text = (root / 'reports' / 'native.json').read_text()
            for secret in ['never-upload', 'private-token', 'GGUF-MODEL-SECRET-BYTES', temporary]:
                self.assertNotIn(secret, text)
            self.assertLessEqual(len(text.encode()), 32768)

    def test_model_fingerprint_rejects_outside_root(self):
        from native_evidence import Evidence
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            owned = root / 'owned'
            owned.mkdir()
            outside = root / 'outside'
            outside.write_bytes(b'not-managed')
            (owned / 'installed-models.json').write_text(json.dumps({'fixture': {'primary': str(outside), 'files': []}}))
            with self.assertRaisesRegex(AssertionError, 'model_root'):
                Evidence().snapshot(owned, model='fixture')

    def test_permission_shapes_options_and_scope_fail_closed(self):
        from native_agent_fixture import permission_choice
        from native_evidence import Evidence
        with tempfile.TemporaryDirectory() as temporary:
            session = Path(temporary)
            good = {'sessionId': 'native', 'requestId': 'turn', 'permissionId': 'one', 'request': {'toolCall': {'title': 'workspace__write_file', 'rawInput': {'path': 'native-ready.txt', 'content': 'NATIVE_READY'}}, 'options': [{'optionId': 'vendor-once', 'kind': 'allow_once'}]}}
            choice, write = permission_choice(good, session, set(), 0, Evidence())
            self.assertEqual((choice, write), ('vendor-once', True))
            for change in [{'sessionId': 'foreign'}, {'requestId': 'foreign'}, {'request': {'subject': []}}, {'request': {'toolCall': [], 'options': []}}, {'request': {'toolCall': good['request']['toolCall'], 'options': [{'kind': 'allow_always', 'optionId': 'broad'}]}}]:
                with self.assertRaises(AssertionError):
                    permission_choice({**good, **change}, session, set(), 0, Evidence())

if __name__ == '__main__':
    unittest.main()
