use crate::{Component, Config, Error, NativeProcess, PreparedLaunch, Resolver, Result, Sender};
use serde_json::{Value, json};
use std::time::Duration;
#[derive(Clone)]
pub struct AcpCommands(Sender);
impl AcpCommands {
    pub async fn warm(&self, id: &str) -> Result<()> {
        self.send("warm", id, None, json!({})).await
    }
    pub async fn open(
        &self,
        id: &str,
        session: &str,
        cwd: &std::path::Path,
        resume: Option<&str>,
    ) -> Result<()> {
        let mut fields = json!({"cwd":cwd});
        if let Some(resume) = resume {
            fields["loadSessionId"] = json!(resume);
        }
        self.send("open", id, Some(session), fields).await
    }
    pub async fn configure(&self, id: &str, session: &str, fields: Value) -> Result<()> {
        self.send("configure", id, Some(session), fields).await
    }
    pub async fn prompt(
        &self,
        id: &str,
        session: &str,
        text: &str,
        context: Option<&str>,
        maximum: Duration,
    ) -> Result<()> {
        let millis =
            u64::try_from(maximum.as_millis()).map_err(|_| Error("invalid_prompt_duration"))?;
        if millis == 0 {
            return Err(Error("invalid_prompt_duration"));
        }
        let mut fields = json!({"text":text,"maxDurationMs":millis});
        if let Some(context) = context {
            fields["context"] = json!(context);
        }
        self.send("prompt", id, Some(session), fields).await
    }
    pub async fn permission(
        &self,
        id: &str,
        session: &str,
        permission: &str,
        option: Option<&str>,
    ) -> Result<()> {
        self.send(
            "permission",
            id,
            Some(session),
            json!({"permissionId":permission,"optionId":option}),
        )
        .await
    }
    pub async fn cancel(&self, id: &str, session: &str, target: &str) -> Result<()> {
        self.send("cancel", id, Some(session), json!({"target":target}))
            .await
    }
    pub async fn close_session(&self, id: &str, session: &str) -> Result<()> {
        self.send("close_session", id, Some(session), json!({}))
            .await
    }
    pub async fn status(&self, id: &str) -> Result<()> {
        self.send("status", id, None, json!({})).await
    }
    pub async fn close(&self, id: &str) -> Result<()> {
        self.send("close", id, None, json!({})).await
    }
    async fn send(&self, kind: &str, id: &str, session: Option<&str>, fields: Value) -> Result<()> {
        if id.trim().is_empty() || id.len() > 256 {
            return Err(Error("invalid_request_identity"));
        }
        let mut frame = fields
            .as_object()
            .cloned()
            .ok_or(Error("invalid_request"))?;
        if ["version", "type", "requestId", "sessionId"]
            .iter()
            .any(|key| frame.contains_key(*key))
        {
            return Err(Error("reserved_request_field"));
        }
        frame.insert("version".into(), json!(3));
        frame.insert("type".into(), json!(kind));
        frame.insert("requestId".into(), json!(id));
        if let Some(session) = session {
            frame.insert("sessionId".into(), json!(session));
        }
        self.0.send_bounded(&Value::Object(frame), 1_048_576).await
    }
}
pub struct AcpClient {
    process: NativeProcess,
}
impl AcpClient {
    pub async fn start(config: &Config, resolver: &dyn Resolver) -> Result<Self> {
        let mut plan = PreparedLaunch::new(config, Component::Acp, resolver)?;
        plan.arguments = vec!["serve".into(), "--control-version".into(), "3".into()];
        let mut process = NativeProcess::spawn(plan).await?;
        let ready = process.next(Duration::from_secs(15)).await?;
        if ready["type"] != "ready" || ready["version"] != 3 {
            return Err(Error("invalid_acp_readiness"));
        }
        Ok(Self { process })
    }
    pub fn commands(&self) -> AcpCommands {
        AcpCommands(self.process.sender())
    }
    pub async fn next(&mut self, timeout: Duration) -> Result<Value> {
        self.process.next(timeout).await
    }
    pub async fn shutdown(&mut self) -> Result<()> {
        self.process.shutdown(Duration::from_secs(5)).await
    }
}
