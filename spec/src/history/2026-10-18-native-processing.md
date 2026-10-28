# Injected native session frame processing

Revision: enclosing Jujutsu change `zmqsxupy`, based on `1dc70422`.

Milestone review: NOTES-06A adds the shared Rust session hook for jj-store M33.
The shared runtime hook and full source qualification pass. Six cold packaged
ACP compatibility cases also pass against the rebuilt desktop. M0–M5 and release
authorization are unchanged. No downstream managed durable conversation or new
Tauri processor factory is claimed.

## Commands

- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-ai-runtime --test acp_processing --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- fmt --all`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-ai-runtime --tests --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --tests --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm check`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm test`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm build`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm package:check --allow-dirty`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm spec:check`
- Consumer: `CARGO_INCREMENTAL=0 MACOSX_DEPLOYMENT_TARGET=13.0 CARGO_BUILD_JOBS=2 pnpm exec tauri build --debug --bundles app --features e2e --config src-tauri/tauri.e2e.conf.json`
- Consumer: `LAPIS_DESKTOP_ACCEPTANCE_ROOT=/Users/stevejuma/code/lapismd/lapis-desktop-history/.local/native-test/processing-refresh-VkvnXQ MACOSX_DEPLOYMENT_TARGET=13.0 CARGO_BUILD_JOBS=2 node --test --test-concurrency=1 tests/desktop/acp-transport.test.mjs`

## Results

Before implementation, the exact new target fails compilation with one E0432:
`FrameInput`, `FramePublication`, `ProcessorFuture` and `SessionProcessor` do not
exist. The before log completed 64 seconds before the first production edit.
The same target initially passes **8 cases**, then the final expanded target
passes **9 cases**, adding startup provider-identity protection and permission
kind preservation. Logs live under jj-store `.local/reusable-conversations/`
as `native-processing-before.log`, `native-processing-focused.log` and
`native-processing-focused-final.log`.

One Rust-owned processor moves from bootstrap to the session actor. Startup
activity, returned opened facts and correlated events pass it before publication.
Processing ordinals include facts; event cursors retain their existing numbering.
No renderer or serialized launch input can select this port. Existing methods
retain their unprocessed contract.

Acceptance uses a host callback that sanitizes test payloads, appends only its
processed records and syncs the file before returning. Every exposed event has a
matching prior captured record. Blocked capture cannot appear in snapshots or
be acknowledged. Startup activity/facts contain no test secret after processing.
Terminal capture failure retains the original unresolved request without raw
fallback or replay. Altered session/request/sequence identities, terminal
task/status and permission option IDs retire the worker. Oversized facts fail
closed and reap startup.

Explicit startup cancellation, abandonment of the startup wait and session close
all retain in-flight processing until it completes. The callback guard confirms
that no admitted future was abandoned. Helpers remain alive during the drain
and are subsequently reaped. Shutdown rejects queued undispatched commands.
The runtime owns processing lifetime; hosts still own durability policy, exact
receipt recovery and persistence-backend teardown.

The affected runtime suite passes **42 cases** before the final ninth addition;
the Tauri host suite passes **18**. The complete source workspace passes
**995 Rust tests**, **two inherited ignored cases**, **45 governance tests** and
**27 renderer tests**. Strict Clippy/TypeScript/check, full workspace/examples
build, all **12 Cargo package** inventories and the **22-file private renderer**
inventory pass. Specification/mdBook and all **10 specification lanes** pass;
the source has **93 requirements**. Debug information and incremental compilation
are disabled for these source commands; normal debug assertions remain enabled.
No storage cleanup, target copying or user cache modification was required.

The isolated `lapis-desktop-history` consumer at `fb238ae2` rebuilds with this
runtime: Vite **50.40s**, native build **29.47s**, **201.96 MiB** acceptance app.
Its six unchanged cold native ACP tests pass in **63.16s**, including exact
permission IDs, isolated workers, renderer reload/reconnect, revocation, unknown
outcomes without replay, actual WebView isolation, guarded persistence and
process cleanup. Each of six launches received one physical foreground click.
No visibility/focus guard was changed. This is default-path compatibility, not
new managed capture adoption. The helpers retain their verified producer
`1dc70422`; only the consumer Rust library is refreshed. Native logs are
`native-processing-desktop-build.log` and `native-processing-native-acp.log`;
consumer fixtures are under `.local/native-test/processing-refresh-VkvnXQ`.

## Failures

- The exact before target fails because the injection API is absent; this is
  capability evidence, not a failing existing runtime behavior.
- No functional failures in the after runs or full source/consumer lanes.
- The inherited compute unwind-table linker warning remains; builds pass.
- `model_capabilities::tests::regen_corpus_file` is an intentionally ignored
  corpus writer. `tests::native_installed_model_starts_and_serves_without_public_discovery`
  requires the explicitly prepared isolated runtime and GGUF weights. Neither
  ignored case is reported as passing.

## Gaps

Tauri policy factory wiring, a Notes-owned sanitizer and durable capture backend,
AgentStore/ThreadStore selection, unknown-write recovery and cold downstream
conversation acceptance remain later NOTES-06/07/08/09 work. The synced test
journal is not a JJ publication, power-loss proof, AgentStore or ThreadStore.
No model/provider, additional platform, baseline, package publication or installed
application is qualified by this source slice.

Next action: expose a host-scoped Tauri processor factory, then connect Notes
native privacy/durable capture and the separately selected conversation backend.
Unattended foreground activation remains a desktop automation gap.

Final document review: five validators, 32 chapters and 93 requirements pass;
spec-first covers ten protected files and two canonical chapters. Progress
retains all 186 imported source records and unchanged M0–M5 status.

Gate result: passed for the shared runtime hook and refreshed default ACP
compatibility; managed Notes capture and downstream persistence remain pending.
