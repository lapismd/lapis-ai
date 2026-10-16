//! ACP session and turn lifecycle.
//!
//! Owns launching an agent subprocess with process-group ownership, negotiating a
//! session, executing one prepared turn against a bound, and stopping the process
//! within bounded grace and reap intervals.
//!
//! This crate does not know how work arrives. Intake, batching and the
//! controlling-application transport belong to `lapis-acp-control`; this crate
//! executes one prepared turn and reports what it observed.
//!
//! Cancellation is bounded and cleanup is best-effort. A timeout is an unknown
//! outcome, never a proven success or a proven failure, and an unreaped child is
//! reported rather than assumed gone.

#![forbid(unsafe_code)]

pub mod configuration;
pub mod launch;
pub mod outcome;
pub mod persistent;
pub mod process;
pub mod turn;

pub use configuration::{ConfigurationReport, SessionConfiguration, SettingOutcome, apply};
pub use launch::{LaunchSpec, describe, spawn};
pub use outcome::{Failure, FailureKind, TimeoutKind, TurnOutcome, TurnStatus};
pub use process::{ProcessGuard, Termination};
pub use turn::{Agent, TurnRequest, run_turn};
