pub use crate::registry::SessionInfo;
use crate::{
    Config, Error, HostContext, HostPolicy, Result, Target,
    policy::label,
    registry::{Registry, Scope, Slot},
};
use lapis_ai_runtime::{AcpSession, SessionRequest, SessionSnapshot};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct AcpHost {
    pub(crate) policy: Arc<dyn HostPolicy>,
    pub(crate) state: Arc<Mutex<Registry>>,
    pub(crate) config: Config,
}
impl AcpHost {
    pub fn new(policy: Arc<dyn HostPolicy>, config: Config) -> Result<Self> {
        if !(1..=64).contains(&config.maximum_sessions)
            || config.request_timeout.is_zero()
            || config.request_timeout > Duration::from_secs(300)
            || !(4..=4096).contains(&config.event_limit)
            || !(1024..=64 * 1024 * 1024).contains(&config.event_byte_limit)
        {
            return Err(Error("invalid_host_configuration"));
        }
        Ok(Self {
            policy,
            state: Arc::new(Mutex::new(Registry::new())),
            config,
        })
    }
    pub(crate) fn scope(&self, context: &HostContext) -> Result<Scope> {
        let admission = self.policy.authorize(context)?;
        if !label(&context.window) || !label(&admission) {
            return Err(Error("denied"));
        }
        self.state
            .lock()
            .map_err(|_| Error("host_unavailable"))?
            .available()?;
        Ok(Scope {
            window: context.window.clone(),
            admission,
        })
    }
    pub fn targets(&self, context: &HostContext) -> Result<Vec<Target>> {
        self.scope(context)?;
        let targets = self.policy.targets(context)?;
        let mut seen = std::collections::HashSet::new();
        if targets.len() > 64
            || targets.iter().any(|target| {
                !label(&target.id) || !label(&target.label) || !seen.insert(&target.id)
            })
        {
            return Err(Error("invalid_targets"));
        }
        Ok(targets)
    }
    pub fn list(&self, context: &HostContext) -> Result<Vec<SessionInfo>> {
        let scope = self.scope(context)?;
        let state = self.state.lock().map_err(|_| Error("host_unavailable"))?;
        state.available()?;
        let mut sessions: Vec<_> = state
            .slots
            .values()
            .filter(|slot| slot.scope == scope)
            .filter_map(Slot::info)
            .collect();
        sessions.sort_by(|left, right| left.session_id.cmp(&right.session_id));
        Ok(sessions)
    }
    pub(crate) fn session(&self, context: &HostContext, id: &str) -> Result<AcpSession> {
        let scope = self.scope(context)?;
        self.state
            .lock()
            .map_err(|_| Error("host_unavailable"))?
            .slot(&scope, id)?
            .session
            .clone()
            .ok_or(Error("session_starting"))
    }
    pub async fn submit(
        &self,
        context: &HostContext,
        id: &str,
        request: String,
        input: SessionRequest,
    ) -> Result<()> {
        self.session(context, id)?
            .submit(request, input)
            .await
            .map_err(Into::into)
    }
    pub fn snapshot(&self, context: &HostContext, id: &str, after: u64) -> Result<SessionSnapshot> {
        self.session(context, id)?
            .snapshot(after)
            .map_err(Into::into)
    }
    pub async fn close(&self, context: &HostContext, id: &str) -> Result<()> {
        let scope = self.scope(context)?;
        let slot = {
            let mut state = self.state.lock().map_err(|_| Error("host_unavailable"))?;
            state.slot(&scope, id)?;
            state.slots.remove(id).ok_or(Error("unknown_session"))?
        };
        Self::stop_slots(vec![slot]).await
    }
}
