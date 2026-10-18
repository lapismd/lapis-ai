# Governance

## Requirements

| ID          | Requirement                                                                                                                                                                                                             |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-GOV-100 | Protected implementation, interfaces, tooling and guidance MUST update mapped owning contracts or record a fresh justified unchanged-contract confirmation, validation and milestone review in the same Jujutsu change. |
| LAI-GOV-101 | Authored specification Markdown MUST remain within 400 physical lines and 32768 UTF-8 bytes. Each requirement MUST have exactly one verification row.                                                                   |
| LAI-GOV-102 | New production Rust and scripts MUST stay within 300 counted lines and tests within 500. Exact imported exceptions MUST have pinned provenance and MUST NOT grow.                                                       |
| LAI-GOV-103 | Evidence MUST record commands, results, failures, affected formats, gaps and next action. Fixture, native inference, interoperability and platform evidence MUST remain distinct.                                       |
| LAI-GOV-104 | Verification MUST start at the exact affected boundary and widen only for shared infrastructure or multiple affected packages. Native and platform gates MUST NOT pass from compilation alone.                          |

The logical ledger lives under current/progress. Evidence lives under history.
Committed records are append-only. Sources and existing user data remain untouched.

Completion checks require linked command/result/failure/gap evidence and completed
dependencies. Source pins, composed imports and non-growing original file counts
are validated separately from platform qualification. Hosted CI and broader
release/native profiles retain their own explicit gates.

Recognized same-repository Dependabot/Changesets automation may omit only
same-change spec-first bookkeeping. Recognition validates bot identity, reserved
branch, base main, repository and the exact checked/merged revision. Lookup errors
fail closed. Canonical integrity, evidence integrity, tests, native acceptance,
package checks and builds remain required; version preparation invents no evidence.

Version preparation formats its regenerated pnpm lockfile before opening the
version PR. A real preparation regression checks that output against the ordinary
formatting gate; bot exceptions never suppress formatting (LAI-REL-003–004).

The canonical release identity is `lapismd/lapis-ai`; automation refuses other
owners. Historical hosting evidence retains original URLs and is supplemented
with transfer evidence rather than rewritten.

The consumer-driven ACP adapter uses the existing runtime crate and a test-only
CLI-to-runtime dependency. Workspace versions, release authorization, dependency
boundaries and the six foundation/release milestones remain unchanged. Its
additional source-slice ledger distinguishes Rust qualification from Tauri IPC
and packaged consumer acceptance; a Changeset records the public API addition
without preparing or publishing a release.

The optional Tauri adapter is a twelfth governed Cargo package. Its own module,
permission and host-policy tests follow the same non-growing source budgets.
The release inventory adds its portable package without changing publication
authorization; version preparation remains a disposable development regression.

The private renderer client lives in `packages/ai`, exposes built public exports
and participates in normal TypeScript check/test/build lanes. Its optional Tauri
entry is separate from the runtime-independent root. Changesets fixtures copy the
workspace and client manifest, and coordinated preparation updates its version
without adding npm publication or modifying canonical evidence.

Adapter teardown qualification includes cancelled caller waits and overlapping
window revocation/global exit. Registry-owned retirement preserves the existing
scope and capacity boundaries; it changes no wire or release authority.

Host-prepared clients and the shared development bundle verifier retain the
existing runtime/service boundaries and LAI-GOV-100–104 evidence rules. Native
hosts own admission and durable state; Node packaging tools remain development
only. The API Changeset does not authorize version preparation or publication.

The session acquisition extraction adds the coordinated `lapis-ai-capture` Cargo
package with explicit dependency boundaries, a crate README and independent
source/profile tests. Existing source provenance, architecture budgets, runtime
qualification and publication gates remain enforced.

Deferred native text groups extend the shared runtime, scoped Tauri host and
typed renderer client under LAI-TAURI-009. Their complete affected source suites
and public package inventories are required; fixture callbacks do not qualify a
packaged native consumer or Notes-owned durable publication. Optional group
fields retain default wire compatibility, and native evidence records whole
group consumption, acknowledgement and uncertain outcomes separately.
