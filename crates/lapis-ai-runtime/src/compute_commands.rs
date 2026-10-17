use crate::{Error, Result, Sender};
use serde_json::{Value, json};

/// Independent submission allows a host to consume progress and stop active work.
#[derive(Clone)]
pub struct ComputeCommands(pub(crate) Sender);
impl ComputeCommands {
    pub async fn send_frame(&self, frame: &Value) -> Result<()> {
        if !matches!(
            frame["action"].as_str(),
            Some("configure" | "prepare" | "download" | "start" | "import" | "remove" | "stop")
        ) || (frame["action"] != "stop"
            && frame["requestId"]
                .as_str()
                .is_none_or(|id| id.trim().is_empty() || id.len() > 256))
        {
            return Err(Error("invalid_request"));
        }
        self.0.send_bounded(frame, 65_535).await
    }
    pub async fn stop(&self) -> Result<()> {
        self.send_frame(&json!({"action":"stop"})).await
    }
}
