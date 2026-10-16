# Future Tauri adapter contract

## Requirements

| ID            | Requirement                                                                                                                                                                                                                                                          |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-TAURI-001 | Future adapters MUST wrap the Rust runtime clients with host-selected executables and admitted workspace roots, scoped opaque session handles, ordered events and explicit cancellation/teardown. Secrets and arbitrary process launch MUST remain on the Rust side. |

This milestone defines the boundary only; no Tauri implementation is qualified.
The adapter maps ACP warm/open/configure/prompt/permission/cancel/close and compute
configuration/model/pairing operations to clients. Each WebView subscription is
owned by its originating window and application instance; teardown releases it.
The host validates root admission before opening a provider session. No request
may override executable selection, credential resolution or trusted MCP launch.
Services remain lazy and independently available. Application conversation state,
context preparation, tool authorization and memory stay in the consuming app.
