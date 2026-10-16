//! One warm agent session owned by an interactive worker.
use crate::interactive_handler::Handler;
use crate::invocation::InvocationConfig;
use lapis_acp_agent::availability::{self, PathProbe};
use lapis_acp_agent::environment;
use lapis_acp_protocol::budget::TurnBudget;
use lapis_acp_protocol::session::{Session, SessionNewRequest, SystemPrompt};
use lapis_acp_protocol::{Connection, Error};
use lapis_acp_runner::{
    Agent, ConfigurationReport, ProcessGuard, SessionConfiguration, SettingOutcome,
};
use serde_json::{Value, json};

/// A negotiated session, closed explicitly or by process ownership on drop.
pub struct WarmSession {
    pub agent: Agent,
    pub session: Session,
    pub initialized: Value,
    pub mode_ready: bool,
}
impl WarmSession {
    /// Spawn only the launch-configured runner, then negotiate or explicitly load.
    pub async fn open(
        config: &InvocationConfig,
        handler: &mut Handler,
        load: Option<&str>,
        cancel: &mut tokio::sync::watch::Receiver<bool>,
    ) -> Result<Self, &'static str> {
        let runner = config
            .configured_runner
            .as_deref()
            .ok_or("runner_required")?;
        let definition = config
            .catalog
            .resolve(runner)
            .map_err(|_| "invalid_runner")?;
        let command = availability::probe(definition, &PathProbe)
            .resolved
            .ok_or("agent_unavailable")?;
        let spec = lapis_acp_runner::LaunchSpec {
            command,
            environment: environment::plan(&config.inherited_env, &definition.env, &[]),
            harness_env: environment::harness_variables(None, runner),
            cwd: Some(config.cwd.clone()),
        };
        let mut child =
            lapis_acp_runner::launch::spawn_quiet(&spec).map_err(|_| "agent_spawn_failed")?;
        let stdout = child.stdout.take().ok_or("protocol_error")?;
        let stdin = child.stdin.take().ok_or("protocol_error")?;
        let mut agent = Agent::new(Connection::new(stdout, stdin), ProcessGuard::new(child));
        let cwd = config.cwd.to_string_lossy();
        let request = SessionNewRequest {
            cwd: &cwd,
            mcp_servers: config.mcp_servers.clone(),
            session_title: None,
            system_prompt: config.system_prompt.as_deref().map(|text| SystemPrompt {
                transport: config.prompt_transport,
                text,
            }),
        };
        let result = tokio::select! {
            result = lapis_acp_runner::persistent::open(&mut agent, handler, &request, load,
                TurnBudget::new(config.idle, config.absolute_cap)) => result,
            () = lapis_acp_runner::persistent::cancellation(cancel) => Err(Error::Protocol("cancelled".into())),
        };
        match result {
            Ok((initialized, session)) => Ok(Self {
                agent,
                session,
                initialized: initialized.capabilities,
                mode_ready: false,
            }),
            Err(error) => {
                close_agent(agent, config).await;
                Err(classify(&error))
            }
        }
    }
    /// Apply model preferences and require a requested execution mode to land.
    pub async fn configure(
        &mut self,
        config: &SessionConfiguration,
        handler: &mut Handler,
    ) -> Result<ConfigurationReport, &'static str> {
        let report = lapis_acp_runner::apply(
            &mut self.agent.connection,
            handler,
            &self.session.id,
            &self.session.raw,
            config,
        )
        .await
        .map_err(|error| classify(&error))?;
        self.mode_ready = config.mode.is_some() && report.mode.applied();
        Ok(report)
    }
    /// Return negotiated facts, excluding raw diagnostic text.
    pub fn facts(&self) -> Value {
        json!({"sessionId":self.session.id,"capabilities":self.initialized,"session":self.session.raw})
    }
    /// Release the connection before bounded process cleanup.
    pub async fn close(self, config: &InvocationConfig) -> bool {
        close_agent(self.agent, config).await
    }
}
async fn close_agent(mut agent: Agent, config: &InvocationConfig) -> bool {
    agent.close().await;
    let Agent {
        connection,
        mut guard,
    } = agent;
    drop(connection);
    guard
        .terminate(config.cancel_grace, config.reap)
        .await
        .confirmed_stopped()
}
/// Classify without exposing adapter text.
pub fn classify(error: &Error) -> &'static str {
    match error {
        Error::Protocol(text) if text == "resume_unsupported" => "resume_unsupported",
        Error::AgentExited => "agent_exited",
        Error::AgentError { .. } => "agent_rejected",
        Error::IdleTimeout(_) | Error::Timeout(_) => "timed_out",
        _ => "protocol_error",
    }
}
/// Safe structured setting report.
pub fn report_value(report: &ConfigurationReport) -> Value {
    let value = |outcome: &SettingOutcome| match outcome {
        SettingOutcome::Applied { value, .. } => json!({"status":"applied","value":value}),
        _ => json!({"status":outcome.label()}),
    };
    json!({"model":value(&report.model),"effort":value(&report.effort),"mode":value(&report.mode)})
}
