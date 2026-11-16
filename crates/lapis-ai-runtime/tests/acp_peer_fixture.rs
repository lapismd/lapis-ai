#![cfg(target_os = "linux")]
#[path = "support/acp_session.rs"]
mod support;
use lapis_ai_runtime::{AcpSession, Environment, Error, NativeProcess, PreparedLaunch};
use std::{fs::OpenOptions, os::unix::fs::PermissionsExt};

#[tokio::test]
async fn protocol_peer_starts_while_a_completed_script_has_an_open_writer() {
    let root = tempfile::tempdir().unwrap();
    let launch = support::plan(root.path());
    let writer = OpenOptions::new()
        .write(true)
        .open(root.path().join(support::PEER_FILE))
        .unwrap();
    let session = AcpSession::start(launch, support::options(root.path(), "fixture-writer"))
        .await
        .unwrap();
    let identity = &support::frames(root.path())[0];
    assert_eq!(
        identity["args"],
        serde_json::json!(["serve", "--control-version", "3"])
    );
    assert_eq!(
        identity["root"],
        root.path()
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(
        session.facts()["adapterSessionId"],
        "provider-fixture-writer"
    );
    session.close().await.unwrap();
    support::reaped(identity["pid"].as_u64().unwrap() as u32).await;
    drop(writer);
}

#[tokio::test]
async fn direct_exec_of_a_write_open_script_remains_a_fail_closed_spawn_error() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("direct-peer.py");
    std::fs::write(
        &executable,
        "#!/usr/bin/env python3\nraise AssertionError('must not execute')\n",
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let writer = OpenOptions::new().write(true).open(&executable).unwrap();
    let mut environment = Environment::default();
    environment
        .0
        .insert("PATH".into(), std::env::var("PATH").unwrap());
    let result = NativeProcess::spawn(PreparedLaunch {
        executable,
        arguments: Vec::new(),
        directory: root.path().into(),
        environment,
    })
    .await;
    assert_eq!(result.err().unwrap(), Error("helper_spawn_failed"));
    drop(writer);
}
