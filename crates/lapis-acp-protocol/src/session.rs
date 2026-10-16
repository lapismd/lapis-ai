//! Session creation, prompting and cancellation payloads.

use serde_json::{Map, Value, json};

use crate::content::McpServer;
use crate::error::Error;
use crate::stop_reason::StopReason;

/// Where an adapter expects standing instructions to travel.
///
/// Adapters disagree. Some read a top-level `systemPrompt`, some read
/// `_meta.systemPrompt` as a string, and some read it as an object with an
/// `append` member so the adapter's own prompt survives. The placement is
/// therefore data selected per runner, not a hardcoded vendor branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPromptTransport {
    /// Top-level `systemPrompt` string.
    Field,
    /// `_meta.systemPrompt` string.
    Meta,
    /// `_meta.systemPrompt` as `{ "append": text }`.
    MetaAppend,
}

/// Standing instructions for a new session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemPrompt<'a> {
    /// Where the text travels.
    pub transport: SystemPromptTransport,
    /// The instructions themselves.
    pub text: &'a str,
}

/// Everything needed to build `session/new` params.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionNewRequest<'a> {
    /// Working directory the agent operates in. This comes from trusted launch
    /// configuration and never from a task document.
    pub cwd: &'a str,
    /// MCP servers to hand to the agent.
    pub mcp_servers: Vec<McpServer>,
    /// Optional standing instructions.
    pub system_prompt: Option<SystemPrompt<'a>>,
    /// Optional human-readable session title.
    pub session_title: Option<&'a str>,
}

/// Build `session/new` params.
///
/// `_meta` is merged rather than replaced, so a system prompt and a session
/// title can both travel without one clobbering the other.
pub fn new_params(request: &SessionNewRequest<'_>) -> Value {
    let mut params = Map::new();
    params.insert("cwd".to_owned(), json!(request.cwd));
    params.insert("mcpServers".to_owned(), json!(request.mcp_servers));
    let mut meta = Map::new();
    if let Some(prompt) = request.system_prompt {
        let value = match prompt.transport {
            SystemPromptTransport::Field => {
                params.insert("systemPrompt".to_owned(), json!(prompt.text));
                None
            }
            SystemPromptTransport::Meta => Some(json!(prompt.text)),
            SystemPromptTransport::MetaAppend => Some(json!({ "append": prompt.text })),
        };
        if let Some(value) = value {
            meta.insert("systemPrompt".to_owned(), value);
        }
    }
    if let Some(title) = request.session_title {
        meta.insert("sessionTitle".to_owned(), json!(title));
    }
    if !meta.is_empty() {
        params.insert("_meta".to_owned(), Value::Object(meta));
    }
    Value::Object(params)
}

/// A created session.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    /// The agent-assigned session identifier.
    pub id: String,
    /// The complete `session/new` result, retained because model and
    /// configuration discovery reads fields this crate does not model.
    pub raw: Value,
}

/// Parse a `session/new` result.
///
/// A missing or non-string `sessionId` is a protocol error: without it no
/// further method can be addressed.
pub fn parse_new_result(result: &Value) -> Result<Session, Error> {
    let id = result
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Protocol("session/new result missing sessionId".to_owned()))?
        .to_owned();
    Ok(Session {
        id,
        raw: result.clone(),
    })
}

/// Build `session/prompt` params from prepared content blocks.
pub fn prompt_params(session_id: &str, blocks: Vec<Value>) -> Value {
    json!({ "sessionId": session_id, "prompt": blocks })
}

/// Build `session/prompt` params from ordered prompt text.
pub fn prompt_text_params(session_id: &str, texts: &[impl AsRef<str>]) -> Value {
    prompt_params(session_id, crate::content::text_blocks(texts))
}

/// Keep caller-prepared context separate from the authored prompt. Embedded
/// resources require negotiated support; the fallback is an ordered text block.
pub fn prompt_context_params(
    session_id: &str,
    text: &str,
    context: Option<&str>,
    embedded: bool,
) -> Value {
    let mut blocks = Vec::new();
    if let Some(context) = context.filter(|value| !value.is_empty()) {
        blocks.push(if embedded {
            json!({"type":"resource","resource":{"uri":"context://lapis-acp/session-context","mimeType":"text/plain","text":context}})
        } else {
            crate::content::text_block(context)
        });
    }
    blocks.push(crate::content::text_block(text));
    prompt_params(session_id, blocks)
}

/// Build `session/cancel` params.
///
/// Cancellation is sent as a notification, so no response is expected and a
/// cancelled turn is observed through the pending prompt's stop reason.
pub fn cancel_params(session_id: &str) -> Value {
    json!({ "sessionId": session_id })
}

/// Parse the stop reason out of a `session/prompt` result.
///
/// An absent `stopReason` and an unrecognized one are both protocol errors. A
/// turn must end in exactly one known reason, because the controlling
/// application decides what to do next from that value alone.
pub fn parse_stop_reason(result: &Value) -> Result<StopReason, Error> {
    let raw = result
        .get("stopReason")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Protocol("session/prompt result missing stopReason".to_owned()))?;
    StopReason::parse(raw).ok_or_else(|| Error::Protocol(format!("unknown stopReason {raw:?}")))
}
