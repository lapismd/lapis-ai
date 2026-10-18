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

LAI-RUNTIME-001/002 retain isolated clients, exact identities and unknown outcomes
with the optional native frame processor in LAI-TAURI-007. A runtime-owned startup
job drains admitted processing before cancellation cleanup or handoff. The host
owns sanitation, durable capture and receipt recovery; the runtime adds no
storage or application-history authority.

ACP client drop defers supervised stdin closure and reap to the owning Tokio
runtime. Cancelling startup must let the control helper retire its separately
grouped provider rather than killing the helper first. Process shutdown retains
the child owner across await points and never signals a process-group identifier
after its leader has been reaped (LAI-RUNTIME-001).

Native adapters may use cancellable session startup and await its cleanup when
an admission is revoked. This extends the same supervised client contract; it
does not add replay, process selection or application-history authority.

Host integrations may select bounded `ProcessLimits` and prepare launches from
explicit native command values. This copies no implicit parent environment.
`AcpClient::start_prepared_with_options` retains admitted launch flags while
fixing Control V3; competing protocol selectors are rejected. The existing
session adapter retains its fixed argv. Readiness is available to native hosts.
Agent discovery and metadata also accept prepared launches with independent
input/output bounds and host deadlines (LAI-RUNTIME-001–004).

Compute exposes an independent bounded command sender and one ordered event
receiver, so progress and Stop do not wait for a long operation's completion.
Native hosts may obtain a non-serializable, redacted connection credential for
local or explicitly admitted peer agent launches. Tokens are zeroized on drop.
Existing convenience methods remain supported; hosts choose one event owner.

Source acquisition is independently owned by the bounded
[capture library](capture.md), without an execution/runtime edge. Its profile
inventory and read-only metadata discovery neither spawn nor authorize an ACP
worker, and do not change LAI-RUNTIME-001–004.
