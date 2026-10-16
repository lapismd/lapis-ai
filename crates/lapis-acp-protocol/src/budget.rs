//! Wire limits and turn budgets.

use std::time::Duration;

use crate::frame::DEFAULT_MAX_FRAME_BYTES;

/// Fixed bounds applied to every request on a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum bytes in one frame before it is rejected.
    pub max_frame_bytes: usize,
    /// How long a write may block before the agent is considered to have
    /// stopped reading its stdin.
    pub write_timeout: Duration,
    /// How long a non-prompt request may take. Prompting uses a
    /// [`TurnBudget`] instead, because a turn legitimately runs for minutes.
    pub request_timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
            write_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(60),
        }
    }
}

/// The two bounds that apply while waiting for a turn to finish.
///
/// The idle bound resets on every frame the agent emits, so a long but visibly
/// progressing turn is not killed. The absolute bound never resets, so a turn
/// that emits output forever still ends. The shorter remaining bound always
/// wins, and a caller may lower either but never escape both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnBudget {
    /// Maximum silence tolerated between frames.
    pub idle: Duration,
    /// Maximum wall clock for the whole exchange.
    pub absolute: Duration,
}

impl TurnBudget {
    /// Build a budget from an idle window and an absolute cap.
    ///
    /// Both are clamped to at least one millisecond so a zero value cannot
    /// produce a deadline that has already passed and a confusing immediate
    /// timeout.
    pub fn new(idle: Duration, absolute: Duration) -> Self {
        let floor = Duration::from_millis(1);
        Self {
            idle: idle.max(floor),
            absolute: absolute.max(floor),
        }
    }

    /// Build a budget that behaves as one hard deadline.
    ///
    /// Both bounds are the same interval, so resetting the idle window can
    /// never extend the exchange past it. This is what non-prompt requests use.
    pub fn fixed(timeout: Duration) -> Self {
        Self::new(timeout, timeout)
    }
}
