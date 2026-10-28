# Injectable application-tool transport

## Requirements

| ID            | Requirement                                                                                                                                                                                                       |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-TOOLS-001 | The reusable MCP stdio engine MUST accept an injected tool handler and immutable bounded tool snapshot without selecting executables, roots, credentials, storage or application policy.                          |
| LAI-TOOLS-002 | Framing, identities, total requests and concurrent calls MUST remain bounded. Duplicate request identities, invalid frames and unsupported methods MUST NOT dispatch a tool or silently lose correlation.         |
| LAI-TOOLS-003 | Cancellation, deadline, input closure and owner drop MUST release owned call futures. Uncertain effects MUST NOT be described as proven failure or replayed automatically.                                        |
| LAI-TOOLS-004 | Only a snapshot-listed tool MAY reach the injected handler. Native hosts MUST separately authenticate IPC and revalidate their original admission, registry identity, scope and approval policy before execution. |

`lapis-ai-app-tools` supplies a reusable MCP stdio engine, adapted from the
existing coding-tool server's protocol surface. A host supplies the reader,
writer, server identity and tool handler. It can embed the engine in a bundled
Rust helper. Construction does not create a process, network listener, file,
credential, application capability or durable transcript.

`Config::validate` exposes the same snapshot/budget check to native hosts before
they allocate transport resources. It adds no launch or admission authority.
The engine captures the tool list once and validates its unique names, schemas
and bounded serialized size. It handles initialize, ping, tools/list,
tools/call and cancellation notifications. Tools are available only after
initialization. Request IDs retain string or signed/unsigned integer identity;
floating, empty, oversized or reused identities refuse dispatch. No request is
retried. Calls beyond capacity reject immediately without waiting in a queue.

Defaults are a 1 MiB input/output frame limit, eight active calls, 4096 total
requests and a 60 second call deadline. Hosts may lower these bounds. A separate
owned writer queues at most maximum-calls plus one bounded responses and keeps
input/EOF/cancellation live while output stalls. Output uses the same deadline;
queue saturation or writer failure retires without dispatch/replay. Responses
preserve the original JSON-RPC identity. Invalid JSON receives a stable parse
error; oversized/unterminated frames retire rather than truncate. Unsupported
methods and unlisted tools do not reach the handler. Tool result payloads remain
host-owned, including their privacy and schema checks.

A call receives a cancellation observer. A cancelled or expired call reports an
unknown execution outcome, never that an effect did not happen. Input closure
and engine drop abort all owned call futures; a dropped handler future must
release its transport ownership and notify its separately owned execution.
The engine owns no detached execution and cannot certify application-side drain.
Native admission, authenticated IPC, vault/registry policy, approvals, guarded
writes, durable effect reconciliation and cold consumer acceptance remain
host-owned prerequisites. The Notes runtime keeps MCP disabled until those
parts are integrated and qualified; this engine alone enables no tool access.
