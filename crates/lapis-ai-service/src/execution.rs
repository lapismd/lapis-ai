use lapis_ai_runtime::{
    AcpSession, PreparedLaunch, Result, SessionOptions, SessionProcessor, SessionRequest,
    SessionSnapshot,
};
use serde_json::Value;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::watch;

pub type ExecutionFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Native-only execution port. Policy has already admitted the exact launch.
/// Implementations must own startup through cancellation and all cleanup effects.
/// An error must never abandon a live process or silently select another backend.
pub trait ExecutionBackend: Send + Sync + 'static {
    fn start(
        &self,
        launch: PreparedLaunch,
        options: SessionOptions,
        cancelled: watch::Receiver<bool>,
        processor: Option<Box<dyn SessionProcessor>>,
    ) -> ExecutionFuture<'_, Arc<dyn ExecutionSession>>;
}

/// One owned worker, independent of whether its execution is local or remote.
/// `close` confirms all execution resources have retired, or reports uncertainty.
/// Implementations preserve the supplied handle and immutable negotiated facts.
pub trait ExecutionSession: Send + Sync + 'static {
    fn id(&self) -> &str;
    fn facts(&self) -> &Value;
    fn submit(&self, id: String, request: SessionRequest) -> ExecutionFuture<'_, ()>;
    fn snapshot(&self, after: u64) -> Result<SessionSnapshot>;
    fn acknowledge(&self, sequence: u64) -> Result<()>;
    fn subscribe(&self) -> Result<watch::Receiver<u64>>;
    fn close(&self) -> ExecutionFuture<'_, ()>;
}

/// Initial execution implementation; starts no Docker daemon or container.
#[derive(Default)]
pub struct HostExecution;
impl ExecutionBackend for HostExecution {
    fn start(
        &self,
        launch: PreparedLaunch,
        options: SessionOptions,
        cancelled: watch::Receiver<bool>,
        processor: Option<Box<dyn SessionProcessor>>,
    ) -> ExecutionFuture<'_, Arc<dyn ExecutionSession>> {
        Box::pin(async move {
            let session = match processor {
                Some(processor) => {
                    AcpSession::start_cancellable_with_processor(
                        launch, options, cancelled, processor,
                    )
                    .await?
                }
                None => AcpSession::start_cancellable(launch, options, cancelled).await?,
            };
            Ok(Arc::new(session) as Arc<dyn ExecutionSession>)
        })
    }
}
impl ExecutionSession for AcpSession {
    fn id(&self) -> &str {
        self.id()
    }
    fn facts(&self) -> &Value {
        self.facts()
    }
    fn submit(&self, id: String, request: SessionRequest) -> ExecutionFuture<'_, ()> {
        Box::pin(self.submit(id, request))
    }
    fn snapshot(&self, after: u64) -> Result<SessionSnapshot> {
        self.snapshot(after)
    }
    fn acknowledge(&self, sequence: u64) -> Result<()> {
        self.acknowledge(sequence)
    }
    fn subscribe(&self) -> Result<watch::Receiver<u64>> {
        self.subscribe()
    }
    fn close(&self) -> ExecutionFuture<'_, ()> {
        Box::pin(self.close())
    }
}
