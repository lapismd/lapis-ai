# lapis-ai-runtime

Supervised Rust ACP, agent and compute clients with verified native resource bundles.

Part of [Lapis AI](https://github.com/lapismd/lapis-ai). Rust 1.99.0.
The controlling application owns trust, credentials, workspace admission,
conversation history and permission policy. An uncertain prompt is never retried.

`AcpSession::start(PreparedLaunch, SessionOptions)` warms and opens one isolated
control-v3 worker. Rust supplies the opaque handle and admitted directory.
`submit` accepts typed configure/prompt/permission/cancel/status requests;
completion means transport admission, while correlated events carry outcomes.
Use `subscribe` before `snapshot(after)` and acknowledge only consumed events.
Detach retains bounded events and the worker. Overflow or uncertain execution
latches failure and unresolved request IDs without replay. `close` supervises
cleanup; dropping the last owner also starts cleanup. This session layer has no
Tauri dependency, transcript persistence or application authorization policy.

Use `AcpSession::start_with_processor` with your `Box<dyn SessionProcessor>` to
sanitize and durably capture startup facts and events before exposure.
`start_cancellable_with_processor` also accepts the native cancellation receiver.
The callback receives an ordered `FrameInput`; return a processed frame only
after the host's capture fence. Failures retire the worker without raw fallback
or replay. Startup cancellation and close drain in-flight processing. The host
still owns privacy rules, storage, receipt recovery and backend teardown; see the
full [adapter contract](https://lapismd.github.io/lapis-ai/current/contracts/tauri.html).

See the [usage guide](https://github.com/lapismd/lapis-ai/blob/main/docs/usage.md),
[specification](https://lapismd.github.io/lapis-ai/) and
[release guide](https://github.com/lapismd/lapis-ai/blob/main/docs/releasing.md).
The repository is preparing its first Cargo publication; consult releases for
available versions. Source builds use `pnpm dev:cargo -- build -p lapis-ai-runtime --locked`.

Apache-2.0. See the included `LICENSE.md` and `NOTICE.md` for attribution.

Hosts with existing orchestration can use `AcpClient` for Control V3,
`AgentClient::{models_prepared, metadata_prepared}` and
`ComputeClient::start_prepared`. `PreparedLaunch::from_command` copies only
explicit native launch values. `ProcessLimits` retains host transport bounds.
Compute's separate `commands()` and `next_event()` keep progress and Stop
responsive; its redacted `connection()` stays in native code.
