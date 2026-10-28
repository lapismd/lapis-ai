# Read-only local session pages

Revision: enclosing Jujutsu change.

LAI-CAPTURE-003 adds transient complete-record JSONL pages with admitted physical
root/file identity and SHA-256 completion fences. It also adds OpenCode SQLite
metadata and selected-session message pages using a read-only, query-only,
trusted-schema-disabled connection and a pinned read transaction. Raw values
remain native-only. Hosts supply roots and own metadata caches, privacy, import
identity, UI and lifetime. No source write, process launch, durable cursor,
provider access, JJ dependency or publication authority is introduced.

LAI-CAPTURE-001/002 retain their root/header/identity requirements. An explicit
root can select an already indexed child without rescanning unrelated folders;
that operation rechecks containment and physical identity. Codex product/header
qualification remains unchanged. Cursor and OpenCode structural import policy
is consumer-owned; an observed format is not a qualified execution profile.

LAI-ARCH-001 now narrowly admits optional `rusqlite` behind `opencode-sqlite`
for this read-only acquisition crate. `session-pages` gates JSONL/SHA-256;
default features remain empty and preserve the existing Serde-only graph.
The public API exposes no connection or SQL command capability. ACP/config,
network, other database, runtime and sibling-checkout exclusions remain enforced.
The boundary regression retains all forbidden edges and qualifies this one new
source dependency. SHA-256 is local snapshot evidence, not durable authority.

## Commands

- `pnpm dev:cargo -- test -p lapis-ai-capture --test local_sessions` (initial before target)
- `pnpm dev:cargo -- test -p lapis-ai-capture --features opencode-sqlite --locked`
- `pnpm dev:cargo -- clippy -p lapis-ai-capture --all-targets --features opencode-sqlite --locked -- -D warnings`
- `pnpm check:boundaries`
- `pnpm dev:cargo -- fmt --all`
- `pnpm dev:cargo -- test --workspace --features lapis-agent/test-support --locked` (initial full run)
- `pnpm dev:cargo -- test --workspace --features lapis-agent/test-support,lapis-ai-capture/opencode-sqlite --locked`
- `pnpm dev:cargo -- test -p lapis-ai-runtime --test lifecycle output_overflow_latches_coverage_loss --locked`
- `pnpm check:architecture`
- `pnpm dev:cargo -- clippy --workspace --all-targets --features lapis-agent/test-support,lapis-ai-capture/opencode-sqlite --locked -- -D warnings`
- `pnpm exec prettier --write scripts/boundaries.mjs scripts/capture-boundaries.test.mjs`
- `pnpm spec:check`
- JJ Store: `pnpm dev:cargo -- check -p jj-store-capture --no-default-features --locked`

## Results

Focused acquisition tests pass: bounded pages, stable digest, complete rows,
source growth/replacement, cancellation, read-only SQLite, schema/symlink
rejection and isolation from later source writes. All source-crate tests and
strict Clippy pass; the fourteen-package boundary audit passes. The existing JJ Store consumer checks with `--locked` and no lockfile changes.
The final explicit-feature workspace suite passes 1067 tests including doc tests.
Strict workspace/all-target Clippy and final specification results are recorded
below; source changes do not modify renderer package exports. Logs are `/tmp/lapis-local-session-*.log`.

## Failures

The before target has two compile failures for the absent session acquisition
module and SQLite dependency. Four initial new tests pass after implementation;
the added WAL isolation case and the complete source crate pass afterward.
One final full workspace attempt encountered the existing runtime overflow
race (`output_overflow_latches_coverage_loss`: sequence 0 arrived before the
coverage-loss latch). Its exact rerun passes. No runtime/test edit was made.
The first specification attempt stopped at formatting of the two changed
boundary scripts; formatted them and repeated the same gate.
No assertion, source identity, complete-record fence or authority boundary was
waived. The new dependency allowance is a governed read-only source capability,
not an exception to conceal an existing defect.

## Gaps

Notes' metadata index, sanitization, cancelled/lost-read cleanup and live import
UI require their own qualification. SQLite source-schema compatibility is
structural and read-only; this does not qualify an OpenCode execution version.
No package, application or service is published, installed or restarted here.

Gate result: source acquisition, default JJ Store consumer and complete source
workspace tests pass. `pnpm spec:check` passes all ten lanes, including 45
governance tests, formatting, fourteen-package boundaries and spec-first.
Desktop native/renderer/cold acceptance is tracked separately in its E-AI-121.
Logs: `/tmp/lapis-local-session-workspace-qualified.log`,
`/tmp/lapis-local-session-final-clippy.log`,
`/tmp/lapis-local-session-workspace-clippy.log`,
`/tmp/lapis-local-session-spec-qualified.log`,
`/tmp/jj-local-session-consumer.log`.

Milestone review: M0–M4 retain their existing completed gates and M5 remains
In progress. This additive acquisition work changes no original milestone,
release, provider, model or other-platform acceptance gate.

Next action: finish source checks and the ordinary Desktop consumer's native,
renderer and cold Storybook acceptance.
