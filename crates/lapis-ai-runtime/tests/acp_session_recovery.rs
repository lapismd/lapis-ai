#![cfg(unix)]
#[path = "support/acp_session.rs"]
mod support;
use lapis_ai_runtime::{AcpSession, Error, NativeProcess, SessionRequest};
use std::time::Duration;
use support::*;

#[tokio::test]
async fn cancel_revokes_live_permission_and_null_decision_never_becomes_approval() {
    let root = tempfile::tempdir().unwrap();
    let session = start(root.path(), "cancel-permission").await;
    configured(&session).await;
    session
        .submit("first".into(), prompt("permission"))
        .await
        .unwrap();
    wait(&session, |s| {
        s.events
            .iter()
            .any(|event| event.frame["type"] == "permission")
    })
    .await;
    session
        .submit(
            "cancel".into(),
            SessionRequest::Cancel {
                target: "first".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        session
            .submit(
                "late".into(),
                SessionRequest::Permission {
                    permission_id: "permission-exact".into(),
                    option_id: Some("provider:allow/once".into())
                }
            )
            .await,
        Err(Error("stale_or_invalid_permission"))
    );
    wait(&session, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    session
        .submit("second".into(), prompt("permission"))
        .await
        .unwrap();
    wait(&session, |s| {
        s.events.iter().any(|event| {
            event.frame["type"] == "permission" && event.frame["requestId"] == "second"
        })
    })
    .await;
    session
        .submit(
            "deny".into(),
            SessionRequest::Permission {
                permission_id: "permission-exact".into(),
                option_id: None,
            },
        )
        .await
        .unwrap();
    wait(&session, |s| {
        s.events
            .iter()
            .filter(|event| event.frame["type"] == "record")
            .count()
            == 2
    })
    .await;
    let log = frames(root.path());
    let answers: Vec<_> = log
        .iter()
        .filter(|frame| frame["type"] == "permission")
        .collect();
    assert_eq!(answers.len(), 1);
    assert!(answers[0]["optionId"].is_null());
    session.close().await.unwrap();
}

#[tokio::test]
async fn byte_limit_stops_large_output_independently_of_frame_count() {
    let root = tempfile::tempdir().unwrap();
    let mut limited = options(root.path(), "byte-limit");
    limited.event_limit = 4096;
    limited.event_byte_limit = 2048;
    let session = AcpSession::start(plan(root.path()), limited).await.unwrap();
    configured(&session).await;
    session
        .submit("large".into(), prompt("large-reply"))
        .await
        .unwrap();
    let snapshot = wait(&session, |s| s.closed).await;
    assert_eq!(snapshot.failure, Some("output_coverage_lost"));
    assert_eq!(snapshot.unresolved_requests, ["large"]);
    assert!(snapshot.events.len() < 10);
    session.close().await.unwrap();
}

#[tokio::test]
async fn cancelling_a_submission_wait_never_replays_or_forgets_the_request() {
    let root = tempfile::tempdir().unwrap();
    let session = start(root.path(), "cancelled-submit").await;
    configured(&session).await;
    let mut submission = Box::pin(session.submit("once".into(), prompt("hello")));
    tokio::select! { biased; result = &mut submission => result.unwrap(), _ = tokio::task::yield_now() => {} }
    drop(submission);
    wait(&session, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    assert_eq!(
        session.submit("once".into(), prompt("again")).await,
        Err(Error("duplicate_request"))
    );
    assert_eq!(
        frames(root.path())
            .iter()
            .filter(|frame| frame["type"] == "prompt")
            .count(),
        1
    );
    session.close().await.unwrap();
}

#[tokio::test]
async fn cancelling_startup_reaps_the_helper_and_does_not_start_a_second_worker() {
    let root = tempfile::tempdir().unwrap();
    let mut launch = plan(root.path());
    launch.environment.0.insert("BOOT_HANG".into(), "1".into());
    let mut limits = options(root.path(), "cancelled-start");
    limits.request_timeout = Duration::from_secs(5);
    let startup = tokio::spawn(AcpSession::start(launch, limits));
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if root.path().join("frames.jsonl").exists()
                && frames(root.path())
                    .iter()
                    .any(|frame| frame["type"] == "warm")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let pid = frames(root.path())[0]["pid"].as_u64().unwrap() as u32;
    startup.abort();
    let _ = startup.await;
    reaped(pid).await;
    assert_eq!(
        frames(root.path())
            .iter()
            .filter(|frame| frame.get("pid").is_some())
            .count(),
        1
    );
}

#[tokio::test]
async fn cancelled_process_shutdown_retains_ownership_of_the_whole_child_group() {
    let root = tempfile::tempdir().unwrap();
    let launch = plan(root.path());
    std::fs::write(&launch.executable, "#!/usr/bin/env python3\nimport os,time,json\nchild=os.fork()\nif child==0:\n time.sleep(120)\n os._exit(0)\nprint(json.dumps({'pid':os.getpid(),'child':child}),flush=True)\ntime.sleep(120)\n").unwrap();
    let mut process = NativeProcess::spawn(launch).await.unwrap();
    let identity = process.next(Duration::from_secs(3)).await.unwrap();
    let pid = identity["pid"].as_u64().unwrap() as u32;
    let child = identity["child"].as_u64().unwrap() as u32;
    let mut close = Box::pin(process.shutdown(Duration::from_secs(30)));
    tokio::select! { biased; result = &mut close => panic!("unexpected shutdown completion: {result:?}"), _ = tokio::task::yield_now() => {} }
    drop(close);
    drop(process);
    reaped(pid).await;
    reaped(child).await;
}

#[tokio::test]
async fn a_new_provider_identity_cannot_silently_replace_explicit_resume() {
    let root = tempfile::tempdir().unwrap();
    let mut selected = options(root.path(), "resume-mismatch");
    selected.resume = Some("existing-provider-session".into());
    assert_eq!(
        AcpSession::start(plan(root.path()), selected).await.err(),
        Some(Error("provider_resume_mismatch"))
    );
    reaped(frames(root.path())[0]["pid"].as_u64().unwrap() as u32).await;
    assert!(
        !frames(root.path())
            .iter()
            .any(|frame| frame["type"] == "prompt")
    );
}
