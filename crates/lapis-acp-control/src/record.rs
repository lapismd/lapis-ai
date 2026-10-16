//! The terminal record and process exit codes.
//!
//! Exactly one record is emitted per attempted invocation. The schema has no field
//! that can carry prompt text, agent output, raw adapter error text or
//! environment content, so sanitization is structural rather than a filter that
//! could be forgotten. A failure carries a stable code drawn from a fixed set.

use lapis_acp_protocol::StopReason;
use serde::Serialize;

/// The terminal record schema version.
pub const RECORD_VERSION: u64 = 1;

/// Outcome codes that are properties of the record rather than of task parsing.
pub mod code {
    /// The absolute turn cap elapsed.
    pub const DEADLINE_EXCEEDED: &str = "deadline_exceeded";
    /// No agent frame arrived inside the idle window.
    pub const IDLE_TIMEOUT: &str = "idle_timeout";
    /// The invocation was interrupted and cleanup ran.
    pub const INTERRUPTED: &str = "interrupted";
    /// Reading the task document did not finish inside its bound.
    pub const CONFIGURATION_TIMEOUT: &str = "configuration_timeout";
}

/// How an invocation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// The turn returned normally. Inspect `stopReason`; this is not proof the
    /// task succeeded.
    Completed,
    /// The invocation failed for a classified reason.
    Failed,
    /// The arguments, launch configuration or task input were unusable, so no
    /// agent was started.
    Invalid,
    /// A bound elapsed and the agent's real state is unknown.
    TimedOut,
    /// Cancellation was requested and the invocation stopped.
    Cancelled,
}

/// One terminal record.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRecord {
    /// Schema version.
    pub version: u64,
    /// The task identifier, null until the document validated.
    pub task_id: Option<String>,
    /// How the invocation ended.
    pub status: Status,
    /// The agent's stop reason, present only on normal completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    /// A stable sanitized code, present only when one applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'static str>,
}

impl TerminalRecord {
    /// A record for a document that never validated, so no identifier is known.
    pub fn invalid(error: &'static str) -> Self {
        Self {
            version: RECORD_VERSION,
            task_id: None,
            status: Status::Invalid,
            stop_reason: None,
            error: Some(error),
        }
    }

    /// A record for a validated document that was refused before any agent start.
    ///
    /// Distinct from [`TerminalRecord::invalid`] only in carrying the identifier,
    /// which is known because the document parsed.
    pub fn invalid_with_id(task_id: impl Into<String>, error: &'static str) -> Self {
        Self {
            version: RECORD_VERSION,
            task_id: Some(task_id.into()),
            status: Status::Invalid,
            stop_reason: None,
            error: Some(error),
        }
    }

    /// A record for a validated document that failed before or during the turn.
    pub fn failed(task_id: impl Into<String>, error: &'static str) -> Self {
        Self {
            version: RECORD_VERSION,
            task_id: Some(task_id.into()),
            status: Status::Failed,
            stop_reason: None,
            error: Some(error),
        }
    }

    /// A record for a turn that returned a stop reason.
    pub fn completed(task_id: impl Into<String>, stop_reason: StopReason) -> Self {
        Self {
            version: RECORD_VERSION,
            task_id: Some(task_id.into()),
            status: Status::Completed,
            stop_reason: Some(stop_reason),
            error: None,
        }
    }

    /// A record for a bound that elapsed.
    pub fn timed_out(task_id: Option<String>, error: &'static str) -> Self {
        Self {
            version: RECORD_VERSION,
            task_id,
            status: Status::TimedOut,
            stop_reason: None,
            error: Some(error),
        }
    }

    /// A record for an interrupted invocation.
    pub fn cancelled(task_id: Option<String>) -> Self {
        Self {
            version: RECORD_VERSION,
            task_id,
            status: Status::Cancelled,
            stop_reason: None,
            error: Some(code::INTERRUPTED),
        }
    }

    /// Serialize to the single line written to stdout.
    pub fn to_line(&self) -> String {
        // Serialization of this struct cannot fail: every field is a primitive,
        // an option or a unit-variant enum.
        serde_json::to_string(self).expect("terminal record serializes")
    }
}

/// Process exit codes. These are a published contract and never change meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// The turn returned normally.
    Ok = 0,
    /// Startup, execution or terminal-output failure.
    Failure = 1,
    /// Invalid arguments, launch configuration or task input.
    Invalid = 2,
    /// The execution deadline elapsed.
    Deadline = 124,
    /// SIGINT, after bounded cleanup ran.
    Sigint = 130,
    /// SIGTERM, after bounded cleanup ran.
    Sigterm = 143,
}

impl ExitCode {
    /// The numeric code passed to the process exit.
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}
