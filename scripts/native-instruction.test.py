#!/usr/bin/env python3
"""Compiled ACP → agent → tools controls, synthetic provider only; no model proof."""
import http.server
import json
from pathlib import Path
import queue
import sys
import threading
import unittest
from unittest.mock import patch

import native_agent_fixture as fixture
from native_evidence import Evidence

BINS = Path(sys.argv.pop(1)).resolve() if len(sys.argv) > 1 else Path('target/debug').resolve()


class NativeInstructionTests(unittest.TestCase):
    def control(self, path):
        replies = queue.Queue()
        replies.put({'role': 'assistant', 'content': None, 'tool_calls': [{
            'id': 'fixture-write', 'type': 'function', 'function': {
                'name': 'workspace__write_file',
                'arguments': json.dumps({'path': path, 'content': 'NATIVE_READY'})}}]})
        replies.put({'role': 'assistant', 'content': 'DONE'})
        requests = []

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                self.answer({'data': [{'id': 'fixture-model'}]})

            def do_POST(self):
                size = int(self.headers['Content-Length'])
                if not 0 < size < 1000000:
                    self.send_error(400)
                    return
                requests.append(json.loads(self.rfile.read(size)))
                self.answer({'id': 'fixture', 'model': 'fixture-model', 'choices': [{
                    'message': replies.get(timeout=5), 'finish_reason': 'stop'}]})

            def answer(self, value):
                body = json.dumps(value).encode()
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.send_header('Content-Length', str(len(body)))
                self.end_headers()
                self.wfile.write(body)

        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        original = fixture.permission_choice
        observed = []

        def choice(frame, session, seen, mutations, evidence):
            self.assertEqual(frame['request']['subject']['toolCall']['rawInput'],
                             {'path': path, 'content': 'NATIVE_READY'})
            self.assertFalse((session / 'native-ready.txt').exists())
            self.assertFalse((session.parent / 'native-ready.txt').exists())
            observed.append(True)
            return original(frame, session, seen, mutations, evidence)

        evidence = Evidence()
        try:
            with patch.object(fixture, 'permission_choice', choice):
                args = [str(BINS / name) for name in ['lapis-acp', 'lapis-agent', 'lapis-agent-tools']]
                if path == 'native-ready.txt':
                    fixture.native_agent(*args, f'http://127.0.0.1:{server.server_port}/v1',
                                         'fixture-only-token', 'fixture-model', evidence)
                else:
                    with self.assertRaisesRegex(AssertionError, 'permission_scope'):
                        fixture.native_agent(*args, f'http://127.0.0.1:{server.server_port}/v1',
                                             'fixture-only-token', 'fixture-model', evidence)
                    self.assertEqual(evidence.events[-1]['rejection']['pathClass'], 'other_inside_fixture')
                    self.assertTrue(evidence.events[-1]['rejection']['contentMarkerMatch'])
            self.assertEqual(observed, [True])
            self.assertEqual(len(requests), 2 if path == 'native-ready.txt' else 1)
            messages = requests[0]['messages']
            instruction = next(x['content'] for x in messages if x['role'] == 'user')
            # Check the actual provider wire, independently of the source constant.
            self.assertIn('workspace__write_file', instruction)
            self.assertIn('{"path":"native-ready.txt","content":"NATIVE_READY"}', instruction)
            self.assertIn('relative to the opened workspace', instruction)
            self.assertIn('no added directory component', instruction)
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=5)
            self.assertFalse(thread.is_alive())

    def test_exact_instruction_reaches_provider_and_real_tools_write(self):
        self.control('native-ready.txt')

    def test_added_session_directory_is_rejected(self):
        self.control('session/native-ready.txt')

    def test_other_inside_path_is_rejected(self):
        self.control('other.txt')


if __name__ == '__main__':
    unittest.main()
