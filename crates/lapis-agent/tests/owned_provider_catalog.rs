use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
};
fn catalog(provider: &str, pages: Vec<(&'static str, u16, Value)>) -> Value {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for (path, status, body) in pages {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut raw = Vec::new();
            let mut buffer = [0; 4096];
            while !raw.windows(4).any(|v| v == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).unwrap();
                assert!(n > 0);
                raw.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8_lossy(&raw);
            assert!(request.starts_with(&format!("GET {path}")), "{request}");
            assert!(request.contains("fixture-provider-secret"));
            let body = body.to_string();
            write!(socket,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_lapis-agent"))
        .arg("--models")
        .env_clear()
        .env("BUZZ_AGENT_PROVIDER", provider)
        .env("BUZZ_AGENT_MODEL", "auto")
        .env("ANTHROPIC_API_KEY", "fixture-provider-secret")
        .env("ANTHROPIC_BASE_URL", format!("http://{address}"))
        .env("OPENAI_COMPAT_API_KEY", "fixture-provider-secret")
        .env("OPENAI_COMPAT_BASE_URL", format!("http://{address}/v1"))
        .env("OPENROUTER_API_KEY", "fixture-provider-secret")
        .env("OPENROUTER_BASE_URL", format!("http://{address}/v1"))
        .env("DATABRICKS_TOKEN", "fixture-provider-secret")
        .env("DATABRICKS_HOST", format!("http://{address}"))
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("fixture-provider-secret"));
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn anthropic_paginates_and_preserves_display_names() {
    let result = catalog(
        "anthropic",
        vec![
            (
                "/v1/models?limit=100",
                200,
                json!({"data":[{"id":"first","display_name":"First"}],"has_more":true,"last_id":"first"}),
            ),
            (
                "/v1/models?limit=100&after_id=first",
                200,
                json!({"data":[{"id":"second"}],"has_more":false}),
            ),
        ],
    );
    assert_eq!(result["status"], "ready");
    assert_eq!(
        result["facts"]["models"]["availableModels"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn openai_compatible_and_openrouter_discover_without_prompts() {
    for provider in ["openai", "openai-compat", "openrouter"] {
        let result = catalog(
            provider,
            vec![(
                "/v1/models",
                200,
                json!({"data":[{"id":"vendor/model","supported_parameters":["tools"]}]}),
            )],
        );
        assert_eq!(result["status"], "ready");
        assert_eq!(
            result["facts"]["models"]["availableModels"][0]["modelId"],
            "vendor/model"
        );
    }
}
#[test]
fn databricks_native_catalogs_and_empty_results_are_honest() {
    let v1 = catalog(
        "databricks",
        vec![(
            "/api/2.0/serving-endpoints",
            200,
            json!({"endpoints":[{"name":"databricks-llama-4","state":{"ready":"READY"},"task":"llm/v1/chat"}]}),
        )],
    );
    assert_eq!(v1["status"], "ready");
    let v2 = catalog(
        "databricks_v2",
        vec![
            ("/api/ai-gateway/v2/endpoints", 200, json!({"endpoints":[]})),
            (
                "/api/2.1/unity-catalog/model-services",
                200,
                json!({"model_services":[]}),
            ),
        ],
    );
    assert_eq!(v2["status"], "empty");
    assert_eq!(v2["facts"]["models"]["availableModels"], json!([]));
}
#[test]
fn authentication_and_malformed_catalogs_do_not_fabricate_models() {
    let denied = catalog(
        "openai",
        vec![(
            "/v1/models",
            401,
            json!({"error":"fixture-provider-secret"}),
        )],
    );
    assert_eq!(denied["status"], "authentication_required");
    assert_eq!(denied["facts"], json!({}));
    let bad = catalog(
        "openai",
        vec![("/v1/models", 200, json!({"data":[{"id":"bad\nidentity"}]}))],
    );
    assert_eq!(bad["status"], "unavailable");
}
