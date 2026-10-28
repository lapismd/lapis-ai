use lapis_acp_protocol::{EnvVar, McpServer};
use serde_json::json;

#[test]
fn mcp_environment_debug_redacts_values_without_changing_the_wire() {
    let sentinel = "native-tool-auth-sentinel";
    let variable = EnvVar::new("LAPIS_NOTES_TOOL_TOKEN", sentinel);
    assert!(!format!("{variable:?}").contains(sentinel));
    let server =
        McpServer::new("lapis-tools", "/native/bridge").env("LAPIS_NOTES_TOOL_TOKEN", sentinel);
    assert!(!format!("{server:?}").contains(sentinel));
    assert_eq!(
        serde_json::to_value(&variable).unwrap(),
        json!({
            "name":"LAPIS_NOTES_TOOL_TOKEN", "value":sentinel
        })
    );
    assert_eq!(
        serde_json::to_value(&server).unwrap()["env"][0]["value"],
        sentinel
    );
}
