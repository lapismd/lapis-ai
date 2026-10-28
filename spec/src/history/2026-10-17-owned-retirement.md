# Registry-owned ACP retirement

Revision: based on `2752b464`, change `pmywwqok`.

Milestone review: this source lifecycle correction passes. Aggregate Tauri
transport remains In progress until the consuming packaged app is rebuilt and
its native ACP checks pass against this correction. M5 release authority is
unchanged; nothing is published.

## Commands

- `pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --test retirement --locked`
- `pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --tests --locked`
- `pnpm check`
- `pnpm test`
- `pnpm build`
- `pnpm package:check --allow-dirty`
- `pnpm spec:check`

## Results

The exact initial retirement regressions failed **0 passed / 2 failed**.
Global shutdown returned success while workers were still alive, both after a
cancelled close wait and while window revocation was already running. Red tests
waited for their delayed peers to retire before reporting failure.

Those exact two cases now pass. The expanded retirement suite passes **4 cases**,
adding reserved capacity until reaping and persistent scoped cleanup failure
without stopping another window. The complete Tauri suite passes **18 cases**:
12 host, 4 retirement and 2 channel cases.

The full workspace passes **973 Rust tests, zero failures, two inherited
ignored cases**, **42 governance cases** and **27 renderer cases**. Strict
Clippy/TypeScript/check, workspace/examples build, formatting, architecture,
boundaries, coordinated versions, specification/mdBook and all **11** Cargo
package inventories pass. The private client inventory retains **22** files;
no source, test fixture or dependency enters that archive.

Removal from the active registry and retirement registration are atomic under
the same lock. A runtime-owned job retains workers independently of caller
waits. Global shutdown/disable join existing and new retirement; scoped revoke
joins only matching admissions. Retiring and unknown-cleanup workers continue
to reserve capacity. Success releases its reservation only after supervised
cleanup; failure remains observable to repeated revoke/shutdown calls. Last host
drop also begins registry-owned retirement. No command, wire or client API changes.

## Failures

- `cancelled_close_wait_does_not_let_shutdown_report_success_before_reaping`
  and `shutdown_awaits_a_window_revocation_that_is_already_in_progress` first
  failed because removed workers belonged to a caller-owned join set. Retained
  registry completion records correct both failures; all wider lanes pass.
- The inherited compute unwind-table linker warning remains; build succeeds.

## Gaps

Before this correction, desktop passed 119 full native checks and the separate
six-case ACP suite. These results identified a qualified consumer boundary but
cannot substitute for refreshing the changed source. Rebuild the packaged
consumer and rerun all directly affected native ACP scenarios before completing
its M10.1 gate or promoting LAI-TAURI-004–006. No real provider/model, desktop AI
UI, tool bridge, credential, compute, additional platform or release is claimed.

Next action: rebuild the consuming native app against this committed correction,
qualify its ACP lifecycle, then continue desktop-owned AI integration.

Gate result: passed for this source lifecycle correction; consumer refresh pending.
