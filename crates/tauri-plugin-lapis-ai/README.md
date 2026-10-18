# tauri-plugin-lapis-ai

Rust-owned ACP sessions for Tauri applications. Part of
[Lapis AI](https://github.com/lapismd/lapis-ai); Apache-2.0, Rust 1.99.0.

The application supplies `HostPolicy`: authorize the invoking window and origin
against its current vault admission, enumerate opaque targets and resolve them
into native launches. The WebView receives no executable, environment, root or
credential authority. Register `Builder::new(policy).build()` and explicitly
grant `lapis-ai:acp-session`; default permissions are empty.
Optional resume names an application binding that native policy must authorize
and resolve; the renderer cannot directly select a provider session identity.

Override `HostPolicy::processor` to return a native `SessionProcessor` for each
worker. The factory receives `HostContext` and its native `SessionBinding`;
startup facts and events pass through that processor before IPC exposure.
Return an error when required capture is unavailable. The default `None`
retains an unprocessed profile. The host fences stale admissions and drains
in-flight processing; the application supplies privacy and durable storage.
Hosts retaining native capture during renderer reload may override
`HostPolicy::authorize_processing` for the exact original binding. Its default
requires current admission; renderer commands and startup always require it.
Full semantics live in the [adapter contract](https://lapismd.github.io/lapis-ai/current/contracts/tauri.html).

Rust hosts revoke only affected admissions on plugin disable, vault closure or
trust changes. Detach/reconnect retains workers and bounded unacknowledged
events. Explicit close, window destruction and actual exit supervise teardown;
cancelled close/exit requests preserve workers. The host never retries prompts.

The library owns transport and worker lifetime. Conversation history, context,
memory, application tools, UI and release qualification remain application-owned.
See the repository specification for current qualification and remaining gates.
