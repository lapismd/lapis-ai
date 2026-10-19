use crate::{Config, Error, framing};
use serde_json::Value;
use tokio::{io::AsyncWrite, sync::mpsc, task::JoinHandle};

/// Keep blocked stdout independent from input cancellation and EOF. Both the
/// queue and its currently encoded frame are bounded; the owner reaps this task.
pub(crate) struct Writer {
    sender: mpsc::Sender<Value>,
    task: Option<JoinHandle<Result<(), Error>>>,
    limit: usize,
}
impl Writer {
    pub fn new<W: AsyncWrite + Unpin + Send + 'static>(mut output: W, config: &Config) -> Self {
        let (sender, mut receiver) = mpsc::channel(config.maximum_calls + 1);
        let limit = config.frame_bytes;
        let deadline = config.call_timeout;
        let task = tokio::spawn(async move {
            while let Some(value) = receiver.recv().await {
                tokio::time::timeout(deadline, framing::write(&mut output, &value, limit))
                    .await
                    .map_err(|_| Error::OutputTimeout)??;
            }
            Ok(())
        });
        Self {
            sender,
            task: Some(task),
            limit,
        }
    }
    pub fn send(&self, value: Value) -> Result<(), Error> {
        if !framing::fits(&value, self.limit) {
            return Err(Error::FrameLimit);
        }
        self.sender.try_send(value).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => Error::OutputCapacity,
            mpsc::error::TrySendError::Closed(_) => Error::OutputTask,
        })
    }
    pub async fn ended(&mut self) -> Error {
        let outcome = self
            .task
            .as_mut()
            .expect("writer is owned until stop")
            .await;
        self.task.take();
        match outcome {
            Ok(Err(error)) => error,
            _ => Error::OutputTask,
        }
    }
    pub async fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            let _ = task.await;
        }
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
