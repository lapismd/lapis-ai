use crate::{Error, Result};
use lapis_ai_runtime::AcpSession;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, atomic::AtomicU64},
};
use tokio::sync::watch;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Scope {
    pub window: String,
    pub admission: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub request_id: String,
    pub target_id: String,
    pub facts: Value,
}
pub(crate) struct Subscription {
    pub id: String,
    pub stop: watch::Sender<bool>,
    pub delivered: Arc<AtomicU64>,
}
impl Drop for Subscription {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}
pub(crate) struct Slot {
    pub scope: Scope,
    pub request: String,
    pub target: String,
    pub session: Option<AcpSession>,
    pub cancel: watch::Sender<bool>,
    pub subscription: Option<Subscription>,
    pub startup_done: Option<watch::Receiver<Option<Result<()>>>>,
}
impl Slot {
    pub fn info(&self) -> Option<SessionInfo> {
        self.session.as_ref().map(|session| SessionInfo {
            session_id: session.id().into(),
            request_id: self.request.clone(),
            target_id: self.target.clone(),
            facts: session.facts().clone(),
        })
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}
pub(crate) struct Registry {
    pub slots: HashMap<String, Slot>,
    pub seen: HashSet<(String, String, String)>,
    pub enabled: bool,
    pub stopped: bool,
}
impl Registry {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
            seen: HashSet::new(),
            enabled: true,
            stopped: false,
        }
    }
    pub fn available(&self) -> Result<()> {
        if !self.enabled || self.stopped {
            Err(Error("disabled"))
        } else {
            Ok(())
        }
    }
    pub fn slot(&self, scope: &Scope, id: &str) -> Result<&Slot> {
        self.available()?;
        self.slots
            .get(id)
            .filter(|slot| slot.scope == *scope)
            .ok_or(Error("unknown_session"))
    }
}
pub(crate) struct Reservation {
    state: Arc<Mutex<Registry>>,
    pub id: String,
    committed: bool,
    done: watch::Sender<Option<Result<()>>>,
    cleanup_result: Result<()>,
}
impl Reservation {
    pub fn new(
        state: Arc<Mutex<Registry>>,
        id: String,
        done: watch::Sender<Option<Result<()>>>,
    ) -> Self {
        Self {
            state,
            id,
            committed: false,
            done,
            cleanup_result: Err(Error("cleanup_unknown")),
        }
    }
    pub fn finished(&mut self, result: Result<()>) {
        self.cleanup_result = result;
    }
    pub fn commit(&mut self, session: AcpSession) -> Result<SessionInfo> {
        let mut state = self.state.lock().map_err(|_| Error("host_unavailable"))?;
        state.available()?;
        let slot = state
            .slots
            .get_mut(&self.id)
            .ok_or(Error("admission_revoked"))?;
        slot.session = Some(session);
        slot.startup_done = None;
        let info = slot.info().ok_or(Error("host_unavailable"))?;
        self.committed = true;
        Ok(info)
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if !self.committed
            && let Ok(mut state) = self.state.lock()
        {
            state.slots.remove(&self.id);
        }
        self.done.send_replace(Some(self.cleanup_result));
    }
}
