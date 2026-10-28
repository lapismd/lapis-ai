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
- Native jobs run workspace tests first, then `cargo clean` of the production
  helper crates and a feature-aligned `pnpm build`. Bin-only crates such as
  `lapis-agent-tools` lose `target/debug/<bin>` under the test profile while
  fingerprints stay Fresh, so a plain rebuild is not enough.

## Unchanged-contract confirmation

Hosted native qualification still runs cold client, compute and bundle
acceptance on both admitted ARM64 targets. Canonical aggregator still requires
green spec, Rust build and Rust test jobs.
