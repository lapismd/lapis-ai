//! Bounded streamed activity and exact permission lifetimes.
use lapis_acp_protocol::handler::SessionHandler;
use lapis_acp_protocol::permission::{PermissionDecision, PermissionRequest};
use lapis_acp_protocol::update::SessionUpdate;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch};

/// One permission, owned only while the adapter awaits a response.
pub struct PendingPermission {
    pub options: Vec<String>,
    pub answer: oneshot::Sender<Option<String>>,
    pub scope: Option<String>,
}
/// Live permission decisions; independent of any durable application state.
pub type Permissions = Arc<Mutex<HashMap<String, PendingPermission>>>;
/// Shared ordered stream for one interactive session.
#[derive(Clone)]
pub struct Events {
    pub output: mpsc::Sender<Value>,
    pub sequence: Arc<AtomicU64>,
    pub cancel: watch::Sender<bool>,
    pub coverage_lost: Arc<AtomicBool>,
    pub scope: Option<String>,
    pub version: u64,
}
struct PermissionLifetime {
    permissions: Permissions,
    id: String,
}
impl Drop for PermissionLifetime {
    fn drop(&mut self) {
        if let Ok(mut map) = self.permissions.lock() {
            map.remove(&self.id);
        }
    }
}
/// Caller-observed handler; content is sent to the trusted parent, never logged.
pub struct Handler {
    pub events: Events,
    pub request_id: String,
    pub permissions: Permissions,
    pub timeout: Duration,
    pub pending_rpc: Option<Value>,
    pub adapter_session: Option<String>,
    pub known_sessions: std::collections::HashSet<String>,
}
impl SessionHandler for Handler {
    fn accept_session(&self, session_id: Option<&str>) -> Result<bool, lapis_acp_protocol::Error> {
        if session_id != self.adapter_session.as_deref()
            && session_id.is_some_and(|id| self.known_sessions.contains(id))
        {
            return Ok(false);
        }
        self.validate_session(session_id)?;
        Ok(true)
    }
    fn validate_session(&self, session_id: Option<&str>) -> Result<(), lapis_acp_protocol::Error> {
        if self
            .adapter_session
            .as_deref()
            .is_some_and(|expected| Some(expected) != session_id)
        {
            return Err(lapis_acp_protocol::Error::Protocol(
                "session_scope_mismatch".into(),
            ));
        }
        Ok(())
    }
    fn pending_permission(&self) -> Option<Value> {
        self.pending_rpc.clone()
    }
    fn session_update(&mut self, update: SessionUpdate) {
        self.events
            .emit("activity", &self.request_id, json!({"update":update.raw()}));
    }
    async fn session_update_async(&mut self, update: SessionUpdate) {
        self.events
            .emit_wait("activity", &self.request_id, json!({"update":update.raw()}))
            .await;
    }
    async fn permission(&mut self, request: PermissionRequest) -> PermissionDecision {
        self.pending_rpc = Some(request.id.clone());
        let id = format!(
            "permission-{}",
            self.events.sequence.fetch_add(1, Ordering::Relaxed) + 1
        );
        let (sender, receiver) = oneshot::channel();
        let options = request
            .options
            .iter()
            .map(|option| option.option_id.clone())
            .collect();
        if let Ok(mut map) = self.permissions.lock() {
            map.insert(
                id.clone(),
                PendingPermission {
                    options,
                    answer: sender,
                    scope: self.events.scope.clone(),
                },
            );
        } else {
            return PermissionDecision::Cancel;
        }
        let lifetime = PermissionLifetime {
            permissions: self.permissions.clone(),
            id: id.clone(),
        };
        self.events
            .emit_wait(
                "permission",
                &self.request_id,
                json!({"permissionId":id,"request":request.raw}),
            )
            .await;
        let mut cancel = self.events.cancel.subscribe();
        let decision = tokio::select! {
            result=tokio::time::timeout(self.timeout,receiver)=>match result {
                Ok(Ok(Some(option)))=>PermissionDecision::Select(option), _=>PermissionDecision::Cancel,
            },
            ()=lapis_acp_runner::persistent::cancellation(&mut cancel)=>PermissionDecision::Cancel,
        };
        self.events
            .emit_wait(
                "permission_resolved",
                &self.request_id,
                json!({"permissionId":id,
            "optionId":match &decision {PermissionDecision::Select(option)=>Some(option),_=>None}}),
            )
            .await;
        self.pending_rpc = None;
        drop(lifetime);
        decision
    }
}
