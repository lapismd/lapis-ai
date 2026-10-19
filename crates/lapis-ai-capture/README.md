# lapis-ai-capture

Read-only discovery of external agent sessions from explicitly approved native
locations. It provides versioned source profiles, physical root/file binding and bounded Codex header inspection, plus
a resumable metadata scanner for Codex and Cursor layouts.

Resolve locations with `discovery::resolve_codex_roots`, open an
`ApprovedCodexRoot`, then call `CodexDirectoryScanner::step` with explicit work
and candidate limits. A candidate is metadata only: the host still admits
activity, reads the supported format, sanitizes and publishes it.

The crate has no JJ, agent-runtime or application dependency. It never reads
credentials, chooses repositories or persists transcript data. Current physical
identity qualification is Unix-only. Source registration does not qualify a
consumer's automatic capture profile.

See the [capture guide](../../docs/capture.md) and
[canonical contract](../../spec/src/current/contracts/capture.md).

Codex CLI 0.156.1 files launched by Lapis ACP use the exact `vscode` /
`lapis-acp` header pair and share the CLI profile. This does not admit other
adapter labels or newer Desktop alpha versions. Imported data remains
untrusted source evidence, never execution authority.
