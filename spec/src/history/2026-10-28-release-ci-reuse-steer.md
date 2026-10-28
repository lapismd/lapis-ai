# Release CI reuse and handoff steer drain

Revision: enclosing Jujutsu change.

Milestone review: release qualification path and one agent regression
determinism fix. Publication authority and admitted crates are unchanged.

## Contract review

LAI-REL-005 Release qualification reuses successful exact-source CI for the
immutable tag SHA (same Plan/Spec/Rust/native/Canonical gates as Changesets)
instead of calling the reusable CI workflow again. LAI-LOCAL-001 handoff-cap
steer coverage still proves mid-turn steers do not reset handoff attempts; the
test drain now waits for both the prompt result and the steer RPC reply so
prompt-complete races cannot drop the steer arm.

## Commands and source results

- `pnpm dev:cargo -- test -p lapis-agent --test handoff_cap_steer --locked`
- Moved the steer-cap case to `crates/lapis-agent/tests/handoff_cap_steer.rs`
  so the frozen `regressions.rs` architecture exception does not grow
- `node --test scripts/release-orchestration.test.mjs` — 12 passed
- `node scripts/ci-spec.mjs` / `pnpm spec:first`

## Unchanged-contract confirmation

Immutable tags, human Version Packages merges, scoped trusted publishing and
both admitted ARM64 native gates remain required. Release still builds archives
after qualification; only the duplicate reusable-CI invoke is removed.
