use crate::{Component, Config, NativeProcess, PreparedLaunch, ProcessLimits, Resolver, Result};
use serde_json::Value;
use std::time::Duration;
pub struct AgentClient;
impl AgentClient {
    pub async fn models(config: &Config, resolver: &dyn Resolver) -> Result<Value> {
        Self::models_prepared(
            PreparedLaunch::new(config, Component::Agent, resolver)?,
            ProcessLimits::default(),
            Duration::from_secs(25),
        )
        .await
    }
    pub async fn metadata(config: &Config, resolver: &dyn Resolver, input: Value) -> Result<Value> {
        Self::metadata_prepared(
            PreparedLaunch::new(config, Component::Agent, resolver)?,
            input,
            ProcessLimits::default(),
            Duration::from_secs(125),
        )
        .await
    }
    pub async fn models_prepared(
        plan: PreparedLaunch,
        limits: ProcessLimits,
        timeout: Duration,
    ) -> Result<Value> {
        Self::request(plan, limits, "--models", None, timeout).await
    }
    pub async fn metadata_prepared(
        plan: PreparedLaunch,
        input: Value,
        limits: ProcessLimits,
        timeout: Duration,
    ) -> Result<Value> {
        Self::request(plan, limits, "--metadata", Some(input), timeout).await
    }
    async fn request(
        mut plan: PreparedLaunch,
        limits: ProcessLimits,
        argument: &str,
        input: Option<Value>,
        timeout: Duration,
    ) -> Result<Value> {
        plan.arguments.clear();
        plan.arguments.push(argument.into());
        let mut child = NativeProcess::spawn_single(plan, limits).await?;
        if let Some(input) = input {
            child.sender().send(&input).await?;
        }
        child.sender().close().await;
        let output = child.next(timeout).await?;
        child.shutdown(Duration::from_secs(5)).await?;
        Ok(output)
    }
}
