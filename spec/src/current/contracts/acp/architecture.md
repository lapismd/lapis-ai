# ACP architecture

These contracts govern the six ACP crates within the larger Lapis AI workspace.
The native agent and compute services have separate provider/network authority
under [services](../services.md). Their dependencies do not belong in ACP.

## Requirements

| ID           | Requirement                                                                                                                                                                                         |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-ARCH-001 | The repository MUST be a Cargo workspace of layered crates whose internal dependencies match the matrix enforced by `scripts/boundaries.mjs`.                                                       |
| LAI-ARCH-002 | `lapis-acp` MUST NOT depend on Nostr, transport, HTTP, TLS or database crates, and the boundary checker MUST fail closed when one appears.                                                          |
| LAI-ARCH-003 | Production sources MUST stay at or below 300 counted lines and test sources at or below 500, with exceptions only as explicit baseline entries.                                                     |
| LAI-ARCH-004 | ACP production modules MUST NOT define test functions inline or import the testkit crate; unit tests MUST use a `tests.rs` sibling module and integration tests MUST use a crate `tests` directory. |
| LAI-ARCH-005 | The controlling application MUST own runner selection, working directory, credentials and prompt content; `lapis-acp` MUST NOT derive them on its own.                                              |
| LAI-ARCH-006 | A crate MUST own its behaviour; the CLI crate MUST compose and MUST NOT reimplement protocol, scheduling or transport logic.                                                                        |

## Ownership and dependency matrix

The protocol crate owns ACP frames on supplied streams. The catalog crate owns
runner data and executable resolution. The runner owns subprocess/session/turn
lifecycle. Control owns task records, queues and the V1–V3 host protocols. The
CLI composes them and the testkit owns deterministic peer fixtures.

`scripts/boundaries.mjs` reads locked Cargo metadata, rejects unknown packages,
unauthorized internal edges, production testkit imports, sibling paths and
forbidden dependencies in ACP or the inert configuration crate. Runtime clients
may use authenticated loopback HTTP for compute; ACP itself remains stdio-only.

`scripts/architecture.mjs` checks new production Rust and scripts at 300 counted
lines and tests at 500. The maintained agent import has exact frozen exceptions
in `scripts/architecture-baseline.json`; their original measured sizes and
source hashes are in `docs/imports/source-manifest.json`. Exceptions may shrink,
never grow. New tests live in separate files. ACP test functions remain outside
production modules. Existing agent tests retain their pinned source placement.

The workspace coordinates versions and commits one lockfile. Every package is
unpublished. `lapis-acp` is the binary/Cargo package; its source directory and
library are `lapis-acp-cli` and `lapis_acp_cli`. The legacy `JJ_ACP_*` launch and
`JJ_ACP_TASK_*` reserved namespaces are retained for compatibility. Requirements
map from the pinned source's JAC identifiers to LAI identifiers without changing
their numeric suffixes. This repository is named `lapis-ai`.

## Qualification

The optional `tauri-plugin-lapis-ai` crate depends on `lapis-ai-runtime`, Tauri
and bounded host/channel support. No ACP crate acquires a Tauri dependency.
The dependency matrix and coordinated release inventory include this adapter;
application admission and durable conversation state remain consumer-owned.

The verification table and milestone ledger own current acceptance. Source
repository live-runner, host, publication and platform results are historical
references. They are not acceptance for renamed helpers. No consumer or public
release is included in this foundation.
