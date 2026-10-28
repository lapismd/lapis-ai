# Bounded native and renderer stream delivery

Revision: enclosing Jujutsu change `zpvyqznk`, based on `25f488e8`.

Milestone review: advances Notes NOTES-09-FILES-C under the standing functional
plugin priority. M0–M5, publication and broader qualification gates stay unchanged.

## Contract review

LAI-TAURI-006 retains exact snapshot validation, consumed-only cursors, bounded
buffering, detach and unknown/no-replay behavior. The client acknowledges at most
32 already queued, individually consumed snapshots in one prefix. In-flight data
counts toward existing count/byte bounds. One scheduled pump owns reentrant
delivery. No public signatures, protocol fields, native caps or grants change.

Native channel delivery coalesces notifications over 16ms and revalidates
admission before publishing the unchanged contiguous snapshot under
LAI-TAURI-002/003/004/006. Initial delivery remains immediate.

LAI-RUNTIME-001 adds bounded pipe backpressure: when full, the same parsed frame
waits at most 100ms for the consumer. No more frames are read during that wait.
Queue/frame limits, request deadlines, genuine overflow retirement and
supervised cancellation remain enforced. The later control fix below also
requires a newly verified helper bundle.

## Commands and source results

- `node --test packages/ai/tests/stream-throughput.test.mjs`: before one failure;
  101 queued snapshots require 101 serial IPC acknowledgements. After passes
  with at most five acknowledgements.
- `pnpm test:client`: builds public exports; all 43 SDK cases pass, including
  capacity, corrupt-prefix and reentrant-owner coverage. `pnpm check:client`
  passes.
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --test stream_cadence --locked`:
  before one failure (203 deliveries for 200 updates); after both cases pass,
  fewer than 50 deliveries and no publication after revocation.
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-ai-runtime --test process_fairness --locked`:
  before one failure (`output_coverage_lost`) with an active consumer of 1,000
  ordered frames through an eight-frame queue.
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-ai-runtime --test process_fairness --test lifecycle --locked`:
  first after passes all nine cases, including genuine stalled-consumer failure.
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-ai-runtime -p tauri-plugin-lapis-ai --locked`:
  first full run: 88 cases pass, including the briefly paused consumer case,
  complete ACP/compute runtime coverage and all 36 Tauri host cases.
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- clippy -p lapis-ai-runtime -p tauri-plugin-lapis-ai --all-targets --locked -- -D warnings`:
  passes.
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- fmt --all`;
  changed-file `pnpm exec prettier --write`; `pnpm check:architecture`;
  `pnpm check:boundaries`; `pnpm check:progress`; `pnpm spec:validate`: pass.

Artifacts: `.local/stream-throughput-*.log`, `.local/process-fairness-*.log`.
No existing assertion, cap, scope, identity or no-replay check is weakened.

## Native consumer findings

Notes E-AI-044 retains the exact real Codex 500-line failures after SDK-only and
presentation-only changes. A diagnostic first identifies the full native journal
(sequence 730 minus acknowledgement 474 = 256 outstanding). Native delivery
coalescing removes that pressure but reveals pipe overflow at only 86 outstanding
journal events (888 minus 802). The pipe regression and fix above address this
second independently bounded queue. A subsequent native run still fails at
the exact journal boundary (963 minus 707 = 256), so lower-pipe fairness alone
is explicitly insufficient. Its cold recovery case passes in 47.32s.

A new `journal_fairness` regression fails before with a consuming subscriber:
80 updates through an eight-event journal with 16ms acknowledgements. The active
actor now waits at most 100ms for space, without growing the journal, reading
more helper frames, replaying processing or changing cursors. A dedicated
capacity notification keeps acknowledgement wakeups separate from delivery.
Original deadlines and close interrupt that wait; startup overflow stays immediate.
The first wider attempt catches one existing atomic-close regression in
`acp_batches`; restoring immediate whole-group publication when capacity is
available corrects it. No existing test is changed. The final affected runtime/host suite passes all
91 cases, including ordered journal handoff, interrupted close and original
request-deadline precedence. Strict affected Clippy passes. Commands and logs:

- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-ai-runtime --test journal_fairness --locked`
  (`.local/journal-fairness-before.log`, one failure).
- Same wrapper with `test -p lapis-ai-runtime --test journal_fairness --test acp_session --test acp_session_recovery --test acp_processing --test acp_batches --locked`
  (`.local/journal-fairness-after-{first,second}.log`; first fails one existing
  batch-close case, second passes).
- Same wrapper with `test -p lapis-ai-runtime -p tauri-plugin-lapis-ai --locked`
  (`.local/journal-fairness-runtime-host.log`, 91 pass).
- Same wrapper with `clippy -p lapis-ai-runtime -p tauri-plugin-lapis-ai --all-targets --locked -- -D warnings`
  (`.local/journal-fairness-clippy.log`, pass).

Native journal backpressure alone still fails the real 500-line test after
36.89s, now with `activity_coverage_lost`. This correctly identifies the helper's
own bounded activity queue, not the repaired native journal. Notes retains that
failed attempt in `.local/ai-streaming-native-backpressure-after.log`.

## Complete helper backpressure

LAI-PROTO-001/003/007/008 add the default-compatible
`SessionHandler::session_update_async` hook. Existing synchronous handlers still
receive the same callback. The connection awaits an overriding delivery hook
inside the original absolute budget before reading the next frame. Control
LAI-CTRL-003/008/009 uses bounded asynchronous output for activity, permissions
and worker records. A full queue retains one current frame for at most 100ms;
failed or abandoned handoff latches loss and cancels the worker. Synchronous
emission keeps immediate overflow behavior. No frame or queue cap grows.

- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-acp-control --test activity_fairness --locked`:
  before one failure with a consuming eight-frame queue and 1,000 packed protocol
  updates. After uses the same test and preserves every sequence/text value.
- Same wrapper with `test -p lapis-acp-control --test activity_fairness --test interactive_overflow --locked`:
  all four final cases pass, including stalled async output, cancelled handoff
  and the unchanged synchronous overflow case.
- First wider test/build commands mistakenly name the directory
  `lapis-acp-cli` as a package; both fail before execution. Correct package:
  `lapis-acp`. Logs `.local/activity-fairness-{affected,helper-build}.log`.
- Same wrapper with `test -p lapis-acp-control -p lapis-acp-protocol -p lapis-acp-runner -p lapis-acp --locked`:
  all 165 cases pass, including native deterministic CLI fixtures.
- Same wrapper with `clippy -p lapis-acp-protocol -p lapis-acp-control -p lapis-acp-runner -p lapis-acp -p lapis-ai-runtime -p tauri-plugin-lapis-ai --all-targets --locked -- -D warnings`:
  passes.
- Same wrapper with `build -p lapis-acp --bin lapis-acp --locked`: passes.
- `pnpm package:stage -- --profile debug --destination dist/codex-local-review-v3 --native-runtime /Users/stevejuma/code/lapismd/lapis-ai/dist/codex-local-review-v2/native-runtime`:
  passes. `verifyBundle` verifies the new bundle; only the device review
  configuration's bundle path changes. Other three helpers match the prior
  verified bundle byte for byte. No installation, publication or service change.

New helper SHA256:
`2ec70683de1a86c1b8e372505366f639e4e8c75bf14c353ec8a2ca46e4ed9dca`.
The original remains available in `dist/codex-local-review-v2`. SDK public
`dist/stream.js` SHA256:
`c8181d2dd3338d38f70eccb51c0cdca902c3de731e21b95708ea96975d94c8f2`.
The Changeset records the additive Rust hook; it authorizes no versioning/release.
The final protocol/agent/runtime/Tauri suite passes 186 cases, including the
new original-absolute-budget regression for an awaited update hook. Command:
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-acp-protocol -p lapis-acp-agent -p lapis-ai-runtime -p tauri-plugin-lapis-ai --locked`.
These overlap the earlier 165-case control/protocol/runner/CLI run; counts are
per command, not an aggregate of unique cases.

## Native consumer acceptance

Notes E-AI-044's unchanged native targets both pass with the final helper bundle:
500 exact response lines saved with the original prompt once, no pending intent,
no error entries and unchanged Note.md (38.81s); real Codex crash/cold recovery
and an explicitly new message without old-prompt replay (44.46s). Combined:
2 pass, 0 fail, 84.07s. Isolated fixtures: `ai-local-streaming-l3XOHf` and
`ai-local-recovery-Il38LO`; consumer log `.local/ai-streaming-native-helper-after.log`.
The app uses built SDK exports and the verified v3 bundle. This closes the named
streaming regression; it does not qualify JJ persistence or other providers.

Source checks include `pnpm check:architecture`, `pnpm check:boundaries`,
`pnpm check:progress`, `pnpm check:client`, `pnpm release:check`,
`pnpm exec spec-validator first`, `pnpm exec spec-validator validate` and
`pnpm format:check`. No package publication or version preparation occurs.

## Gaps

Full Rust workspace, unrelated compute service, broader provider/platform/release
and visual baseline qualification remain deferred by the user. Runtime tests
include the directly affected compute client; this does not qualify inference.
Native Notes acceptance is recorded above; it is separate from distribution
and broader platform qualification.

Next action: refresh the ordinary Notes review app without resetting its data;
retain deferred provider, JJ and distribution gates.

Gate result: pass (named source and native consumer streaming regression)
