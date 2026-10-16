use super::request::{Kind, SessionRequest, label};
use crate::{Error, Result};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use tokio::time::Instant;

pub struct Pending {
    pub kind: Kind,
    pub deadline: Instant,
}
pub struct Routing {
    pub pending: HashMap<String, Pending>,
    seen: HashSet<String>,
    active: Option<String>,
    permissions: HashMap<String, HashSet<String>>,
    native_sequence: u64,
    mode_ready: bool,
    adapter_session: String,
}
impl Routing {
    pub fn new(sequence: u64, adapter_session: String) -> Self {
        Self {
            pending: HashMap::new(),
            seen: HashSet::new(),
            active: None,
            permissions: HashMap::new(),
            native_sequence: sequence,
            mode_ready: false,
            adapter_session,
        }
    }
    pub fn admit(&mut self, id: &str, request: &SessionRequest, timeout: Duration) -> Result<()> {
        request.validate(id)?;
        if self.seen.contains(id) {
            return Err(Error("duplicate_request"));
        }
        if self.seen.len() >= 4000 {
            return Err(Error("request_limit"));
        }
        if self.pending.len() >= 32 {
            return Err(Error("queue_full"));
        }
        let kind = request.kind();
        if matches!(kind, Kind::Prompt | Kind::Configure) && self.active.is_some() {
            return Err(Error("session_busy"));
        }
        match request {
            SessionRequest::Prompt { .. } if !self.mode_ready => {
                return Err(Error("execution_mode_required"));
            }
            SessionRequest::Permission {
                permission_id,
                option_id,
            } => {
                if !self
                    .permissions
                    .get(permission_id)
                    .is_some_and(|options| option_id.as_ref().is_none_or(|id| options.contains(id)))
                {
                    return Err(Error("stale_or_invalid_permission"));
                }
                self.permissions.remove(permission_id);
            }
            SessionRequest::Cancel { target } => {
                if self.active.as_deref() != Some(target.as_str()) {
                    return Err(Error("not_in_flight"));
                }
                self.permissions.clear();
            }
            _ => {}
        }
        self.seen.insert(id.into());
        self.pending.insert(
            id.into(),
            Pending {
                kind,
                deadline: Instant::now() + request.timeout(timeout),
            },
        );
        if matches!(kind, Kind::Prompt | Kind::Configure) {
            self.active = Some(id.into());
        }
        if kind == Kind::Configure {
            self.mode_ready = false;
        }
        Ok(())
    }
    /// Validate before delivery. False means the worker is no longer reusable.
    pub fn receive(&mut self, session: &str, frame: &Value) -> Result<bool> {
        let kind = frame["type"]
            .as_str()
            .ok_or(Error("invalid_session_frame"))?;
        if frame["version"] != 3 {
            return Err(Error("invalid_session_frame"));
        }
        if frame.get("sessionId").is_some_and(|value| value != session) {
            return Err(Error("session_scope_mismatch"));
        }
        let id = frame["requestId"]
            .as_str()
            .ok_or(Error("uncorrelated_response"))?;
        let pending = self.pending.get(id).ok_or(Error("uncorrelated_response"))?;
        let expected = pending.kind;
        if let Some(sequence) = frame.get("sequence") {
            let sequence = sequence
                .as_u64()
                .filter(|value| *value > self.native_sequence)
                .ok_or(Error("event_order_invalid"))?;
            self.native_sequence = sequence;
        }
        if matches!(
            kind,
            "started" | "configured" | "activity" | "permission" | "permission_resolved" | "record"
        ) && frame["sessionId"] != session
        {
            return Err(Error("session_scope_mismatch"));
        }
        let terminal = match kind {
            "queued" if matches!(expected, Kind::Configure | Kind::Prompt) => false,
            "started" if expected == Kind::Prompt => {
                if frame["adapterSessionId"] != self.adapter_session {
                    return Err(Error("session_scope_mismatch"));
                }
                false
            }
            "activity" if self.active.as_deref() == Some(id) => false,
            "permission" if self.active.as_deref() == Some(id) => {
                let permission = frame["permissionId"]
                    .as_str()
                    .filter(|id| label(id))
                    .ok_or(Error("invalid_permission_frame"))?;
                let options = frame["request"]["options"]
                    .as_array()
                    .filter(|options| !options.is_empty() && options.len() <= 64)
                    .ok_or(Error("invalid_permission_frame"))?;
                let mut ids = HashSet::new();
                for option in options {
                    let id = option["optionId"]
                        .as_str()
                        .filter(|id| label(id))
                        .ok_or(Error("invalid_permission_frame"))?;
                    if !ids.insert(id.to_owned()) {
                        return Err(Error("invalid_permission_frame"));
                    }
                }
                if self.permissions.len() >= 64
                    || self.permissions.insert(permission.into(), ids).is_some()
                {
                    return Err(Error("invalid_permission_frame"));
                }
                false
            }
            "permission_resolved" if self.active.as_deref() == Some(id) => {
                let permission = frame["permissionId"]
                    .as_str()
                    .ok_or(Error("invalid_permission_frame"))?;
                self.permissions.remove(permission);
                false
            }
            "configured" if expected == Kind::Configure => {
                self.mode_ready = frame["modeReady"] == true;
                true
            }
            "record" if expected == Kind::Prompt => {
                if frame["record"]["taskId"] != id
                    || frame["record"]["version"] != 1
                    || !matches!(
                        frame["record"]["status"].as_str(),
                        Some("completed" | "cancelled" | "outcome_unknown")
                    )
                {
                    return Err(Error("invalid_terminal_record"));
                }
                true
            }
            "ack" if matches!(expected, Kind::Permission | Kind::Cancel) => true,
            "status" if expected == Kind::Status => true,
            "error" => true,
            _ => return Err(Error("unexpected_session_response")),
        };
        if terminal {
            self.pending.remove(id);
            if self.active.as_deref() == Some(id) {
                self.active = None;
                self.permissions.clear();
            }
        }
        Ok(!(kind == "record" && frame["sessionUsable"] != true
            || kind == "error"
                && matches!(
                    frame["code"].as_str(),
                    Some(
                        "timed_out"
                            | "agent_exited"
                            | "protocol_error"
                            | "cancelled"
                            | "agent_rejected"
                    )
                )))
    }
}
