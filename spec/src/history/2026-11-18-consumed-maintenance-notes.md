# Consumed maintenance notes

Revision: enclosing Jujutsu change on52a8236d3407136042e1165cf87016015a5269c5.
Milestone review: M0–M4 retain Done; M5 retains In progress. LAI-GOV-100/103/105
and LAI-REL-008 retain source ownership, portable/native separation and release
authorization. The original version-runtime qualification, private release
recipe and bootstrap tuple remain byte-identical.

## Commands

- Actual CLI2.31.1 `version` in a disposable source-derived private workspace
  with a genuine controlled runtime patch and the generated empty managed note.
- `node --test scripts/dependency-automation/note-consumption.test.mjs`
- `node --test scripts/dependency-automation/*.test.mjs`
- Default bounded Docker `pnpm validation:all`, canonical spec-first, doctor,
  QMD discovery and the required format/architecture/progress checks.

## Results

The actual CLI removes the managed note and manual release note, updates both
private versions and their changelogs, and leaves the complete SDK dependency
fact tuple unchanged. Before repair, the production-default source reader
throws ENOENT at the deleted note. After repair the same command accepts the
same candidate without rewriting any candidate bytes.

The immutable actual output fixture is bound by SHA256
`7b93c5b11aa52cf3e709b98c2840b94b539f8339677ada42bf80cae437d86a67`.
Both Git and JJ diff parsing retain the deleted paths. Source-only reader
injection in tests supplies the recorded immutable baseline; the production
CLI still uses its unchanged captured revision-range reader.

Focused cases pass16/16 after12 passed/4 failed before repair; the full source
intake suite passes72/72. Tests cover absent-only lifecycle detection, the
complete fact tuple, initial bootstrap, genuine released SDK intake, malformed/
runtime/null/oversized present notes, raw namespace/count inventory, contradictory
consumed inventories, directories and live/dangling/external symlinks.

The loader skips only `lstat` absence. Existing files retain the strict regular
file, canonical path,8192-byte and UTF-8 checks. The verifier excludes an
explicitly absent note from byte validation only when baseline and candidate
dependency facts, including policy and registry integrity, are identical.
All present notes still validate before bootstrap or same-version returns.

Full portable validation passes on snapshot
`d26ec07e9c5dac8e91665110476224012541ca58`, source pack SHA256
`092ab68ba1fac29bb7d9e1d5f06ea8bc8d29a84c9fdba8271cd74a05c2e96860`,
run `lapis-ai-run-d70020d3-1b6a-4a2d-9a8e-c5062c1bd887`. The unchanged
qualified image runs Node24.15.0/pnpm10.34.5/Rust1.99.0 with4CPUs,12GiB,
2GiB shared memory and dropped capabilities. Complete checks, workspace tests,
build, workflow/package checks and all11 canonical lanes pass:1,078 Rust cases
with two inherited ignored,43 renderer,71 governance and72 source-intake cases.
The separate hosted-style Node gate passes, including explicit base/head
spec-first (four protected files/two chapters), client build/tests and workflow
linting. Doctor passes10 checks with its existing portable-alias warning; QMD
semantic discovery succeeds and registry-latest SDK0.2.1 matches the source.
Only the completed results record changes after that full portable run; its
formatter and canonical consumers are rechecked without repeating Rust tests.

## Failures

The original actual reproduction failed once with ENOENT. Four focused failures
exposed the two missing lifecycle boundaries. The first architecture check
found302 counted caller lines; note loading moved to its existing owning helper
instead of adding an exception. The separate native Node gate reached workflow linting but the host lacked
`actionlint`; the exact1.7.12 binary from the already-qualified image in an
owned temporary tool directory makes that same gate pass. No repository tools,
assertions or budgets were weakened.

## Gaps

This is controlled actual Changesets consumption, not a live dependency-bot
version PR or publication. The empty-only private whole-version macro still
returns false without consumption; its original qualification is unchanged.
No manifest, lock, Cargo input, source policy, bootstrap, qualified recipe or
security decision changes. Native/model/platform and release acceptance remain
separate from portable fixture evidence.

Next action: hosted exact-source CI and ordinary merge, followed by the owning
coordinator's live acceptance. No source exception or publication is authorized.
Gate result: passed for the recorded deterministic portable scope.
