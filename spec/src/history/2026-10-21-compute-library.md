# Managed compute inventory, progress and failures

Revision: enclosing Jujutsu change, based on `ea27e0d1`.

LAI-RUNTIME-003 adds a process-free inventory over the existing helper receipts,
including missing files, hard-link deduplication, imported names and preparation
receipts. Reads create no process, identity, directory or network request.
The helper emits operation-correlated model/runtime download and import facts,
and allowlisted failure categories instead of discarded dependency causes.
LAI-CONFIG-003 and LAI-RUNTIME-001/002/004 retain their isolation, credential,
shutdown and unknown-outcome contracts. No storage or execution authority moves
to the renderer. New public types and the additive API have a minor changeset.

## Commands

- `pnpm dev:cargo -- test -p lapis-ai-runtime --test compute_inventory`:
  three pass; inert absent directory, legacy receipts, deduplicated aliases,
  missing files, invalid/escaping receipts, preparation and error allowlisting.
- `pnpm dev:cargo -- test -p lapis-compute --test control`: two pass;
  authenticated readiness and immediate sequential operation acknowledgement.
- `pnpm dev:cargo -- test --workspace --features lapis-agent/test-support --locked`:
  1065 Rust/doc cases pass across 103 targets; two inherited ignored.
- First strict affected Clippy run: two failures, needless `Ok(...?)` and a
  collapsible progress conditional. Both repaired without lint suppression.
- `pnpm dev:cargo -- test -p lapis-compute --test control -p lapis-ai-runtime --test compute_inventory --locked`:
  five pass after runtime/import progress support.
- `pnpm dev:cargo -- test -p lapis-compute --test library --locked`: one pass;
  real helper import progress, correlated failure, persisted filename/size and
  explicit removal use disposable fake weights, without native inference.
- `pnpm dev:cargo -- clippy -p lapis-ai-runtime -p lapis-compute --all-targets --locked -- -D warnings`:
  passes after the two repairs.

## Results

Source-focused acceptance passes. `CARGO_BUILD_JOBS=2 pnpm test` finishes with
43 client tests, 45 governance tests and 1066 Rust/doc tests across 104 targets
passing; two inherited Rust cases remain ignored. Logs are
`.local/compute-library-all-tests-final.log`. The first complete attempt hit the
existing timing-sensitive `handoff_cap_binds_within_a_single_turn` regression;
its exact rerun passed, and the complete rerun passed without changing its
assertions or deadlines. `pnpm spec:check` passes ten specification lanes after
the repairs below (`.local/compute-library-spec-final-2.log`).

## Failures

The two intermediate Clippy failures above are retained. Specification checks
first rejected a missing SUMMARY entry, then an accidental helper-to-runtime
dependency and missing owning service/governance mappings. The inert facts and
reader now live in configuration, with runtime re-exports; the helper acquires
no runtime edge. All checks retain the original dependency allowlists. The inherited macOS
linker unwind-size warning occurs on linked compute binaries; it is separate
from Rust Clippy and no warning policy is relaxed.

## Legacy names and native diagnosis

The old Desktop acceptance build presented imported content IDs as model names.
A new regression reads legacy receipts without a saved filename and recovers
`general.name` from a bounded 64 KiB GGUF header. Truncated, invalid and oversized
headers retain the weights and use a neutral imported-model label. Explicit
names still win; ordinary model references keep a readable final component.
`CARGO_BUILD_JOBS=2 pnpm dev:cargo -- test -p lapis-ai-runtime --test compute_inventory -p lapis-compute --test library --test control --locked`
passes seven tests, including the legacy name cases
(`.local/compute-library-name-tests.log`). Receipt and model identities do not
change. No inference or full weight parsing occurs during inventory reads.

Desktop's isolated hHYUTK diagnostic run completes Prepare, Import and Start,
then rejects the app-tool turn with native HTTP502: failed to initialize chat
grammar sampler. Request shape is 5635 tool bytes, 86 system bytes and 3653 user
bytes, distinct from context exhaustion. Temporary synthetic-profile-only
capture was removed before committing; it recorded no credentials or full
prompts. The owning Desktop evidence is E-AI-123. Ordinary review and acceptance
bundles must both be rebuilt from final committed sources before final intake.

## Gaps

Real loaded-model inference, Desktop controls and two-device pairing are consumer
gates. Dummy GGUF import acceptance proves receipt/progress handling only.
Existing receipt formats remain readable; legacy runtime caches require an
explicit idempotent Prepare to record their preparation facts. No weights are
downloaded merely to populate inventory.

Milestone review: M0–M5 status and release/publication authority remain unchanged.
This extends M3's native compute client; Desktop M10.4 retains its original gates.

Next action: integrate the public native API in Desktop and qualify a real local
model separately from these deterministic helper checks.

## Final source verification

- `CARGO_BUILD_JOBS=2 pnpm test`: 43 client and 45 governance tests pass;
  1067 Rust/doc cases pass across 104 targets, with two inherited ignored.
  `.local/compute-library-all-tests-names.log`.
- `CARGO_BUILD_JOBS=2 pnpm dev:cargo -- clippy -p lapis-ai-config -p lapis-ai-runtime -p lapis-compute --all-targets --locked -- -D warnings`:
  passes. `.local/compute-library-clippy-names.log`.
- `pnpm spec:check`: all ten lanes pass, including unchanged dependency
  boundaries, budgets, formatting, governance and spec-first mapping.
  `.local/compute-library-spec-names.log`.

The source slice is verified independently of the pending loaded-model app-tool
and physical-peer consumer gates. The enclosing source revision, rather than a
future hash embedded in itself, identifies this change. Final consumer helper
intake and ordinary review rebuild use the resulting committed revision.
