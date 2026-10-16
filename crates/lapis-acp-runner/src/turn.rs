//! Executing one prepared turn against a launched agent.

use std::time::Duration;

use lapis_acp_protocol::budget::TurnBudget;
use lapis_acp_protocol::content::McpServer;
use lapis_acp_protocol::envelope::method;
use lapis_acp_protocol::handler::SessionHandler;
use lapis_acp_protocol::initialize::{ClientCapabilities, ClientInfo};
use lapis_acp_protocol::session::{self, SessionNewRequest, SystemPrompt};
use lapis_acp_protocol::{Connection, StopReason};
use serde_json::Value;
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::watch;

use crate::configuration::{self, SessionConfiguration};
use crate::outcome::TurnOutcome;
use crate::process::ProcessGuard;

/// A launched agent: its protocol connection and the handle that stops it.
pub struct Agent {
    /// The ACP connection over the child's stdio.
    pub connection: Connection<ChildStdout, ChildStdin>,
    /// Ownership of the child process and its cleanup bounds.
    pub guard: ProcessGuard,
}

impl Agent {
    /// Pair a connection with the process that serves it.
    pub fn new(connection: Connection<ChildStdout, ChildStdin>, guard: ProcessGuard) -> Self {
        Self { connection, guard }
    }

    /// Close the agent's stdin so it can exit on its own.
    ///
    /// Called before termination. An agent that exits in response needs no kill,
    /// and one that does not is then killed within the usual bounds. A failure to
    /// close is not fatal, because termination runs either way.
    pub async fn close(&mut self) {
        let _ = self.connection.close().await;
    }
}

/// One prepared turn.
#[derive(Debug, Clone)]
pub struct TurnRequest<'a> {
    /// Prompt text blocks, in order.
    pub prompt: &'a [String],
    /// Working directory, from trusted launch configuration only.
    pub cwd: &'a str,
    /// Standing instructions and where the adapter expects them.
    pub system_prompt: Option<SystemPrompt<'a>>,
    /// Optional session title.
    pub session_title: Option<&'a str>,
    /// MCP servers to hand to the agent.
    pub mcp_servers: Vec<McpServer>,
    /// Idle and absolute bounds for the turn.
    pub budget: TurnBudget,
    /// How long to wait for a cooperative stop after requesting cancellation.
    pub cancel_grace: Duration,
    /// Model, reasoning effort and execution mode to request.
    pub configuration: SessionConfiguration,
}

/// Wait until the cancellation signal is set, or forever if it never is.
///
/// A dropped sender means nobody can cancel any more, so waiting forever on the
/// prompt is correct rather than treating it as a cancellation.
async fn wait_for_cancel(cancel: &mut watch::Receiver<bool>) {
    loop {
        if *cancel.borrow_and_update() {
            return;
        }
        if cancel.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

fn classify(value: Result<Value, lapis_acp_protocol::Error>, session_id: &str) -> TurnOutcome {
    match value {
        Ok(result) => match session::parse_stop_reason(&result) {
            Ok(StopReason::Cancelled) => TurnOutcome::cancelled(Some(session_id.to_owned())),
            Ok(reason) => TurnOutcome::completed(reason, session_id.to_owned()),
            Err(error) => TurnOutcome::from_protocol(&error, Some(session_id.to_owned())),
        },
        Err(error) => TurnOutcome::from_protocol(&error, Some(session_id.to_owned())),
    }
}

/// Run one turn: negotiate, create a session, prompt, and stop.
///
/// The prompt is sent before the cancellation race begins, so the agent's request
/// id is known and the wait can be interrupted and then resumed rather than
/// abandoned. Abandoning it would leave the agent owing a response nobody reads.
///
/// Model selection is best-effort by design. An agent that does not offer the
/// requested model keeps its own default rather than failing the turn, because a
/// preference is not a requirement and a refused preference is still a usable
/// session.
pub async fn run_turn<H: SessionHandler>(
    agent: &mut Agent,
    request: &TurnRequest<'_>,
    handler: &mut H,
    cancel: &mut watch::Receiver<bool>,
) -> TurnOutcome {
    let info = ClientInfo::lapis_acp();
    // Startup and session setup share the turn budget, so a hung adapter is
    // bounded by the caller's idle and absolute limits rather than getting a
    // separate unbounded phase of its own.
    if let Err(error) = agent
        .connection
        .initialize_within(
            handler,
            &info,
            ClientCapabilities::default(),
            request.budget,
        )
        .await
    {
        return TurnOutcome::from_protocol(&error, None);
    }
    let new_request = SessionNewRequest {
        cwd: request.cwd,
        mcp_servers: request.mcp_servers.clone(),
        system_prompt: request.system_prompt,
        session_title: request.session_title,
    };
    let session = match agent
        .connection
        .session_new_within(handler, &new_request, request.budget)
        .await
    {
        Ok(session) => session,
        Err(error) => return TurnOutcome::from_protocol(&error, None),
    };
    let session_id = session.id.clone();

    // Configuration is applied before the turn and never fails it. An adapter
    // that refuses a preference still has a usable session; only a transport-class
    // failure, which means the stream itself may be corrupt, aborts.
    let report = match configuration::apply(
        &mut agent.connection,
        handler,
        &session_id,
        &session.raw,
        &request.configuration,
    )
    .await
    {
        Ok(report) => report,
        Err(error) => return TurnOutcome::from_protocol(&error, Some(session_id)),
    };

    let params = session::prompt_text_params(&session_id, request.prompt);
    let id = match agent
        .connection
        .send_request(method::SESSION_PROMPT, params)
        .await
    {
        Ok(id) => id,
        Err(error) => {
            return TurnOutcome::from_protocol(&error, Some(session_id));
        }
    };

    let raced = tokio::select! {
        result = agent.connection.await_response(handler, id, request.budget) => Some(result),
        () = wait_for_cancel(cancel) => None,
    };
    let Some(result) = raced else {
        // Cooperative cancellation first: the agent may stop cleanly and report a
        // real stop reason, which is better evidence than a kill.
        let _ = agent.connection.session_cancel(&session_id).await;
        let drain = TurnBudget::fixed(request.cancel_grace);
        return match agent.connection.await_response(handler, id, drain).await {
            Ok(value) => classify(Ok(value), &session_id).with_configuration(report),
            Err(_) => TurnOutcome::cancelled(Some(session_id)).with_configuration(report),
        };
    };
    classify(result, &session_id).with_configuration(report)
}
