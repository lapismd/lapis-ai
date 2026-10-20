//! Typed ACP tool transports. Hosts own endpoint admission and authentication.
use crate::content::{EnvVar, McpServer};
use serde::Serialize;

/// One trusted MCP server declaration, encoded without changing stdio fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum McpServerConfig {
    /// Existing executable-based MCP transport.
    Stdio(McpServer),
    /// Host-admitted HTTP transport, such as Cursor's application tools.
    Http(HttpMcpServer),
}

impl McpServerConfig {
    /// Name shared by both transport variants.
    pub fn name(&self) -> &str {
        match self {
            Self::Stdio(server) => &server.name,
            Self::Http(server) => &server.name,
        }
    }
}

impl From<McpServer> for McpServerConfig {
    fn from(server: McpServer) -> Self {
        Self::Stdio(server)
    }
}

impl From<HttpMcpServer> for McpServerConfig {
    fn from(server: HttpMcpServer) -> Self {
        Self::Http(server)
    }
}

/// ACP HTTP MCP declaration. This data creates no listener or network request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HttpMcpServer {
    #[serde(rename = "type")]
    transport: HttpTransport,
    /// Host-owned name.
    pub name: String,
    /// Host-admitted endpoint; credentials belong in headers, never the URL.
    pub url: String,
    /// Header name/value pairs. Debug output redacts values as for MCP env.
    pub headers: Vec<EnvVar>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum HttpTransport {
    Http,
}

impl HttpMcpServer {
    /// Build a declaration. The controlling host must admit its URL and headers.
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            transport: HttpTransport::Http,
            name: name.into(),
            url: url.into(),
            headers: Vec::new(),
        }
    }

    /// Add a native-resolved header. Values are redacted from Debug output.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push(EnvVar::new(name, value));
        self
    }
}
