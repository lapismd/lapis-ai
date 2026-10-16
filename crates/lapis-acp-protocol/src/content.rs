//! Content blocks and MCP server declarations.

use serde::Serialize;
use serde_json::{Value, json};

/// One environment variable for an MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvVar {
    /// Variable name.
    pub name: String,
    /// Variable value. A secret here travels in the frame, so callers should
    /// prefer the agent's inherited environment when a value is sensitive.
    pub value: String,
}

impl EnvVar {
    /// Build one variable.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// An MCP server handed to an agent at `session/new`.
///
/// This is the `McpServerStdio` shape from the ACP schema. All four fields are
/// required by the schema; `args` and `env` may be empty but must be present,
/// which is why they are vectors rather than options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpServer {
    /// Server name the agent uses to refer to it.
    pub name: String,
    /// Executable to spawn.
    pub command: String,
    /// Arguments; may be empty.
    pub args: Vec<String>,
    /// Environment; may be empty.
    pub env: Vec<EnvVar>,
}

impl McpServer {
    /// Build a server with no arguments and no environment.
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    /// Add an argument.
    pub fn arg(mut self, value: impl Into<String>) -> Self {
        self.args.push(value.into());
        self
    }

    /// Add an environment variable.
    pub fn env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push(EnvVar::new(name, value));
        self
    }
}

/// Build one text content block.
pub fn text_block(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

/// Build content blocks from ordered prompt text.
pub fn text_blocks(texts: &[impl AsRef<str>]) -> Vec<Value> {
    texts.iter().map(|text| text_block(text.as_ref())).collect()
}
