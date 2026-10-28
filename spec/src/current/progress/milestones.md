# Milestones

| Task | Status      | Dependencies | Gate                                                                   | Evidence                                                                   | Next action                                                |
| ---- | ----------- | ------------ | ---------------------------------------------------------------------- | -------------------------------------------------------------------------- | ---------------------------------------------------------- |
| M0   | Done        | none         | Standalone workspace, governance, imports and provenance               | [Foundation](../../history/2026-10-16-foundation.md)                       | Maintain source pins and frozen budgets                    |
| M1   | Done        | M0           | Renamed ACP and retained deterministic suites                          | [Foundation](../../history/2026-10-16-foundation.md)                       | Consumer adoption is separate                              |
| M2   | Done        | M1           | Agent, tools and compute imports with fixture acceptance               | [Foundation](../../history/2026-10-16-foundation.md)                       | Preserve provider and wire boundaries                      |
| M3   | Done        | M2           | TOML, clients, isolation and Rust examples                             | [Foundation](../../history/2026-10-16-foundation.md)                       | Maintain direct client acceptance                          |
| M4   | Done        | M3           | macOS ARM64 Metal, Linux aarch64 CPU, mixed-version and staged bundles | [Qualification](../../history/2026-10-16-qualification.md)                 | Qualify broader native and release profiles separately     |
| M5   | In progress | M4           | Hosted CI, Changesets, Cargo release automation and spec Pages         | [Organization transfer](../../history/2026-10-16-organization-transfer.md) | Verify hosted product CI and retain version PR #1 unmerged |

[CI Cargo reuse](../../history/2026-10-28-ci-cargo-reuse.md) keeps one
feature-aligned native compile before ARM qualification, replaces the
wall-clock stream-delivery cap with a 2× coalescing bound, and restores
Changesets/Release reusable-CI caller permissions so Version Packages can open.
[Changesets CI reuse](../../history/2026-10-28-changesets-ci-reuse.md) makes tag
qualification await successful exact-source CI for the candidate SHA instead of
duplicating native acceptance. [Merge queue and compute busy](../../history/2026-10-28-merge-queue-compute-busy.md)
admits main only after required checks on the post-merge commit and releases the
compute busy slot before terminal frames. M5 stays open.

[Codex compact effort](../../history/2026-10-19-codex-effort.md) adds a
private Control V3 compatibility profile for Notes local-file chat. Generic IDs,
M0–M5 gates and publication authority stay unchanged; full wider qualification
is deferred by the user while focused source and live consumer work advances.

[Renderer acknowledgement prefixes](../../history/2026-10-19-renderer-ack-batches.md)
advances bounded local-file streaming in Notes; no milestone or release gate changes.

## Consumer-driven native adapter

[Neutral host extraction](../../history/2026-10-22-neutral-acp-service.md) moves
the existing session service behind an owned Tokio runtime, preserving the
Tauri adapter. Focused source/adapter and full workspace qualification pass.
Hosted execution and Notes persistence remain separate H04/H05 gates.

This work does not depend on or change M5 release authorization. Desktop M10.1
uses local verified public outputs and remains incomplete until Tauri/native
acceptance passes.

| Slice            | Status | Gate                                                                                        | Evidence                                                              | Next action                                            |
| ---------------- | ------ | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------ |
| ACP session      | Done   | Correlation, exact permissions, ordered bounded reconnect, isolation and supervised cleanup | [Owned ACP session](../../history/2026-10-17-acp-session.md)          | Complete; continue to host-scoped Tauri transport      |
| Rust Tauri host  | Done   | Host-scoped native policy, bounded commands/channels and supervised startup revocation      | [Native Tauri transport](../../history/2026-10-17-tauri-transport.md) | Continue to renderer and native consumer qualification |
| Renderer client  | Done   | Typed public outputs, bounded consumed delivery and correlated uncertain outcomes           | [Typed renderer](../../history/2026-10-17-acp-renderer.md)            | Continue to cold packaged WebView qualification        |
| Owned retirement | Done   | Caller cancellation and overlapping revocation/exit retain supervised cleanup and capacity  | [Owned retirement](../../history/2026-10-17-owned-retirement.md)      | Complete; refresh native consumer                      |
| Tauri transport  | Done   | Host-scoped commands/channels, permissions, renderer client and cold native acceptance      | [Packaged ACP Tauri](../../history/2026-10-17-native-tauri.md)        | Complete; desktop UI/tools and optional compute follow |

## jj-store consumer integration

[Retained native processing](../../history/2026-10-18-retained-processing.md)
adds a Rust-only exact-binding capture grant for renderer suspension. Five
focused cases, all 34 host cases and full source qualification pass. NOTES-07 remains
the sole active downstream task. Native Notes capture adoption and cold managed
conversation acceptance remain separate. M0–M5 and publication are unchanged.

[Native processor factory](../../history/2026-10-18-tauri-processing.md) adds the
Tauri host policy hook and native scope guard over session processing. Nine
focused cases, all 27 Tauri host cases, full source qualification (1,004 Rust
passes, two inherited ignored) and six cold packaged ACP compatibility cases
pass. Native foreground assistance remains explicit. Notes privacy/durable
backend adoption remains separate. No M0–M5 or publication status is changed.

[Native frame processing](../../history/2026-10-18-native-processing.md) adds a
host-injected pre-publication hook to the owned ACP session layer. Its nine focused
runtime cases, full source qualification and six cold packaged ACP compatibility
cases pass. Tauri policy factory wiring and downstream privacy/durable storage
remain separate Notes M33 integration slices. M0–M5 and publication authorization are unchanged.

[Host clients](../../history/2026-10-17-host-clients.md) extends the M3 client
surface for host-prepared Control V3 launches, bounded agent discovery and
independent compute command/events. M0–M4 and the existing Tauri qualification
retain their recorded status; M5 publication is unchanged. jj-store consumer
and cold native qualification are separate from this shared source slice.

## Reusable session acquisition

[Local session pages](../../history/2026-10-21-local-session-pages.md) adds
transient JSONL reads and pinned, read-only OpenCode SQLite source transactions.
Notes owns its rebuildable metadata index, admission, privacy and progressive
UI. M0–M5 and publication authority remain unchanged; source and consumer
qualification are recorded separately.

[Pinned Lapis ACP headers](../../history/2026-10-19-codex-lapis-acp-source.md)
adds only the observed Codex CLI 0.156.1 adapter pair. Other source/version
qualification and original M0–M5 gates remain unchanged.

[Filesystem acquisition](../../history/2026-10-17-capture-discovery.md) qualifies
the independent approved-root scanner, versioned product inventory and bounded
Codex file inspection. The complete workspace suite, strict checks, build and
public package contents pass. Publishing-consumer routing, leases/cursors and
broader source formats remain jj-store M33 work. M0–M5 status and publication
authorization are unchanged.

[Deferred native text groups](../../history/2026-10-18-native-batches.md) adds
the NOTES-06C2 runtime/scoped-host/client transport boundary. Seven runtime,
two native-host and twelve renderer cases pass. Full source qualification
passes (1,013 Rust tests, two inherited ignored; 39 renderer/45 governance).
Cold native batch acceptance remains pending; Notes privacy/durable storage and JJ backend
adoption remain separate. M0–M5 and release authorization are unchanged.

## Application-tool transport

[Trusted HTTP declarations](../../history/2026-10-20-http-mcp-declarations.md)
add typed host-owned HTTP MCP configuration beside retained stdio for desktop
Cursor integration. Parsing/encoding and real launcher delivery are distinct
from the pending authenticated HTTP tool host and desktop M10.2–M10.4 acceptance.
Original M0–M5 status and publication authority remain unchanged.

[Native host boundary](../../history/2026-10-19-native-tools-boundary.md) exposes
pre-allocation snapshot validation and redacts MCP environment Debug values
without changing exact wire delivery. The affected protocol/engine cases pass;
mounted native bridge qualification remains Notes-owned TOOLS-01B/C.

[Injectable MCP engine](../../history/2026-10-19-app-tools-engine.md) advances
TOOLS-01A for Notes while JJ persistence stays deferred. Its standalone Rust
byte-stream, cancellation and packaging checks qualify the engine only. Native
authentication, host policy/registry wiring, actual Codex reads/guarded edits and
cold consumer cleanup remain TOOLS-01B/C. M0–M5 and publication gates stay.

## Managed conversation request preparation

[Native outgoing preparation](../../history/2026-10-20-request-preparation.md)
adds LAI-TAURI-011 for the Desktop managed-conversation integration. The actor
fences provider dispatch behind a drained Rust callback; the Tauri wrapper uses
current command admission, independently of retained incoming capture. M0–M5
and publication authority stay unchanged. Native Desktop adoption and its full
renderer/context/restart gates remain separate.

[Session effort](../../history/2026-10-20-session-effort.md) exposes the bundled
agent’s existing reasoning control per ACP session. M0–M5 retain their gates;
Desktop native/provider and optional compute acceptance remain separate.

[Compute cleanup](../../history/2026-10-21-compute-stop.md) preserves supervised
shutdown after an independently requested stop. M0–M5 and provider/model gates
retain their existing status.

[Compute inventory](../../history/2026-10-21-compute-library.md) extends M3 with
process-free installed-model facts, correlated progress and safe operation
failures. Original M0–M5 status and publication gates remain unchanged; Desktop
model inference and two-device pairing remain consumer acceptance.

## Hosted execution port

[Host execution](../../history/2026-10-22-host-execution.md) adds the native
backend/session boundary with supervised host processes as the default. M0–M5
retain their status; hosted H04 admission and transport remain consumer work.
Docker execution, model inference and platform/provider gates remain separate.

## Portable local tooling

[Docker validation](../../history/2026-10-25-docker-validation.md) makes portable
checks use bounded isolated workers over the exact source revision. Full
portable check/test/build/package/spec qualification passes, with 1,076 Rust,
43 renderer and 57 governance cases. Native/model qualification stays separate. Original
M0–M5 status, model/platform qualification and publication remain unchanged.
