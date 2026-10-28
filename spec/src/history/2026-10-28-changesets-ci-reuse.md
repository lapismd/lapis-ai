# Changesets reuses exact-source CI for tag qualification

Revision: enclosing Jujutsu change.

Milestone review: release orchestration and cold-acceptance diagnostics only.
Publication authority, admitted crates and native profile claims are unchanged.

## Contract review

LAI-REL-002 and LAI-REL-005 still require exact-source CI before an immutable
tag. Product text now states that Changesets reuses a successful CI run for the
candidate SHA instead of spawning a second reusable CI workflow. Native
acceptance gates remain Plan, Spec, Rust build, both ARM64 natives and Canonical
checks. Absolute `outcome_unknown` cold agent turns keep one bounded retry,
stderr capture and a 300s turn margin so remaining native jobs diagnose flakes
instead of failing silently after 180s.

## Commands and source results

- `node --test scripts/release-orchestration.test.mjs` — 12 passed (green reuse,
  workflow_call ignore, tip dispatch when missing, fail-closed red/off-tip)
- `JJ_STORE_VALIDATION_RUNTIME=native pnpm spec:first` — passed (4 protected
  files, 2 canonical chapters)
- `pnpm release:check` — coordinated metadata verified at 0.2.0
- Changesets `qualify` runs `await-ci` instead of reusable `ci.yml`

## Unchanged-contract confirmation

Human-merged Version Packages PRs, immutable tags, scoped trusted publishing and
hosted native cold acceptance on both admitted ARM64 targets remain required.
Local historical qualification still does not substitute for hosted CI results.
