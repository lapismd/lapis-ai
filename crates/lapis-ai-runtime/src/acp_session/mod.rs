//! Rust-owned sessions; applications retain admission, credentials and history.
mod actor;
mod bootstrap;
mod journal;
mod processing;
mod request;
mod routing;
mod startup;
use crate::{Error, PreparedLaunch, Result};
pub use journal::{SessionEvent, SessionSnapshot};
pub use processing::{FrameInput, FramePublication, ProcessorFuture, SessionProcessor};
pub use request::SessionRequest;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot, watch};

/// Trusted Rust inputs. Never deserialize a renderer-supplied launch or directory.
pub struct SessionOptions {
    pub handle: String,
    pub directory: PathBuf,
    pub resume: Option<String>,
    pub request_timeout: Duration,
    pub event_limit: usize,
    pub event_byte_limit: usize,
}
impl SessionOptions {
    pub fn new(handle: String, directory: PathBuf) -> Self {
        Self {
            handle,
            directory,
            resume: None,
            request_timeout: Duration::from_secs(30),
            event_limit: 256,
            event_byte_limit: 8 * 1024 * 1024,
        }
    }
    fn validate(&self) -> Result<()> {
        if !request::label(&self.handle)
            || self.resume.as_ref().is_some_and(|id| !request::label(id))
            || self.request_timeout.is_zero()
            || self.request_timeout > Duration::from_secs(300)
            || !(4..=4096).contains(&self.event_limit)
            || !(1024..=64 * 1024 * 1024).contains(&self.event_byte_limit)
        {
            return Err(Error("invalid_session_options"));
        }
        Ok(())
    }
}
struct Inner {
    id: String,
    facts: Value,
    journal: Arc<Mutex<journal::Journal>>,
    commands: mpsc::Sender<actor::Envelope>,
    stop: watch::Sender<bool>,
    done: watch::Receiver<bool>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}
/// Clones share one worker. Dropping the last owner begins supervised shutdown.
#[derive(Clone)]
pub struct AcpSession(Arc<Inner>);
impl AcpSession {
    pub async fn start(plan: PreparedLaunch, options: SessionOptions) -> Result<Self> {
        let (_owner, cancel) = watch::channel(false);
        Self::start_cancellable(plan, options, cancel).await
    }
    /// Native owners can revoke startup and await complete supervised cleanup.
    pub async fn start_cancellable(
        plan: PreparedLaunch,
        options: SessionOptions,
        cancel: watch::Receiver<bool>,
    ) -> Result<Self> {
        startup::start(plan, options, cancel, None).await
    }
    /// Process startup facts and events before exposure. The host owns durability.
    pub async fn start_with_processor(
        plan: PreparedLaunch,
        options: SessionOptions,
        processor: Box<dyn SessionProcessor>,
    ) -> Result<Self> {
        let (_owner, cancel) = watch::channel(false);
        Self::start_cancellable_with_processor(plan, options, cancel, processor).await
    }
    /// Cancellation drains any admitted processing before startup cleanup finishes.
    pub async fn start_cancellable_with_processor(
        plan: PreparedLaunch,
        options: SessionOptions,
        cancel: watch::Receiver<bool>,
        processor: Box<dyn SessionProcessor>,
    ) -> Result<Self> {
        startup::start(plan, options, cancel, Some(processor)).await
    }
    pub fn id(&self) -> &str {
        &self.0.id
    }
    pub fn facts(&self) -> &Value {
        &self.0.facts
    }
    pub async fn submit(&self, request_id: String, request: SessionRequest) -> Result<()> {
        request.validate(&request_id)?;
        if *self.0.stop.borrow() {
            return Err(Error("session_closed"));
        }
        let (reply, received) = oneshot::channel();
        self.0
            .commands
            .try_send(actor::Envelope {
                id: request_id,
                request,
                reply,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => Error("queue_full"),
                mpsc::error::TrySendError::Closed(_) => Error("session_closed"),
            })?;
        received.await.map_err(|_| Error("submission_unknown"))?
    }
    pub fn snapshot(&self, after: u64) -> Result<SessionSnapshot> {
        self.0
            .journal
            .lock()
            .map_err(|_| Error("session_unavailable"))?
            .snapshot(after)
    }
    pub fn acknowledge(&self, sequence: u64) -> Result<()> {
        self.0
            .journal
            .lock()
            .map_err(|_| Error("session_unavailable"))?
            .acknowledge(sequence)
    }
    /// Notifications carry no transcript. Subscribe before reading a snapshot.
    pub fn subscribe(&self) -> Result<watch::Receiver<u64>> {
        Ok(self
            .0
            .journal
            .lock()
            .map_err(|_| Error("session_unavailable"))?
            .subscribe())
    }
    /// Detach drops only the consumer subscription; it never closes this session.
    pub async fn close(&self) -> Result<()> {
        self.0.stop.send_replace(true);
        let mut done = self.0.done.clone();
        tokio::time::timeout(Duration::from_secs(30), async {
            while !*done.borrow_and_update() {
                done.changed().await.map_err(|_| Error("cleanup_unknown"))?;
            }
            Ok::<(), Error>(())
        })
        .await
        .map_err(|_| Error("cleanup_unknown"))??;
        self.0
            .journal
            .lock()
            .map_err(|_| Error("session_unavailable"))?
            .cleanup_result()
    }
}
