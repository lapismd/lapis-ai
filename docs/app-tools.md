# Embedding application tools

`lapis-ai-app-tools` is the shared MCP stdio engine for an application's existing
tool registry. It is independent of JJ, Tauri, storage and the ACP worker. It
does not discover tools, select a root, create a listener or grant execution.

Consume its checked source workspace or verified Cargo archive until a registry
release is separately qualified. The source cohort version is not a claim that
the package has been published.

## A minimal host

The host supplies an immutable tool snapshot and an asynchronous handler. This
example exposes static metadata only; an actual application must authenticate
its private IPC and reauthorize the original registry/scope before dispatch.

```rust
use lapis_ai_app_tools::{Config, Tool, ToolCall, ToolError, ToolFuture, ToolHandler, serve};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::watch;

struct Metadata;

impl ToolHandler for Metadata {
    fn tools(&self) -> Vec<Tool> {
        vec![Tool {
            name: "app.metadata".into(),
            description: "Read static application metadata".into(),
            input_schema: json!({"type": "object", "additionalProperties": false}),
        }]
    }

    fn call(&self, call: ToolCall, cancelled: watch::Receiver<bool>) -> ToolFuture<'_> {
        Box::pin(async move {
            if *cancelled.borrow() {
                return Err(ToolError::Unknown);
            }
            if call.arguments != json!({}) {
                return Err(ToolError::InvalidInput);
            }
            Ok(json!({"content": [{"type": "text", "text": "Example application"}]}))
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), lapis_ai_app_tools::Error> {
    serve(tokio::io::stdin(), tokio::io::stdout(), Arc::new(Metadata), Config::default()).await
}
```

Supply an owned async reader/writer instead of stdio when composing a test or
another native transport. The output is owned by a supervised writer task and
must be `Send + 'static`. A stalled writer cannot block input cancellation or
EOF cleanup. Do not write diagnostic text to the MCP output stream.

## Bounds and protocol

The engine implements initialize, ping, tools/list, tools/call and
notifications/cancelled. It advertises protocol version 2025-06-18 and a fixed
tool list. Tools cannot be called before initialization. Optional MCP transport
features such as resources, prompts, HTTP and subscriptions are not provided.

Defaults are 1 MiB per input/output frame, eight active calls, 4096 total input
messages and a 60 second call/output deadline. Hosts can lower those bounds.
The tool snapshot has at most 256 unique entries and must fit a 64 KiB list
response, including a maximum-length request identity. Output has a bounded
queue of maximum-calls plus one, and one frame being written. Capacity exhaustion
retires the bridge; there is no unbounded queue or silent frame loss.

MCP request IDs retain string or signed/unsigned integer identity. Strings are
nonempty, at most 256 UTF-8 bytes and contain no control characters. Floats and
reused IDs refuse dispatch. A reused identity retires the binding instead of
executing again. Invalid JSON and unsupported methods receive stable errors
without echoing the input. Oversized or unterminated frames retire the bridge.

Only names in the original snapshot can reach the handler. Names are at most
128 bytes and use ASCII letters, digits, underscores, hyphens or periods.
Descriptions are at most 4096 bytes; input schemas must be JSON objects. The
engine does not interpret schemas. The host applies its existing validator and
returns an MCP result object, including content and optional structured content.
Its result privacy, schema and size policy remain application-owned.

## Admission, cancellation and uncertain effects

The snapshot is presentation, not authorization. A production handler must
revalidate current native admission, the exact original tool registration,
conversation scope and required approval before an effect. Do not accept
provider-supplied paths, credentials or executable choices as native authority.
Registry removal must reject a stale snapshot through the handler. Create a new
binding for a changed tool list; the engine advertises no list-change events.

`Unavailable` and `InvalidInput` mean a proven rejection before dispatch.
After dispatch, use `Unknown` whenever completion cannot be established. The
engine never retries. Cancellation and deadlines signal the observer before
dropping the host future and return an unknown outcome; that cannot prove an
application-side effect was rolled back.

A dropped host future must release its transport ownership and notify any
separately owned execution. Input closure or engine drop releases all owned
call and writer futures. Applications still drain their actual execution before
releasing native authority. Successful transport closure is not a durable tool
receipt. The host owns effect reconciliation and transcript persistence.

Notes' existing registry, scope, approval UI and guarded edits remain its own
policy. Its authenticated native IPC, mounted Codex tool path and cold cleanup
are separate consumer work; this library alone enables no tool access.

See the [canonical contract](../spec/src/current/contracts/app-tools.md) and
[crate overview](../crates/lapis-ai-app-tools/README.md).
