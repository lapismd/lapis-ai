//! Injectable MCP stdio transport. Hosts retain all execution and admission policy.
#![forbid(unsafe_code)]

mod calls;
mod framing;
mod protocol;
mod server;
mod writer;

use serde::Serialize;
use serde_json::Value;
pub use server::serve;
use std::{future::Future, pin::Pin, time::Duration};
use tokio::sync::watch;

/// Immutable public MCP description; schema interpretation belongs to the host.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// A correlated call, without execution, filesystem or authentication authority.
pub struct ToolCall {
    pub request_id: Value,
    pub name: String,
    pub arguments: Value,
}

/// Stable transport outcomes; provider or application diagnostics are not echoed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolError {
    /// The host proves it rejected the call before dispatching an effect.
    Unavailable,
    /// Host validation rejected the input before dispatching an effect.
    InvalidInput,
    /// The host cannot prove whether an effect occurred. Never retry implicitly.
    Unknown,
}

pub type ToolFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, ToolError>> + Send + 'a>>;

/// One host-owned binding. The snapshot never grants execution authority.
pub trait ToolHandler: Send + Sync + 'static {
    fn tools(&self) -> Vec<Tool>;
    /// Reauthorize the original scope/registration and apply approvals here.
    /// Dropping this future must release its transport ownership and signal
    /// separately owned execution; the host remains responsible for its drain.
    fn call(&self, call: ToolCall, cancelled: watch::Receiver<bool>) -> ToolFuture<'_>;
}

/// Hosts may lower capacity/deadlines, never raise the documented hard bounds.
#[derive(Clone)]
pub struct Config {
    pub name: String,
    pub version: String,
    pub frame_bytes: usize,
    pub maximum_calls: usize,
    pub maximum_requests: usize,
    pub call_timeout: Duration,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            name: "lapis-tools".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            frame_bytes: 1_048_576,
            maximum_calls: 8,
            maximum_requests: 4096,
            call_timeout: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid tool bridge configuration")]
    Configuration,
    #[error("tool bridge frame limit exceeded")]
    FrameLimit,
    #[error("tool bridge frame is incomplete")]
    IncompleteFrame,
    #[error("tool bridge request identity was reused")]
    DuplicateRequest,
    #[error("tool bridge request limit exceeded")]
    RequestLimit,
    #[error("tool bridge call task ended unexpectedly")]
    CallTask,
    #[error("tool bridge output capacity exhausted")]
    OutputCapacity,
    #[error("tool bridge output deadline exceeded")]
    OutputTimeout,
    #[error("tool bridge output task ended unexpectedly")]
    OutputTask,
    #[error("tool bridge I/O failed")]
    Io(#[from] std::io::Error),
}
