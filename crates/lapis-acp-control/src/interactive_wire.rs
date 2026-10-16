//! Strict interactive control requests and versioned response construction.
use serde::Deserialize;
use serde_json::{Value, json};

/// Maximum encoded interactive frame size.
pub const MAX_FRAME: usize = 1_048_576;
/// Commands for one caller-configured session; no executable or path fields.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    /// Create or load a session without prompting.
    Open {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(default, rename = "loadSessionId")]
        load_session_id: Option<String>,
    },
    /// Apply preferences before prompting.
    Configure {
        #[serde(rename = "requestId")]
        request_id: String,
        model: Option<String>,
        effort: Option<String>,
        mode: Option<String>,
    },
    /// Submit prepared text.
    Prompt {
        #[serde(rename = "requestId")]
        request_id: String,
        text: String,
        /// Context is data in a separate block, never session configuration.
        #[serde(default)]
        context: Option<String>,
        #[serde(rename = "maxDurationMs")]
        max_duration_ms: u64,
    },
    /// Cancel a queued or running request.
    Cancel {
        #[serde(rename = "requestId")]
        request_id: String,
        target: String,
    },
    /// Answer one currently pending permission.
    Permission {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "permissionId")]
        permission_id: String,
        #[serde(rename = "optionId")]
        option_id: Option<String>,
    },
    /// Observe process-local state.
    Status {
        #[serde(rename = "requestId")]
        request_id: String,
    },
    /// End this session and process ownership.
    Close {
        #[serde(rename = "requestId")]
        request_id: String,
    },
}
impl Command {
    /// Correlation only; the controlling application owns durable deduplication.
    pub fn id(&self) -> &str {
        match self {
            Self::Open { request_id, .. }
            | Self::Configure { request_id, .. }
            | Self::Prompt { request_id, .. }
            | Self::Cancel { request_id, .. }
            | Self::Permission { request_id, .. }
            | Self::Status { request_id }
            | Self::Close { request_id } => request_id,
        }
    }
}
/// Parse strict version 2 without ever accepting executable task configuration.
pub fn parse(line: &str) -> Result<Command, &'static str> {
    if line.len() > MAX_FRAME {
        return Err("frame_limit");
    }
    let mut value: Value = serde_json::from_str(line).map_err(|_| "invalid_frame")?;
    if value.get("version").and_then(Value::as_u64) != Some(2) {
        return Err("unsupported_control_version");
    }
    value
        .as_object_mut()
        .ok_or("invalid_frame")?
        .remove("version");
    let command: Command = serde_json::from_value(value).map_err(|_| "invalid_request")?;
    if !label(command.id()) {
        return Err("invalid_request");
    }
    match &command {
        Command::Prompt {
            text,
            max_duration_ms,
            context,
            ..
        } if (text.trim().is_empty()
            && context.as_ref().is_none_or(|value| value.trim().is_empty()))
            || *max_duration_ms == 0
            || context
                .as_ref()
                .is_some_and(|value| value.len() > 512 * 1024 || value.contains('\0')) =>
        {
            return Err("invalid_request");
        }
        Command::Open {
            load_session_id: Some(id),
            ..
        } if !label(id) => return Err("invalid_request"),
        _ => {}
    }
    Ok(command)
}
fn label(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
/// Construct a correlated version-2 frame.
pub fn frame(kind: &str, id: Option<&str>, body: Value) -> Value {
    let mut value = body.as_object().cloned().unwrap_or_default();
    value.insert("version".into(), json!(2));
    value.insert("type".into(), json!(kind));
    if let Some(id) = id {
        value.insert("requestId".into(), json!(id));
    }
    Value::Object(value)
}
/// A classified error never contains adapter diagnostics or prompt content.
pub fn error(id: Option<&str>, code: &str) -> Value {
    frame("error", id, json!({"code":code}))
}
