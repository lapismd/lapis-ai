# ACP compatibility and upstream port

The ACP-only contracts below preserve the decomposed jj-acp port. The maintained
Buzz agent is a separate copied implementation, attributed in
`docs/agent-upstream.md`; it is outside the no-verbatim-ACP-source boundary.

## Requirements

| ID             | Requirement                                                                                                                                                           |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-COMPAT-001 | The upstream port source MUST be pinned by repository, path and commit in `docs/upstream-lock.md` together with a per-file disposition map.                           |
| LAI-COMPAT-002 | Adopted upstream attribution MUST be recorded in `docs/licenses/NOTICE.md`, and upstream repository-level agent instructions MUST NOT be copied into this repository. |
| LAI-COMPAT-003 | Upstream drift MUST be detectable by a script that materializes the pinned source into ignored scratch storage and verifies recorded digests.                         |
| LAI-COMPAT-004 | The Rust toolchain MUST be pinned and formatting MUST be checked by both the normal and the specification check lanes.                                                |
| LAI-COMPAT-005 | Integration tests MUST reach a terminal outcome without a model provider, a network endpoint or a credential.                                                         |
| LAI-COMPAT-006 | Qualification against a real installed runner MUST be recorded separately and MUST NOT be inferred from fixture-peer tests.                                           |
| LAI-COMPAT-007 | Every disposition in the port map MUST be one of ported, ported-with-changes, dropped or not-ported, and a dropped region MUST name the reason.                       |

## Pins and drift

`docs/upstream-lock.md`, `docs/upstream/disposition.json` and
`docs/upstream/lock.json` retain the original ACP design/port audit against
block/buzz at 2664d14316790a57ea44b3f97c57440b70e43436. The Lapis AI import
manifest separately pins jj-acp, JJ Store, maintained Buzz and Mesh sources.

`node scripts/upstream-sync.mjs --self-test` checks the original ACP pin and
map offline. Explicit `--fetch` materializes the pinned source only under
`.local/upstream` and compares its digests. `--write` changes the recorded lock
and requires its own governed review. No normal build needs this source tree.
The closed disposition vocabulary remains ported, ported-with-changes, dropped
and not-ported. Repository-level upstream agent guidance is never adopted.

The deterministic ACP suite covers framing, configuration, bounded queues,
permissions, deadlines, crashes and process cleanup without provider access.
Installed cloud adapters remain independently unqualified here. The native Lapis
agent and local-model acceptance are recorded separately; the original source's
live Codex/Cursor and JJ Store host evidence is not inherited acceptance.

`docs/imports/acp-release-reference.md` is historical source context. This new
repository has no publication, signing or hosted release qualification.
