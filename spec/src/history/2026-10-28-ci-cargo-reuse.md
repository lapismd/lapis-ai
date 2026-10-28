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
- Hosted CI runs `spec:check` and Node governance first with no Rust toolchain,
  fetch, or compile. `cargo metadata --no-deps` (boundaries) and `cargo fmt`
  stay in the Rust build job.
- Rust build compiles once (clippy, feature-aligned workspace/examples, nextest
  archive, package check, publish dry-run), uploads the archive, and test jobs
  only run that archive. Re-running failed test jobs reuses the producer
  artifact; dependency caches use `cache-on-failure` and same-repo `save-if`.
- Native jobs build once with matching features, then
  `LAPIS_AI_SKIP_NATIVE_BUILD=1` for qualification.

## Unchanged-contract confirmation

Hosted native qualification still runs cold client, compute and bundle
acceptance on both admitted ARM64 targets. Canonical aggregator still requires
green spec, Rust build and Rust test jobs.
