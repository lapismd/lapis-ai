//! Runner declarations.
//!
//! A runner is data. Nothing in this module can execute a shell command, fetch
//! anything or install anything, which is what makes an untrusted catalog file
//! safe to load.

use lapis_acp_protocol::EnvVar;

use crate::error::{Error, Result};

/// Maximum length of a runner identifier.
pub const ID_MAX_LENGTH: usize = 64;

/// Maximum length of a human-readable label.
pub const LABEL_MAX_LENGTH: usize = 100;

/// One complete argv that could launch this runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// Executable name or absolute path.
    pub command: String,
    /// Arguments, which may be empty.
    pub args: Vec<String>,
}

impl CommandSpec {
    /// Build an argv with no arguments.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
        }
    }

    /// Build an argv from a command and its arguments.
    pub fn with_args(command: impl Into<String>, args: &[impl AsRef<str>]) -> Self {
        Self {
            command: command.into(),
            args: args.iter().map(|arg| arg.as_ref().to_owned()).collect(),
        }
    }
}

/// How a runner expects MCP servers to be handed to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransport {
    /// Servers are spawned by the agent over stdio.
    Stdio,
    /// Servers are reached over a local HTTP endpoint.
    Http,
}

impl McpTransport {
    /// The wire spelling of this transport.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Http => "http",
        }
    }

    /// Parse a wire spelling.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "stdio" => Some(Self::Stdio),
            "http" => Some(Self::Http),
            _ => None,
        }
    }
}

/// A complete runner declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct RunnerDefinition {
    /// Stable identifier used to select this runner.
    pub id: String,
    /// Human-readable name shown to an operator.
    pub label: String,
    /// Whether the runner may be resolved at all.
    pub enabled: bool,
    /// Candidate argvs in preference order; the first resolvable one wins.
    pub commands: Vec<CommandSpec>,
    /// Environment floor applied beneath caller overrides.
    pub env: Vec<EnvVar>,
    /// How this runner consumes MCP servers.
    pub mcp_transport: McpTransport,
    /// Short operator guidance shown when no candidate resolves.
    pub install_hint: Option<String>,
    /// Documentation link shown when no candidate resolves.
    pub install_instructions_url: Option<String>,
}

/// Report whether an identifier is well formed.
///
/// The pattern is a leading lowercase alphanumeric followed by up to 63 more
/// characters from the lowercase alphanumeric, dot, underscore and hyphen set.
/// It is checked by hand rather than with a regular expression so this crate
/// needs no regex dependency.
pub fn valid_id(id: &str) -> bool {
    let mut characters = id.chars();
    let bytes: Vec<char> = id.chars().collect();
    if bytes.is_empty() || bytes.len() > ID_MAX_LENGTH {
        return false;
    }
    let first = characters.next().expect("length was checked");
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    bytes[1..].iter().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '_' | '-')
    })
}

impl RunnerDefinition {
    /// Declare an enabled runner with one candidate argv and no environment.
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        command: CommandSpec,
    ) -> Result<Self> {
        Self::try_from_parts(
            id.into(),
            label.into(),
            vec![command],
            Vec::new(),
            McpTransport::Stdio,
        )
    }

    /// Declare a runner from already-collected parts, validating all of them.
    pub fn try_from_parts(
        id: String,
        label: String,
        commands: Vec<CommandSpec>,
        env: Vec<EnvVar>,
        mcp_transport: McpTransport,
    ) -> Result<Self> {
        if !valid_id(&id) {
            return Err(Error::InvalidId(id));
        }
        let trimmed = label.trim();
        if trimmed.is_empty() || trimmed.chars().count() > LABEL_MAX_LENGTH {
            return Err(Error::InvalidLabel(id));
        }
        if commands.is_empty()
            || commands.iter().any(|spec| {
                spec.command.trim().is_empty() || spec.args.iter().any(|arg| arg.trim().is_empty())
            })
        {
            return Err(Error::EmptyCommand(id));
        }
        Ok(Self {
            id,
            label: trimmed.to_owned(),
            enabled: true,
            commands,
            env,
            mcp_transport,
            install_hint: None,
            install_instructions_url: None,
        })
    }

    /// Set the operator guidance shown when no candidate resolves.
    pub fn with_install(mut self, hint: impl Into<String>, url: impl Into<String>) -> Self {
        self.install_hint = Some(hint.into());
        self.install_instructions_url = Some(url.into());
        self
    }

    /// Mark this runner unavailable for resolution without removing it.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Add an environment floor entry.
    pub fn with_env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push(EnvVar::new(name, value));
        self
    }
}
