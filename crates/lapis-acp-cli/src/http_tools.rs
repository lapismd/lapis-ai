//! Trusted loopback MCP declarations; parsing performs no network IO.
use lapis_acp_protocol::{HttpMcpServer, McpServerConfig};
use serde::Deserialize;
use std::{collections::BTreeSet, net::IpAddr};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HttpTool {
    #[serde(rename = "type")]
    transport: Transport,
    name: String,
    url: String,
    headers: Vec<Header>,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Transport {
    Http,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    name: String,
    value: String,
}

pub(super) fn validate(
    tool: HttpTool,
    names: &mut BTreeSet<String>,
) -> Result<McpServerConfig, ()> {
    let _ = tool.transport;
    if tool.name.is_empty()
        || tool.name.len() > 64
        || !tool
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        || !names.insert(tool.name.clone())
        || tool.url.len() > 4096
        || tool
            .url
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
        || tool.headers.len() > 32
    {
        return Err(());
    }
    let url = url::Url::parse(&tool.url).map_err(|_| ())?;
    if url.scheme() != "http"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.port_or_known_default().is_none_or(|port| port == 0)
        || match url.host() {
            Some(url::Host::Ipv4(ip)) => !IpAddr::V4(ip).is_loopback(),
            Some(url::Host::Ipv6(ip)) => !IpAddr::V6(ip).is_loopback(),
            _ => true,
        }
        || !explicit_port(&tool.url)
    {
        return Err(());
    }
    let mut server = HttpMcpServer::new(tool.name, tool.url);
    let mut headers = BTreeSet::new();
    for header in tool.headers {
        let name = header.name.to_ascii_lowercase();
        if header.name.is_empty()
            || header.name.len() > 128
            || !header
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || matches!(
                name.as_str(),
                "host" | "connection" | "content-length" | "transfer-encoding"
            )
            || !headers.insert(name)
            || header.value.len() > 4096
            || header.value.bytes().any(|b| b.is_ascii_control())
        {
            return Err(());
        }
        server = server.header(header.name, header.value);
    }
    Ok(server.into())
}

fn explicit_port(value: &str) -> bool {
    let Some(authority) = value
        .strip_prefix("http://")
        .and_then(|v| v.split('/').next())
    else {
        return false;
    };
    // SocketAddr requires literal IP spelling and a port, avoiding URL host aliases.
    authority
        .parse::<std::net::SocketAddr>()
        .is_ok_and(|address| address.port() != 0)
}
