//! `initialize` negotiation.

use serde_json::{Value, json};

use crate::envelope::method;
use crate::error::Error;

/// The protocol version this client asks for.
pub const PROTOCOL_VERSION: u64 = 2;

/// Protocol versions this client can speak.
pub const SUPPORTED_VERSIONS: [u64; 2] = [1, 2];

/// Identifies this client to an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientInfo {
    /// Client name reported in `clientInfo.name`.
    pub name: String,
    /// Client version reported in `clientInfo.version`.
    pub version: String,
}

impl ClientInfo {
    /// The default identity for this repository's harness.
    pub fn lapis_acp() -> Self {
        Self {
            name: "lapis-acp".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

/// Filesystem and terminal capabilities advertised to an agent.
///
/// Everything defaults to false. This harness does not implement
/// `fs/read_text_file`, `fs/write_text_file` or any `terminal/*` method, so
/// advertising them would invite requests that can only fail. An agent that
/// needs file access uses its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClientCapabilities {
    /// Whether `fs/read_text_file` is implemented.
    pub read_text_file: bool,
    /// Whether `fs/write_text_file` is implemented.
    pub write_text_file: bool,
    /// Whether `terminal/*` methods are implemented.
    pub terminal: bool,
}

impl ClientCapabilities {
    fn to_value(self) -> Value {
        json!({
            "fs": {
                "readTextFile": self.read_text_file,
                "writeTextFile": self.write_text_file,
            },
            "terminal": self.terminal,
        })
    }
}

/// Build `initialize` params.
pub fn params(info: &ClientInfo, capabilities: ClientCapabilities) -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "clientCapabilities": capabilities.to_value(),
        "clientInfo": { "name": info.name, "version": info.version },
    })
}

/// The `initialize` method name, exposed so callers cannot misspell it.
pub const METHOD: &str = method::INITIALIZE;

/// A negotiated `initialize` result.
#[derive(Debug, Clone, PartialEq)]
pub struct InitializeResult {
    /// The protocol version the agent selected.
    pub protocol_version: u64,
    /// Agent name when reported.
    pub agent_name: Option<String>,
    /// Agent version when reported.
    pub agent_version: Option<String>,
    /// The raw `agentCapabilities` object, or null when absent.
    pub capabilities: Value,
}

impl InitializeResult {
    /// Report whether the agent advertises a named capability path.
    ///
    /// The path is a slice of object keys, so `&["promptCapabilities", "image"]`
    /// probes a nested flag. A missing intermediate key is false rather than an
    /// error.
    ///
    /// A boolean leaf is honoured as a boolean. Presence alone is not
    /// advertisement: adapters send explicit `false` for facilities they do not
    /// support, and treating `"audio": false` as advertised would send content the
    /// agent has already said it cannot take. A non-boolean, non-null leaf is an
    /// object or a marker whose presence is the advertisement, as with
    /// `loadSession: true` and `mcpCapabilities: { http: true }`.
    pub fn advertises(&self, path: &[&str]) -> bool {
        let mut cursor = &self.capabilities;
        for key in path {
            match cursor.get(*key) {
                Some(next) => cursor = next,
                None => return false,
            }
        }
        match cursor {
            Value::Null => false,
            Value::Bool(advertised) => *advertised,
            _ => true,
        }
    }
}

/// Parse an `initialize` result, rejecting a version this client cannot speak.
///
/// An agent selects the version it wants to use. When that selection is outside
/// [`SUPPORTED_VERSIONS`] the session cannot continue, and continuing anyway
/// would produce confusing failures later rather than a clear one now.
pub fn parse_result(result: &Value) -> Result<InitializeResult, Error> {
    let protocol_version = result
        .get("protocolVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| Error::Protocol("initialize result missing protocolVersion".to_owned()))?;
    if !SUPPORTED_VERSIONS.contains(&protocol_version) {
        return Err(Error::Protocol(format!(
            "agent selected unsupported protocol version {protocol_version}; supported versions are {:?}",
            SUPPORTED_VERSIONS
        )));
    }
    let info = result.get("agentInfo");
    Ok(InitializeResult {
        protocol_version,
        agent_name: info
            .and_then(|value| value.get("name"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        agent_version: info
            .and_then(|value| value.get("version"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        capabilities: result
            .get("agentCapabilities")
            .cloned()
            .unwrap_or(Value::Null),
    })
}
