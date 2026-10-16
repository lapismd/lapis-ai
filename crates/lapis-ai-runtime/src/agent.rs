use crate::{Component, Config, NativeProcess, PreparedLaunch, Resolver, Result};
use serde_json::Value;
use std::time::Duration;
pub struct AgentClient;
impl AgentClient {
    pub async fn models(config: &Config, resolver: &dyn Resolver) -> Result<Value> {
        Self::request(config, resolver, "--models", None, Duration::from_secs(25)).await
    }
    pub async fn metadata(config: &Config, resolver: &dyn Resolver, input: Value) -> Result<Value> {
        Self::request(
            config,
            resolver,
            "--metadata",
            Some(input),
            Duration::from_secs(125),
        )
        .await
    }
    async fn request(
        config: &Config,
        resolver: &dyn Resolver,
        argument: &str,
        input: Option<Value>,
        timeout: Duration,
    ) -> Result<Value> {
        let mut plan = PreparedLaunch::new(config, Component::Agent, resolver)?;
        plan.arguments.push(argument.into());
        let mut child = NativeProcess::spawn_single(plan).await?;
        if let Some(input) = input {
            child.sender().send(&input).await?;
        }
        child.sender().close().await;
        let output = child.next(timeout).await?;
        child.shutdown(Duration::from_secs(5)).await?;
        Ok(output)
    }
}
