#![cfg(unix)]
use lapis_ai_runtime::{
    AcpClient, AcpOptions, AgentClient, ComputeClient, Error, PreparedLaunch, ProcessLimits,
};
use serde_json::json;
use std::{path::Path, time::Duration};

fn launch(root: &Path, fixture: &str) -> PreparedLaunch {
    // These committed executables are never opened for writing by parallel tests.
    let executable = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    let mut command = std::process::Command::new(executable);
    command
        .current_dir(root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin");
    PreparedLaunch::from_command(&command).unwrap()
}

#[tokio::test]
async fn prepared_v3_acp_preserves_launch_and_exact_frames() {
    let root = tempfile::tempdir().unwrap();
    let mut plan = launch(root.path(), "host-acp.py");
    plan.arguments = vec!["--runner".into(), "fixture".into()];
    let mut client = AcpClient::start_prepared_with_options(plan, AcpOptions::default())
        .await
        .unwrap();
    assert_eq!(client.readiness()["version"], 3);
    let frame = json!({"version":3,"sessionId":"session","type":"permission","requestId":"decision","permissionId":"exact","optionId":"provider-option"});
    client.commands().send_frame(&frame).await.unwrap();
    assert_eq!(client.next(Duration::from_secs(2)).await.unwrap(), frame);
    assert!(
        client
            .commands()
            .send_frame(&json!({"version":2,"type":"prompt","requestId":"wrong"}))
            .await
            .is_err()
    );
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn compute_progress_and_stop_are_independent_of_operation_completion() {
    let root = tempfile::tempdir().unwrap();
    let mut plan = launch(root.path(), "host-compute.py");
    plan.override_environment("HOME", root.path().to_str().unwrap())
        .unwrap();
    plan.override_environment("JJ_STORE_COMPUTE_TOKEN", "private-fixture-token")
        .unwrap();
    let mut client = ComputeClient::start_prepared(plan, ProcessLimits::default())
        .await
        .unwrap();
    assert!(!format!("{:?}", client.connection()).contains("private-fixture-token"));
    assert_eq!(
        client
            .connection()
            .peer_endpoint(&"a".repeat(64))
            .unwrap()
            .path(),
        format!("/peer/{}/v1", "a".repeat(64))
    );
    assert!(client.connection().peer_endpoint("../escape").is_err());
    let sender = client.commands();
    client.next_event(Duration::from_secs(1)).await.unwrap();
    sender
        .send_frame(&json!({"requestId":"download-once","action":"download"}))
        .await
        .unwrap();
    let progress = client.next_event(Duration::from_secs(2)).await.unwrap();
    assert_eq!(progress["requestId"], "download-once");
    assert_eq!(progress["progress"], 42);
    sender.stop().await.unwrap();
    // The command sender can stop an active operation before final ownership cleanup.
    assert!(client.next_event(Duration::from_secs(2)).await.is_err());
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn prepared_agent_keeps_host_environment_and_output_limits() {
    let root = tempfile::tempdir().unwrap();
    let prepare = || {
        let mut plan = launch(root.path(), "host-agent.py");
        plan.override_environment("HOST_ONLY", "retained").unwrap();
        plan
    };
    let accepted =
        AgentClient::models_prepared(prepare(), ProcessLimits::default(), Duration::from_secs(2))
            .await
            .unwrap();
    assert_eq!(accepted["status"], "ready");
    assert_eq!(accepted["extra"].as_str().unwrap().len(), 100);
    let limits = ProcessLimits {
        maximum_frame_bytes: 64,
        ..Default::default()
    };
    assert_eq!(
        AgentClient::models_prepared(prepare(), limits, Duration::from_secs(2))
            .await
            .err()
            .unwrap(),
        Error("helper_protocol_failed")
    );
}
