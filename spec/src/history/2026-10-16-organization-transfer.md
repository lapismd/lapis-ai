# E-ORG-TRANSFER-001: canonical organization repository

Revision: enclosing Jujutsu change `kkwumuxz`.

Milestone review: M0–M4 retain their qualified contracts. M5 remains In progress
for hosted qualification and the separately authorized initial release. The
Version Packages PR must remain open and unmerged.

## Scope and unchanged contracts

At the user's explicit request, the public repository moved from
`stevejuma/lapis-ai` to `lapismd/lapis-ai`. Source history and PR #1 were preserved.
Current package metadata, documentation links, release policy, API guards and
workflow ownership checks now use `lapismd`. Local origin uses the organization
SSH remote. Crate names, versions, source/native pins, wire protocols, provider
behavior and client configuration remain unchanged. Prior historical records
retain their original URLs and results.

LAI-REL-001–007, LAI-GOV-100 and the owning ACP/service/runtime requirements retain
all product gates. Bootstrap/trusted-publisher instructions now use the
organization as owner. No registry token, release tag, package publication or
PR merge was performed.

## Commands

- Inspected Jujutsu status, organization membership/admin role and destination
  repository availability using `gh api` and `gh repo view`.
- `gh api repos/stevejuma/lapis-ai/transfer --method POST -f new_owner=lapismd`.
- `jj git remote set-url origin git@github.com:lapismd/lapis-ai.git`.
- `gh repo edit lapismd/lapis-ai --homepage https://lapismd.github.io/lapis-ai/`.
- Verified the transferred public repository, Pages configuration, release
  environment, Actions token defaults and open PR with GitHub API/read commands.
- `pnpm test:governance`; `pnpm check`; `pnpm test`; `pnpm build`.
- `pnpm package:check --allow-dirty`; `pnpm dev:cargo -- publish --workspace --exclude lapis-compute --locked --dry-run --allow-dirty`.
- Formatting, specification, workflow lint and source searches verify the new
  canonical owner while preserving old historical evidence.

## Results

The live repository is <https://github.com/lapismd/lapis-ai>, PUBLIC with admin
access. [Version Packages #1](https://github.com/lapismd/lapis-ai/pull/1) remains
OPEN with no auto-merge request. The protected `release` environment retained
maintainer review and version-tag restrictions. Actions retained read-only
default tokens and explicit bot PR creation. Pages retained workflow deployment
and now reports <https://lapismd.github.io/lapis-ai/>.

The initial version-branch native jobs both passed: Linux ARM64 CPU and macOS
ARM64 native inference, permissions, bundled resources and Rust clients. See
[the preserved run](https://github.com/lapismd/lapis-ai/actions/runs/38893963408).
Its canonical formatting failure is fixed by the preceding follow-up change.
The transfer cancelled the still-running main jobs; completed Linux native
acceptance was retained. Cancellation is not a product failure.

Organization-bound governance tests passed (41 tests). Full check/Clippy,
builds, ten specification-check lanes and 932 Rust tests passed, with zero
failures and two explicitly ignored tests. Package inventory and publication
dry-run passed for all ten admitted crates with the new owner metadata.
Workflow lint passed. Fresh owner-bound CI and Pages run after the source push.

## Failures

No transfer or owner-bound regression failed. Package inventory initially
refused the uncommitted metadata as designed; its explicit --allow-dirty local
verification then passed. The preceding version-lock
formatting failure and its fix remain recorded in
[follow-up evidence](2026-10-16-release-followup.md). The first Actions runs were
interrupted by the repository transfer and require fresh organization runs.

## Gaps

Fresh organization CI and Pages deployment are pending.
Initial Cargo publication still requires protected namespace-bootstrap credentials
and per-crate trusted-publisher setup for owner `lapismd`. Native-only compute,
other platform/signing gaps and the unmerged version PR are unchanged.

Gate result: public transfer and owner-bound checks passed; final organization
CI/Pages qualification pending.

Next action: push the validated owner updates, verify Pages and version-branch
checks under lapismd, and retain PR #1 unmerged.
