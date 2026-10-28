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
- Workflow and build lane share `lapis-agent/test-support` so native CI builds
  once, tests against those artifacts, and skips the second qualification compile.
- `LAPIS_AI_SKIP_NATIVE_BUILD=1` requires the production helpers and examples
  already present under `target/<profile>`.

## Unchanged-contract confirmation

Hosted native qualification still runs cold client, compute and bundle
acceptance on both admitted ARM64 targets. Canonical lanes still run clippy,
build, test, package check and publish dry-run.
