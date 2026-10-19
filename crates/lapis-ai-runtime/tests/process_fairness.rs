#![cfg(unix)]
use lapis_ai_config::SystemResolver;
use lapis_ai_runtime::{Component, Config, NativeProcess, PreparedLaunch, ProcessLimits};
use std::{os::unix::fs::PermissionsExt, time::Duration};

async fn packed_process() -> (tempfile::TempDir, NativeProcess) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("peer.py");
    std::fs::write(&path, "#!/usr/bin/env python3\nimport sys,json\nsys.stdout.write(''.join(json.dumps({'number':i})+'\\n' for i in range(1000)))\nsys.stdout.flush()\nsys.stdin.read()\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let config = Config::parse("version=1\n[runtime]\nagent='peer.py'", root.path()).unwrap();
    let process = NativeProcess::spawn_with_limits(
        PreparedLaunch::new(&config, Component::Agent, &SystemResolver).unwrap(),
        ProcessLimits {
            queued_frames: 8,
            ..ProcessLimits::default()
        },
    )
    .await
    .unwrap();
    (root, process)
}

async fn drain(mut process: NativeProcess) {
    let result = async {
        for number in 0..1000 {
            let frame = process.next(Duration::from_secs(3)).await?;
            assert_eq!(frame["number"], number);
        }
        Ok::<(), lapis_ai_runtime::Error>(())
    }
    .await;
    let cleanup = process.shutdown(Duration::from_secs(1)).await;
    assert!(
        result.is_ok(),
        "Active consumer lost a queued burst: {result:?}"
    );
    cleanup.unwrap();
}

#[tokio::test]
async fn an_active_reader_preserves_a_packed_burst_larger_than_the_pipe_queue() {
    let (_root, process) = packed_process().await;
    drain(process).await;
}

#[tokio::test]
async fn a_consumer_resuming_after_a_brief_pause_preserves_the_same_order() {
    let (_root, mut process) = packed_process().await;
    assert_eq!(
        process.next(Duration::from_secs(3)).await.unwrap()["number"],
        0
    );
    tokio::time::sleep(Duration::from_millis(25)).await;
    let result = async {
        for number in 1..1000 {
            assert_eq!(
                process.next(Duration::from_secs(3)).await?["number"],
                number
            );
        }
        Ok::<(), lapis_ai_runtime::Error>(())
    }
    .await;
    let cleanup = process.shutdown(Duration::from_secs(1)).await;
    assert!(
        result.is_ok(),
        "Briefly delayed consumer lost output: {result:?}"
    );
    cleanup.unwrap();
}
