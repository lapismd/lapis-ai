# E-RELEASE-FOLLOWUP-001: hosted setup and version-lock formatting

Revision: enclosing Jujutsu change `mqkmvpos`.

Milestone review: M0–M4 remain Done. M5 remains In progress while hosted product
CI completes. Initial publication is intentionally deferred: Version Packages
must remain open. This record supplements the immutable
[setup evidence](2026-10-16-release-setup.md).

## Scope and unchanged contracts

LAI-REL-003–004 require the ordinary formatting gate on generated version PRs.
Version preparation now formats `pnpm-lock.yaml` after regenerating it, and the
real preparation regression checks that output. Bot recognition and all product,
package, canonical and release-identity checks remain unchanged. ACP protocols,
provider behavior, source pins and native ABI retain their qualified contracts.
No Rust production source changes in this follow-up.

## Commands

- `bash scripts/native-qualification.sh debug` (started with the setup change,
  completed after its first push).
- `gh api repos/stevejuma/lapis-ai/pages --method POST -f build_type=workflow`.
- `jj git push --bookmark main`; inspected hosted jobs with `gh run view`,
  `gh run watch`, `gh run list` and the job-log API.
- `curl --fail --silent --show-error` against the published root and
  `/current/contracts/release.html`.
- `gh pr view 1 --json title,state,isDraft,autoMergeRequest,files`; read the
  version-branch package metadata and verified version 0.1.1/private coordinator.
- `gh api repos/stevejuma/lapis-ai/tags`; `gh release list`.
- `node --test --test-name-pattern='real Changesets' scripts/changesets.test.mjs`
  before and after the fix.
- `pnpm check`; `pnpm test`; `pnpm build`; `pnpm spec:check`.

## Results

The local cold macOS ARM64 native script passed every step: allow and deny
clients, native preparation/download/start, real inference, approved ACP tool
edit, four-helper staging, bundle acceptance and the Rust compute client.
It used fresh private roots and retained Mesh 0.76.0-rc9 / ABI 0.1.44. The macOS
compute linker retained the prior large-unwind-table warning; execution passed.

[Changesets](https://github.com/stevejuma/lapis-ai/actions/runs/38893876529)
passed and created [Version Packages #1](https://github.com/stevejuma/lapis-ai/pull/1).
The PR is OPEN, at 0.1.1, with auto-merge disabled. It changes only the consumed
note, version metadata, locks, changelog and installation examples. It contains
no invented canonical specification or milestone evidence. CI and Pages were
explicitly dispatched for its branch.

[Main Pages](https://github.com/stevejuma/lapis-ai/actions/runs/38893876495)
passed and deployed <https://stevejuma.github.io/lapis-ai/>. Both HTTP document
checks passed. The [version-branch Pages build](https://github.com/stevejuma/lapis-ai/actions/runs/38893965489)
passed and skipped deployment. GitHub PR-event runs required approval; explicit
dispatch supplied the actual version-branch checks.

The formatting regression changed from one failure to zero. The complete local
follow-up check, build and test passed: 41 governance tests, 932 Rust tests,
zero failures and two explicitly ignored Rust tests. All ten specification-check lanes passed. No package contents or Cargo graph changed after
the successful ten-crate publication dry-run in the setup evidence.

## Failures

[Initial version CI](https://github.com/stevejuma/lapis-ai/actions/runs/38893963408)
failed its ordinary formatting gate on `pnpm-lock.yaml`. Offline pnpm lock
regeneration changes its formatting; version preparation had formatted other
outputs only. The new real preparation assertion reproduced the exact failure,
then passed with the lock added to formatting. No check was disabled.

This installed JJ rejects the legacy `--allow-new` push option. Retrying with
`jj git push --bookmark main` succeeded and preserved commit signing.

## Gaps

Hosted main and updated version-branch product CI are pending. No release tag,
GitHub release or crate was published. The protected `release` environment is
maintainer-reviewed and restricted to version tags; no credential was added.
Initial namespace bootstrap and ten per-crate trusted-publisher registrations
remain prerequisites when the first release is authorized. Signing, notarization,
other native platforms and consumer integration remain separate work.

Gate result: local fix, cold native acceptance, Changesets and Pages passed;
hosted product CI pending.

Next action: push the formatting fix, verify updated hosted checks and retain
Version Packages #1 open and unmerged.
