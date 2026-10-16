//! Typed protocol failures.
//!
//! Turn deadlines and cancellation drain bounds are policy and belong to
//! `lapis-acp-runner`. This crate reports what can go wrong on the wire,
//! including the observation that no frame arrived inside a window the caller
//! supplied.

use serde_json::Value;

/// Errors produced while speaking ACP to an agent.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The underlying byte stream failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A frame was not valid JSON.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// The agent closed its stdout before answering.
    #[error("agent stream ended before a response arrived")]
    AgentExited,

    /// A single frame exceeded the configured byte cap.
    #[error("frame exceeded the {limit} byte cap")]
    FrameTooLarge {
        /// The cap that was exceeded.
        limit: usize,
    },

    /// No response arrived within the configured request interval.
    #[error("agent did not respond within {0:?}")]
    Timeout(std::time::Duration),

    /// No frame at all arrived within the caller's idle window.
    ///
    /// This is an observation, not a verdict: the agent may still be working
    /// and may still produce output later.
    #[error("no agent frame arrived for {0:?}")]
    IdleTimeout(std::time::Duration),

    /// The agent stopped reading its stdin.
    #[error("agent stopped reading stdin for {0:?}")]
    WriteTimeout(std::time::Duration),

    /// A frame violated the protocol contract.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// The agent returned a JSON-RPC error object.
    #[error("agent reported error (code {code}): {message}")]
    AgentError {
        /// The JSON-RPC error code, or the default when absent.
        code: i64,
        /// The agent's message, or the whole error object when absent.
        message: String,
    },

    /// The negotiated agent does not advertise a requested capability.
    #[error("agent does not advertise {0}")]
    Unsupported(&'static str),
}

/// The JSON-RPC code used when an agent asks for a method we do not implement.
pub const METHOD_NOT_FOUND: i64 = -32601;

/// The JSON-RPC code used when an agent reports a failure without one.
pub const DEFAULT_AGENT_ERROR_CODE: i64 = -32000;

/// Build [`Error::AgentError`] from a JSON-RPC error object.
///
/// The numeric code is preserved. When `message` is missing or not a string the
/// whole object is used instead, so provider-specific detail such as a `data`
/// field survives rather than being replaced by an empty message.
pub fn agent_error(error: &Value) -> Error {
    let code = error
        .get("code")
        .and_then(Value::as_i64)
        .unwrap_or(DEFAULT_AGENT_ERROR_CODE);
    let message = match error.get("message").and_then(Value::as_str) {
        Some(message) => message.to_owned(),
        None => error.to_string(),
    };
    Error::AgentError { code, message }
}
