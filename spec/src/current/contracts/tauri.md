# Native ACP adapter and Tauri contract

## Requirements

| ID            | Requirement                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-TAURI-001 | Future adapters MUST wrap the Rust runtime clients with host-selected executables and admitted workspace roots, scoped opaque session handles, ordered events and explicit cancellation/teardown. Secrets and arbitrary process launch MUST remain on the Rust side.                                                                                                                                    |
| LAI-TAURI-002 | The reusable ACP session layer MUST own one worker per concurrent conversation, correlate every response, fence session and permission identities, preserve exact option IDs and reject overlapping turns. Lost output, deadlines and uncertain writes MUST retire the worker without replay.                                                                                                           |
| LAI-TAURI-003 | Detach/reconnect MUST preserve a bounded acknowledged event journal with monotonic cursors. Coverage loss MUST remain observable independently of queued events. Explicit close and owner teardown MUST perform supervised process cleanup; cancelled consumer waits MUST NOT orphan workers.                                                                                                           |
| LAI-TAURI-004 | The Tauri host MUST derive caller context from the invoking WebView, authorize every operation against the current host admission, and scope opaque worker/subscription handles to that window and admission generation. Only Rust policy MAY resolve targets into launches, directories and credentials.                                                                                               |
| LAI-TAURI-005 | Explicit disable, admission revocation, window destruction and actual exit MUST cancel startup and release affected workers/subscriptions. Cancelled close/exit requests MUST preserve workers. Command permissions MUST default to empty and require an explicit ACP-session grant.                                                                                                                    |
| LAI-TAURI-006 | The reusable renderer client MUST correlate before dispatch, buffer bounded early channel delivery, validate session/subscription identity and contiguous journal cursors, and acknowledge only consumed events. Detach MUST preserve native workers; uncertain submissions MUST NOT replay automatically.                                                                                              |
| LAI-TAURI-007 | Rust hosts MAY inject one native session frame processor. When selected, returned startup facts and every emitted event MUST pass it before exposure. Processing MUST preserve protocol identities, permission choices and bounds; failure MUST retire without raw fallback or replay. Startup cancellation and close MUST drain admitted processing.                                                   |
| LAI-TAURI-008 | Native processor factories MUST receive the reserved worker/open-request/target/admission binding and actual caller context. Current scope MUST be checked before/after construction and each processing call. Factory/processor failures MUST NOT fall back to unprocessed execution. Revocation and last-owner teardown MUST drain admitted processing without retaining the host in a wrapper cycle. |

The ACP session, native host and typed client are qualified together in a
packaged macOS ARM64 consumer. Rust, renderer and native evidence remain distinct.
Desktop application orchestration and compute Tauri integration retain their
separate consumer gates.
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

## Tauri host boundary

`tauri-plugin-lapis-ai` wraps the owned session layer. Its Rust `HostPolicy`
authorizes the invoking window/origin and returns a current opaque admission
generation. Targets are opaque catalog choices; resolving one supplies a prepared
launch and admitted directory entirely on the Rust side. No renderer command
accepts a path, program, environment, credential, MCP launch or admission override.
An optional renderer resume value names a consumer binding, never a provider
session directly. Native policy must admit that binding for the current vault
and return the provider identity. A resume response must match the admitted
provider identity; mismatch or unsupported resume never falls back to new work.

Open requests reserve bounded slots before awaiting startup and retain request
identity after cancellation or uncertain acknowledgement. A changed admission,
disabled host or removed slot cannot publish a late worker. Applications call
scoped revocation on vault closure, plugin disable and trust changes; global
disable is a separate Rust-only control. Re-enable never resurrects old handles.
Startup completion includes its cleanup result. A failed cleanup must propagate
through revocation/shutdown rather than treating task termination as a clean stop.
Retirement remains registry-owned after slots are removed or a caller's wait is
cancelled. Global shutdown must await cleanup already started by window teardown.
Retiring workers continue to consume capacity until reaped; scoped failures stay
observable without stopping unrelated windows (LAI-TAURI-005).

Only one current channel subscription owns acknowledgements for a session.
Replacing/detaching a channel preserves the worker and its unacknowledged
journal. Stale subscription acknowledgements and cursors beyond delivered events
fail closed. Channel failure detaches delivery; bounded retention still retires a
worker if coverage cannot be preserved. Cold packaged WebView acceptance remains
distinct from Rust host-policy and channel-sink tests.

## Renderer client

The private migration package `@lapismd/ai` exposes built public exports: a
runtime-independent typed client at the root and the optional Tauri transport
at `./tauri`. Only the latter imports Tauri IPC. It carries raw control-v3
frames; application event mapping, transcript persistence, context and memory
remain consumer policy. Permission choices preserve exact provider option IDs,
including explicit cancellation through a null option. No semantic fallback
approves a request.

Pending correlation is registered before submission. Channel delivery may precede
the attach invocation's subscription token; bounded delivery waits for that token
before consuming. Every snapshot must match the current worker/subscription and
cover the next contiguous journal range. A consumer callback must complete before
the cursor advances or acknowledgement is sent. A failed callback, lost channel,
invalid cursor or unknown submission remains observable without replay. Reattach
uses the last consumed cursor and fences obsolete deliveries. Lost open replies
are recoverable by listing native request identities, never by retrying open.

The private client shares the coordinated version for local public-build
consumption. It has no npm publication or tag lane. Native/model qualification
remains separate from its deterministic transport tests.

## Native frame processing

`AcpSession::start_with_processor` and
`start_cancellable_with_processor` accept a Rust-owned `SessionProcessor`.
Existing start methods retain their unprocessed contract. No renderer input
can select a processor, persistence backend or launch policy.

One processor instance moves from bootstrap to the session actor. It sees
validated startup activity, the final opened facts and normal correlated events
serially, before journal insertion or facts return. Unexposed warm/queued
bootstrap control responses are not transcript publications. A host may sanitize
the frame and durably capture it; successful callback completion is the host's
capture fence. The runtime supplies no storage, receipt recovery or redaction
policy. It never acknowledges raw processing through a renderer subscription.

Each input includes the native handle, an increasing processing ordinal and a
facts/event publication kind. Event publications include their unchanged journal
cursor; facts consume an ordinal without adding a journal event. The host owns
stable capture identities and unknown-write recovery. A callback must own and
finish its work before returning, rather than spawn detached persistence.

Processed output preserves control version/type, request/session/provider IDs,
provider sequence, permission and option IDs/kinds, terminal task/status and
control outcome flags. Output remains bounded by the session byte limit.
Invalid output or processing errors retire the worker without exposing the raw
frame. A terminal-frame failure retains the original unresolved request even
if protocol routing already recognized its terminal response. No prompt replay,
silent fallback or additional execution authority is introduced.

A runtime-owned startup job survives cancellation of its caller's wait.
Cancellation interrupts transport waits but drains an already admitted processor
future before startup cleanup. A completed session that loses its startup caller
is closed under supervision. The actor also finishes its current processing
future before close/reaping. Hosts still drain their persistence backend before
releasing admission; this hook alone does not qualify a downstream durable
conversation or a packaged native consumer.

## Host processor factory

`HostPolicy::processor` is a Rust-only extension. Its default `Ok(None)` retains
the unprocessed profile; hosts requiring capture return a processor or an error,
never `None` on a failed capture dependency. No IPC field, new permission or
renderer setting selects this factory.

The host constructs `SessionBinding` from its reserved slot: generated native
worker ID, original admitted open request, resolved target and opaque admission
generation. `HostContext` supplies the native invoking window and origin. These
values describe the binding; they do not grant storage or process authority.
Factories construct owned processors outside registry locks, inside the existing
registry-owned startup job. Duplicate requests, denied targets/origins and full
capacity cannot allocate another processor. Failed construction retains the
original request identity and cleans its unstarted reservation without launching
an unprocessed worker.

The host rechecks current admission and slot ownership before and after factory
construction. A scoped wrapper checks the same native worker/window/generation
before and after every processor future. Revoked, disabled or removed slots
cannot publish a late result. Processing already admitted still drains before
retirement releases its reservation. Native scope changes never grant the old
worker a replacement generation or affect an unrelated window.

The wrapper retains policy and registry state, rather than `Arc<AcpHost>`, so
last-host drop can trigger supervised retirement. Application processors must
also avoid retaining the host in their own ownership cycle. Applications still
own privacy, storage admission, exact receipts and persistence-backend drain.
