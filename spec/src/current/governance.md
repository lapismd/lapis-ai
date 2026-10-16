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
