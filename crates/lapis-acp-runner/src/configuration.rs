//! Applying model, reasoning effort and execution mode to a session.
//!
//! Three settings, three discovery surfaces, one error classification. The
//! classification is the load-bearing part: a transport-class failure means the
//! stream may be corrupt and must not be reused, while an application-level
//! rejection means the adapter politely refused and the session is still usable on
//! its own defaults. Conflating them either loses an agent that needs replacing or
//! throws away a perfectly good session.
//!
//! Nothing here guesses. A setting the adapter does not offer is reported as
//! unsupported, one it offers but not at that value is reported as unavailable, and
//! the value actually sent is always the adapter's own spelling.

use lapis_acp_protocol::config_option::{
    EffortResolution, ModeResolution, ModelSwitch, resolve_effort, resolve_mode,
    resolve_model_switch,
};
use lapis_acp_protocol::handler::SessionHandler;
use lapis_acp_protocol::{Connection, Error};
use serde_json::Value;
use tokio::process::{ChildStdin, ChildStdout};

/// What a caller asked the session to be configured with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionConfiguration {
    /// Model to select.
    pub model: Option<String>,
    /// Reasoning effort to select.
    pub effort: Option<String>,
    /// Execution mode to select.
    pub mode: Option<String>,
}

impl SessionConfiguration {
    /// Report whether nothing was requested.
    pub fn is_empty(&self) -> bool {
        self.model.is_none() && self.effort.is_none() && self.mode.is_none()
    }
}

/// Why one setting did not land.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SettingOutcome {
    /// The caller did not request it.
    #[default]
    NotRequested,
    /// The adapter accepted it. Carries what was actually sent.
    Applied {
        /// The mechanism used, for diagnostics.
        via: String,
        /// The adapter's own value spelling.
        value: String,
    },
    /// The adapter returned an error for the request. The session is still
    /// usable on its own default.
    Rejected {
        /// The adapter's JSON-RPC error code, when it sent one.
        code: Option<i64>,
    },
    /// The adapter exposes no surface for this setting at all.
    Unsupported,
    /// The adapter exposes a surface but not this value.
    Unavailable,
    /// The name exists under a different category, so applying it as this setting
    /// would be a guess. Carries the category for diagnostics.
    NotThisSetting {
        /// The category that does offer the value.
        offered_under: Option<String>,
    },
}

impl SettingOutcome {
    /// Report whether the setting landed.
    pub fn applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
    }

    /// A short stable label, safe to publish.
    ///
    /// The adapter's error text is deliberately not included: it is untrusted and
    /// can quote prompt content, so it belongs on stderr and never in a record.
    pub fn label(&self) -> &'static str {
        match self {
            Self::NotRequested => "not-requested",
            Self::Applied { .. } => "applied",
            Self::Rejected { .. } => "rejected",
            Self::Unsupported => "unsupported",
            Self::Unavailable => "unavailable",
            Self::NotThisSetting { .. } => "not-this-setting",
        }
    }
}

/// What happened to each requested setting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigurationReport {
    /// Outcome for the model selection.
    pub model: SettingOutcome,
    /// Outcome for the reasoning effort.
    pub effort: SettingOutcome,
    /// Outcome for the execution mode.
    pub mode: SettingOutcome,
}

impl ConfigurationReport {
    fn empty() -> Self {
        Self {
            model: SettingOutcome::NotRequested,
            effort: SettingOutcome::NotRequested,
            mode: SettingOutcome::NotRequested,
        }
    }

    /// The settings that were requested but did not land, for diagnostics.
    pub fn shortfalls(&self) -> Vec<(&'static str, &SettingOutcome)> {
        [
            ("model", &self.model),
            ("effort", &self.effort),
            ("mode", &self.mode),
        ]
        .into_iter()
        .filter(|(_, outcome)| {
            !matches!(
                outcome,
                SettingOutcome::NotRequested | SettingOutcome::Applied { .. }
            )
        })
        .collect()
    }
}

fn is_transport_class(error: &Error) -> bool {
    !matches!(error, Error::AgentError { .. })
}

/// Apply the requested configuration to a freshly created session.
///
/// Order matters: the model is selected first, because an adapter rebuilds its
/// option catalogue for the new model and may expose a different reasoning-effort
/// identifier afterwards. The capabilities used for effort resolution are therefore
/// taken from the switch response when it carries any, and only fall back to the
/// original session result.
///
/// A transport-class error aborts and propagates so the caller can replace the
/// agent rather than keep talking over a stream that may be corrupt. An
/// application-level rejection is recorded and the remaining settings still apply.
pub async fn apply<H: SessionHandler>(
    connection: &mut Connection<ChildStdout, ChildStdin>,
    handler: &mut H,
    session_id: &str,
    session_raw: &Value,
    config: &SessionConfiguration,
) -> Result<ConfigurationReport, Error> {
    let mut report = ConfigurationReport::empty();
    if config.is_empty() {
        return Ok(report);
    }

    // Capabilities the effort and mode steps resolve against, refreshed by a
    // successful model switch so a pre-switch catalogue is never mistaken for the
    // selected model's.
    let mut capabilities = session_raw.clone();

    if let Some(model) = &config.model {
        match resolve_model_switch(session_raw, model) {
            None => report.model = SettingOutcome::Unavailable,
            Some(switch) => {
                let (via, result) = match &switch {
                    ModelSwitch::ConfigOption {
                        config_id,
                        option_value,
                    } => (
                        format!("configOption {config_id}"),
                        connection
                            .session_set_config_option(handler, session_id, config_id, option_value)
                            .await,
                    ),
                    ModelSwitch::SetModel { model_id } => (
                        "set_model".to_owned(),
                        connection
                            .session_set_model(handler, session_id, model_id)
                            .await,
                    ),
                };
                match result {
                    Ok(value) => {
                        if value.get("configOptions").is_some_and(|v| !v.is_null()) {
                            capabilities = value;
                        }
                        report.model = SettingOutcome::Applied {
                            via,
                            value: model.clone(),
                        };
                    }
                    Err(error) if is_transport_class(&error) => return Err(error),
                    Err(error) => report.model = rejection(&error),
                }
            }
        }
    }

    if let Some(effort) = &config.effort {
        let base = config
            .model
            .as_deref()
            .map(lapis_acp_protocol::model_id_base);
        report.effort = match resolve_effort(&capabilities, effort, base) {
            EffortResolution::Unsupported => SettingOutcome::Unsupported,
            EffortResolution::Unavailable => SettingOutcome::Unavailable,
            EffortResolution::ConfigOption { config_id, value } => {
                match connection
                    .session_set_config_option(handler, session_id, &config_id, &value)
                    .await
                {
                    Ok(_) => SettingOutcome::Applied {
                        via: format!("configOption {config_id}"),
                        value,
                    },
                    Err(error) if is_transport_class(&error) => return Err(error),
                    Err(error) => rejection(&error),
                }
            }
            EffortResolution::ModelId { model_id } => {
                match connection
                    .session_set_model(handler, session_id, &model_id)
                    .await
                {
                    Ok(_) => SettingOutcome::Applied {
                        via: "set_model".to_owned(),
                        value: model_id,
                    },
                    Err(error) if is_transport_class(&error) => return Err(error),
                    Err(error) => rejection(&error),
                }
            }
        };
    }

    if let Some(mode) = &config.mode {
        report.mode = match resolve_mode(&capabilities, mode) {
            ModeResolution::Unsupported => SettingOutcome::Unsupported,
            ModeResolution::Unknown => SettingOutcome::Unavailable,
            ModeResolution::NotASessionMode { offered_under } => {
                SettingOutcome::NotThisSetting { offered_under }
            }
            ModeResolution::Switch { config_id, value } => {
                match connection
                    .session_set_config_option(handler, session_id, &config_id, &value)
                    .await
                {
                    Ok(_) => SettingOutcome::Applied {
                        via: format!("configOption {config_id}"),
                        value,
                    },
                    Err(error) if is_transport_class(&error) => return Err(error),
                    Err(error) => rejection(&error),
                }
            }
        };
    }

    Ok(report)
}

fn rejection(error: &Error) -> SettingOutcome {
    match error {
        Error::AgentError { code, .. } => SettingOutcome::Rejected { code: Some(*code) },
        _ => SettingOutcome::Rejected { code: None },
    }
}
