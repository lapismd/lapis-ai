#![cfg(unix)]
use lapis_ai_runtime::{
    AcpClient, AcpOptions, AgentClient, ComputeClient, PreparedLaunch, ProcessLimits,
};
use serde_json::json;
use std::{os::unix::fs::PermissionsExt, path::Path, time::Duration};

fn launch(root: &Path, body: &str) -> PreparedLaunch {
    let executable = root.join("host-peer.py");
    std::fs::write(&executable, format!("#!/usr/bin/env python3\n{body}")).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
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
    let mut plan = launch(
        root.path(),
        "import json,sys\nassert '--runner' in sys.argv\nassert sys.argv[sys.argv.index('--control-version')+1]=='3'\nprint(json.dumps({'type':'ready','version':3}),flush=True)\nfor line in sys.stdin: print(line.strip(),flush=True)",
    );
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
    let mut plan = launch(
        root.path(),
        "import json,sys,os\nassert os.getcwd()==os.path.realpath(os.environ['HOME'])\nprint(json.dumps({'type':'ready','endpoint':'http://127.0.0.1:12345/v1','identity':'retained'}),flush=True)\nfor line in sys.stdin:\n frame=json.loads(line)\n if frame['action']=='stop': break\n print(json.dumps({'type':'progress','requestId':frame['requestId'],'progress':42}),flush=True)",
    );
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
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn prepared_agent_keeps_host_environment_and_output_limits() {
    let root = tempfile::tempdir().unwrap();
    let mut plan = launch(
        root.path(),
        "import os,json,sys\nassert sys.argv[1]=='--models'\nassert os.environ['HOST_ONLY']=='retained'\nassert 'JJ_STORE_TEST_SERVER_CREDENTIAL' not in os.environ\nsys.stdout.write(json.dumps({'status':'ready','extra':'x'*100}))",
    );
    plan.override_environment("HOST_ONLY", "retained").unwrap();
    let limits = ProcessLimits {
        maximum_frame_bytes: 64,
        ..Default::default()
    };
    assert!(
        AgentClient::models_prepared(plan, limits, Duration::from_secs(2))
            .await
            .is_err()
    );
}
