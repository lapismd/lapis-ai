# Qualified version runtime guard

Revision: enclosing Jujutsu change.

Milestone review: M0–M4 unchanged; M5 remains in progress. This tooling slice
adds LAI-REL-008 without changing runtime, native/model/platform or release gates.

## Contract review

The shared guard is pinned to Lapis CI
`2f90eb7a035f841f0757d3a2cffc959cbecdfff0`. It runs after existing locked Cargo
bootstrap and before the repository-scoped dependency App is minted. Its public
source-owned qualification file retains the exact 17 input bindings from the
successful [whole-recipe proof](https://github.com/lapismd/lapis-ci/actions/runs/43814544020).
Node22, CLI2.31.1, pnpm10.34.5, Rust1.99.0, the real `pnpm release:version`
command, offline operations, App scope and dispatch/tag credentials are retained.
No package manifest, lockfile, Changeset, Cargo version or publication changes.

## Whole-recipe receipt audit

The complete downloaded artifact matches GitHub's archive digest and the retained
receipt bytes. The main-only manual workflow succeeded on the pinned helper;
its AI native step executed, while the alternative Node24 step was unselected.
The immutable AI source is `3e29e16f15652ba239d256af49ffcba314355a5e`.

All 77 source input hashes and 17 guard bindings matched the immutable3e source
before fixture repairs or guard adoption.
Actual Node22.23.5, pnpm10.34.5 and official Rust1.99.0 executable hashes match the
reviewed case. Real release checking and locked Cargo bootstrap succeeded before
the unchanged offline source recipe. The private coordinator/client prepare
0.2.0→0.3.0 only in the disposable fixture; all 15 Cargo packages and internal
ranges/lock entries remain coordinated. Both changelogs change. Sixteen READMEs
were reviewed; the root README contains the sole applicable install-version
marker and updates. Within that proof, all59 canonical files, including milestone
evidence, remain byte-identical. The genuine ancestor note is consumed as historical validation
input with `currentReleaseIntent=false`; no Deno, writer App or publication occurs.
The operational receipt is retained outside tracked source rather than embedded
in the public specification.

This adoption changes workflow/specification metadata and adds a regression;
its exact bound recipe files, root manifests/lock and configuration still match
the qualified source. Source integration checks remain distinct from that proof.

## Commands

- Official Node22.23.5 `node --test scripts/version-runtime-guard.test.mjs`:
  2 failed before adoption, then 2 passed / 0 failed after adoption.
- Default-Docker `pnpm test:governance` on unchanged main: 69 passed. Main plus
  the new tests before adoption: 69 passed / 2 failed at the intended assertions.
- Default-Docker `pnpm test:governance` after adoption: 71 passed / 0 failed.
- The immutable pinned helper accepts these exact qualification inputs under
  official Node22.23.5, independently of the disposable whole-recipe proof.
- Default-Docker `pnpm spec:check`: all 11 lanes passed.
- Explicit Node-only `node scripts/ci-spec.mjs` on the source base/head passed:
  validators, spec-first, book, governance, formatting, renderer check/build,
  43 renderer tests and actionlint1.7.12.
- `pnpm audit --json`: 109 dependencies, zero findings.
- Complete default-Docker `pnpm validation:all` passed after the separately
  evidenced [fixture repair](2026-11-16-acp-peer-fixture.md): Rust1078/0 with two
  existing explicit ignored entries, renderer43, governance69, Clippy/check/build,
  actionlint, metadata/contents for14 publishable crates and spec11.
- Final guard integration: official Node22 exact target2/2; pinned immutable
  helper accepts all17 bindings; Docker specification all11 lanes including
  governance71/71; exact Node-only prepush gate including renderer43 and
  actionlint passed. The fully qualified fixture source remains byte-identical
  to the preceding commit; only workflow/specification and guard tests change.

## Results

The original complete integration run exposed writable-script races in two
owned fixture families. Their separate source commit retains all production
contracts, exact assertions and timeouts, and the complete workspace integration
then passes. All17 bound recipe inputs and root manifest/lock bytes remain
unchanged. Hosted adoption checks remain separate.

The new checks bind the reviewed qualification digest, source file hashes,
existing Node/Rust/native setup, exact writer command and required step ordering.
No existing product, security, interaction or native assertion is weakened.

## Failures

The initial host test process could not import YAML from a stale ignored
installation. Clean main directly declares and locks YAML2.9.1; its fresh Docker
suite passes. A frozen host dependency refresh using an owned task store restores
the declared graph without changing package/lock bytes. This is not a source
parser defect and no dependency/API workaround is introduced. The two intended
pre-adoption failures are recorded separately after the assertions execute.

## Gaps

The existing Spec Validator0.2.0 doctor expects `spec/src/verification.md` although
the owning configuration uses `current/verification.md`; it does not pass.
QMD is deliberately disabled and semantic search reports its documented rg
fallback. No broad validator change is folded into this slice.
Hosted adoption checks and the next eligible real generated version PR remain
separate acceptance. Native/model/platform and publication gates stay unchanged.

Next action: review the source PR and require its normal exact-source checks.
Do not manufacture version intent or publish to test this guard.
