use crate::{Error, Result};
use std::time::Duration;

/// Host-selected transport bounds, independent of application orchestration.
#[derive(Clone, Copy, Debug)]
pub struct ProcessLimits {
    pub maximum_frame_bytes: usize,
    pub maximum_request_bytes: usize,
    pub queued_frames: usize,
    pub write_timeout: Duration,
}
impl Default for ProcessLimits {
    fn default() -> Self {
        Self {
            maximum_frame_bytes: 4 * 1024 * 1024,
            maximum_request_bytes: 4 * 1024 * 1024,
            queued_frames: 256,
            write_timeout: Duration::from_secs(10),
        }
    }
}
impl ProcessLimits {
    pub(crate) fn validate(self) -> Result<Self> {
        if self.maximum_frame_bytes == 0
            || self.maximum_frame_bytes > 4 * 1024 * 1024
            || self.maximum_request_bytes == 0
            || self.maximum_request_bytes > 4 * 1024 * 1024
            || self.queued_frames == 0
            || self.queued_frames > 256
            || self.write_timeout.is_zero()
        {
            return Err(Error("invalid_process_limits"));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AcpOptions {
    pub limits: ProcessLimits,
    pub readiness_timeout: Duration,
    pub shutdown_grace: Duration,
}
impl Default for AcpOptions {
    fn default() -> Self {
        Self {
            limits: ProcessLimits {
                maximum_frame_bytes: 1_048_576,
                ..Default::default()
            },
            readiness_timeout: Duration::from_secs(15),
            shutdown_grace: Duration::from_secs(5),
        }
    }
}
