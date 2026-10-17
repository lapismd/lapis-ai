# Tauri ACP native transport

Revision: based on `4fcfff74`, change `nxttwlku`.

Milestone review: the Rust host source slice passes. The aggregate Tauri
transport and desktop M10.1 remain In progress until the typed renderer client
and cold packaged WebView gate pass. M5 release authorization is unchanged.

## Commands

- `pnpm dev:cargo -- fmt --all`
- `pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --tests --locked`
- `pnpm dev:cargo -- test -p lapis-ai-runtime -p lapis-acp --tests --locked`
- `pnpm check`
- `pnpm test`
- `pnpm build`
- `pnpm package:check --allow-dirty`
- `pnpm spec:check`

## Results

The focused Rust host/channel suite passes **14 cases**: 12 host and 2 stream
cases. It covers lazy startup, denied origins/targets, caller/admission isolation,
disable/re-enable, stale handles, deduplication, pending limits, lost open replies,
cancelled caller waits, startup revocation/cleanup, native resume bindings and
current-subscription acknowledgement/reconnect/channel failure.

The full workspace passes **969 Rust tests, zero failures, two inherited
ignored cases**, and **42 governance tests, zero failures**. The affected runtime
suite has 30 cases, including refusal of a silently replaced provider resume ID.
Five actual compiled control-v3 cases use deterministic provider peers.

Strict Clippy/check, workspace build, public runtime examples, all **11**
publishable crate metadata/content inventories, formatting, architecture,
boundaries and specification/mdBook checks pass. The workspace contains twelve
Cargo packages; compute remains native-only.

`tauri-plugin-lapis-ai` has empty default permissions and an explicit
`lapis-ai:acp-session` grant. Invoking WebViews supply caller context; native
policy owns vault admission, opaque targets, prepared launches and resume
bindings. Registry-owned startup survives a lost caller reply. Revocation waits
for supervised cleanup and reports unknown cleanup honestly. One current
subscription owns acknowledgements; detach/channel failure preserves workers
and bounded unacknowledged events. No uncertain request is replayed.

## Failures

- `startup_cleanup_failure_is_not_reported_as_successful_revocation` failed
  **0 passed / 1 failed** before correction because revocation returned success
  after startup cleanup failed. Completion now carries its cleanup result.
  The exact regression then passed **1 passed / 0 failed**, and is included in
  the passing full suite.
- The first strict check found **one Clippy needless-borrow error** after
  bootstrap extraction. Corrected; the exact strict check now passes.
- A formatter attempt stopped at the storage guard (**29.8 GiB free, 30 GiB
  required**). That attempt executed neither formatting nor the proposed test.
  The user explicitly approved removing only the inactive
  `lapis-ai/target/debug/incremental` compiler cache. After confirming no Cargo
  process/open files and holding the exclusive Cargo build lock, that directory
  alone was removed. The local receipt records 6,334,868 allocated KiB and free
  space increasing from 30.22 to 35.96 GiB. All checks resumed successfully.
- The inherited compute linker unwind-table warning remains; build succeeds.

## Gaps

These are Rust host-policy/channel-sink and deterministic provider results.
They do not qualify an actual WebView, packaged desktop, real model/provider,
other platform or release. LAI-TAURI-004–005 remain Planned until the native
gate. The typed renderer client and consumer qualification remain outstanding.
Ordinary profiles, vaults, models, compiled binaries and application data were
preserved; the storage guard was not changed.

Next action: implement the reusable typed renderer client, consume its built
public exports in desktop and run cold packaged WebView acceptance.

Gate result: passed for the Rust host source slice; aggregate transport pending.
