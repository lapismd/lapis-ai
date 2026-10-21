use lapis_ai_config::ComputeInventory;
use serde_json::{Value, json};
use std::{process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[tokio::test]
async fn real_helper_reports_correlated_import_progress_persistent_inventory_and_safe_failures() {
    let root = tempfile::tempdir().unwrap();
    let weights = root.path().join("Example.gguf");
    std::fs::write(&weights, [b"GGUF".as_slice(), &[0u8; 20]].concat()).unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_lapis-compute"))
        .current_dir(root.path())
        .env_clear()
        .env("HOME", root.path())
        .env("PATH", "/usr/bin:/bin")
        .env("JJ_STORE_COMPUTE_IDENTITY", "24".repeat(32))
        .env("JJ_STORE_COMPUTE_TOKEN", "fixture-token-value")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut input = child.stdin.take().unwrap();
    let ready = tokio::time::timeout(Duration::from_secs(15), output.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&ready).unwrap()["type"],
        "ready"
    );
    for (action, id, expected) in [
        ("start", "missing", "error"),
        ("import", "import", "completed"),
        ("remove", "remove", "completed"),
    ] {
        let inventory = ComputeInventory::read(root.path()).unwrap();
        let model = inventory
            .models
            .first()
            .map(|m| m.id.clone())
            .unwrap_or_default();
        let request = json!({"action":action,"requestId":id,"path":weights,"model":model});
        input
            .write_all(format!("{request}\n").as_bytes())
            .await
            .unwrap();
        let mut progress = false;
        loop {
            let line = tokio::time::timeout(Duration::from_secs(10), output.next_line())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            let frame: Value = serde_json::from_str(&line).unwrap();
            if frame["type"] == "progress" {
                assert_eq!(frame["requestId"], id);
                assert!(frame["total_bytes"].as_u64().is_some());
                progress = true;
            }
            if frame["requestId"] == id
                && matches!(frame["type"].as_str(), Some("completed" | "error"))
            {
                assert_eq!(frame["type"], expected);
                if action == "start" {
                    assert_eq!(frame["code"], "model_required");
                }
                if action == "import" {
                    assert!(progress);
                    let model = &ComputeInventory::read(root.path()).unwrap().models[0];
                    assert_eq!(model.name, "Example.gguf");
                    assert_eq!(model.bytes, Some(24));
                }
                break;
            }
        }
    }
    assert!(
        ComputeInventory::read(root.path())
            .unwrap()
            .models
            .is_empty()
    );
    input.write_all(b"{\"action\":\"stop\"}\n").await.unwrap();
    drop(input);
    assert!(
        tokio::time::timeout(Duration::from_secs(20), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
