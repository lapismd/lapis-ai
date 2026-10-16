//! JSON-RPC envelopes and inbound frame classification.

use serde_json::{Value, json};

/// The JSON-RPC version this crate speaks.
pub const JSONRPC_VERSION: &str = "2.0";

/// ACP method names.
pub mod method {
    /// Capability and protocol version negotiation.
    pub const INITIALIZE: &str = "initialize";
    /// Open a session.
    pub const SESSION_NEW: &str = "session/new";
    /// Submit one prepared turn.
    pub const SESSION_PROMPT: &str = "session/prompt";
    /// Request cooperative cancellation of an in-flight turn.
    pub const SESSION_CANCEL: &str = "session/cancel";
    /// Streamed turn progress from agent to client.
    pub const SESSION_UPDATE: &str = "session/update";
    /// Agent asks the client to approve or refuse an action.
    pub const SESSION_REQUEST_PERMISSION: &str = "session/request_permission";
    /// Select a model when the agent advertises the capability.
    pub const SESSION_SET_MODEL: &str = "session/set_model";
    /// Select a configuration option such as a reasoning effort.
    pub const SESSION_SET_CONFIG_OPTION: &str = "session/set_config_option";
}

/// Build a request frame carrying the next allocated id.
pub fn request(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "method": method,
        "params": params,
    })
}

/// Build a notification frame, which deliberately omits `id`.
///
/// The absence of `id` is the JSON-RPC distinguisher between a request and a
/// notification, so a notification must never be given one.
pub fn notification(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "method": method,
        "params": params,
    })
}

/// Build a success response to an agent-initiated request.
pub fn result_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": JSONRPC_VERSION, "id": id, "result": result })
}

/// Build an error response to an agent-initiated request.
pub fn error_response(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "error": { "code": code, "message": message.into() },
    })
}

/// An inbound frame, classified by JSON-RPC shape.
#[derive(Debug, Clone, PartialEq)]
pub enum Inbound {
    /// A response to one of our requests that carried a result.
    Response {
        /// The echoed request id, numeric or string.
        id: Value,
        /// The `result` payload.
        result: Value,
    },
    /// A response to one of our requests that carried an error object.
    Failure {
        /// The echoed request id.
        id: Value,
        /// The raw JSON-RPC error object.
        error: Value,
    },
    /// An agent-initiated notification; no reply is expected.
    Notification {
        /// The method name.
        method: String,
        /// The `params` payload, or null when absent.
        params: Value,
    },
    /// An agent-initiated request; a reply is required or the agent will hang.
    Request {
        /// The agent's request id.
        id: Value,
        /// The method name.
        method: String,
        /// The `params` payload, or null when absent.
        params: Value,
    },
    /// A frame that is neither a response, a request nor a notification.
    Malformed,
}

/// Classify one inbound frame.
///
/// A frame carrying both `id` and `method` is an agent-initiated request, not a
/// response, even when the id happens to equal an outstanding request id. That
/// distinction is what stops a correlated reply from being mistaken for an
/// agent question.
pub fn classify(frame: &Value) -> Inbound {
    let id = frame.get("id").cloned();
    match frame.get("method").and_then(Value::as_str) {
        Some(method) => match id {
            Some(id) => Inbound::Request {
                id,
                method: method.to_owned(),
                params: frame.get("params").cloned().unwrap_or(Value::Null),
            },
            None => Inbound::Notification {
                method: method.to_owned(),
                params: frame.get("params").cloned().unwrap_or(Value::Null),
            },
        },
        None => match id {
            Some(id) => match frame.get("error") {
                Some(error) if !error.is_null() => Inbound::Failure {
                    id,
                    error: error.clone(),
                },
                _ => Inbound::Response {
                    id,
                    result: frame.get("result").cloned().unwrap_or(Value::Null),
                },
            },
            None => Inbound::Malformed,
        },
    }
}

/// Report whether a response id equals an expected numeric request id.
///
/// Comparison is by JSON value, so a numeric id matches and a string id does
/// not. An agent that answers with a different id type is not answering us.
pub fn matches_id(id: &Value, expected: u64) -> bool {
    *id == json!(expected)
}
