# Native outgoing request preparation

Revision: enclosing Jujutsu change `nktzmvow`, based on `b120a3c2`.

Milestone review: LAI-TAURI-011 adds the source request fence needed for Desktop
M10.2/M13.3 managed conversations. M0–M5 statuses, original gates and release
permission remain unchanged. This source hook does not qualify mounted Desktop
chat, persistence/context adoption, packaged tools, compute or provider backup.

## Commands

Source Rust commands use `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
CARGO_PROFILE_DEV_DEBUG=0`; tests also use `RUST_TEST_THREADS=2` where noted.
The complete qualification also sets `CARGO_PROFILE_TEST_DEBUG=0`.
Logs are under `.local/verification/managed-request/`.

- `pnpm dev:cargo -- test -p lapis-ai-runtime --test acp_request_processing --locked`
- `pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --test request_processing --locked`
- `pnpm dev:cargo -- fmt --all`
- `RUST_TEST_THREADS=2 pnpm dev:cargo -- test -p lapis-ai-runtime -p tauri-plugin-lapis-ai --tests --locked`

## Results

The new exact runtime target fails before implementation with E0432 and E0407:
`RequestInput`, `RequestProcessorFuture` and `prepare_request` are absent.
`runtime-before.log` preserves the result. After implementation its **four
cases pass** in 0.48 seconds (`runtime-after.log`).

One actor-owned immutable request callback runs after validation, mode readiness,
correlation and overlap checks, before any helper command is sent. A blocked
callback cannot dispatch or advance the event journal. Failure retains the
original unresolved identity and retires without a prompt. Close and expired
requests drain the callback before refusing dispatch. Cancelling the caller's
wait neither abandons native preparation nor permits replay. Default processors
and no-processor sessions retain their existing request behavior.

The distinct Tauri exact target initially fails: the wrapper inherited the
new default and skipped its inner callback, allowing a provider prompt before
preparation (`host-before.log`). After explicit delegation and current command
admission checks its **one case passes** in 0.15 seconds (`host-after.log`).
A retained incoming-capture grant remains valid during the test's admission
change, but cannot permit outgoing dispatch. A second window remains usable and
all helper processes are reaped. The wrapper retains no host ownership cycle.

## Maintenance

The first read-only `pnpm dev:disk` attempt returns command-not-found: this source
has only its `dev:cargo` disk preflight. The preflight requires 30 GiB, while
27.80 GiB was free. Native app/fixture data and staged helpers are preserved.
`du` reported 8.1 GiB under the source's `target/debug`, including 284 MiB of
incremental cache. Process inventory found no Cargo/rustc or source-target
executables in use.

An initial attempt to acquire both macOS lock types self-conflicted at `lockf`
after `flock`. It stopped before deletion and released its descriptor. The
successful maintenance used one nonblocking Cargo-compatible lock, verified all
open target handles, and removed only compiler subdirectories. Source
executables, `.local`, consumer targets, bundles and the running review app were
left in place. Available space increased from **27.80 to 34.73 GiB**.

```python
import fcntl, os, shutil, subprocess
from pathlib import Path
root = Path('/Users/stevejuma/code/lapismd/lapis-ai/target/debug')
assert root.resolve() == root and root.is_dir() and not root.is_symlink()
fd = os.open(root / '.cargo-lock', os.O_RDWR | os.O_NOFOLLOW)
try:
    fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    check = subprocess.run(
        ['/usr/sbin/lsof', '-Fpn', '+D', str(root.parent)],
        text=True, capture_output=True)
    assert check.returncode in (0, 1), check.stderr
    owners = {int(line[1:]) for line in check.stdout.splitlines()
              if line.startswith('p')}
    assert owners <= {os.getpid()}
    for name in ['incremental', 'build', 'deps']:
        target = root / name
        if target.exists():
            assert target.is_dir() and not target.is_symlink()
            shutil.rmtree(target)
finally:
    os.close(fd)
```

The inline maintenance command also printed available-space values before and
after removal. No cleanup ran concurrently with Cargo.

## Failures

- Runtime before: two expected missing-API compile errors; fixed by the native
  hook and exports.
- Tauri before: one expected interaction failure showing early dispatch;
  fixed by wrapper delegation and current-admission fencing.
- Read-only disk inventory command was unavailable; direct source-owned
  inventory identified the actual cache paths.
- The first dual-lock maintenance attempt self-conflicted before mutation;
  the single-lock/open-handle-verified retry completed safely.
- The initial `pnpm check` stopped at Prettier for the two new requirement
  tables. Formatting those documents fixed that failure. The next full check
  reached strict Clippy and rejected the test's nested tuple collection as
  `type_complexity`; naming the test-only call type fixes the diagnostic without
  an allowlist or production change.

## Complete source integration

The shared runtime and Tauri processor interface affects both native consumers,
so the source guide requires complete workspace qualification before adoption.
The affected runtime/host test command passes **96 tests in 21 binaries**, with
no failures or ignored cases (`affected-tests.log`). Complete source results:

- `pnpm check`: pass, including strict workspace/all-target Clippy, client
  types, formatting, contracts, source pins, architecture, boundaries and
  progress (`source-check-qualified.log`).
- `pnpm test`: **1,055 Rust passes, zero failures and two inherited ignored**
  cases across 99 test/doc-test targets; **43 renderer and 45 governance
  passes**, no failures or skips (`source-test.log`). These fixture tests do
  not qualify live providers or a mounted consumer.
  The ignored cases are corpus-file regeneration and the explicitly prepared
  native/GGUF-model scenario; neither is silently counted as passed.
- `pnpm build`: pass for workspace binaries, public renderer exports and the
  runtime examples (`source-build.log`). The existing `lapis-compute` linker
  compact-unwind warning remains recorded; no warning suppression is added.
- `pnpm package:check --allow-dirty`: pass for 13 Cargo crate inventories and
  24 private renderer package files (`source-package.log`). No publication.
- `pnpm spec:check`: all **10 lanes** pass, including strict source checks,
  45 governance cases, mdBook and mapped spec-first (`source-spec.log`).

LAI-TAURI-011 is Implemented. M0–M5 retain their exact original statuses,
dependencies and gates. Source correctness and public package contents do not
substitute for Desktop native/UI acceptance or authorize release.

## Gaps

Native Desktop adoption, automatic authored-message fencing, complete source projection,
renderer storage, context/skills/memory and full cold chat recovery remain
consumer gates. No package publication, signing, installed tooling change,
provider account or remote repository mutation is authorized by this record.

Next action: connect the Desktop managed
processor and prove real native authored/assistant capture without replay.

Gate result: source qualification passes; downstream adoption remains separate.
