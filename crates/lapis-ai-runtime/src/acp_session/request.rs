use crate::{AcpCommands, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

/// Data-only input. Launch, directory, credentials and tools are Rust host policy.
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionRequest {
    Configure {
        model: Option<String>,
        effort: Option<String>,
        mode: Option<String>,
    },
    Prompt {
        text: String,
        context: Option<String>,
        max_duration_ms: u64,
    },
    Permission {
        permission_id: String,
        option_id: Option<String>,
    },
    Cancel {
        target: String,
    },
    Status {},
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Configure,
    Prompt,
    Permission,
    Cancel,
    Status,
}
pub fn label(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
impl SessionRequest {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Configure { .. } => Kind::Configure,
            Self::Prompt { .. } => Kind::Prompt,
            Self::Permission { .. } => Kind::Permission,
            Self::Cancel { .. } => Kind::Cancel,
            Self::Status {} => Kind::Status,
        }
    }
    pub fn validate(&self, id: &str) -> Result<()> {
        if !label(id) || id.starts_with("native-") {
            return Err(Error("invalid_request_identity"));
        }
        if serde_json::to_vec(self).map_or(true, |value| value.len() > 1_047_000) {
            return Err(Error("request_limit"));
        }
        match self {
            Self::Configure {
                model,
                effort,
                mode,
            } => {
                if [model, effort, mode]
                    .iter()
                    .any(|value| value.as_ref().is_some_and(|value| !label(value)))
                {
                    return Err(Error("invalid_configuration"));
                }
            }
            Self::Prompt {
                text,
                context,
                max_duration_ms,
            } => {
                if *max_duration_ms == 0
                    || *max_duration_ms > 86_400_000
                    || (text.trim().is_empty()
                        && context.as_ref().is_none_or(|value| value.trim().is_empty()))
                    || text.contains('\0')
                    || context
                        .as_ref()
                        .is_some_and(|value| value.len() > 512 * 1024 || value.contains('\0'))
                {
                    return Err(Error("invalid_prompt"));
                }
            }
            Self::Permission {
                permission_id,
                option_id,
            } => {
                if !label(permission_id) || option_id.as_ref().is_some_and(|value| !label(value)) {
                    return Err(Error("invalid_permission"));
                }
            }
            Self::Cancel { target } if !label(target) => {
                return Err(Error("invalid_cancel_target"));
            }
            _ => {}
        }
        Ok(())
    }
    pub fn timeout(&self, request: Duration) -> Duration {
        match self {
            Self::Prompt {
                max_duration_ms, ..
            } => Duration::from_millis(*max_duration_ms) + request,
            _ => request,
        }
    }
    pub async fn send(&self, commands: &AcpCommands, id: &str, session: &str) -> Result<()> {
        match self {
            Self::Configure {
                model,
                effort,
                mode,
            } => {
                commands
                    .configure(
                        id,
                        session,
                        json!({"model":model,"effort":effort,"mode":mode}),
                    )
                    .await
            }
            Self::Prompt {
                text,
                context,
                max_duration_ms,
            } => {
                commands
                    .prompt(
                        id,
                        session,
                        text,
                        context.as_deref(),
                        Duration::from_millis(*max_duration_ms),
                    )
                    .await
            }
            Self::Permission {
                permission_id,
                option_id,
            } => {
                commands
                    .permission(id, session, permission_id, option_id.as_deref())
                    .await
            }
            Self::Cancel { target } => commands.cancel(id, session, target).await,
            Self::Status {} => commands.status(id).await,
        }
    }
}
