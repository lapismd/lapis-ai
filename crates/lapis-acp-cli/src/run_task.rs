//! Executing one prepared task.
//!
//! Composition only. The behaviour lives in `lapis-acp-control` and
//! `lapis-acp-runner`; this module assembles trusted launch configuration, reads
//! the document within its bound, and reduces the result to one record and one
//! exit code.

use std::path::PathBuf;

use lapis_acp_agent::Catalog;
use lapis_acp_agent::availability::PathProbe;
use lapis_acp_agent::config as catalog_config;
use lapis_acp_control::invocation::{Invocation, InvocationConfig, invoke_with_probe};
use lapis_acp_control::record::{ExitCode, TerminalRecord, code as record_code};
use lapis_acp_control::task::{TaskRejection, code};
use lapis_acp_control::{TaskSource, input};
use lapis_acp_protocol::session::SystemPromptTransport;
use lapis_acp_runner::configuration::SessionConfiguration;
use tokio::sync::watch;

use crate::args::LaunchOptions;

/// Bound on reading launch configuration, separate from the execution deadline.
pub const CONFIGURATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// A finished invocation: one record, one exit code, and stderr diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    /// The single terminal record for stdout.
    pub record: TerminalRecord,
    /// The exit code, unless a cancellation signal overrides it.
    pub exit: ExitCode,
    /// Set when a cancellation signal ended the run, so the caller can substitute
    /// the signal-specific code it actually observed.
    pub cancelled: bool,
    /// Operator diagnostics for stderr.
    pub diagnostics: Vec<String>,
}

impl RunResult {
    fn from(invocation: Invocation) -> Self {
        Self {
            record: invocation.record,
            exit: invocation.exit,
            cancelled: invocation.cancelled,
            diagnostics: invocation.diagnostics,
        }
    }

    fn refused(record: TerminalRecord, diagnostics: Vec<String>) -> Self {
        Self {
            record,
            exit: ExitCode::Invalid,
            cancelled: false,
            diagnostics,
        }
    }
}

fn build_catalog(options: &LaunchOptions) -> std::result::Result<Catalog, TaskRejection> {
    let mut catalog = Catalog::built_in().map_err(|error| {
        TaskRejection::new(code::INVALID_LAUNCH_CONFIGURATION, error.to_string())
    })?;
    if let Some(path) = &options.catalog {
        let definitions =
            catalog_config::read_catalog(std::path::Path::new(path)).map_err(|error| {
                TaskRejection::new(code::INVALID_LAUNCH_CONFIGURATION, error.to_string())
            })?;
        catalog
            .extend(lapis_acp_agent::Tier::ConfigFile, definitions)
            .map_err(|error| {
                TaskRejection::new(code::INVALID_LAUNCH_CONFIGURATION, error.to_string())
            })?;
    }
    Ok(catalog)
}

fn read_system_prompt(
    options: &LaunchOptions,
) -> std::result::Result<Option<String>, TaskRejection> {
    let Some(path) = &options.system_prompt_file else {
        return Ok(None);
    };
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) => Err(TaskRejection::new(
            record_code::CONFIGURATION_TIMEOUT,
            format!("cannot read the system prompt file: {error}"),
        )),
    }
}

/// Read the task document from its source, within the read bound.
pub async fn read_document(
    source: &TaskSource,
) -> std::result::Result<lapis_acp_control::TaskDocument, TaskRejection> {
    match source {
        TaskSource::Stdin => input::read_stdin().await,
        TaskSource::Path(path) => input::read_path(path).await,
    }
}

/// Execute one prepared task to a terminal record.
pub async fn execute(
    options: &LaunchOptions,
    source: &TaskSource,
    cancel: &mut watch::Receiver<bool>,
) -> RunResult {
    // The document is read first: an unusable document must not spawn an agent.
    let document = match read_document(source).await {
        Ok(document) => document,
        Err(rejection) => {
            return RunResult::refused(
                TerminalRecord::invalid(rejection.code),
                vec![rejection.reason],
            );
        }
    };

    let config = match invocation_config(options) {
        Ok(config) => config,
        Err(rejection) => {
            return RunResult::refused(
                TerminalRecord::invalid_with_id(document.task_id.clone(), rejection.code),
                vec![rejection.reason],
            );
        }
    };
    RunResult::from(invoke_with_probe(&config, document, cancel, &PathProbe).await)
}

/// Assemble the trusted launch configuration every entry point shares.
///
/// The one-shot runner and the persistent control loop build the same
/// configuration, so a launch option cannot mean one thing in `run` and something
/// else in `serve`. The working directory is the launch directory unless trusted
/// configuration overrides it, and no task document is ever consulted for it.
pub fn invocation_config(
    options: &LaunchOptions,
) -> std::result::Result<InvocationConfig, TaskRejection> {
    let catalog = build_catalog(options)?;
    let system_prompt = read_system_prompt(options)?;
    let cwd = match &options.cwd {
        Some(configured) => configured.clone(),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    Ok(InvocationConfig {
        catalog,
        mcp_servers: crate::tools::read(options.tools.as_deref())?,
        configured_runner: options.runner.clone(),
        cwd,
        idle: options.idle_timeout,
        absolute_cap: options.max_turn_duration,
        cancel_grace: options.cancel_grace,
        reap: options.reap_timeout,
        configuration: SessionConfiguration {
            model: options.model.clone(),
            effort: options.effort.clone(),
            mode: options.mode.clone(),
        },
        system_prompt,
        prompt_transport: options.prompt_transport,
        inherited_env: std::env::vars().collect(),
    })
}

/// Report the prompt transport spelling, for diagnostics and tests.
pub fn transport_name(transport: SystemPromptTransport) -> &'static str {
    match transport {
        SystemPromptTransport::Field => "field",
        SystemPromptTransport::Meta => "meta",
        SystemPromptTransport::MetaAppend => "meta-append",
    }
}
