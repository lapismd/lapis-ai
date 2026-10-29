# CI single feature-aligned Cargo compile

Revision: enclosing Jujutsu change.

Milestone review: CI and regression-evidence only. Publication gates unchanged.

## Contract review

LAI-REL-002 still requires Clippy, tests, builds and native acceptance. Clippy
remains a separate rustc-driver wave. The normal debug compile for bins,
examples and tests now runs once per Rust build and once per native target
before tests and qualification.

## Commands and source results

- `node --test scripts/ci-automation.test.mjs`
- `node scripts/ci-spec.mjs` including workflow lint

## Unchanged-contract confirmation

Failed Rust build or native jobs still cancel the rest of the run. Canonical
checks still require green Spec and Rust build. Native qualification still
skips a second build when `LAPIS_AI_SKIP_NATIVE_BUILD=1` after the shared
feature-aligned compile.
