# Configuration and supervised clients

## Requirements

| ID              | Requirement                                                                                                                                                                                                                                                            |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-CONFIG-001  | Explicitly selected version-1 TOML MUST configure runtime paths, ACP launch/catalog/tools, agent provider/environment references and compute limits/peers. Unknown fields and unsupported versions MUST fail before affected startup.                                  |
| LAI-CONFIG-002  | Explicit API or CLI values MUST override TOML, which overrides retained legacy environment inputs and defaults. Relative paths MUST resolve against the selected file. No working-directory configuration discovery is permitted.                                      |
| LAI-CONFIG-003  | Reading configuration MUST NOT spawn, download, authenticate or infer. Persisted configuration MUST contain credential references rather than secret values; secret resolution occurs only at launch and diagnostics MUST omit values.                                 |
| LAI-RUNTIME-001 | Native clients MUST own isolated child environments, working directories, bounded frame transport, readiness and supervised shutdown. Compute caches MUST remain beneath its selected managed directory. They MUST NOT mutate parent environment or current directory. |
| LAI-RUNTIME-002 | ACP clients MUST preserve request/session identities and exact permission option IDs. Timeouts and lost acknowledgements MUST remain unknown outcomes without automatic prompt replay.                                                                                 |
| LAI-RUNTIME-003 | The agent client MUST support bounded model discovery and metadata requests. Compute clients MUST expose configuration, explicit model lifecycle, events and pairing without conversation policy or durable orchestration.                                             |
| LAI-RUNTIME-004 | Host credential callbacks and private token-source IPC MUST remain host-owned. Application secrets, credentials and token sources MUST NOT appear in serialized configuration, debug output or logs.                                                                   |

Library publication and resource manifests use the coordinated Cargo package
version under the [release contract](release.md). Configuration and wire version
numbers remain independent. The bundle producer must read the coordinator version
rather than retain a literal source-baseline version.

Crate documentation and publication metadata follow `lapismd/lapis-ai` after
the ownership transfer. Runtime, configuration and bundle semantics are unchanged.

The owned ACP session adapter adds typed correlation and bounded reconnect state
above the raw client (LAI-RUNTIME-001–002); it does not change control-v3 or add
conversation policy. Rust hosts may start an ACP client with a `PreparedLaunch`
after native credential resolution; the client fixes its control-version argv.
The [adapter contract](tauri.md) governs this additional lifecycle boundary.

ACP client drop defers supervised stdin closure and reap to the owning Tokio
runtime. Cancelling startup must let the control helper retire its separately
grouped provider rather than killing the helper first. Process shutdown retains
the child owner across await points and never signals a process-group identifier
after its leader has been reaped (LAI-RUNTIME-001).
