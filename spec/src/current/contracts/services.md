# Native services and packaging

LAI-LOCAL-001 exposes the bundled agent's existing reasoning-effort control as
an ACP thought-level configuration option. Each session owns its selected
effort, inherited initially from launch configuration; only idle sessions can
change it. Unknown values and unsupported provider values refuse without
changing state. Prompts capture the session's effort into their own configuration
while retaining the original provider/token source and permission policy.

## Requirements

| ID              | Requirement                                                                                                                                                                                                               |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-LOCAL-001   | The maintained Buzz-derived agent MUST retain provider adapters, permission gates, model discovery, metadata summaries and token-source support with complete Apache attribution and source pins.                         |
| LAI-COMPUTE-001 | Compute MUST retain authenticated loopback inference, explicit model preparation/download/import/start/stop, bounded limits and mutually admitted inference-only peers. Sharing defaults off and startup never downloads. |
| LAI-COMPUTE-002 | Lapis compute MUST preserve the jj-store/compute/1 ALPN, invitation representation and request/response semantics. Mixed-version acceptance MUST use isolated pinned legacy helpers.                                      |
| LAI-PACKAGE-001 | Staged local bundles MUST include production executables, licenses and a TOML manifest with target, versions, hashes and native ABI pins. Test executables, secrets and model weights MUST be excluded.                   |
| LAI-PACKAGE-002 | macOS ARM64 and Linux CPU acceptance MUST separately prove cold native inference and an approved ACP tool operation. Unavailable prerequisites MUST leave their gates pending.                                            |

Initial inference remains non-streaming. No cross-application singleton, public
discovery or automatic cloud fallback is introduced. Consumer migration remains separate. Hosting and release automation follow the
[release contract](release.md). The Cargo and native-runtime pins remain exact.

Cargo packages retain the agent capability manifest and normative corpus inside
`crates/lapis-agent/data/`. Relocation changes no bytes, capability rules, provider
behavior or wire contract (LAI-LOCAL-001); required package inputs travel together.

The agent declares its Reqwest query feature explicitly so provider pagination
compiles outside the workspace. This preserves the already qualified request
behavior and removes an accidental dependency on compute feature unification.

The canonical GitHub owner is now `lapismd`. Service documentation and Cargo
metadata follow this identity; providers, native pins and wire contracts remain
unchanged (LAI-LOCAL-001, LAI-COMPUTE-001–002).

Bundle verification rejects symbolic links in manifests, helper files and native
file path components, including links within the bundle. Consumer startup uses
only the verified regular files (LAI-PACKAGE-001).

LAI-COMPUTE-001 reports explicit operation progress with its request identity,
stage and known byte counts for weights, runtime artifacts and GGUF imports.
Long operations serialize through one busy slot; the slot MUST clear before the
terminal completed or error frame so the next control command is not rejected
while the worker task winds down. Completion follows managed receipt persistence. Imported filenames are optional
display metadata in the existing index. Explicit removal retains managed-path
and shared-file protection. Failures use bounded allowlisted categories without
dependency diagnostics. Model indexes and preparation facts are readable while
off through the inert configuration layer, re-exported by the native runtime;
neither helper acquires a client/runtime dependency. Engine startup still checks
the current native pin and installed weights. LAI-COMPUTE-002 peer ALPN,
authentication and routing semantics are unchanged.
