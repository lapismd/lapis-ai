#![cfg(unix)]
#[path = "support/acp_session.rs"]
mod acp_session;
#[path = "support/acp_batches.rs"]
mod support;
use acp_session::{configured, frames, options, prompt, wait};
use lapis_ai_runtime::AcpSession;
use std::time::Duration;
use support::{Capture, Mode, captured, plan};

fn limits(root: &std::path::Path) -> lapis_ai_runtime::SessionOptions {
    let mut value = options(root, "managed");
    value.request_timeout = Duration::from_secs(2);
    value
}
#[tokio::test]
async fn deferred_text_and_terminal_frame_share_one_native_cursor_and_acknowledgement() {
    let root = tempfile::tempdir().unwrap();
    let (capture, _) = Capture::new(root.path(), Mode::Capture);
    let session =
        AcpSession::start_with_processor(plan(root.path(), false), limits(root.path()), capture)
            .await
            .unwrap();
    configured(&session).await;
    session
        .submit("turn".into(), prompt("hello"))
        .await
        .unwrap();
    let snapshot = wait(&session, |s| {
        s.events.iter().any(|e| e.frame["type"] == "record")
    })
    .await;
    let event = snapshot.events.last().unwrap();
    assert_eq!(event.before.len(), 1);
    assert_eq!(
        event.before[0]["update"]["content"]["text"],
        "captured safe tail"
    );
    assert_eq!(
        event
            .frames()
            .map(|f| f["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["activity", "record"]
    );
    assert_eq!(
        snapshot
            .events
            .iter()
            .map(|e| e.sequence)
            .collect::<Vec<_>>(),
        (1..=snapshot.sequence).collect::<Vec<_>>()
    );
    let records = captured(root.path());
    assert!(records.iter().any(|r| r["publication"] == event.sequence
        && r["before"] == serde_json::to_value(&event.before).unwrap()));
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("unprocessed")
    );
    session.acknowledge(event.sequence).unwrap();
    assert!(session.snapshot(event.sequence).unwrap().events.is_empty());
    assert!(session.snapshot(0).is_err());
    session.close().await.unwrap();
}
#[tokio::test]
async fn facts_flush_into_the_unexposed_startup_event_without_inventing_a_cursor() {
    let root = tempfile::tempdir().unwrap();
    let (capture, _) = Capture::new(root.path(), Mode::Capture);
    let session =
        AcpSession::start_with_processor(plan(root.path(), true), limits(root.path()), capture)
            .await
            .unwrap();
    let snapshot = session.snapshot(0).unwrap();
    assert_eq!(snapshot.sequence, 1);
    assert_eq!(snapshot.events.len(), 1);
    let event = &snapshot.events[0];
    assert!(event.before.is_empty());
    assert_eq!(event.after.len(), 1);
    assert_eq!(event.frames().count(), 2);
    assert_eq!(event.after[0]["requestId"], "native-warm");
    assert!(event.after[0].get("sequence").is_none());
    assert_eq!(session.facts()["type"], "opened");
    assert_eq!(
        captured(root.path())[1]["publication"],
        serde_json::Value::Null
    );
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("unprocessed")
    );
    session.acknowledge(1).unwrap();
    session.close().await.unwrap();
}
#[tokio::test]
async fn forged_prefix_identity_and_permission_frames_fail_without_terminal_fallback() {
    for field in ["sessionId", "requestId", "message", "kind", "sequence"] {
        let root = tempfile::tempdir().unwrap();
        let (capture, _) = Capture::new(root.path(), Mode::BadPrefix(field));
        let session = AcpSession::start_with_processor(
            plan(root.path(), false),
            limits(root.path()),
            capture,
        )
        .await
        .unwrap();
        configured(&session).await;
        session
            .submit("turn".into(), prompt("hello"))
            .await
            .unwrap();
        let snapshot = wait(&session, |s| s.closed).await;
        assert_eq!(snapshot.failure, Some("processed_batch_invalid"));
        assert_eq!(snapshot.unresolved_requests, ["turn"]);
        assert!(!snapshot.events.iter().any(|e| e.frame["type"] == "record"));
        assert_eq!(
            frames(root.path())
                .iter()
                .filter(|f| f["type"] == "prompt")
                .count(),
            1
        );
        session.close().await.unwrap();
    }
}
#[tokio::test]
async fn blocked_terminal_capture_and_cancelled_close_keep_the_entire_group_owned() {
    let root = tempfile::tempdir().unwrap();
    let (capture, control) = Capture::new(root.path(), Mode::BlockRecord);
    let session =
        AcpSession::start_with_processor(plan(root.path(), false), limits(root.path()), capture)
            .await
            .unwrap();
    configured(&session).await;
    session
        .submit("turn".into(), prompt("hello"))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), control.entered.notified())
        .await
        .unwrap();
    let before = session.snapshot(0).unwrap();
    assert!(before.events.iter().all(|e| e.before.is_empty()));
    assert!(session.acknowledge(before.sequence + 1).is_err());
    let owner = session.clone();
    let close = tokio::spawn(async move { owner.close().await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!close.is_finished());
    close.abort();
    let _ = close.await;
    control.release.notify_one();
    session.close().await.unwrap();
    let after = session.snapshot(0).unwrap();
    assert_eq!(after.events.last().unwrap().before.len(), 1);
    assert_eq!(after.events.last().unwrap().frame["type"], "record");
}
#[tokio::test]
async fn cancelled_startup_drains_the_facts_prefix_before_reaping() {
    let root = tempfile::tempdir().unwrap();
    let (capture, control) = Capture::new(root.path(), Mode::BlockFacts);
    let (cancel, signal) = tokio::sync::watch::channel(false);
    let prepared = plan(root.path(), true);
    let opts = limits(root.path());
    let opening = tokio::spawn(async move {
        AcpSession::start_cancellable_with_processor(prepared, opts, signal, capture).await
    });
    tokio::time::timeout(Duration::from_secs(5), control.entered.notified())
        .await
        .unwrap();
    cancel.send_replace(true);
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!opening.is_finished());
    control.release.notify_one();
    assert_eq!(opening.await.unwrap().err().unwrap().0, "startup_cancelled");
    assert_eq!(
        captured(root.path()).last().unwrap()["before"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[tokio::test]
async fn group_count_and_retained_byte_limits_fail_atomically() {
    for mode in [Mode::TooMany, Mode::LargePrefix] {
        let root = tempfile::tempdir().unwrap();
        let (capture, _) = Capture::new(root.path(), mode);
        let mut opts = limits(root.path());
        opts.event_byte_limit = 4096;
        let session = AcpSession::start_with_processor(plan(root.path(), false), opts, capture)
            .await
            .unwrap();
        configured(&session).await;
        session
            .submit("turn".into(), prompt("hello"))
            .await
            .unwrap();
        let snapshot = wait(&session, |s| s.closed).await;
        assert_eq!(
            snapshot.failure,
            Some(if matches!(mode, Mode::TooMany) {
                "processed_batch_invalid"
            } else {
                "output_coverage_lost"
            })
        );
        assert_eq!(snapshot.unresolved_requests, ["turn"]);
        assert!(
            !snapshot
                .events
                .iter()
                .any(|e| !e.before.is_empty() || e.frame["type"] == "record")
        );
        session.close().await.unwrap();
    }
}
#[tokio::test]
async fn default_sessions_keep_the_original_serialized_event_shape() {
    let root = tempfile::tempdir().unwrap();
    let session = AcpSession::start(plan(root.path(), false), limits(root.path()))
        .await
        .unwrap();
    configured(&session).await;
    let snapshot = session.snapshot(0).unwrap();
    for event in &snapshot.events {
        let encoded = serde_json::to_value(event).unwrap();
        assert_eq!(encoded.as_object().unwrap().len(), 2);
        assert!(encoded.get("frame").is_some());
        assert!(encoded.get("sequence").is_some());
        assert_eq!(event.frames().count(), 1);
    }
    session.close().await.unwrap();
}
