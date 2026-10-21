# Configuration and supervised clients

LAI-CAPTURE-003's transient local JSONL/SQLite acquisition changes no
LAI-CONFIG-003 or LAI-RUNTIME-001/002/004 behavior: it runs no helper, reads no
runtime configuration or credentials, and grants no execution or replay rights.

LAI-RUNTIME-001/003 permit an independent compute command sender to stop active
work before final client cleanup. Successful supervised reap confirms shutdown
even when that prior stop closed the helper input. Reap failures remain failures;
this cannot establish completion of an interrupted model operation.

MCP environment diagnostics follow LAI-CONFIG-003/RUNTIME-004: values are
redacted even in nested protocol Debug output; exact wire delivery is unchanged.

The pinned Lapis ACP source-header addition in
[the capture contract](capture.md) preserves LAI-CONFIG-003 and
LAI-RUNTIME-001/002/004: parsing file evidence launches no worker, reads no
configuration or credentials and grants no runtime admission. Host execution and
unknown-outcome semantics remain unchanged.

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

LAI-TAURI-009 extends native processing with bounded deferred text groups. The
existing event cursor owns every ordered frame in its group; primary control
identities remain unchanged. Facts-triggered text joins the unexposed startup
journal without adding a cursor. Client acknowledgements prune complete groups
and account for all retained payload bytes. Native supervision still drains the
whole callback, and failures preserve unresolved requests without replay.

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

When a packed helper-output burst fills the frame queue, the reader may retain
its one already parsed frame for at most 100ms while the consumer drains. It
reads no additional frame during that handoff. Queue/frame caps and request
deadlines remain unchanged. A closed or still-full queue latches
`output_coverage_lost`; consumer cancellation and supervised shutdown still
release the reader. This implements bounded backpressure under LAI-RUNTIME-001
without suppressing genuine coverage loss or replaying uncertain work.

Compute exposes an independent bounded command sender and one ordered event
receiver, so progress and Stop do not wait for a long operation's completion.
Native hosts may obtain a non-serializable, redacted connection credential for
local or explicitly admitted peer agent launches. Tokens are zeroized on drop.
Existing convenience methods remain supported; hosts choose one event owner.

LAI-RUNTIME-003 additionally exposes a bounded, process-free managed model
inventory from an explicitly selected helper directory. It reports installed
and missing weights, deduplicated sizes and preparation receipts without paths,
credentials, directory creation or inference. Existing installed-model receipts
remain readable. Legacy GGUF receipts may recover general.name from at most
64 KiB of validated managed-file metadata; malformed or oversized headers use
a neutral imported-model label, without changing content identities or receipts.
Missing files never imply installed weights; invalid indexes
remain failures rather than empty libraries. Helper progress is correlated with
the explicit operation and includes stage and known byte counts. Terminal
failures carry allowlisted actionable categories, never raw dependency errors;
hosts distinguish acknowledged failure from transport loss or uncertain effects.

Source acquisition is independently owned by the bounded
[capture library](capture.md), without an execution/runtime edge. Its profile
inventory and read-only metadata discovery neither spawn nor authorize an ACP
worker, and do not change LAI-RUNTIME-001–004.

[Application-tool transport](app-tools.md) owns the injectable MCP engine. It
adds no ACP launch, credential resolution or application authority under
LAI-RUNTIME-001–004; native bridge integration remains a consumer gate.

LAI-TAURI-011 adds optional Rust-owned preparation of validated outgoing
requests before helper dispatch. LAI-RUNTIME-001/002 retain isolated processes,
exact identities and unknown outcomes: the actor owns and drains preparation,
then rechecks close/deadline. The host owns sanitation, storage and receipts;
this callback adds no wire field, launch authority or automatic replay. Native
Tauri command admission remains current even when inbound capture is retained.
