//! Strict v3 envelopes with session scope separate from adapter identity.
use crate::interactive_wire::{self, Command};
use serde::Deserialize;
use serde_json::{Value, json};

pub enum Request {
    Warm(String),
    Session {
        command: Command,
        scope: String,
        cwd: Option<String>,
    },
    Release {
        id: String,
        scope: String,
    },
    Worker(Command),
}
impl Request {
    pub fn id(&self) -> &str {
        match self {
            Self::Warm(id) | Self::Release { id, .. } => id,
            Self::Session { command, .. } | Self::Worker(command) => command.id(),
        }
    }
    pub fn scope(&self) -> Option<&str> {
        match self {
            Self::Session { scope, .. } | Self::Release { scope, .. } => Some(scope),
            _ => None,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Special {
    #[serde(rename = "requestId")]
    id: String,
}
fn label(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
pub fn parse(line: &str) -> Result<Request, &'static str> {
    if line.len() > interactive_wire::MAX_FRAME {
        return Err("frame_limit");
    }
    let mut value: Value = serde_json::from_str(line).map_err(|_| "invalid_frame")?;
    if value["version"].as_u64() != Some(3) {
        return Err("unsupported_control_version");
    }
    let map = value.as_object_mut().ok_or("invalid_frame")?;
    map.remove("version");
    let kind = map
        .get("type")
        .and_then(Value::as_str)
        .ok_or("invalid_request")?
        .to_owned();
    if matches!(kind.as_str(), "warm" | "close_session") {
        map.remove("type");
        let scope = if kind == "close_session" {
            Some(
                map.remove("sessionId")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .filter(|s| label(s))
                    .ok_or("invalid_request")?,
            )
        } else {
            None
        };
        let Special { id } = serde_json::from_value(value).map_err(|_| "invalid_request")?;
        if !label(&id) {
            return Err("invalid_request");
        }
        return Ok(match scope {
            Some(scope) => Request::Release { id, scope },
            None => Request::Warm(id),
        });
    }
    let worker = matches!(kind.as_str(), "status" | "close");
    let scope = if worker {
        None
    } else {
        Some(
            map.remove("sessionId")
                .and_then(|v| v.as_str().map(str::to_owned))
                .filter(|s| label(s))
                .ok_or("invalid_request")?,
        )
    };
    let cwd = if kind == "open" {
        Some(
            map.remove("cwd")
                .and_then(|v| v.as_str().map(str::to_owned))
                .filter(|s| !s.is_empty() && s.len() <= 4096)
                .ok_or("invalid_request")?,
        )
    } else {
        None
    };
    map.insert("version".into(), json!(2));
    let command = interactive_wire::parse(&value.to_string())?;
    Ok(match scope {
        Some(scope) => Request::Session {
            command,
            scope,
            cwd,
        },
        None => Request::Worker(command),
    })
}
pub fn frame(kind: &str, id: Option<&str>, body: Value) -> Value {
    let mut value = interactive_wire::frame(kind, id, body);
    value["version"] = json!(3);
    value
}
pub fn error(id: Option<&str>, code: &str) -> Value {
    frame("error", id, json!({"code":code}))
}
