//! Classification of `session/update` notifications.
//!
//! Updates are streamed turn progress. This crate recognizes the common shapes
//! so a caller can act on them, and passes the raw payload through in every
//! case so an unrecognized extension is never dropped.

use serde_json::Value;

/// The update discriminator used when an agent omits or mis-types it.
pub const UNKNOWN_KIND: &str = "unknown";

/// One streamed session update.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionUpdate {
    /// Visible assistant text.
    AgentMessageChunk {
        /// The chunk text when present.
        text: Option<String>,
        /// The complete update payload.
        raw: Value,
    },
    /// Reasoning text, which is not part of the visible answer.
    AgentThoughtChunk {
        /// The chunk text when present.
        text: Option<String>,
        /// The complete update payload.
        raw: Value,
    },
    /// A tool invocation began.
    ToolCall {
        /// Agent-assigned tool call identifier when present.
        tool_call_id: Option<String>,
        /// Human-readable title when present.
        title: Option<String>,
        /// Tool kind when present.
        kind: Option<String>,
        /// The complete update payload.
        raw: Value,
    },
    /// A tool invocation changed state.
    ToolCallUpdate {
        /// Agent-assigned tool call identifier when present.
        tool_call_id: Option<String>,
        /// New status when present.
        status: Option<String>,
        /// The complete update payload.
        raw: Value,
    },
    /// A plan or todo list was published or revised.
    Plan {
        /// The complete update payload.
        raw: Value,
    },
    /// The advertised slash commands changed.
    AvailableCommandsUpdate {
        /// The complete update payload.
        raw: Value,
    },
    /// An update this crate does not model.
    Unknown {
        /// The discriminator the agent sent.
        kind: String,
        /// The complete update payload.
        raw: Value,
    },
}

impl SessionUpdate {
    /// The raw payload, whatever the variant.
    pub fn raw(&self) -> &Value {
        match self {
            Self::AgentMessageChunk { raw, .. }
            | Self::AgentThoughtChunk { raw, .. }
            | Self::ToolCall { raw, .. }
            | Self::ToolCallUpdate { raw, .. }
            | Self::Plan { raw }
            | Self::AvailableCommandsUpdate { raw }
            | Self::Unknown { raw, .. } => raw,
        }
    }

    /// Report whether this update indicates observable agent activity.
    ///
    /// A tool call is the strongest signal that a turn is progressing rather
    /// than merely streaming text, which is what a caller uses to decide
    /// whether an idle window should reset.
    pub fn is_activity(&self) -> bool {
        matches!(self, Self::ToolCall { .. } | Self::ToolCallUpdate { .. })
    }
}

fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Read the `sessionUpdate` discriminator out of notification params.
pub fn kind(params: &Value) -> &str {
    params
        .pointer("/update/sessionUpdate")
        .and_then(Value::as_str)
        .unwrap_or(UNKNOWN_KIND)
}

/// Classify `session/update` notification params.
///
/// Classification never fails. An unrecognized discriminator becomes
/// [`SessionUpdate::Unknown`] carrying the raw payload, so a new agent
/// extension degrades to pass-through instead of breaking a turn.
pub fn parse(params: &Value) -> SessionUpdate {
    let update = params.pointer("/update").cloned().unwrap_or(Value::Null);
    let text = || string(update.pointer("/content").unwrap_or(&Value::Null), "text");
    match kind(params) {
        "agent_message_chunk" => SessionUpdate::AgentMessageChunk {
            text: text(),
            raw: update,
        },
        "agent_thought_chunk" => SessionUpdate::AgentThoughtChunk {
            text: text(),
            raw: update,
        },
        "tool_call" => SessionUpdate::ToolCall {
            tool_call_id: string(&update, "toolCallId"),
            title: string(&update, "title"),
            kind: string(&update, "kind"),
            raw: update,
        },
        "tool_call_update" => SessionUpdate::ToolCallUpdate {
            tool_call_id: string(&update, "toolCallId"),
            status: string(&update, "status"),
            raw: update,
        },
        "plan" => SessionUpdate::Plan { raw: update },
        "available_commands_update" => SessionUpdate::AvailableCommandsUpdate { raw: update },
        other => SessionUpdate::Unknown {
            kind: other.to_owned(),
            raw: update,
        },
    }
}
