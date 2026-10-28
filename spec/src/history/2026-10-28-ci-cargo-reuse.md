# CI Cargo reuse and stream cadence bound

Revision: enclosing Jujutsu change; based on `e4e4e276`.

Milestone review: CI and regression-evidence only. Publication gates unchanged.

## Contract review

LAI-TAURI-006 cadence remains a 16ms coalesced native channel. No public
signatures, protocol fields, native caps or grants change. The paced delivery
regression keeps full event preservation and ordered cursors; it now requires at
least 2× coalescing instead of an absolute delivery cap that is wall-clock
sensitive under loaded hosted runners.

## Commands and source results

- Focused: `LAPIS_AI_VALIDATION_RUNTIME=native pnpm dev:cargo -- test -p lapis-ai-service --test stream_cadence --locked`
- Hosted CI runs `node scripts/ci-spec.mjs` first with no Rust toolchain or
  fetch. `format:check`, `boundaries`, and `test:governance` run after fetch in
  Rust build (offline Changesets prep and rustfmt need that bootstrap).
- Rust build compiles once (clippy, feature-aligned workspace/examples, nextest
  archive + in-tree nextest run, package check, publish dry-run). Archive
  execution across jobs cannot remap `env!("CARGO_BIN_EXE_*")` paths, so tests
  run on the builder. Dependency caches use `cache-on-failure` and same-repo
  `save-if`. Declaring every runtime example restores builds after an explicit
  `[[example]]` disabled Cargo auto-discovery.
- The build lane uses `--workspace --bins --examples` so production CLIs are
  emitted; `--examples` alone builds example targets and skips package bins.
  Native jobs still test first with `--test-threads=1` (matches jj-store
  native), then rebuild with that lane before qualification. Parallel helper
  peer spawns on Linux ARM previously failed open with `helper_spawn_failed`.
  Callers of the reusable CI workflow (Changesets and Release) must grant
  `actions: read` after CI declared that permission; without it the orchestrator
  failed at startup and never opened the Version Packages PR.   `release:check` requires `packages/ai/CHANGELOG.md` for the coordinated client
  version so changesets/action cannot fail after version prep with ENOENT.

## Unchanged-contract confirmation

Hosted native qualification still runs cold client, compute and bundle
acceptance on both admitted ARM64 targets. Canonical aggregator still requires
green spec, Rust build and Rust test jobs.
