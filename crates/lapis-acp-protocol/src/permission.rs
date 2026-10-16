//! `session/request_permission` handling.
//!
//! The protocol crate normalizes a request and builds a response. It never
//! decides: the choice belongs to the caller, which in this repository means
//! the controlling application. Upstream auto-approved with `allow_once`
//! because it ran in an unattended auto mode; that default is deliberately not
//! ported, and an unanswered request resolves to cancellation instead.

use serde_json::{Value, json};

use crate::error::Error;

/// One selectable option in a permission request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionOption {
    /// The identifier to echo back when selecting this option.
    pub option_id: String,
    /// Human-readable label when present.
    pub name: Option<String>,
    /// Semantic kind when present, such as `allow_once` or `reject_once`.
    pub kind: Option<String>,
}

/// An agent's request for approval.
#[derive(Debug, Clone, PartialEq)]
pub struct PermissionRequest {
    /// The JSON-RPC id to answer with; numeric or string.
    pub id: Value,
    /// The available options, in the order the agent sent them.
    pub options: Vec<PermissionOption>,
    /// The complete `params` payload, retained because agents attach tool
    /// detail this crate does not model.
    pub raw: Value,
}

impl PermissionRequest {
    /// Build a request from a classified inbound frame's id and params.
    ///
    /// A missing `options` array is a protocol error rather than an empty
    /// list, because an empty list leaves nothing to select and answering
    /// would be meaningless.
    pub fn from_parts(id: Value, params: Value) -> Result<Self, Error> {
        let options = params
            .get("options")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Protocol("permission request missing options".to_owned()))?
            .iter()
            .map(|option| PermissionOption {
                option_id: option
                    .get("optionId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                name: option
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                kind: option
                    .get("kind")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            })
            .collect::<Vec<_>>();
        Ok(Self {
            id,
            options,
            raw: params,
        })
    }

    /// Find the first option carrying a semantic kind.
    ///
    /// Option identifiers are adapter-defined, so selection is always by kind
    /// and never by a hardcoded identifier.
    pub fn option_with_kind(&self, kind: &str) -> Option<&PermissionOption> {
        self.options
            .iter()
            .find(|option| option.kind.as_deref() == Some(kind))
    }

    /// The first option with a usable identifier, if any.
    pub fn first_selectable(&self) -> Option<&PermissionOption> {
        self.options
            .iter()
            .find(|option| !option.option_id.is_empty())
    }
}

/// How a caller answered a permission request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Select the option carrying this identifier.
    Select(String),
    /// Cancel the request, which leaves the turn unresolved.
    Cancel,
}

/// Build the JSON-RPC response frame for a decision.
pub fn response(id: &Value, decision: &PermissionDecision) -> Value {
    let outcome = match decision {
        PermissionDecision::Select(option_id) => {
            json!({ "outcome": "selected", "optionId": option_id })
        }
        PermissionDecision::Cancel => json!({ "outcome": "cancelled" }),
    };
    json!({ "jsonrpc": "2.0", "id": id, "result": { "outcome": outcome } })
}
