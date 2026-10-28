use crate::{
    interactive_handler::Events,
    interactive_wire::{MAX_FRAME, frame},
};
use serde_json::{Value, json};
use std::{sync::atomic::Ordering, time::Duration};
use tokio::sync::mpsc;

impl Events {
    fn lose_coverage(&self) {
        self.coverage_lost.store(true, Ordering::Release);
        self.cancel.send_replace(true);
    }
    fn prepare(&self, kind: &str, id: &str, mut body: Value) -> Option<Value> {
        if kind == "record" && self.coverage_lost.load(Ordering::Acquire) {
            body["record"]["status"] = json!("outcome_unknown");
            body["record"]["error"] = json!("activity_coverage_lost");
            body["sessionUsable"] = json!(false);
        }
        body["sequence"] = json!(self.sequence.fetch_add(1, Ordering::Relaxed) + 1);
        if let Some(scope) = &self.scope {
            body["sessionId"] = json!(scope);
        }
        let mut value = frame(kind, Some(id), body);
        value["version"] = json!(self.version);
        if serde_json::to_vec(&value).map_or(true, |bytes| bytes.len() > MAX_FRAME) {
            self.lose_coverage();
            return None;
        }
        Some(value)
    }
    /// Synchronous callers retain immediate bounded-overflow failure.
    pub fn emit(&self, kind: &str, id: &str, body: Value) {
        if let Some(value) = self.prepare(kind, id, body)
            && self.output.try_send(value).is_err()
        {
            self.lose_coverage();
        }
    }
    /// Backpressure holds one frame for at most 100ms, without enlarging the queue.
    pub async fn emit_wait(&self, kind: &str, id: &str, body: Value) {
        let Some(value) = self.prepare(kind, id, body) else {
            return;
        };
        match self.output.try_send(value) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Closed(_)) => self.lose_coverage(),
            Err(mpsc::error::TrySendError::Full(value)) => {
                // Dropping a pending handoff cannot leave a completed outcome.
                let mut guard = PendingEmission {
                    events: self,
                    accepted: false,
                };
                guard.accepted = matches!(
                    tokio::time::timeout(Duration::from_millis(100), self.output.send(value)).await,
                    Ok(Ok(()))
                );
            }
        }
    }
}
struct PendingEmission<'a> {
    events: &'a Events,
    accepted: bool,
}
impl Drop for PendingEmission<'_> {
    fn drop(&mut self) {
        if !self.accepted {
            self.events.lose_coverage();
        }
    }
}
