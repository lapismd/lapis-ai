# Native ACP adapter and Tauri contract

## Requirements

| ID            | Requirement                                                                                                                                                                                                                                                                                   |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-TAURI-001 | Future adapters MUST wrap the Rust runtime clients with host-selected executables and admitted workspace roots, scoped opaque session handles, ordered events and explicit cancellation/teardown. Secrets and arbitrary process launch MUST remain on the Rust side.                          |
| LAI-TAURI-002 | The reusable ACP session layer MUST own one worker per concurrent conversation, correlate every response, fence session and permission identities, preserve exact option IDs and reject overlapping turns. Lost output, deadlines and uncertain writes MUST retire the worker without replay. |
| LAI-TAURI-003 | Detach/reconnect MUST preserve a bounded acknowledged event journal with monotonic cursors. Coverage loss MUST remain observable independently of queued events. Explicit close and owner teardown MUST perform supervised process cleanup; cancelled consumer waits MUST NOT orphan workers. |

The session layer is the first implementation slice. Tauri command, WebView and
packaged desktop qualification remain separate, pending gates.
The adapter maps ACP warm/open/configure/prompt/permission/cancel/close and compute
configuration/model/pairing operations to clients. Each WebView subscription is
owned by its originating window and application instance; teardown releases it.
The host validates root admission before opening a provider session. No request
may override executable selection, credential resolution or trusted MCP launch.
Services remain lazy and independently available. Application conversation state,
context preparation, tool authorization and memory stay in the consuming app.

## Owned ACP sessions

`lapis-ai-runtime` exposes an owned session around `AcpClient`. Its Rust caller
supplies a unique opaque handle, admitted directory and already prepared launch;
these are not renderer-controlled launch arguments. Canonical session directories
must remain under the launch root, including after symlink resolution. Warm and
open never submit a prompt. Provider resume is explicit and unsupported resume
fails without silently creating or replaying a conversation.

Configure, prompt, permission, cancel and status use bounded typed inputs. A
successful submit acknowledges transport admission only; correlated events carry
the outcome. Mode selection must succeed before prompting. One configuration or
prompt can be active; cancel, status and exact live permission responses remain
available while it runs. Reused request IDs never dispatch twice. Authored text
and injected context remain separate control fields.

Ordered event cursors cover all forwarded frames, independent of the provider's
sequence namespace. Only explicit acknowledgement prunes retained events. A
cursor older than that boundary reports a gap. Detaching neither acknowledges nor
closes a worker; retained events are bounded by count and bytes. Overflow, invalid
correlation, output failure or request deadline latches failure, identifies
unresolved requests and stops the process. No automatic continuation or retry is
performed. These facts are not a durable transcript or a provider-session deletion
claim. The consuming host still owns admission, generation fencing and history.
