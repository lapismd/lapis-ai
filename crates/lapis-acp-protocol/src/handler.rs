//! The inbound-traffic hook a caller supplies while a request is outstanding.

use std::future::Future;

use serde_json::Value;

use crate::permission::{PermissionDecision, PermissionRequest};
use crate::update::SessionUpdate;

/// Receives agent-initiated traffic that arrives while awaiting a response.
///
/// Every method has a default that ignores the event, so a caller implements
/// only what it cares about. The one exception is [`permission`](Self::permission),
/// whose default is cancellation: an unanswered approval request must never
/// silently become an approval.
///
/// The approval method returns `impl Future + Send` rather than being declared
/// `async`, so an implementor whose decision awaits a controlling application
/// still yields a `Send` future and the connection stays usable inside a spawned
/// task. The trait is consumed through generics, never through `dyn`.
pub trait SessionHandler {
    /// An unanswered permission's RPC identity, for cooperative cancellation.
    fn pending_permission(&self) -> Option<Value> {
        None
    }

    /// Validate inbound adapter session identity before delivering traffic.
    fn validate_session(&self, _session_id: Option<&str>) -> Result<(), crate::Error> {
        Ok(())
    }
    /// Known inactive session traffic can be discarded without changing the current scope.
    fn accept_session(&self, session_id: Option<&str>) -> Result<bool, crate::Error> {
        self.validate_session(session_id)?;
        Ok(true)
    }

    /// A `session/update` notification arrived.
    fn session_update(&mut self, _update: SessionUpdate) {}

    /// The agent asked for approval.
    ///
    /// The default refuses by cancelling. A caller that wants unattended
    /// approval must choose it explicitly, select an option by semantic kind
    /// rather than by identifier, and record that it did so.
    fn permission(
        &mut self,
        request: PermissionRequest,
    ) -> impl Future<Output = PermissionDecision> + Send {
        drop(request);
        async { PermissionDecision::Cancel }
    }

    /// A notification other than `session/update` arrived.
    fn notification(&mut self, _method: &str, _params: &Value) {}

    /// An agent-initiated request this crate does not implement arrived.
    ///
    /// A JSON-RPC method-not-found error is returned to the agent
    /// automatically, because silence would leave it waiting forever. This hook
    /// exists so a caller can observe what was refused.
    fn request(&mut self, _method: &str, _params: &Value) {}

    /// A frame was neither a response, a request nor a notification.
    fn malformed(&mut self, _frame: &Value) {}
}

/// A handler that records everything and approves nothing.
///
/// Useful for tests and for a caller that only needs the terminal stop reason.
#[derive(Debug, Default)]
pub struct RecordingHandler {
    /// Classified session updates in arrival order.
    pub updates: Vec<SessionUpdate>,
    /// Method names of other notifications in arrival order.
    pub notifications: Vec<String>,
    /// Method names of unimplemented agent requests in arrival order.
    pub requests: Vec<String>,
    /// Frames that could not be classified.
    pub malformed: usize,
    /// Permission requests received, in arrival order.
    pub permissions: usize,
}

impl SessionHandler for RecordingHandler {
    fn session_update(&mut self, update: SessionUpdate) {
        self.updates.push(update);
    }

    fn permission(
        &mut self,
        _request: PermissionRequest,
    ) -> impl Future<Output = PermissionDecision> + Send {
        self.permissions += 1;
        async { PermissionDecision::Cancel }
    }

    fn notification(&mut self, method: &str, _params: &Value) {
        self.notifications.push(method.to_owned());
    }

    fn request(&mut self, method: &str, _params: &Value) {
        self.requests.push(method.to_owned());
    }

    fn malformed(&mut self, _frame: &Value) {
        self.malformed += 1;
    }
}
