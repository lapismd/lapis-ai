# Retained native processing authority

Revision: enclosing Jujutsu change `wzpntuuv`, based on `8a8f9149`.

Milestone review: NOTES-07 needs native capture to survive renderer suspension
without granting commands to that renderer. This source prerequisite adds an
optional exact-binding policy hook. Source checks, tests, builds and package
inventories pass. M0–M5, native consumer acceptance and publication authority
are unchanged.

## Commands

- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --test retained_processing --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- fmt --all`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --tests --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- check -p tauri-plugin-lapis-ai --all-targets --locked`
- `pnpm check:architecture`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm check`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm test`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm build`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm package:check --allow-dirty`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm spec:check`
- `pnpm spec:first`

## Results

Before production edits, the exact capability target fails E0407 because
`HostPolicy::authorize_processing` does not exist. After implementation, the
same target passes five cases with fresh native fixture processes.
All 34 Tauri host tests and the affected all-target crate check pass. Because
this public policy hook changes shared native processing lifetime, final
integration runs the complete source workspace checks, tests, builds and
package/specification lanes required by the repository guide.

The default hook delegates to current renderer admission. An overriding Rust
host receives the original native context and complete worker/open-request/
target/admission binding. Before and after each callback, the wrapper requires
the original generation and exact live registry slot. The wrapper retains no
host ownership cycle. Commands, construction and startup completion still
require current renderer admission.

Retained capture completes during renderer suspension, while list, targets,
snapshot, attach, detach, acknowledgement, submit, close and another open stay
denied. Reattachment sees the same worker and filtered retained events with one
original prompt. A retained grant cannot bypass factory admission or publish a
startup session while suspended. A wrong returned generation prevents the
callback; revocation during an admitted callback drains it but suppresses its
output. Registry revocation and global disable independently retire and drain
even while the policy grant remains. Removed handles never return on enable.

The crate README describes the extension briefly; detailed semantics are in
LAI-TAURI-008/010. IPC, wire formats, permissions, unprocessed defaults and
prompt retry policy are unchanged. A Changeset records the public Rust API
addition without authorizing publication.

Source check passes strict all-target workspace Clippy, renderer TypeScript,
architecture, boundaries, release metadata, progress, imported source pins and
formatting. The full workspace passes 1,018 Rust tests, with two inherited
ignored tests, plus 39 renderer and 45 governance cases. Workspace/example
builds pass. All 12 Cargo inventories and the 24-file private renderer package
inventory pass. The new fixture poller tolerates an unfinished concurrent
append until the complete terminal record is readable; its five exact cases
and the full suite pass again after that test-only fix, with the same totals.
Logs live under `.local/retained-processing/`.

The final complete specification run passes all ten lanes. Validation covers
five validators, 35 chapters and 96 requirements. Spec-first passes with six
protected files and two canonical chapters. Architecture budgets and all 186
imported source pins remain unchanged.

## Failures

- The before target fails E0407 for the missing policy method.
- Initial after target: five pass, no failures.
- Initial full check stopped in document validation: the verification table
  accepts only Implemented/Planned, and the new evidence was missing from
  SUMMARY. Corrected both before repeating qualification.
- The inherited compute unwind-table linker warning remains; build succeeds.
- The first complete specification run passes nine lanes but fails spec-first
  because the mapped governance chapter is missing. Added its actual retained
  authority/source-qualification boundary and repeated the exact target.
- An initial patch using the wrong governance context is rejected without
  changes; corrected the context before editing.

## Gaps

Notes must implement and qualify its native retained grant;
these fixture processors do not qualify durable JJ capture, managed chat or
packaged conversation restart. Live provider, additional platform, visual and
release gates remain separate.

Next action: complete source qualification, then consume the verified extension
in the Notes native capture owner.

Gate result: passed for the shared source prerequisite. Notes adoption and
managed native conversation acceptance remain pending.
