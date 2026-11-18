"""Bounded public fixture metadata only: no model bytes, env or raw stderr."""
from collections import deque
import hashlib
import json
import os
from pathlib import Path
import re


def digest(path, limit=4 * 1024**3):
    path = Path(path)
    size = path.stat().st_size
    assert path.is_file() and not path.is_symlink() and size <= limit, "hash_input"
    value = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            value.update(chunk)
    return {'bytes': size, 'sha256': value.hexdigest()}


def utf8_length(value):
    if not isinstance(value, str):
        return None
    try:
        return len(value.encode())
    except UnicodeEncodeError:
        return None


class Evidence:
    def __init__(self, directory=None, stage='native'):
        assert re.fullmatch(r'[a-z0-9-]{1,64}', stage), 'diagnostic_stage'
        self.directory = Path(directory) if directory else None
        self.stage = stage
        self.phase = 'startup'
        self.events = deque(maxlen=64)
        self.total = 0
        self.helpers = {}
        self.stderr_hash = hashlib.sha256()
        self.stderr_bytes = 0
        self.fingerprints = {}
        self.errors = []

    def frame(self, frame):
        self.total += 1
        kind = frame.get('type')
        item = {'type': kind if kind in {'ready', 'warmed', 'opened', 'configured',
            'queued', 'activity', 'permission', 'permission_resolved', 'record',
            'ack', 'closed', 'error', 'eof', 'invalid_frame'} else 'other'}
        if kind == 'record':
            record = frame.get('record')
            record = record if isinstance(record, dict) else {}
            item['status'] = record.get('status') if record.get('status') in {
                'completed', 'failed', 'cancelled', 'outcome_unknown'} else 'invalid'
            item['timeoutKind'] = record.get('timeoutKind') if record.get('timeoutKind') in {
                'Absolute', 'Idle', None} else 'other'
            item['quiescent'] = frame.get('quiescent') is True
            item['sessionUsable'] = frame.get('sessionUsable') is True
        self.events.append(item)

    def rejected_permission(self, call, session):
        """Classify a refused request without retaining any raw tool/argument text."""
        title = call.get('title')
        tools = {'workspace__write_file': 'write_file', 'workspace__read_file': 'read_file',
            'workspace__search': 'search', 'workspace__shell': 'shell'}
        tool = tools.get(title, 'other') if isinstance(title, str) else 'invalid'
        args = call.get('rawInput')
        shape = 'object' if isinstance(args, dict) else ('null' if args is None else
            'array' if isinstance(args, list) else 'string' if isinstance(args, str) else
            'boolean' if isinstance(args, bool) else 'number' if isinstance(args, (int, float)) else 'invalid')
        keys = set(args) if isinstance(args, dict) else set()
        keyset = ('write_file' if keys == {'path', 'content'} else 'read_file' if keys == {'path'}
            else 'search' if keys == {'query'} else 'other')
        path = args.get('path') if isinstance(args, dict) else None
        path_class = 'invalid'
        path_bytes = utf8_length(path)
        if path_bytes is not None and 0 < path_bytes <= 4096 and '\0' not in path:
            # Lexical classification only: this never admits or resolves a requested path.
            root = os.path.abspath(session)
            target = os.path.join(root, 'native-ready.txt')
            normalized = os.path.normpath(path if os.path.isabs(path) else os.path.join(root, path))
            path_class = ('exact_relative' if path == 'native-ready.txt' else
                'normalized_absolute_same_fixture' if normalized == target and os.path.isabs(path) else
                'normalized_relative_same_fixture' if normalized == target else
                'other_inside_fixture' if os.path.commonpath([root, normalized]) == root else 'outside_fixture')
        content = args.get('content') if isinstance(args, dict) else None
        detail = {'tool': tool, 'inputShape': shape, 'keyset': keyset, 'pathClass': path_class,
            'contentIsString': isinstance(content, str),
            'contentBytes': utf8_length(content), 'contentUtf8Valid': utf8_length(content) is not None,
            'contentMarkerMatch': isinstance(content, str) and content.strip() == 'NATIVE_READY'}
        if self.events and self.events[-1]['type'] == 'permission':
            self.events[-1]['rejection'] = detail
        else:
            self.events.append({'type': 'permission', 'rejection': detail})

    def stderr(self, chunk):
        data = chunk.encode('utf-8', errors='replace')
        self.stderr_bytes += len(data)
        self.stderr_hash.update(data)

    def fail(self, code):
        return AssertionError(json.dumps({'code': code, 'phase': self.phase,
            'terminal': next((x for x in reversed(self.events) if x['type'] == 'record'), None)}))

    def helper(self, name, path):
        self.helpers[name] = digest(Path(path).resolve())

    def snapshot(self, root, runtime=None, model=None):
        """Only explicit private fixture storage or caller-selected runtime assets."""
        root = Path(root).resolve()
        self.fingerprints['modelSelectionSha256'] = hashlib.sha256((model or '').encode()).hexdigest()
        index = root / 'installed-models.json'
        if index.exists():
            assert index.stat().st_size <= 262144, 'model_index_bound'
            entry = json.loads(index.read_text())[model]
            paths = [Path(entry['primary']), *map(Path, entry['files'])]
            assert len(paths) <= 16, 'model_file_bound'
            seen = set()
            model_hashes = []
            for path in paths:
                assert path.resolve().is_relative_to(root), 'model_root'
                identity = (path.stat().st_dev, path.stat().st_ino)
                if identity not in seen:
                    seen.add(identity)
                    model_hashes.append(digest(path))
            self.fingerprints['models'] = model_hashes
        runtime = Path(runtime) if runtime else root / 'native-runtime'
        if runtime.exists():
            hashes = []
            visited = 0
            for path in runtime.rglob('*'):
                visited += 1
                assert visited <= 256, 'runtime_entry_bound'
                if path.is_dir():
                    continue
                assert len(hashes) < 64 and path.resolve().is_relative_to(runtime.resolve()), 'runtime_file_bound'
                hashes.append(digest(path.resolve()))
            self.fingerprints['runtime'] = sorted(hashes, key=lambda item: item['sha256'])

    def write(self, outcome):
        value = {'version': 1, 'stage': self.stage, 'phase': self.phase,
            'outcome': outcome, 'helpers': self.helpers,
            'source': os.environ.get('LAPIS_AI_NATIVE_SOURCE') if re.fullmatch(r'[a-f0-9]{40}', os.environ.get('LAPIS_AI_NATIVE_SOURCE', '')) else None,
            'run': os.environ.get('GITHUB_RUN_ID') if os.environ.get('GITHUB_RUN_ID', '').isdigit() else None,
            'attempt': os.environ.get('GITHUB_RUN_ATTEMPT') if os.environ.get('GITHUB_RUN_ATTEMPT', '').isdigit() else None,
            'fingerprints': self.fingerprints, 'events': list(self.events),
            'eventCount': self.total, 'stderr': {'bytes': self.stderr_bytes,
                'sha256': self.stderr_hash.copy().hexdigest()}, 'diagnosticErrors': self.errors}
        data = json.dumps(value, sort_keys=True)
        assert len(data.encode()) <= 32768, 'diagnostic_bound'
        if self.directory:
            try:
                self.directory.mkdir(parents=True, exist_ok=True, mode=0o700)
                (self.directory / (self.stage + '.json')).write_text(data + '\n')
            except OSError:
                self.errors.append('report_write_failed')
        return value
