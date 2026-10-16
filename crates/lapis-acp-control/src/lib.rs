//! Controlling application boundary.
//!
//! Owns the versioned task document, the single terminal record emitted per
//! attempted invocation, the published exit codes, bounded document input, and the
//! composition that turns a validated document into one executed turn.
//!
//! This crate replaces the upstream Nostr relay intake wholesale. The controlling
//! application is the trust boundary: it selects the runner, the working
//! directory, the credentials environment and the prompt. There is no author
//! gating, no membership discovery and no identity check beyond refusing a task
//! addressed to a different runner.
//!
//! Diagnostics stay on stderr. Stdout carries terminal records and, in a future
//! persistent mode, control frames, so a controlling application can parse it
//! unconditionally.

#![forbid(unsafe_code)]

pub mod fleet;
mod fleet_wire;
mod fleet_worker;
pub mod input;
pub mod interactive;
pub mod interactive_handler;
pub mod interactive_session;
pub mod interactive_wire;
mod interactive_worker;
pub mod invocation;
pub mod protocol;
pub mod record;
pub mod scheduler;
pub mod serve;
pub mod task;
pub mod transport;
mod warm_worker;

pub use input::{TaskSource, read, read_bytes, read_path, read_stdin};
pub use invocation::{Invocation, InvocationConfig, invoke, invoke_with_probe};
pub use protocol::{
    CONTROL_VERSION, Request, Response, cancel_frame, code_control, parse_request,
    parse_request_text, runners_frame, scoped_submit_frame, shutdown_frame, status_frame,
    submit_frame,
};
pub use record::{ExitCode, RECORD_VERSION, Status, TerminalRecord};
pub use scheduler::{DEFAULT_SCOPE, ServeLimits};
pub use serve::{ServeConfig, ServeExit, runner_summaries, serve};
pub use task::{TASK_VERSION, TaskDocument, TaskRejection, code, parse, parse_text};
pub use transport::{FrameSink, FrameSource, LineSink, LineSource, stdio};
