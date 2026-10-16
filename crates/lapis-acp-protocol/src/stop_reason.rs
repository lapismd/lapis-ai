//! Turn termination reasons.

use serde::{Deserialize, Serialize};

/// Why an agent stopped a turn.
///
/// A turn terminates in exactly one of these. An absent or unrecognized
/// `stopReason` is a protocol error rather than a silent default, because the
/// controlling application acts on this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The agent completed the turn normally.
    EndTurn,
    /// The turn was cancelled through `session/cancel`.
    Cancelled,
    /// The agent reached its token limit.
    MaxTokens,
    /// The agent reached its per-turn request limit.
    MaxTurnRequests,
    /// The agent refused the prompt. Refused turns are dropped from agent history.
    Refusal,
}

impl StopReason {
    /// Parse a `stopReason` from the wire format.
    ///
    /// Matching is case-insensitive so an agent that sends `END_TURN` or
    /// `Cancelled` is handled without a protocol error.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "end_turn" => Some(Self::EndTurn),
            "cancelled" => Some(Self::Cancelled),
            "max_tokens" => Some(Self::MaxTokens),
            "max_turn_requests" => Some(Self::MaxTurnRequests),
            "refusal" => Some(Self::Refusal),
            _ => None,
        }
    }

    /// The wire spelling of this reason.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EndTurn => "end_turn",
            Self::Cancelled => "cancelled",
            Self::MaxTokens => "max_tokens",
            Self::MaxTurnRequests => "max_turn_requests",
            Self::Refusal => "refusal",
        }
    }
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
