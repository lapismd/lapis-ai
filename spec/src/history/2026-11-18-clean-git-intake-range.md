# Immutable committed intake range

Revision: enclosing Jujutsu change on main87629985f60e74a793158dc1a442ad3f6a9b9ed0.
Milestone review: M0–M4 retain their status; M5 remains in progress. LAI-GOV-103/104
and LAI-REL-002/008 retain the source, release, security and native boundaries.

The source adapter previously read old facts from one committed parent but used
Git’s default working-tree comparison for changed paths. A clean committed
candidate therefore supplied no managed-note or unexpected-source paths.
The owning adapter now captures one parent and explicitly compares it to Git
HEAD or the JJ working revision. The released validator API is unchanged.

## Commands

- `node --test scripts/dependency-automation/source-intake-git.test.mjs`
- `node --test scripts/dependency-automation/*.test.mjs`
- `pnpm validation:all` with the default bounded Docker worker.
- Final canonical validation, exact-main spec-first, doctor, QMD search,
  formatter and governance checks accompany this slice.

## Results

The valid real released SDK0.2.0→0.2.1 transport fixture passed before repair,
but five invalid clean committed candidates were falsely accepted: runtime
intent, malformed note metadata, an oversized note, a null YAML document and
unrelated source. The new captured-range assertion also failed before repair.
After repair the focused transport suite passes10/10 and the complete source
intake suite56/56, including explicit Git/JJ selection and missing ancestry.
Fixture paths are canonicalized for systems whose temporary root is a symlink.

Full portable validation passed on snapshot8073d39b34f2a511270946b4ef89927037e24e84,
source pack SHA256390b2f6ce1b51defd997575e7d6a69bc2da28bdec7d85525d5aaa747084c6877,
run lapis-ai-run-7eb7c878-a879-41e5-b86a-ef51a9eee006. Its default4CPU/12GiB/2GiB
shared-memory Docker worker ran check, complete workspace tests, build,
workflow/package checks and all11 canonical lanes. Renderer43, source-intake56
and governance71 cases passed. The later test-path canonicalization and this
record require their focused and document-consuming checks, not another Rust
compile. No production code changes followed that full portable run.

## Failures

The first fixture materializer omitted the empty Git refs directory, causing
seven fixture-setup failures. Those were corrected before the measured valid
baseline (one pass and six failures) and final transport runs. No assertions,
source scopes, note bounds, ignored tests or security gates were relaxed.
The first record validation detected its missing book SUMMARY entry; that
documentation wiring was added before final canonical acceptance. Long inline
evidence links initially expanded the formatted verification table past its
byte budget; compact reference links preserve that same evidence within budget.

## Gaps

This controlled historical released-pair fixture is not a current bot update.
Portable success does not qualify native macOS/ARM/model or release execution.
Manifests, locks, bootstrap tuple, exact recipe and active security decisions
are unchanged. No publication, dependency exception or release authorization
is introduced.

Next action: hosted exact-source CI, ordinary merge and fresh live coordinator
acceptance; preserve all independent product/native/publisher gates.
Gate result: passed for the recorded portable and source regression scope.
