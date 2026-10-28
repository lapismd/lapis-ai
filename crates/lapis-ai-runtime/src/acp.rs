use crate::{
    AcpOptions, Component, Config, Error, NativeProcess, PreparedLaunch, Resolver, Result, Sender,
};
use serde_json::{Value, json};
use std::time::Duration;
#[derive(Clone)]
pub struct AcpCommands(Sender);
impl AcpCommands {
    /// Forward an admitted host frame without losing extension fields or exact identities.
    pub async fn send_frame(&self, frame: &Value) -> Result<()> {
        if frame["version"] != 3
            || frame["requestId"]
                .as_str()
                .is_none_or(|id| id.trim().is_empty() || id.len() > 256)
            || frame["type"].as_str().is_none()
        {
            return Err(Error("invalid_request"));
        }
        self.0.send_bounded(frame, 1_048_576).await
    }
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
    // Taken only by Drop, so all callable methods retain a process owner.
    process: Option<NativeProcess>,
    ready: Value,
    shutdown_grace: Duration,
}
impl AcpClient {
    pub async fn start(config: &Config, resolver: &dyn Resolver) -> Result<Self> {
        Self::start_prepared(PreparedLaunch::new(config, Component::Acp, resolver)?).await
    }
    /// Rust host-only launch after admission and native credential resolution.
    pub async fn start_prepared(plan: PreparedLaunch) -> Result<Self> {
        let (_owner, mut cancel) = tokio::sync::watch::channel(false);
        Self::start_prepared_cancellable(plan, &mut cancel).await
    }
    /// Host-only launch flags, with Control V3 fixed by the shared client.
    pub async fn start_prepared_with_options(
        plan: PreparedLaunch,
        options: AcpOptions,
    ) -> Result<Self> {
        let (_owner, mut cancel) = tokio::sync::watch::channel(false);
        Self::start_host(plan, options, &mut cancel).await
    }
    pub(crate) async fn start_prepared_cancellable(
        mut plan: PreparedLaunch,
        cancel: &mut tokio::sync::watch::Receiver<bool>,
    ) -> Result<Self> {
        // Preserve the existing session adapter's fixed-argv contract.
        plan.arguments.clear();
        Self::start_host(plan, AcpOptions::default(), cancel).await
    }
    async fn start_host(
        mut plan: PreparedLaunch,
        options: AcpOptions,
        cancel: &mut tokio::sync::watch::Receiver<bool>,
    ) -> Result<Self> {
        if *cancel.borrow() {
            return Err(Error("startup_cancelled"));
        }
        if plan.arguments.iter().any(|arg| {
            arg == "serve" || arg == "--control-version" || arg.starts_with("--control-version=")
        }) || options.readiness_timeout.is_zero()
            || options.shutdown_grace.is_zero()
        {
            return Err(Error("invalid_acp_launch"));
        }
        plan.arguments.splice(
            0..0,
            ["serve".into(), "--control-version".into(), "3".into()],
        );
        let mut client = Self {
            process: Some(NativeProcess::spawn_with_limits(plan, options.limits).await?),
            ready: Value::Null,
            shutdown_grace: options.shutdown_grace,
        };
        let ready = tokio::select! {
            biased;
            _ = cancel.changed() => Err(Error("startup_cancelled")),
            ready = client.next(options.readiness_timeout) => ready,
        };
        let result = ready.and_then(|ready| {
            if ready["type"] != "ready" || ready["version"] != 3 {
                Err(Error("invalid_acp_readiness"))
            } else {
                client.ready = ready;
                Ok(())
            }
        });
        if let Err(error) = result {
            client
                .shutdown()
                .await
                .map_err(|_| Error("startup_cleanup_unknown"))?;
            return Err(error);
        }
        Ok(client)
    }
    pub fn readiness(&self) -> &Value {
        &self.ready
    }
    pub fn commands(&self) -> AcpCommands {
        AcpCommands(self.process.as_ref().expect("owned until drop").sender())
    }
    pub async fn next(&mut self, timeout: Duration) -> Result<Value> {
        self.process
            .as_mut()
            .ok_or(Error("session_closed"))?
            .next(timeout)
            .await
    }
    pub async fn shutdown(&mut self) -> Result<()> {
        self.process
            .as_mut()
            .ok_or(Error("session_closed"))?
            .shutdown(self.shutdown_grace)
            .await
    }
}
impl Drop for AcpClient {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take()
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            // Closing control stdin lets lapis-acp cancel and reap its separately
            // grouped provider. Killing only the harness would orphan that group.
            let grace = self.shutdown_grace;
            runtime.spawn(async move {
                let _ = process.shutdown(grace).await;
            });
        }
    }
}
