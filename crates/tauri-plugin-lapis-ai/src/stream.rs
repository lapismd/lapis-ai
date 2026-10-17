use crate::{AcpHost, Error, HostContext, Result, registry::Subscription};
use lapis_ai_runtime::SessionSnapshot;
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamMessage {
    pub subscription_id: String,
    pub session_id: String,
    pub snapshot: SessionSnapshot,
}
pub type Sink = Arc<dyn Fn(StreamMessage) -> std::result::Result<(), ()> + Send + Sync>;
impl AcpHost {
    pub fn attach(
        &self,
        context: &HostContext,
        id: &str,
        after: u64,
        sink: Sink,
    ) -> Result<String> {
        let scope = self.scope(context)?;
        let session = self.session(context, id)?;
        session.snapshot(after)?;
        let changed = session.subscribe()?;
        let subscription = Uuid::new_v4().to_string();
        let (stop, mut cancelled) = watch::channel(false);
        let delivered = Arc::new(AtomicU64::new(after));
        {
            let mut state = self.state.lock().map_err(|_| Error("host_unavailable"))?;
            state.slot(&scope, id)?;
            let slot = state.slots.get_mut(id).ok_or(Error("unknown_session"))?;
            slot.subscription = Some(Subscription {
                id: subscription.clone(),
                stop,
                delivered: delivered.clone(),
            });
        }
        let policy = self.policy.clone();
        let context = context.clone();
        let id = id.to_owned();
        let token = subscription.clone();
        // Synchronous Tauri commands may run outside an entered Tokio context.
        tauri::async_runtime::spawn(async move {
            let mut changed = changed;
            let mut cursor = after;
            loop {
                if *cancelled.borrow()
                    || !policy
                        .authorize(&context)
                        .is_ok_and(|admission| admission == scope.admission)
                {
                    break;
                }
                let Ok(snapshot) = session.snapshot(cursor) else {
                    break;
                };
                cursor = snapshot.sequence;
                delivered.store(cursor, Ordering::Release);
                let closed = snapshot.closed;
                if sink(StreamMessage {
                    subscription_id: token.clone(),
                    session_id: id.clone(),
                    snapshot,
                })
                .is_err()
                {
                    break;
                }
                if closed {
                    let _ = session.close().await;
                    if !*cancelled.borrow()
                        && policy
                            .authorize(&context)
                            .is_ok_and(|admission| admission == scope.admission)
                        && let Ok(snapshot) = session.snapshot(cursor)
                    {
                        let _ = sink(StreamMessage {
                            subscription_id: token,
                            session_id: id,
                            snapshot,
                        });
                    }
                    break;
                }
                tokio::select! { biased; _ = cancelled.changed() => break, result = changed.changed() => if result.is_err() { break; } }
            }
        });
        Ok(subscription)
    }
    pub fn detach(&self, context: &HostContext, id: &str, subscription: &str) -> Result<()> {
        let scope = self.scope(context)?;
        let mut state = self.state.lock().map_err(|_| Error("host_unavailable"))?;
        let slot = state.slot(&scope, id)?;
        if !slot
            .subscription
            .as_ref()
            .is_some_and(|stream| stream.id == subscription)
        {
            return Err(Error("stale_subscription"));
        }
        state
            .slots
            .get_mut(id)
            .ok_or(Error("unknown_session"))?
            .subscription
            .take();
        Ok(())
    }
    pub fn acknowledge(
        &self,
        context: &HostContext,
        id: &str,
        subscription: &str,
        sequence: u64,
    ) -> Result<()> {
        let scope = self.scope(context)?;
        let state = self.state.lock().map_err(|_| Error("host_unavailable"))?;
        let slot = state.slot(&scope, id)?;
        let stream = slot
            .subscription
            .as_ref()
            .filter(|stream| stream.id == subscription)
            .ok_or(Error("stale_subscription"))?;
        if sequence > stream.delivered.load(Ordering::Acquire) {
            return Err(Error("invalid_event_cursor"));
        }
        slot.session
            .as_ref()
            .ok_or(Error("session_starting"))?
            .acknowledge(sequence)
            .map_err(Into::into)
    }
}
