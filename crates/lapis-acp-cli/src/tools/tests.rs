use super::*;
use serde_json::json;
struct File(std::path::PathBuf);
impl File {
    fn new(bytes: &[u8]) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "lapis-acp-tools-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::write(&path, bytes).unwrap();
        Self(path)
    }
    fn read(&self) -> Result<Vec<McpServerConfig>, TaskRejection> {
        read(self.0.to_str())
    }
}
impl Drop for File {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
#[test]
fn trusted_tools_refuse_duplicate_names_relative_commands_and_reserved_identity() {
    let valid = json!({"name":"workspace","command":"/bin/false","args":[],"env":[]});
    assert_eq!(
        File::new(json!([valid]).to_string().as_bytes())
            .read()
            .unwrap()
            .len(),
        1
    );
    assert!(
        File::new(json!([valid, valid]).to_string().as_bytes())
            .read()
            .is_err()
    );
    for (field, value) in [
        ("command", json!("relative")),
        ("env", json!([{"name":"JJ_ACP_TASK_ID","value":"spoof"}])),
        ("name", json!("bad\nname")),
    ] {
        let mut tool = valid.clone();
        tool[field] = value;
        assert!(
            File::new(json!([tool]).to_string().as_bytes())
                .read()
                .is_err()
        );
    }
}
#[test]
fn trusted_tools_are_bounded_and_diagnostics_do_not_echo_values() {
    let file = File::new(&vec![b' '; 65537]);
    assert!(file.read().is_err());
    let file=File::new(br#"[{"name":"workspace","command":"/bin/false","args":[],"env":[{"name":"JJ_ACP_TASK_ID","value":"secret-test-value"}]}]"#);
    let error = file.read().err().unwrap();
    assert!(!format!("{error:?}").contains("secret-test-value"));
    assert!(read(None).unwrap().is_empty());
}

#[test]
fn trusted_http_tools_preserve_headers_and_share_the_name_budget() {
    let http = json!({"type":"http","name":"cursor","url":"http://127.0.0.1:3210/mcp",
        "headers":[{"name":"Authorization","value":"Bearer sentinel-http-tool"}]});
    let stdio = json!({"name":"stdio","command":"/native/bridge","args":[],"env":[]});
    let servers = File::new(json!([http, stdio]).to_string().as_bytes())
        .read()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&servers).unwrap(),
        json!([http, stdio])
    );
    assert!(!format!("{servers:?}").contains("sentinel-http-tool"));
    let mut duplicate = stdio.clone();
    duplicate["name"] = json!("cursor");
    assert!(
        File::new(json!([http, duplicate]).to_string().as_bytes())
            .read()
            .is_err()
    );
    let mut mixed = http;
    mixed["command"] = json!("/native/bridge");
    assert!(
        File::new(json!([mixed]).to_string().as_bytes())
            .read()
            .is_err()
    );
}

#[test]
fn trusted_http_tools_refuse_remote_ambiguous_or_credential_urls() {
    for url in [
        "https://example.com:443/mcp",
        "http://example.com:3210/mcp",
        "http://0.0.0.0:3210/mcp",
        "http://localhost:3210/mcp",
        "http://127.0.0.1/mcp",
        "http://127.0.0.1:0/mcp",
        "http://user:sentinel@127.0.0.1:3210/mcp",
        "http://127.0.0.1:3210/mcp?token=sentinel",
        "http://127.0.0.1:3210/mcp#sentinel",
        "http://127.0.0.1:3210/mcp\r\nInjected: sentinel",
        "http://2130706433:3210/mcp",
    ] {
        let tool = json!({"type":"http","name":"cursor","url":url,"headers":[]});
        let error = File::new(json!([tool]).to_string().as_bytes())
            .read()
            .unwrap_err();
        assert!(!format!("{error:?}").contains("sentinel"));
    }
    for url in ["http://127.0.0.1:80/mcp", "http://[::1]:3210/mcp"] {
        let tool = json!({"type":"http","name":"cursor","url":url,"headers":[]});
        assert!(
            File::new(json!([tool]).to_string().as_bytes())
                .read()
                .is_ok()
        );
    }
}

#[test]
fn trusted_http_tools_refuse_header_injection_and_case_folded_duplicates() {
    for headers in [
        json!([{"name":"Authorization","value":"Bearer token\r\nInjected: bad"}]),
        json!([{"name":"Host","value":"elsewhere"}]),
        json!([{"name":"Content-Length","value":"0"}]),
        json!([{"name":"Transfer-Encoding","value":"chunked"}]),
        json!([{"name":"X-Token","value":"a"},{"name":"x-token","value":"b"}]),
        json!([{"name":"bad name","value":"a"}]),
    ] {
        let tool = json!({"type":"http","name":"cursor","url":"http://127.0.0.1:3210/mcp","headers":headers});
        assert!(
            File::new(json!([tool]).to_string().as_bytes())
                .read()
                .is_err()
        );
    }
}

#[test]
fn authored_http_authorization_requires_an_environment_reference() {
    let literal = br#"[[tools]]
type = "http"
name = "cursor"
url = "http://127.0.0.1:3210/mcp"
headers = [{name = "Authorization", value = "Bearer must-not-persist"}]
"#;
    assert!(crate::toml_tools::normalize("tools.toml", literal).is_err());
    let referenced = br#"[[tools]]
type = "http"
name = "cursor"
url = "http://127.0.0.1:3210/mcp"
headers = [{name = "Authorization", env = "PATH"}]
"#;
    let normalized = crate::toml_tools::normalize("tools.toml", referenced).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&normalized).unwrap();
    assert_eq!(
        value[0]["headers"][0]["value"],
        std::env::var("PATH").unwrap()
    );
    assert!(value[0]["headers"][0].get("env").is_none());
}
