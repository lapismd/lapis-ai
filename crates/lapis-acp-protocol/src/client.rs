//! The request/response connection that drives one ACP agent.

use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite, BufReader};
use tokio::time::Instant;

use crate::budget::{Limits, TurnBudget};
use crate::envelope::{self, Inbound, method as m};
use crate::error::{Error, METHOD_NOT_FOUND, agent_error};
use crate::handler::SessionHandler;
use crate::initialize::{self, ClientCapabilities, ClientInfo, InitializeResult};
use crate::permission::{self, PermissionDecision, PermissionRequest};
use crate::session::{self, Session, SessionNewRequest};
use crate::stop_reason::StopReason;
use crate::update;

/// One ACP conversation over a byte stream pair.
///
/// The connection is generic over the streams rather than owning a child
/// process, so it can be exercised against an in-memory pipe with no subprocess
/// and no model provider. Process ownership, respawn and cleanup belong to
/// `lapis-acp-runner`.
#[derive(Debug)]
pub struct Connection<R, W> {
    reader: crate::frame::FrameReader<BufReader<R>>,
    writer: crate::frame::FrameWriter<W>,
    next_id: u64,
    limits: Limits,
    negotiated: Option<u64>,
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Connection<R, W> {
    /// Wrap a stream pair with default limits.
    pub fn new(reader: R, writer: W) -> Self {
        Self::with_limits(reader, writer, Limits::default())
    }

    /// Wrap a stream pair with explicit limits.
    pub fn with_limits(reader: R, writer: W, limits: Limits) -> Self {
        Self {
            reader: crate::frame::FrameReader::with_limit(
                BufReader::new(reader),
                limits.max_frame_bytes,
            ),
            writer: crate::frame::FrameWriter::new(writer),
            next_id: 1,
            limits,
            negotiated: None,
        }
    }

    /// The protocol version the agent selected, before `initialize` it is `None`.
    pub fn negotiated_version(&self) -> Option<u64> {
        self.negotiated
    }

    /// How many frames were skipped as blank or unparseable.
    pub fn skipped_frames(&self) -> u64 {
        self.reader.skipped()
    }

    /// The limits this connection enforces.
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Negotiate the protocol version under the default request limit.
    pub async fn initialize<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        info: &ClientInfo,
        capabilities: ClientCapabilities,
    ) -> Result<InitializeResult, Error> {
        let budget = TurnBudget::fixed(self.limits.request_timeout);
        self.initialize_within(handler, info, capabilities, budget)
            .await
    }

    /// Negotiate the protocol version under an explicit budget.
    ///
    /// A caller executing one prepared turn passes the turn's own budget, so
    /// adapter startup shares the idle and absolute bounds with the turn instead
    /// of getting a separate unbounded phase. A hung adapter is then reported as
    /// the bound that actually elapsed.
    pub async fn initialize_within<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        info: &ClientInfo,
        capabilities: ClientCapabilities,
        budget: TurnBudget,
    ) -> Result<InitializeResult, Error> {
        let params = initialize::params(info, capabilities);
        let result = self
            .call_within(handler, m::INITIALIZE, params, budget)
            .await?;
        let negotiated = initialize::parse_result(&result)?;
        self.negotiated = Some(negotiated.protocol_version);
        Ok(negotiated)
    }

    /// Open a session under the default request limit.
    pub async fn session_new<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        request: &SessionNewRequest<'_>,
    ) -> Result<Session, Error> {
        let budget = TurnBudget::fixed(self.limits.request_timeout);
        self.session_new_within(handler, request, budget).await
    }

    /// Open a session under an explicit budget.
    pub async fn session_new_within<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        request: &SessionNewRequest<'_>,
        budget: TurnBudget,
    ) -> Result<Session, Error> {
        let params = session::new_params(request);
        let result = self
            .call_within(handler, m::SESSION_NEW, params, budget)
            .await?;
        session::parse_new_result(&result)
    }

    /// Submit one prepared turn and wait for its stop reason.
    pub async fn session_prompt<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        session_id: &str,
        texts: &[impl AsRef<str>],
        budget: TurnBudget,
    ) -> Result<StopReason, Error> {
        let blocks: Vec<String> = texts.iter().map(|text| text.as_ref().to_owned()).collect();
        let references: Vec<&str> = blocks.iter().map(String::as_str).collect();
        self.session_prompt_blocks(handler, session_id, &references, budget)
            .await
    }

    /// Submit one prepared turn from pre-built content blocks.
    pub async fn session_prompt_blocks<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        session_id: &str,
        texts: &[&str],
        budget: TurnBudget,
    ) -> Result<StopReason, Error> {
        let params = session::prompt_text_params(session_id, texts);
        let result = self
            .call_within(handler, m::SESSION_PROMPT, params, budget)
            .await?;
        session::parse_stop_reason(&result)
    }

    /// Request cooperative cancellation of an in-flight turn.
    ///
    /// This is a notification: no response arrives, and the cancelled turn is
    /// observed through the pending prompt's stop reason.
    pub async fn session_cancel(&mut self, session_id: &str) -> Result<(), Error> {
        let params = session::cancel_params(session_id);
        self.notify(m::SESSION_CANCEL, params).await
    }

    /// Select a model through the unstable path.
    pub async fn session_set_model<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        session_id: &str,
        model_id: &str,
    ) -> Result<Value, Error> {
        let params = crate::config_option::set_model_params(session_id, model_id);
        self.call(handler, m::SESSION_SET_MODEL, params).await
    }

    /// Select a configuration option such as a reasoning effort.
    pub async fn session_set_config_option<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        session_id: &str,
        config_id: &str,
        value: &str,
    ) -> Result<Value, Error> {
        let params = crate::config_option::set_config_option_params(session_id, config_id, value);
        self.call(handler, m::SESSION_SET_CONFIG_OPTION, params)
            .await
    }

    /// Close the write half, signalling that no further request will arrive.
    pub async fn close(&mut self) -> Result<(), Error> {
        self.writer.close().await
    }

    /// Answer a permission request outside the read loop.
    ///
    /// A caller that defers a decision past the turn it interrupted uses this,
    /// and takes responsibility for answering exactly once.
    pub async fn answer_permission(
        &mut self,
        id: &Value,
        decision: &PermissionDecision,
    ) -> Result<(), Error> {
        self.write_frame(&permission::response(id, decision)).await
    }

    /// Send a request and wait for its response under the default request limit.
    pub async fn call<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        method: &str,
        params: Value,
    ) -> Result<Value, Error> {
        let budget = TurnBudget::fixed(self.limits.request_timeout);
        self.call_within(handler, method, params, budget).await
    }

    /// The id the next request will carry.
    ///
    /// Exposed so a caller can send a request and await its response in two
    /// steps, which is what racing a turn against a cancellation signal needs:
    /// the wait must be interruptible without abandoning the request, because the
    /// agent still owes an answer for that id.
    pub fn next_request_id(&self) -> u64 {
        self.next_id
    }

    /// Write a request without waiting, returning the id it used.
    pub async fn send_request(&mut self, method: &str, params: Value) -> Result<u64, Error> {
        let id = self.next_id;
        self.next_id += 1;
        self.write_frame(&envelope::request(id, method, params))
            .await?;
        Ok(id)
    }

    /// Wait for the response to an already-sent request.
    ///
    /// Agent-initiated traffic is serviced while waiting, exactly as it is during
    /// [`call_within`](Self::call_within).
    pub async fn await_response<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        id: u64,
        budget: TurnBudget,
    ) -> Result<Value, Error> {
        self.read_until_response(handler, id, budget).await
    }

    /// Send a request and wait for its response under an explicit budget.
    pub async fn call_within<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        method: &str,
        params: Value,
        budget: TurnBudget,
    ) -> Result<Value, Error> {
        let id = self.send_request(method, params).await?;
        self.await_response(handler, id, budget).await
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<(), Error> {
        self.write_frame(&envelope::notification(method, params))
            .await
    }

    async fn write_frame(&mut self, frame: &Value) -> Result<(), Error> {
        tokio::time::timeout(self.limits.write_timeout, self.writer.write(frame))
            .await
            .map_err(|_| Error::WriteTimeout(self.limits.write_timeout))?
    }

    /// Read frames until the response matching `expected` arrives.
    ///
    /// Agent-initiated traffic is serviced while waiting, because an agent that
    /// asks for approval before finishing a turn would otherwise deadlock
    /// against a client that only reads after the turn ends.
    async fn read_until_response<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        expected: u64,
        budget: TurnBudget,
    ) -> Result<Value, Error> {
        let start = Instant::now();
        let absolute = start + budget.absolute;
        let mut idle = start + budget.idle;
        loop {
            let deadline = idle.min(absolute);
            let frame = tokio::time::timeout_at(deadline, self.reader.next())
                .await
                .map_err(|_| {
                    if Instant::now() >= absolute {
                        Error::Timeout(budget.absolute)
                    } else {
                        Error::IdleTimeout(budget.idle)
                    }
                })??;
            let Some(frame) = frame else {
                return Err(Error::AgentExited);
            };
            idle = Instant::now() + budget.idle;
            let dispatched =
                tokio::time::timeout_at(absolute, self.dispatch(handler, &frame, expected))
                    .await
                    .map_err(|_| Error::Timeout(budget.absolute))??;
            if let Some(result) = dispatched {
                return Ok(result);
            }
        }
    }

    /// Service one inbound frame, returning the matched result when it is ours.
    async fn dispatch<H: SessionHandler>(
        &mut self,
        handler: &mut H,
        frame: &Value,
        expected: u64,
    ) -> Result<Option<Value>, Error> {
        match envelope::classify(frame) {
            Inbound::Response { id, result } if envelope::matches_id(&id, expected) => {
                Ok(Some(result))
            }
            Inbound::Failure { id, error } if envelope::matches_id(&id, expected) => {
                Err(agent_error(&error))
            }
            // A late response to an abandoned request is skipped, not matched.
            Inbound::Response { .. } | Inbound::Failure { .. } => Ok(None),
            Inbound::Notification { method, params } => {
                if method == m::SESSION_UPDATE {
                    if handler.accept_session(params.get("sessionId").and_then(Value::as_str))? {
                        handler.session_update_async(update::parse(&params)).await;
                    }
                } else {
                    handler.notification(&method, &params);
                }
                Ok(None)
            }
            Inbound::Malformed => {
                handler.malformed(frame);
                Ok(None)
            }
            Inbound::Request { id, method, params } => {
                if method == m::SESSION_REQUEST_PERMISSION {
                    let active =
                        handler.accept_session(params.get("sessionId").and_then(Value::as_str))?;
                    let request = PermissionRequest::from_parts(id.clone(), params.clone())?;
                    let decision = if active {
                        handler.permission(request).await
                    } else {
                        PermissionDecision::Cancel
                    };
                    self.write_frame(&permission::response(&id, &decision))
                        .await?;
                } else {
                    handler.request(&method, &params);
                    let reply = envelope::error_response(
                        id,
                        METHOD_NOT_FOUND,
                        format!("method not found: {method}"),
                    );
                    // A write failure here means the agent is gone; surfacing it
                    // is better than looping on a broken pipe.
                    self.write_frame(&reply).await?;
                }
                Ok(None)
            }
        }
    }
}
