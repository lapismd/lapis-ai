# ACP protocol peer fixture launch

Revision: enclosing Jujutsu change.

Milestone review: M0–M4 unchanged; M5 remains in progress. This test-fixture repair
preserves LAI-RUNTIME-001/002 and LAI-TAURI-007/009, every startup/drain assertion,
all deadlines, child/process-group ownership and production fail-closed spawning.

## Diagnosis

The complete portable workspace run found one runtime failure before this repair:
`acp_batches::cancelled_startup_drains_the_facts_prefix_before_reaping` could not
start its unique temporary `peer.py`: `Text file busy (os error 26)`. No peer
started; the test subsequently reached its existing five-second processor-entry
timeout. Rust groups completed to that boundary totalled 966 passed, one failed
and one ignored; the run stopped before remaining integration lanes.

Unchanged main `3e29e16f15652ba239d256af49ffcba314355a5e` passed the exact
single target and one seven-test family control. A bounded unmodified family
probe stopped on its first failure, iteration9 of at most50: eight families
passed, then `default_sessions_keep_the_original_serialized_event_shape` failed
with the same spawn error (62 passed / one failed across nine families).
Thirty traced families passed all210 tests; these observed controls do not waive
the original failure. Their trace nevertheless captures a real inheritance
window: iteration24 thread553 opens writable CLOEXEC fd21 at .901790; another
thread552 clones child554 at .901851; the writer closes at .901873; child554
executes only at .905304. Thus another child can retain the writable descriptor
after the creating thread closes it. A same-image controlled fork probe confirms
that this foreign descriptor causes ETXTBSY after the parent close, while the
explicit Python interpreter can read the identical completed file. Closing the
foreign writer restores direct execution. No files were shared across test roots.

The first repaired full run passed the ACP suites but exposed the same error in
`host_clients::compute_progress_and_stop_are_independent_of_operation_completion`:
two passed / one failed. Its three-test helper writes fresh executable Python
bodies concurrently. The exact compute control passed; a bounded unchanged host
family stopped at iteration2 with the same failure (five passed / one failed).
That host-fixture source was byte-identical to main. Its static Python bodies now
live in committed executable test fixtures rather than files mutated during
parallel startup. Exact prepared argv, host environment, limits, progress and
Stop assertions remain; the Agent negative requires `helper_protocol_failed`
instead of allowing an unrelated spawn error to satisfy a generic error check.
A normal-limit Agent control first proves the exact fixture and admitted host
environment return the expected payload before the unchanged64-byte rejection.

## Owning repair

The shared protocol fixture declares the already-required PATH-resolved Python3
interpreter and stores its owned entrypoint as `serve` in the isolated cwd. The
session adapter still supplies precisely `serve --control-version 3`; it still
clears untrusted prepared arguments. Python now records its actual `sys.argv`,
which contains that declared command, instead of omitting the former script-as-
program name via `sys.argv[1:]`. No synthetic argument rewriting is performed.

Specialized processor/batch fixtures mutate their owned script, never the
interpreter executable. The raw process recovery fixture supplies its script
path explicitly. Existing protocol, argv, root, provider identity, PID, child-
group reaping, cancellation and drain assertions remain intact. Production
runtime code is unchanged; a separate Linux negative test proves that actual
direct execution of a write-open script still returns `helper_spawn_failed`.
No sleep, retry, timeout adjustment, dependency, package, lockfile or version
intent is introduced. Existing direct-executable lifecycle coverage remains.

## Commands and results

- Default-Docker `pnpm test -- -p lapis-ai-runtime --test acp_peer_fixture`:
  one passed / one failed before; two passed / zero failed after.
- Original exact failing startup target: one passed after repair. All seven
  directly affected ACP families: 43 passed / zero failed. A matched bounded
  50-family post-repair control passed350/350.
- Host-client exact compute target and all three host tests passed. A matched
  bounded50-family post-repair control passed150/150, including the normal-limit
  Agent control and exact output-limit rejection.
- Complete default-Docker workspace checking/Clippy, renderer43 tests,
  governance69 tests, Rust1078 tests, build and actionlint passed. Rust retains
  two existing ignored entries: the corpus writer (not a check) and the real
  installed-model test requiring explicitly prepared native runtime/weights.
- Cargo metadata checking then refused the intentional nine crate edits because
  a nonempty Jujutsu snapshot's colocated GitHEAD is its parent. WorkerHEAD was
  main3e; all nine reported files exactly matched intended source. The clean
  empty child after committing the verified fixture supplies the required Git
  state; no allow-dirty flag or package/harness relaxation is introduced.
- Final source specification check passed all11 lanes. The exact Node-only
  prepush gate passed, including renderer43 tests, formatter and actionlint.
  Clean-commit metadata and complete-workspace receipts follow below.

## Failures and gaps

The initial main-control invocation omitted Cargo's separator before `--exact`
and executed no Rust tests; the corrected controls above are recorded separately.
The original integration failure remains recorded rather than replaced by a
passing retry. Portable Linux evidence does not establish model, provider,
macOS/native packaging or publication qualification. The pre-existing Spec
Validator nested-verification doctor diagnostic and disabled-QMD fallback remain.

Next action: complete exact, family and full integration qualification before
sharing the verified fixture repair and dependent version-runtime guard slice.

## Clean-commit completion receipt

After the required Jujutsu commit checkpoint, complete default-Docker
`pnpm validation:all` passed on fixture commit
`82e2f629418f301543de2f106f0bcea6bc5169e4` through its content-identical empty
child `e6568c7899af82eda9e2a66cd6e607ca13fbf12b`. Receipt
`lapis-ai-run-f3b081bd-6a62-49d6-8a69-1c2100971607` records passed status:
Rust1078 passed / zero failed / the two existing ignored entries above,
renderer43, governance69, workspace Clippy/check/build, actionlint, publication
metadata and contents for14 publishable crates, and all11 specification lanes.
No allow-dirty flag, omitted package check or weakened validation was used.
The dependent guard adoption changes no fixture or production runtime code.
