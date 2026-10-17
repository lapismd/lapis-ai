use crate::{
    Component, ComputeCommands, ComputeConnection, Config, Error, NativeProcess, PreparedLaunch,
    ProcessLimits, Resolver, Result,
};
use serde_json::{Value, json};
use std::{collections::VecDeque, time::Duration};
pub struct ComputeClient {
    process: NativeProcess,
    connection: ComputeConnection,
    events: VecDeque<Value>,
}
impl std::fmt::Debug for ComputeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputeClient")
            .field("endpoint", &self.connection.endpoint)
            .finish_non_exhaustive()
    }
}
impl ComputeClient {
    pub async fn start(config: &Config, resolver: &dyn Resolver) -> Result<Self> {
        let plan = PreparedLaunch::new(config, Component::Compute, resolver)?;
        Self::start_prepared(plan, ProcessLimits::default()).await
    }
    pub async fn start_prepared(plan: PreparedLaunch, limits: ProcessLimits) -> Result<Self> {
        lapis_ai_config::launch::private_directory(&plan.directory)?;
        let token = plan
            .environment
            .0
            .get("JJ_STORE_COMPUTE_TOKEN")
            .filter(|s| s.len() >= 8)
            .cloned()
            .ok_or(Error("compute_token_required"))?;
        let mut process = NativeProcess::spawn_with_limits(plan, limits).await?;
        let ready = process.next(Duration::from_secs(15)).await?;
        let endpoint = reqwest::Url::parse(
            ready["endpoint"]
                .as_str()
                .ok_or(Error("invalid_compute_readiness"))?,
        )
        .map_err(|_| Error("invalid_compute_readiness"))?;
        if ready["type"] != "ready"
            || endpoint.scheme() != "http"
            || endpoint.host_str() != Some("127.0.0.1")
            || endpoint.path() != "/v1"
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(Error("invalid_compute_readiness"));
        }
        let mut events = VecDeque::new();
        events.push_back(ready);
        Ok(Self {
            process,
            connection: ComputeConnection {
                endpoint,
                token: zeroize::Zeroizing::new(token),
            },
            events,
        })
    }
    pub fn commands(&self) -> ComputeCommands {
        ComputeCommands(self.process.sender())
    }
    pub fn connection(&self) -> &ComputeConnection {
        &self.connection
    }
    pub async fn configure(&mut self, id: &str, config: Value) -> Result<Value> {
        self.command(
            id,
            "configure",
            json!({"config":config}),
            Duration::from_secs(15),
        )
        .await
    }
    pub async fn prepare(&mut self, id: &str) -> Result<Value> {
        self.command(id, "prepare", json!({}), Duration::from_secs(360))
            .await
    }
    pub async fn download(&mut self, id: &str) -> Result<Value> {
        self.command(id, "download", json!({}), Duration::from_secs(3600))
            .await
    }
    pub async fn start_model(&mut self, id: &str) -> Result<Value> {
        self.command(id, "start", json!({}), Duration::from_secs(240))
            .await
    }
    pub async fn import(&mut self, id: &str, path: &std::path::Path) -> Result<Value> {
        self.command(
            id,
            "import",
            json!({"path":path}),
            Duration::from_secs(3600),
        )
        .await
    }
    pub async fn remove(&mut self, id: &str, model: &str) -> Result<Value> {
        self.command(
            id,
            "remove",
            json!({"model":model}),
            Duration::from_secs(60),
        )
        .await
    }
    async fn command(
        &mut self,
        id: &str,
        action: &str,
        fields: Value,
        timeout: Duration,
    ) -> Result<Value> {
        let mut request = fields
            .as_object()
            .cloned()
            .ok_or(Error("invalid_request"))?;
        if id.is_empty() || id.len() > 256 {
            return Err(Error("invalid_request_identity"));
        }
        request.insert("action".into(), json!(action));
        request.insert("requestId".into(), json!(id));
        self.process
            .sender()
            .send_bounded(&Value::Object(request), 65_535)
            .await?;
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let frame = self
                .process
                .next(deadline.saturating_duration_since(tokio::time::Instant::now()))
                .await?;
            let terminal = matches!(
                frame["type"].as_str(),
                Some("configured" | "completed" | "error")
            );
            if frame["requestId"] == id && terminal {
                return if frame["type"] == "error" {
                    Err(Error(match frame["code"].as_str() {
                        Some("compute_busy") => "compute_busy",
                        Some("compute_operation_failed") => "compute_operation_failed",
                        _ => "compute_request_failed",
                    }))
                } else {
                    Ok(frame)
                };
            }
            if self.events.len() >= 256 {
                return Err(Error("output_coverage_lost"));
            }
            self.events.push_back(frame);
        }
    }
    pub async fn next_event(&mut self, timeout: Duration) -> Result<Value> {
        if let Some(event) = self.events.pop_front() {
            return Ok(event);
        }
        self.process.next(timeout).await
    }
    pub async fn infer(&self, responses: bool, input: Value) -> Result<Value> {
        if input["stream"] == true {
            return Err(Error("streaming_unavailable"));
        }
        let mut endpoint = self.connection.endpoint.clone();
        endpoint.set_path(if responses {
            "/v1/responses"
        } else {
            "/v1/chat/completions"
        });
        reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|_| Error("http_unavailable"))?
            .post(endpoint)
            .bearer_auth(self.connection.bearer_token())
            .json(&input)
            .send()
            .await
            .map_err(|_| Error("inference_unknown"))?
            .error_for_status()
            .map_err(|_| Error("inference_failed"))?
            .json()
            .await
            .map_err(|_| Error("invalid_inference_response"))
    }
    pub async fn shutdown(&mut self) -> Result<()> {
        let sent = self.commands().stop().await;
        let stopped = self.process.shutdown(Duration::from_secs(20)).await;
        stopped.and(sent)
    }
}
