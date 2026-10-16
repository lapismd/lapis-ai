//! Terminal outcomes of one prepared turn.
//!
//! An outcome records what was observed. A timeout is an unknown outcome, never
//! a proven success and never a proven failure, because the agent may still be
//! running and may still be producing side effects.

use lapis_acp_protocol::{Error as ProtocolError, StopReason};

use crate::configuration::ConfigurationReport;

/// Why a turn did not complete normally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// The agent executable could not be spawned.
    Spawn,
    /// The agent closed its stream before answering.
    AgentExited,
    /// A frame violated the protocol contract.
    Protocol,
    /// The agent returned a JSON-RPC error object.
    AgentError,
    /// The agent stopped reading its stdin.
    WriteTimeout,
}

/// Which bound ended a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutKind {
    /// No frame arrived inside the idle window.
    Idle,
    /// The absolute wall-clock cap elapsed.
    Absolute,
}

/// How a turn ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStatus {
    /// The agent returned a stop reason.
    Completed,
    /// The turn failed for a known reason.
    Failed,
    /// A bound elapsed; the agent's actual state is unknown.
    TimedOut,
    /// Cancellation was requested and the turn stopped.
    Cancelled,
}

/// A non-completion, with detail for operator diagnostics.
///
/// `detail` is diagnostic text and MUST NOT be copied into a terminal record or
/// any other caller-visible output. It can contain an agent's own error message,
/// which is untrusted and may quote prompt content or a credential the agent was
/// given. Callers publish [`Failure::kind`] and [`Failure::agent_code`] instead,
/// both of which come from a fixed set and are safe to emit.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    /// The classified reason.
    pub kind: FailureKind,
    /// Untrusted diagnostic detail, for stderr only.
    pub detail: String,
    /// The agent's JSON-RPC error code when one arrived.
    pub agent_code: Option<i64>,
}

impl Failure {
    /// Build a failure with no agent code.
    pub fn new(kind: FailureKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
            agent_code: None,
        }
    }

    /// Build a failure carrying the agent's own error code.
    pub fn agent(code: i64, detail: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::AgentError,
            detail: detail.into(),
            agent_code: Some(code),
        }
    }
}

/// The complete result of executing one prepared turn.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnOutcome {
    /// How the turn ended.
    pub status: TurnStatus,
    /// The stop reason, present when the agent returned one.
    pub stop_reason: Option<StopReason>,
    /// The failure, present only when the turn did not complete.
    pub failure: Option<Failure>,
    /// Which bound elapsed, present only on a timeout.
    pub timeout: Option<TimeoutKind>,
    /// The session identifier when one was created.
    pub session_id: Option<String>,
    /// What happened to each requested session setting.
    pub configuration: ConfigurationReport,
}

impl TurnOutcome {
    /// A completed turn carrying its stop reason.
    pub fn completed(stop_reason: StopReason, session_id: String) -> Self {
        Self {
            status: TurnStatus::Completed,
            stop_reason: Some(stop_reason),
            failure: None,
            timeout: None,
            session_id: Some(session_id),
            configuration: ConfigurationReport::default(),
        }
    }

    /// A turn that failed for a known reason.
    pub fn failed(failure: Failure, session_id: Option<String>) -> Self {
        Self {
            status: TurnStatus::Failed,
            stop_reason: None,
            failure: Some(failure),
            timeout: None,
            session_id,
            configuration: ConfigurationReport::default(),
        }
    }

    /// A turn ended by a bound, whose true agent state is unknown.
    pub fn timed_out(timeout: TimeoutKind, session_id: Option<String>) -> Self {
        Self {
            status: TurnStatus::TimedOut,
            stop_reason: None,
            failure: None,
            timeout: Some(timeout),
            session_id,
            configuration: ConfigurationReport::default(),
        }
    }

    /// A turn that stopped after cancellation was requested.
    pub fn cancelled(session_id: Option<String>) -> Self {
        Self {
            status: TurnStatus::Cancelled,
            stop_reason: Some(StopReason::Cancelled),
            failure: None,
            timeout: None,
            session_id,
            configuration: ConfigurationReport::default(),
        }
    }

    /// Attach the configuration report to an outcome.
    pub fn with_configuration(mut self, configuration: ConfigurationReport) -> Self {
        self.configuration = configuration;
        self
    }

    /// Classify a protocol error into a timeout or a failure.
    pub fn from_protocol(error: &ProtocolError, session_id: Option<String>) -> Self {
        match error {
            ProtocolError::IdleTimeout(_) => Self::timed_out(TimeoutKind::Idle, session_id),
            ProtocolError::Timeout(_) => Self::timed_out(TimeoutKind::Absolute, session_id),
            ProtocolError::AgentError { code, message } => {
                Self::failed(Failure::agent(*code, message.clone()), session_id)
            }
            ProtocolError::AgentExited => Self::failed(
                Failure::new(FailureKind::AgentExited, "agent stream ended"),
                session_id,
            ),
            ProtocolError::WriteTimeout(_) => Self::failed(
                Failure::new(FailureKind::WriteTimeout, error.to_string()),
                session_id,
            ),
            other => Self::failed(
                Failure::new(FailureKind::Protocol, other.to_string()),
                session_id,
            ),
        }
    }
}
