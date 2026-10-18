use crate::Result;
use lapis_ai_runtime::{PreparedLaunch, SessionProcessor};
use serde::Serialize;
use std::{path::PathBuf, time::Duration};
/// Supplied by native IPC, never deserialized from renderer arguments.
#[derive(Clone)]
pub struct HostContext {
    pub window: String,
    pub url: String,
}
#[derive(Clone, Serialize)]
pub struct Target {
    pub id: String,
    pub label: String,
}
pub struct Launch {
    pub prepared: PreparedLaunch,
    pub directory: PathBuf,
    pub resume: Option<String>,
}
/// Metadata from a reserved native slot, never deserialized from IPC arguments.
/// `session_id` is the host worker handle, not a provider or conversation ID.
#[derive(Clone)]
pub struct SessionBinding {
    pub session_id: String,
    pub request_id: String,
    pub target_id: String,
    pub admission: String,
}
pub trait HostPolicy: Send + Sync + 'static {
    /// Return the current host-owned window/vault admission generation.
    fn authorize(&self, context: &HostContext) -> Result<String>;
    fn targets(&self, context: &HostContext) -> Result<Vec<Target>>;
    /// Resolve a consumer binding only after checking its current admission.
    fn resolve(
        &self,
        context: &HostContext,
        target: &str,
        resume_binding: Option<&str>,
    ) -> Result<Launch>;
    /// Construct one processor for this admitted worker. Required capture must
    /// return Some or an error; None explicitly retains the unprocessed profile.
    fn processor(
        &self,
        _context: &HostContext,
        _binding: &SessionBinding,
    ) -> Result<Option<Box<dyn SessionProcessor>>> {
        Ok(None)
    }
}
#[derive(Clone)]
pub struct Config {
    pub maximum_sessions: usize,
    pub request_timeout: Duration,
    pub event_limit: usize,
    pub event_byte_limit: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            maximum_sessions: 32,
            request_timeout: Duration::from_secs(30),
            event_limit: 256,
            event_byte_limit: 8 * 1024 * 1024,
        }
    }
}
pub(crate) fn label(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
