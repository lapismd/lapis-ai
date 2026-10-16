//! Real helper regression: terminal acknowledgement releases the operation slot.
use serde_json::{Value, json};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[tokio::test]
async fn acknowledged_operation_allows_the_next_command_immediately() {
    let root = tempfile::tempdir().unwrap();
    let model = root.path().join("fixture.gguf");
    std::fs::write(&model, [b"GGUF".as_slice(), &[0u8; 20]].concat()).unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_lapis-compute"))
        .current_dir(root.path())
        .env_clear()
        .env("HOME", root.path())
        .env("PATH", "/usr/bin:/bin")
        .env("JJ_STORE_COMPUTE_IDENTITY", "22".repeat(32))
        .env("JJ_STORE_COMPUTE_TOKEN", "fixture-token-value")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut input = child.stdin.take().unwrap();
    let ready = tokio::time::timeout(std::time::Duration::from_secs(15), output.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&ready).unwrap()["type"],
        "ready"
    );
    for index in 0..100 {
        for action in ["import", "configure"] {
            let id = format!("{action}-{index}");
            let request = json!({"action":action,"requestId":id,"path":model,"config":{}});
            input
                .write_all(format!("{request}\n").as_bytes())
                .await
                .unwrap();
            input.flush().await.unwrap();
            loop {
                let line =
                    tokio::time::timeout(std::time::Duration::from_secs(10), output.next_line())
                        .await
                        .unwrap()
                        .unwrap()
                        .unwrap();
                let frame: Value = serde_json::from_str(&line).unwrap();
                assert_ne!(
                    frame["type"], "error",
                    "terminal acknowledgement left the helper busy: {frame}"
                );
                if frame["requestId"] == id
                    && matches!(frame["type"].as_str(), Some("completed" | "configured"))
                {
                    break;
                }
            }
        }
    }
    input.write_all(b"{\"action\":\"stop\"}\n").await.unwrap();
    drop(input);
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(20), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
#[tokio::test]
async fn startup_configuration_is_visible_without_downloads_and_http_requires_authentication() {
    let root = tempfile::tempdir().unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_lapis-compute"))
        .current_dir(root.path())
        .env_clear()
        .env("HOME", root.path())
        .env("PATH", "/usr/bin:/bin")
        .env("JJ_STORE_COMPUTE_IDENTITY", "23".repeat(32))
        .env("JJ_STORE_COMPUTE_TOKEN", "fixture-token-value")
        .env(
            "LAPIS_AI_COMPUTE_CONFIG",
            r#"{"model":"startup-model","sharing":false}"#,
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
    let line = tokio::time::timeout(std::time::Duration::from_secs(15), output.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let ready: Value = serde_json::from_str(&line).unwrap();
    let url = format!("{}/models", ready["endpoint"].as_str().unwrap());
    let http = reqwest::Client::new();
    assert_eq!(http.get(&url).send().await.unwrap().status(), 401);
    let models: Value = http
        .get(&url)
        .bearer_auth("fixture-token-value")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(models["data"][0]["id"], "startup-model");
    assert!(!root.path().join("installed-models.json").exists());
    let mut input = child.stdin.take().unwrap();
    input.write_all(b"{\"action\":\"stop\"}\n").await.unwrap();
    drop(input);
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(20), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
