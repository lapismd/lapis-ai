# Version PR App authentication

Revision: enclosing Jujutsu change.

Milestone review: M0–M4 unchanged; M5 remains in progress. This source tooling
slice does not qualify native releases or authorize package/tag publication.

## Contract review

LAI-REL-003 requires repository-scoped dependency App authentication for generated
version PRs. LAI-REL-002/004/005 retain exact-source CI, human review, native gates
and existing dispatch/tag credentials. LAI-GOV-100–105 source limits and routing
remain unchanged. Checkout does not retain credentials; missing App credentials
fail rather than selecting the workflow token.

## Commands

- `node --test scripts/version-pr-auth.test.mjs`: two failures before the workflow
  change; two passes afterward.
- `LAPIS_AI_VALIDATION_RUNTIME=native pnpm check:governance`: passed.
- `TMPDIR=/home/steve/.cache/lapis-maintenance-validation LAPIS_AI_VALIDATION_RUNTIME=native pnpm test:governance`:
  69 passed, zero failed.
- `pnpm exec prettier --check .`, `pnpm exec spec-validator first`,
  `pnpm exec spec-validator build`, and pinned actionlint on the changed workflow:
  passed.
- `LAPIS_AI_VALIDATION_RUNTIME=native pnpm check:client` and
  `LAPIS_AI_VALIDATION_RUNTIME=native pnpm test:client`: passed; 43 client tests.
- `TMPDIR=/home/steve/.cache/lapis-maintenance-validation LAPIS_AI_VALIDATION_RUNTIME=native pnpm spec:check`:
  all 11 specification and tooling lanes passed.
- `node scripts/ci-spec.mjs` with explicit native tooling runtime, pinned
  actionlint and current Jujutsu base/head: the complete Node-only Spec gate passed.

## Results

Parsed workflow checks cover canonical main, repository/permission scope,
fail-closed minting, non-persistent checkout, API commits, disabled direct release
creation, action pins and preserved dispatch/tag credentials. Release orchestration
continues to require both native targets and exact-source CI.

## Failures

The initial broader governance run had one existing Changesets fixture failure:
the temporary filesystem had 21 GiB available against the unchanged 30 GiB Cargo
guard. The exact fixture reproduced on unchanged main. Selecting a temporary
directory on the home filesystem and the documented explicit native tooling
runtime repaired the environment; the complete 69-test suite then passed.

## Gaps

No Rust/runtime source changed. Native model/platform, complete Rust builds/tests,
hosted App-written candidate acceptance and publication remain separate gates.
The existing doctor reports the absent root verification chapter although this
repository config owns `current/verification.md`; QMD is disabled and search
reports its rg fallback. This slice does not change that validator integration.

Next action: review the source PR and qualify the next eligible generated PR
through its normal exact-source/native checks. Do not manufacture a version bump.
