# Neutral ACP host service

Revision: enclosing change, parent `ce0d4179`.

Milestone review: M0–M5 retain their original status. Hosted H04.1 extracts the
existing session host; it does not qualify hosted execution, durable Notes
conversations, providers, model inference, Linux or package publication.

## Commands

- `pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --tests --locked`
- `node --test scripts/acp-service-boundary.test.mjs`
- `pnpm dev:cargo -- check -p lapis-ai-service -p tauri-plugin-lapis-ai --offline`
- `pnpm dev:cargo -- test -p lapis-ai-service --test runtime_owner --locked`
- `pnpm dev:cargo -- test -p lapis-ai-service --test host invalid_resource_limits_are_rejected_without_startup --locked`
- `pnpm dev:cargo -- test -p lapis-ai-service -p tauri-plugin-lapis-ai --locked`
- `pnpm dev:cargo -- tree -p lapis-ai-service --edges normal --prefix none`
- `pnpm check`
- `pnpm test`
- `pnpm build`
- `pnpm spec:check`

## Results

The original Tauri boundary passed all 37 cases before extraction. The same
fixture cases now run directly against the Tauri-independent service; two new
runtime ownership cases also pass. Three Tauri adapter cases preserve synchronous
construction, native caller/configuration refusal and irreversible shutdown.
The two new dependency-boundary regressions failed before and pass afterward.
Full check passes, including strict workspace Clippy. Full tests pass 43 renderer
cases, 47 governance cases and 1,072 Rust cases with two inherited ignored tests.
Workspace and runtime-example builds pass. All ten specification lanes pass.
The normal service graph has no Tauri dependency. The existing runtime's optional
Compute connection code retains its HTTP client dependency; this service never
constructs it or acquires network/credential authority.

`lapis-ai-service` owns admission, session registry, processors, streams and
supervised retirement. It retains an explicit Tokio handle. Tauri supplies its
own handle and preserves its public command/type boundary through a thin adapter.
One native event processor still owns capture; detach never authorizes replay.
No source budgets, imports, protocol bytes or permission defaults were relaxed.

## Failures

The first neutral suite passed 11 host cases but failed the process-free resource
limit fixture because it constructed a valid neutral host outside any runtime.
That fixture now enters an owned Tokio test runtime; its exact case passes.
The Tauri adapter separately proves its former synchronous construction remains
available. This is no exception to the neutral runtime lifetime contract.
An initial governance attempt omitted the new evidence's SUMMARY entry; the entry
is added without altering the validator. Workspace test/build linking reports the
existing large Compute/agent unwind-table warning; no linker flag or warning
suppression is introduced and strict Clippy remains green.

## Gaps

Gate result: passed for the neutral source extraction and adapter compatibility.

The execution backend port, hosted admission/transport, headless conversation
engine and downstream native integration remain subsequent slices. This adds a
coordinated Cargo library without publishing it or changing native services.

Next action: consume the service from the private hosted companion with host-owned
admission/persistence, then qualify the independent execution backend port.
