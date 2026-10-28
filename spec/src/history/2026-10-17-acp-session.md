# Owned ACP session adapter

Revision: Jujutsu change `zumtsrsm`, based on `cf59a8fa`.

Milestone review: M0–M4 retain their completed gates. M5 release work is unchanged.
The ACP session source slice passes LAI-TAURI-002–003. Desktop M10.1 and the Tauri
transport remain incomplete. This source slice qualifies the reusable Rust
session boundary, not packaged WebView delivery or a live provider.

## Commands

On Darwin ARM64, Rust 1.99.0, Node 26.9.0 and pnpm 10.34.5:

1. `pnpm dev:cargo -- test -p lapis-ai-runtime --test lifecycle --locked`
2. `pnpm dev:cargo -- test -p lapis-ai-runtime --tests --locked`
3. `pnpm dev:cargo -- test -p lapis-acp --test owned_session --locked`
4. `pnpm dev:cargo -- test -p lapis-acp --test owned_session cancelled_startup_stops_the_real_provider_process_group --locked`
5. `pnpm dev:cargo -- test -p lapis-ai-runtime --test acp_session_recovery cancelled_process_shutdown_retains_ownership_of_the_whole_child_group --locked`
6. `pnpm check`
7. `pnpm test`
8. `pnpm build`
9. `pnpm package:check --allow-dirty`
10. `pnpm spec:check`

## Results

- Original raw-client baseline: **8 passed, 0 failed**.
- Runtime integration: **29 passed**: 12 session, 5 recovery, 8 retained lifecycle
  and 4 resource cases. No failure or ignore in that crate.
- Actual compiled control-v3 binary: **5 passed**, using deterministic ACP
  providers. Warm/open never prompt; follow-ups reuse the negotiated session.
  Exact approvals, authored/context separation, cancellation, stale answers,
  provider scope failure, independent workers and cancelled startup pass.
- Complete workspace: **954 Rust cases passed, 0 failed, 2 inherited ignores**;
  **41 governance cases passed**. Tests use isolated development fixtures and
  generated compute-test roots, not an ordinary user vault or profile.
- Full check passes specifications, portable release metadata, architecture,
  boundaries, progress/import pins, upstream pins, formatting and strict Clippy.
- All workspace binaries and runtime examples build. All ten publishable crate
  inventories pass package checks. No package, tag or release is published.
- Ordered journals retain bounded count/bytes until acknowledgement; old cursors
  fail explicitly. Overflow/failure remains latched after retained events drain.
  Lost acknowledgements, deadlines and process exits never replay a prompt.
- Prepared launches remain Rust-owned. Canonical directory/symlink confinement
  and strict data-only requests reject path/environment/tool overrides.
- ACP drop closes control stdin under supervised cleanup, allowing the harness
  to reap its separately grouped provider. Raw process shutdown retains child
  ownership across cancelled waits and never signals a group after reaping its
  leader. The whole-group cancellation regression checks both process IDs.

## Failures

- Initial session compilation needed an explicit result error type. The first
  executable run had **8 passes / 4 failures**, then **11 / 1**, then **12 / 0**.
  One failure exposed Serde ignoring extra fields on a unit enum variant;
  the status command now uses a strict empty-struct variant. The other failures
  were fixture correlation timing, an overwritten fake session identity and a
  Python keyword/parameter collision. Assertions were retained.
- First real control-v3 composition had **0 passes / 3 failures** because the
  adapter omitted the protocol's `started` frame. Handling it now verifies the
  negotiated provider identity. The exact target then passed all three cases;
  additional cancellation cases increased the final target to five.
- The exact cancelled-startup regression first had **0 passes / 1 failure**:
  immediate harness termination orphaned the separately grouped provider. The
  test bounded and removed its deliberately blocked peer. Supervised ACP drop
  changed that same target to **1 pass / 0 failures**.
- A misplaced test-helper attribute prevented two test binaries compiling;
  moving it to the module start fixed compilation. One strict Clippy finding
  requested a collapsed conditional and was fixed without a lint suppression.
- Both broad builds emitted the inherited compute unwind-table linker warning.
  Builds passed. No warning threshold changed.

## Gaps

Tauri IPC, renderer client, scoped window/vault admission, packaged desktop,
real model providers and authenticated native app tools remain separate gates.
Python is a development-only deterministic peer. These tests do not qualify a
bundled Node/Python runtime. No user data, models or service installations change.

The two inherited ignored cases are the explicit corpus writer and installed
GGUF/native-runtime inference. Native inference and other-platform qualification
remain governed separately. Conversation durability, context policy and memory
remain application-owned. Source APIs are locally consumable; releases stay
deferred. Earlier M4 evidence is preserved without rerunning model preparation.

Next action: implement the host-scoped Tauri transport and renderer client, then
qualify a cold packaged WebView before completing desktop M10.1.

Gate result: passed for the owned ACP session source slice only.
