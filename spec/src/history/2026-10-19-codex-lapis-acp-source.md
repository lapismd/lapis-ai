# Pinned Codex Lapis ACP source headers

Revision: enclosing Jujutsu change yvosmmot.

Notes' ordinary import review found no candidates. A native-only header inventory
of recent files showed Codex CLI 0.156.1 launched by Lapis ACP with the exact
`source=vscode` / `originator=lapis-acp` pair. No native paths, session IDs,
headers or source bodies were logged. Two newer Desktop alpha files remain
outside the qualified product inventory.

LAI-CAPTURE-002 adds that exact pair to the existing Codex CLI profile. The
product version remains 0.156.1. Other adapter names, CLI/adapter combinations
and Desktop alpha versions still refuse. LAI-CAPTURE-001 physical root/file
identity, bounded complete header inspection, raw native-only values and zero
execution/publication authority remain unchanged. Hosts retain normalization,
privacy, context and durable custody. The shared normalizer and record format
are unchanged; this adds no renderer parser or duplicate consumer logic.

## Commands

- `pnpm dev:cargo -- test -p lapis-ai-capture --test lapis_acp_surface --locked --offline`
- `pnpm dev:cargo -- fmt --all`
- `pnpm dev:cargo -- test -p lapis-ai-capture --locked --offline`
- `pnpm dev:cargo -- clippy -p lapis-ai-capture --all-targets --locked --offline -- -D warnings`
- `pnpm check:architecture`
- `pnpm check:boundaries`
- `node --test scripts/capture-boundaries.test.mjs`
- Scoped Prettier, `pnpm check:progress`, `pnpm spec:first`, `pnpm spec:validate`

## Results

The exact source regression first fails one case because the observed Lapis ACP
pair is rejected. After the narrow profile change, all ten source-crate tests
pass, including exact accepted source bytes, wrong-version/adapter rejection,
complete bounded headers, symlinks, physical replacement and metadata discovery.
Strict all-target source Clippy, architecture limits, fourteen package boundaries
and the one dependency-boundary regression pass. The crate remains dependent
only on Serde and Serde JSON in its normal graph. No runtime or application
configuration is read or changed.

Notes owns its actual-file and cold import consumer acceptance in E-AI-055;
jj-store owns its transient snapshot consumer checks in the enclosing progress
record. The crate README and full capture guide describe the supported pair and
retain the distinction between source evidence and execution authority.

Gate result: passed for the pinned native header profile.

## Failures

Spec-first initially requires runtime/governance reviews; add explicit
unchanged-contract confirmation with owning requirement IDs.
The pre-change exact regression has one failure. Its positive expected result
and all negative cases remain unchanged after the fix. No physical identity,
version, complete-header, privacy or authority assertion is relaxed. Logs are
ignored `.local/import-acp-before.log`, `import-acp-qualified.log`,
`import-acp-clippy.log` and the corresponding architecture/boundary receipts.

## Gaps

The user prioritizes usable local Codex and defers broader verification. No
complete workspace suite, release/package publication, additional provider,
Desktop alpha parser or other-platform qualification is claimed by this focused
source slice. Original M0–M5 definitions and release authorization remain intact;
M3 retains its existing qualification and M5 stays In progress. This profile
addition does not qualify automatic capture, JJ publication or native model
execution. No source transcript or credential is edited.

Milestone review: M0–M4 retain their original completed gates and M5 remains
In progress. Only selected source evidence extends; original dependencies,
requirements, completion gates and next actions remain unchanged. The consumer
still has to qualify its read admission, privacy and local snapshot lifecycle.

Next action: rebuild Notes' native consumer, prove the observed Lapis ACP file
profile through its cold local import, then refresh the ordinary review app.
