# Native Tauri processor factory

Revision: enclosing Jujutsu change `tnkvtoun`, based on `6ce65ebb`.

Milestone review: NOTES-06B wires the shared native processor into the Tauri
host. Nine focused cases, full source qualification and six cold packaged
compatibility cases pass. M0–M5 and release authorization are unchanged. Notes capture,
its durable backend and managed native acceptance remain later work.

## Commands

- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --test processing --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- fmt --all`
- `pnpm check:architecture`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --tests --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm check`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm test`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm build`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm package:check --allow-dirty`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm spec:check`
- Consumer: `CARGO_INCREMENTAL=0 MACOSX_DEPLOYMENT_TARGET=13.0 CARGO_BUILD_JOBS=2 pnpm exec tauri build --debug --bundles app --features e2e --config src-tauri/tauri.e2e.conf.json`
- Consumer: `LAPIS_DESKTOP_ACCEPTANCE_ROOT=/Users/stevejuma/code/lapismd/lapis-desktop-history/.local/native-test/factory-refresh-11PRhd MACOSX_DEPLOYMENT_TARGET=13.0 CARGO_BUILD_JOBS=2 node --test --test-concurrency=1 tests/desktop/acp-transport.test.mjs`
- Final documents: `pnpm spec:validate && pnpm check:progress && pnpm spec:first && pnpm exec prettier --check spec/src/history/2026-10-18-tauri-processing.md spec/src/current/progress/milestones.md`

## Results

The exact before target fails with E0432/E0407: `SessionBinding` and the
`HostPolicy::processor` method do not exist. The completed baseline precedes
production edits. The same target now passes **nine cases**.

One native factory receives the reserved worker/open-request/target/admission
binding and actual caller context. It runs outside registry locks, under the
existing startup owner. Denied inputs and duplicate/capacity failures cannot
allocate another processor. A failed factory creates no native process and
cannot choose a raw fallback. Original request identities remain retained.

Admission and slot ownership are checked before/after construction and each
processing call. A changed generation during resolve or factory construction
prevents launch. Revocation while facts are held drains the callback and child
before freeing capacity; no session facts escape. Capture cannot be overtaken
by channel delivery or acknowledgement. A generation change while capture is
held blocks publication without stopping an independent window.

The wrapper holds registry/policy state, not the host itself. Last-host drop
still starts retirement with a blocked callback. Cancelled close waits cannot
orphan processing before global shutdown drains it. Guards confirm that no
admitted callback was abandoned. Captured test records are sanitized and synced
before callback completion; they are not JJ publication or power-loss evidence.

Default policies retain an explicit unprocessed profile. No IPC field,
permission, launch selection, protocol identity or renderer wire format changes.
The crate README describes the new Rust factory; full semantics live in the
canonical LAI-TAURI-008 contract.

The complete affected Tauri host suite passes **27 cases**. Full source passes
**1,004 Rust tests**, **two inherited ignored**, **45 governance** and **27
renderer cases**. Strict Clippy/TypeScript, architecture/boundaries/format,
workspace/example builds, all **12 Cargo package** inventories, the **22-file
private renderer** inventory and all **10 specification lanes** pass. Validation
has five validators, 33 chapters and 94 requirements. Logs are retained under
jj-store `.local/reusable-conversations/native-factory-{host,check,test,build,package,spec}.log`.
The already-live qualification process was observed and allowed to complete;
it was not restarted when this goal resumed.

The isolated desktop consumer at `242e1fe5` rebuilds against this source:
Vite **38.74s**, native **26.48s**, **201.98 MiB** acceptance app. Its six
unchanged cold ACP cases pass in **67.23s**, retaining ordered frames, exact
permissions, independent workers, reload/reconnect, revocation, unknown outcomes
without replay, actual WebView isolation, guarded close and provider reaping.
Each cold launch received a native foreground click through computer use.
Focus/visibility assertions stay unchanged; unattended activation is not proved.
The test policy retains its default unprocessed profile, so this is compatibility
evidence, not Notes capture adoption. Verified helper resources retain producer
`1dc70422`. Logs: `native-factory-desktop-build.log` and
`native-factory-native-acp.log`; isolated fixtures: `factory-refresh-11PRhd`.

## Failures

- The before capability target cannot compile against the absent factory API.
- No failures in the initial nine-case after target.
- No failures in full source or six cold native compatibility cases.
- The inherited compute unwind-table linker warning remains; builds pass.
- `model_capabilities::tests::regen_corpus_file` remains an intentionally ignored
  corpus writer. The installed-model native case requires isolated runtime/GGUF
  preparation and remains ignored; neither is counted as passing.

## Gaps

Notes-owned privacy, durable capture, explicit AgentStore/ThreadStore backends,
unknown-write recovery and complete managed native acceptance remain separate
NOTES-06C/D/07/08/09 work. No model/provider, additional platform, unattended
activation, baseline approval, installed application or release is qualified.

Next action: inject the Notes capture processor through this qualified native
factory, with an admitted privacy scope and receipt-aware durable storage.

Gate result: passed for NOTES-06B's native factory and default ACP compatibility;
managed Notes capture and its downstream acceptance remain pending.

Final document validation passes: five validators, 33 chapters, 94 requirements;
186 imported source records; spec-first seven protected files/one canonical
chapter; selected formatting passes. Milestone gates and retained source pins
are unchanged.
