# Lapis AI service

This library owns scoped ACP workers, bounded event subscriptions, native frame
processing and supervised retirement. It has no Tauri or HTTP dependency.
Applications supply admission, prepared launches, credentials, tools and durable
conversation processing through `HostPolicy`.

Construct `AcpHost::new` within the owning Tokio runtime, or use `with_runtime`
with its explicit handle. The runtime must stay alive through awaited service
shutdown. Synchronous attachment and drop use that captured handle. The legacy
`HostContext.window` field names an opaque caller scope, not native authority.

`tauri-plugin-lapis-ai` supplies the desktop transport. Hosted consumers supply
their own authenticated context; accepting a browser field never grants process
or filesystem authority. Unknown submissions are never replayed automatically.
