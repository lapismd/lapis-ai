//! Process negotiation independent of caller-scoped sessions.
use crate::interactive_handler::Handler;
use crate::interactive_session::classify;
use crate::invocation::InvocationConfig;
use lapis_acp_agent::{
    availability::{self, PathProbe},
    environment,
};
use lapis_acp_protocol::{
    Connection, Error,
    budget::TurnBudget,
    initialize::{ClientCapabilities, ClientInfo, InitializeResult},
    session::{Session, SessionNewRequest, SystemPrompt},
};
use lapis_acp_runner::{Agent, ProcessGuard};
use std::path::Path;
use tokio::sync::watch;

pub struct WarmWorker {
    pub agent: Agent,
    pub initialized: InitializeResult,
}
impl WarmWorker {
    pub async fn start(
        config: &InvocationConfig,
        handler: &mut Handler,
        cancel: &mut watch::Receiver<bool>,
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
        let info = ClientInfo::lapis_acp();
        let result = tokio::select! {
            result = agent.connection.initialize_within(handler, &info, ClientCapabilities::default(), TurnBudget::new(config.idle, config.absolute_cap)) => result,
            () = lapis_acp_runner::persistent::cancellation(cancel) => Err(Error::Protocol("cancelled".into())),
        };
        match result {
            Ok(initialized) => Ok(Self { agent, initialized }),
            Err(error) => {
                Self::stop_agent(agent, config).await;
                Err(classify(&error))
            }
        }
    }
    pub async fn open(
        &mut self,
        config: &InvocationConfig,
        handler: &mut Handler,
        cwd: &str,
        load: Option<&str>,
        cancel: &mut watch::Receiver<bool>,
    ) -> Result<Session, &'static str> {
        let root = config
            .cwd
            .canonicalize()
            .map_err(|_| "invalid_session_directory")?;
        let path = Path::new(cwd)
            .canonicalize()
            .map_err(|_| "invalid_session_directory")?;
        if !path.is_dir() || !path.starts_with(&root) {
            return Err("invalid_session_directory");
        }
        let cwd = path.to_string_lossy();
        let budget = TurnBudget::new(config.idle, config.absolute_cap);
        let request = SessionNewRequest {
            cwd: &cwd,
            mcp_servers: config.mcp_servers.clone(),
            session_title: None,
            system_prompt: config.system_prompt.as_deref().map(|text| SystemPrompt {
                transport: config.prompt_transport,
                text,
            }),
        };
        let result = async {
            if let Some(id) = load {
                if !self.initialized.advertises(&["loadSession"]) {
                    return Err(Error::Protocol("resume_unsupported".into()));
                }
                handler.adapter_session = Some(id.into());
                let raw = self
                    .agent
                    .connection
                    .call_within(
                        handler,
                        "session/load",
                        serde_json::json!({"sessionId":id,"cwd":cwd,"mcpServers":config.mcp_servers}),
                        budget,
                    )
                    .await?;
                Ok(Session { id: id.into(), raw })
            } else {
                self.agent
                    .connection
                    .session_new_within(handler, &request, budget)
                    .await
            }
        };
        tokio::select! {
            result = result => result.map_err(|e| classify(&e)),
            () = lapis_acp_runner::persistent::cancellation(cancel) => Err("cancelled"),
        }
    }
    pub async fn stop(self, config: &InvocationConfig) -> bool {
        Self::stop_agent(self.agent, config).await
    }
    async fn stop_agent(mut agent: Agent, config: &InvocationConfig) -> bool {
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
}
