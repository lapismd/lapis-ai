use lapis_acp_protocol::{HttpMcpServer, McpServer, McpServerConfig, SessionNewRequest};
use serde_json::json;

#[test]
fn mixed_transports_preserve_the_stdio_wire_and_redact_http_credentials() {
    let credential = "Bearer native-http-tool-sentinel";
    let servers: Vec<McpServerConfig> = vec![
        McpServer::new("stdio-tools", "/native/bridge").into(),
        HttpMcpServer::new("cursor-tools", "http://127.0.0.1:3210/mcp")
            .header("Authorization", credential)
            .into(),
    ];
    assert!(!format!("{servers:?}").contains(credential));
    let request = SessionNewRequest {
        cwd: "/vault",
        mcp_servers: servers,
        system_prompt: None,
        session_title: None,
    };
    assert_eq!(
        lapis_acp_protocol::session::new_params(&request)["mcpServers"],
        json!([
            {"name":"stdio-tools","command":"/native/bridge","args":[],"env":[]},
            {"type":"http","name":"cursor-tools","url":"http://127.0.0.1:3210/mcp",
             "headers":[{"name":"Authorization","value":credential}]}
        ])
    );
    assert_eq!(request.mcp_servers[0].name(), "stdio-tools");
    assert_eq!(request.mcp_servers[1].name(), "cursor-tools");
}
