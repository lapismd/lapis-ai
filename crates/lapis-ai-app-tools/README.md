# lapis-ai-app-tools

A bounded MCP stdio engine for a native application's existing tools. The host
injects an immutable tool snapshot and async handler, then supplies its stdin and
stdout (or another owned byte stream) to `serve`. Calls retain their JSON-RPC
identity and support bounded concurrency, cancellation and deadlines.

```rust
use std::sync::Arc;
use lapis_ai_app_tools::{Config, ToolHandler, serve};

async fn run(handler: Arc<dyn ToolHandler>) -> Result<(), lapis_ai_app_tools::Error> {
    serve(tokio::io::stdin(), tokio::io::stdout(), handler, Config::default()).await
}
```

The host owns authentication, current vault admission, registry identity,
approvals, guarded edits, result privacy and execution drain. This library
creates no listener, process, credential or transcript store. Cancelled or
expired calls have unknown effects and are never automatically replayed.

See the [embedding guide](../../docs/app-tools.md) for usage, bounds and lifetime
rules, and the [contract](../../spec/src/current/contracts/app-tools.md) for native
integration gates. Source protocol reference:
`crates/lapis-agent-tools/src/main.rs` (same repository, Apache-2.0).
