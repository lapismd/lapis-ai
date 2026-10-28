# Reusable filesystem session acquisition

Revision: enclosing Jujutsu change tmtkkozk.

The new `lapis-ai-capture` crate owns bounded approved-root discovery, the
existing source-profile inventory and raw Codex header inspection. It imports
only Serde/Serde JSON, with no JJ, runtime, process, Tauri or network edge.
Public native paths/header bytes have no serialization or Debug representation.
JJ normalization, sidecar formats, source/cursor identities, evidence hashes,
private spools and publication receipts remain publishing-consumer concerns.

## Provenance

Extracted from jj-store `e583ba9f1e47262fa8cde43d7d19773ffc9de4eb`, Apache-2.0.
Original source fingerprints before adapter migration:

- `crates/jj-store-capture/src/codex_discovery.rs`: SHA-256 `ca77b4bf97cb1ae7cf055101b79210cb9fbf211be560833e6048a1855f94d052`.
- `crates/jj-store-capture/src/codex_discovery/scanner.rs`: SHA-256 `0972c1d1d24cadb39cabb6fefea57347d70935bb118ae64ce3061ea78bc01914`.
- `crates/jj-store-capture/src/codex_transcript/file.rs`: SHA-256 `7c65aedd67ca17486ef2ac95b458d79f93669df0ed33ecf35714c1c17818851a`.
- `crates/jj-store-capture/src/profile.rs`: SHA-256 `f05a1548b07cb48cb2996f9dbfd91ef80aefef78dfe92b7974dd85dc9fa39c2f`.

The generic Cursor scanner accepts an explicit consumer hint extension and has
no JJ-specific sidecar name. Native current-binding observations retain unknown
IO separately from known absence/rebinding. Availability fences cannot prove
retirement. File inspection retains exact source bytes for consumer-owned
witness hashing, with the original one-MiB complete-header bound and version
checks. Source library metadata is not permission to read or publish a session.

## Commands

- `CARGO_INCREMENTAL=0 pnpm dev:cargo -- test -p lapis-ai-capture --offline`
- `CARGO_INCREMENTAL=0 pnpm dev:cargo -- test -p lapis-ai-capture --test cursor_discovery --locked`
- `CARGO_INCREMENTAL=0 pnpm dev:cargo -- clippy -p lapis-ai-capture --all-targets --locked -- -D warnings`
- `CARGO_INCREMENTAL=0 pnpm dev:cargo -- tree -p lapis-ai-capture --edges normal --locked`
- `node --test scripts/capture-boundaries.test.mjs`
- `pnpm test:governance`
- `node scripts/package-check.mjs --allow-dirty`
- `CARGO_INCREMENTAL=0 pnpm test`
- `CARGO_INCREMENTAL=0 pnpm check`
- `CARGO_INCREMENTAL=0 pnpm build`
- `pnpm spec:check`

## Results

The exact new-crate suite passes nine cases: four retained filesystem discovery
regressions, Cursor layout/explicit hint behavior, product inventory, two bounded
header/identity cases and the original absence/unknown-IO unit proof. All-target
strict source-crate Clippy passes. Normal dependency inspection contains only
Serde/Serde JSON. The new negative boundary case passes.

The complete `pnpm test` lane passes 27 renderer-client cases, 45 governance
cases and 986 Rust cases across the thirteen-package workspace (two existing
ignored cases remain). Complete `pnpm check` passes specifications, release
metadata, architecture, boundaries, progress/import pins, upstream checks,
format, renderer types and workspace all-target strict Clippy. `pnpm build`
builds the client, complete Rust workspace and runtime examples.

All twelve registry package contents pass the unchanged required-file, private
input and license gates. Coordinated version stays 0.1.1. The complete canonical
check passes ten lanes, including all 45 governance cases and mdBook. Logs live
under ignored `.local/capture/`.

Gate result: passed for the bounded filesystem acquisition foundation.

## Failures

The first new Cursor fixture compared its logical macOS `/var` temporary path
to the correctly canonical `/private/var` candidate. One case failed. The fixture
now compares the expected canonical physical path; the exact case and complete
new-crate suite pass. No production identity check or assertion was weakened.
The standalone initial one-unit proof preceded the added integration tests.
Workspace build retains the existing macOS large `__eh_frame` linker warning;
all build commands exit successfully.

## Gaps

Publishing-consumer routing and full capture recovery remain separate jj-store
M33 work. Versioned profile registration does not qualify automatic Git-folder
capture or every provider source format. Windows physical identity, new native
models/providers, packaged desktop AI behavior and release/publication are not
qualified by these deterministic filesystem tests. Existing platform evidence
and M0–M5 status retain their original scope.

Milestone review: M0–M4 remain complete within their recorded gates; M5 stays
In progress with its existing publication authorization. The bounded discovery
and file-inspection primitive is qualified independently. Consumer migration,
source lease/cursor orchestration and further source formats retain their own
gates. LAI-CAPTURE-001/002 are implemented; runtime, Tauri and provider contracts
are unchanged. No upstream checkout, installed service or user data directory is mutated.

Next action: route jj-store discovery/header adapters through these public
exports, retaining its exact legacy sidecar, normalization and receipt semantics.
