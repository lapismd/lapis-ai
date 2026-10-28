//! Host-admitted ACP sessions, independent of a desktop or HTTP transport.
#![forbid(unsafe_code)]
mod cleanup;
mod error;
mod execution;
mod host;
mod lifecycle;
mod opening;
mod policy;
mod processing;
mod registry;
mod stream;
pub use error::{Error, Result};
pub use execution::{ExecutionBackend, ExecutionFuture, ExecutionSession, HostExecution};
pub use host::{AcpHost, SessionInfo};
pub use lapis_ai_runtime as runtime;
pub use policy::{Config, HostContext, HostPolicy, Launch, SessionBinding, Target};
pub use stream::{Sink, StreamMessage};
