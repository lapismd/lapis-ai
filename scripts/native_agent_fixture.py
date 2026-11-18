"""Explicit permission handling for one disposable native model fixture."""
import json
from pathlib import Path
import queue
import subprocess
import tempfile
import threading
import time
from native_evidence import Evidence

AGENT_TURN_MS = 300000
MAX_PERMISSIONS = 16


def permission_choice(frame, session, seen, mutations, evidence):
    evidence.phase = 'permission'
    if frame.get('sessionId') != 'native' or frame.get('requestId') != 'turn':
        raise evidence.fail('permission_scope')
    identity = frame.get('permissionId')
    if not isinstance(identity, str) or not identity or len(identity) > 256 or identity in seen:
        raise evidence.fail('permission_identity')
    if len(seen) >= MAX_PERMISSIONS:
        raise evidence.fail('permission_bound')
    request = frame.get('request', {})
    if not isinstance(request, dict):
        raise evidence.fail('permission_scope')
    subject = request.get('subject')
    if subject is not None:
        if not isinstance(subject, dict) or 'toolCall' in request or subject.get('type') != 'tool_call':
            raise evidence.fail('permission_scope')
        call = subject.get('toolCall', {})
    else:
        call = request.get('toolCall', {})
    if not isinstance(call, dict):
        raise evidence.fail('permission_scope')
    args = call.get('rawInput')
    title = call.get('title')
    write = title == 'workspace__write_file'
    legal = isinstance(args, dict) and (
        write and set(args) == {'path', 'content'} and args['path'] == 'native-ready.txt'
        and isinstance(args['content'], str) and len(args['content'].encode()) <= 128
        and args['content'].strip() == 'NATIVE_READY'
        or title == 'workspace__read_file' and set(args) == {'path'} and args['path'] == 'native-ready.txt'
        or title == 'workspace__search' and set(args) == {'query'}
        and isinstance(args['query'], str) and 0 < len(args['query'].encode()) <= 4096)
    if not legal:
        evidence.rejected_permission(call, session)
        raise evidence.fail('permission_scope')
    target = session / 'native-ready.txt'
    if target.is_symlink() or (not mutations and target.exists()):
        raise evidence.fail('unapproved_side_effect')
    if target.exists() and (target.stat().st_size > 128 or target.read_text().strip() != 'NATIVE_READY'):
        raise evidence.fail('unexpected_side_effect')
    options = request.get('options', [])
    if not isinstance(options, list) or not 1 <= len(options) <= 8:
        raise evidence.fail('permission_options')
    if any(not isinstance(x, dict) for x in options):
        raise evidence.fail('permission_options')
    identifiers = [x.get('optionId') for x in options]
    if any(not isinstance(x, str) or not x or len(x) > 256 for x in identifiers) or len(set(identifiers)) != len(identifiers):
        raise evidence.fail('permission_options')
    allowed = [x.get('optionId') for x in options if x.get('kind') == 'allow_once']
    if len(allowed) != 1 or not isinstance(allowed[0], str) or not allowed[0] or len(allowed[0]) > 256:
        raise evidence.fail('permission_options')
    seen.add(identity)
    return allowed[0], write


def native_agent(acp, agent, tools, endpoint, token, model, evidence=None):
    evidence = evidence or Evidence()
    for name, path in [('acp', acp), ('agent', agent), ('tools', tools)]:
        evidence.helper(name, path)
    with tempfile.TemporaryDirectory(prefix='lapis-ai-native-agent-') as temporary:
        root = Path(temporary)
        session = root / 'session'
        session.mkdir()
        catalog = root / 'catalog.json'
        catalog.write_text(json.dumps({'runners': [{'id': 'local', 'label': 'Local', 'command': agent, 'args': [], 'env': {}}]}))
        declarations = root / 'tools.json'
        declarations.write_text(json.dumps([{'name': 'workspace', 'command': tools, 'args': [], 'env': []}]))
        environment = {'HOME': str(root), 'PATH': '/usr/bin:/bin', 'BUZZ_AGENT_PROVIDER': 'openai',
            'OPENAI_COMPAT_API': 'chat', 'OPENAI_COMPAT_MODEL': model, 'OPENAI_COMPAT_API_KEY': token,
            'OPENAI_COMPAT_BASE_URL': endpoint, 'BUZZ_AGENT_REQUIRE_REPLY': '0',
            'BUZZ_AGENT_MAX_CONTEXT_TOKENS': '8192', 'BUZZ_AGENT_MAX_OUTPUT_TOKENS': '1024'}
        child = subprocess.Popen([acp, 'serve', '--control-version', '3', '--runner', 'local',
            '--catalog', str(catalog), '--tools', str(declarations), '--cwd', str(root)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8", errors="replace", env=environment)
        frames = queue.Queue(maxsize=128)
        stopped = threading.Event()
        def publish(frame):
            while not stopped.is_set():
                try:
                    frames.put(frame, timeout=0.1)
                    return
                except queue.Full:
                    pass
        def read_frames():
            while not stopped.is_set():
                line = child.stdout.readline(65537)
                if not line:
                    publish({'type': 'eof'})
                    return
                try:
                    frame = json.loads(line) if len(line.encode()) <= 65536 else None
                except ValueError:
                    frame = None
                publish(frame if isinstance(frame, dict) and isinstance(frame.get('type'), str) else {'type': 'invalid_frame'})
        def read_stderr():
            for chunk in iter(lambda: child.stderr.read(4096), ''):
                evidence.stderr(chunk)
        readers = [threading.Thread(target=read_frames, daemon=True), threading.Thread(target=read_stderr, daemon=True)]
        for reader in readers:
            reader.start()
        def send(kind, request, **extra):
            child.stdin.write(json.dumps({'version': 3, 'type': kind, 'requestId': request, **extra}) + '\n')
            child.stdin.flush()
        def next_frame(deadline):
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise evidence.fail('acceptance_deadline')
            try:
                frame = frames.get(timeout=remaining)
            except queue.Empty:
                raise evidence.fail('acceptance_deadline') from None
            evidence.frame(frame)
            if frame.get('type') in {'error', 'eof', 'invalid_frame'}:
                raise evidence.fail('control_' + frame['type'])
            return frame
        def receive(kind, timeout=30):
            evidence.phase = kind
            deadline = time.monotonic() + timeout
            while True:
                frame = next_frame(deadline)
                if frame.get('type') == kind:
                    return frame
                if frame.get('type') in {'record', 'permission'}:
                    raise evidence.fail('unexpected_' + frame['type'])
        outcome = 'failed'
        try:
            receive('ready')
            send('warm', 'warm')
            receive('warmed')
            send('open', 'open', sessionId='native', cwd=str(session))
            receive('opened')
            send('configure', 'configure', sessionId='native', mode='ask')
            if not receive('configured').get('modeReady'):
                raise evidence.fail('ask_mode_unavailable')
            evidence.phase = 'turn'
            deadline = time.monotonic() + AGENT_TURN_MS / 1000
            send('prompt', 'turn', sessionId='native', maxDurationMs=AGENT_TURN_MS,
                text='Call workspace__write_file with exactly this JSON: {"path":"native-ready.txt","content":"NATIVE_READY"}. The path is relative to the opened workspace, with no added directory component. You may read that file or search this disposable workspace. Then reply DONE. Use no other mutations or shell commands.')
            seen = set()
            mutations = 0
            while True:
                frame = next_frame(deadline)
                if frame.get('type') == 'permission':
                    try:
                        choice, write = permission_choice(frame, session, seen, mutations, evidence)
                    except AssertionError:
                        if frame.get('sessionId') == 'native' and isinstance(frame.get('permissionId'), str):
                            send('permission', 'reject', sessionId='native', permissionId=frame['permissionId'], optionId=None)
                        raise
                    send('permission', 'decision-' + str(len(seen)), sessionId='native', permissionId=frame['permissionId'], optionId=choice)
                    mutations += int(write)
                    evidence.phase = 'turn'
                elif frame.get('type') == 'record':
                    if frame.get('sessionId') != 'native' or frame.get('requestId') != 'turn':
                        raise evidence.fail('terminal_scope')
                    terminal = frame.get('record')
                    if not isinstance(terminal, dict):
                        raise evidence.fail('terminal_invalid')
                    if terminal.get('status') != 'completed' or not mutations:
                        raise evidence.fail('terminal_not_completed' if mutations else 'terminal_without_write')
                    break
            target = session / 'native-ready.txt'
            if target.is_symlink() or not target.is_file() or target.stat().st_size > 128 or target.read_text().strip() != 'NATIVE_READY':
                raise evidence.fail('completion_side_effect')
            send('close', 'close')
            receive('closed')
            if child.wait(timeout=20) != 0:
                raise evidence.fail('close_status')
            outcome = 'passed'
        finally:
            stopped.set()
            try:
                child.stdin.close()
            except OSError:
                evidence.errors.append('stdin_close_failed')
            try:
                if child.poll() is None:
                    child.terminate()
                    try:
                        child.wait(timeout=20)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        child.wait(timeout=20)
            except (OSError, subprocess.TimeoutExpired):
                evidence.errors.append('child_cleanup_failed')
            for reader, stream in zip(readers, (child.stdout, child.stderr)):
                reader.join(timeout=2)
                if reader.is_alive():
                    evidence.errors.append('reader_cleanup_failed')
                else:
                    stream.close()
            evidence.write('failed' if evidence.errors else outcome)
            if outcome == 'passed' and evidence.errors:
                raise evidence.fail('cleanup_diagnostics_failed')
        print('Native model ACP permission and coding edit: passed', flush=True)
