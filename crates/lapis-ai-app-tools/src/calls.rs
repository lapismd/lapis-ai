use crate::{Config, Error, ToolCall, ToolError, ToolHandler, protocol};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::{sync::watch, task::JoinSet};

#[derive(Default)]
pub(crate) struct Calls {
    active: HashMap<String, watch::Sender<bool>>,
    jobs: JoinSet<(String, Value)>,
}
impl Calls {
    pub fn len(&self) -> usize {
        self.active.len()
    }
    pub fn cancel(&self, key: &str) {
        if let Some(cancel) = self.active.get(key) {
            let _ = cancel.send(true);
        }
    }
    pub fn start(
        &mut self,
        key: String,
        call: ToolCall,
        handler: Arc<dyn ToolHandler>,
        config: &Config,
    ) {
        let (cancel, mut cancelled) = watch::channel(false);
        self.active.insert(key.clone(), cancel.clone());
        let timeout = config.call_timeout;
        self.jobs.spawn(async move {
            let id = call.request_id.clone();
            let invocation = handler.call(call, cancelled.clone());
            // Keep the host future alive until cancellation has been signalled;
            // select otherwise drops losing branches before running this branch.
            tokio::pin!(invocation);
            let outcome = tokio::select! {
                biased;
                _ = cancelled.changed() => Err(ToolError::Unknown),
                _ = tokio::time::sleep(timeout) => {
                    let _ = cancel.send(true);
                    Err(ToolError::Unknown)
                },
                outcome = &mut invocation => outcome,
            };
            let value = match outcome {
                Ok(value) if value.is_object() => value,
                Ok(_) => protocol::failure(ToolError::Unknown),
                Err(error) => protocol::failure(error),
            };
            (key, protocol::result(id, value))
        });
    }
    pub async fn next(&mut self) -> Result<Value, Error> {
        let (key, value) = self
            .jobs
            .join_next()
            .await
            .ok_or(Error::CallTask)?
            .map_err(|_| Error::CallTask)?;
        self.active.remove(&key);
        Ok(value)
    }
    pub async fn stop(&mut self) {
        for cancel in self.active.values() {
            let _ = cancel.send(true);
        }
        self.jobs.shutdown().await;
        self.active.clear();
    }
}
impl Drop for Calls {
    fn drop(&mut self) {
        for cancel in self.active.values() {
            let _ = cancel.send(true);
        }
        self.jobs.abort_all();
    }
}
