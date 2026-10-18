#![cfg(unix)]
#[path = "support/acp_session.rs"]
mod acp_session;
#[path = "support/acp_processing.rs"]
mod support;
use acp_session::{configured, frames, options, plan, prompt, reaped, stopped, wait};
use lapis_ai_runtime::{AcpSession, Error};
use std::{sync::atomic::Ordering, time::Duration};
use support::{Behavior, Capture, captured, startup_frames};

async fn entered(control: &support::Control) {
    tokio::time::timeout(Duration::from_secs(5), control.entered.notified())
        .await
        .unwrap();
}
fn limits(root: &std::path::Path) -> lapis_ai_runtime::SessionOptions {
    let mut value = options(root, "managed");
    value.request_timeout = Duration::from_secs(2);
    value
}
#[tokio::test]
async fn startup_activity_facts_and_events_are_processed_before_exposure() {
    let root = tempfile::tempdir().unwrap();
    let (processor, _) = Capture::new(root.path(), Behavior::Record);
    let session = AcpSession::start_with_processor(
        startup_frames(root.path()),
        limits(root.path()),
        processor,
    )
    .await
    .unwrap();
    assert_eq!(session.facts()["session"]["label"], "facts-[redacted]");
    assert_eq!(captured(root.path()).len(), 2);
    assert_eq!(session.snapshot(0).unwrap().events.len(), 1);
    configured(&session).await;
    session
        .submit("turn".into(), prompt("hello"))
        .await
        .unwrap();
    let snapshot = wait(&session, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    let captures = captured(root.path());
    for (index, capture) in captures.iter().enumerate() {
        assert_eq!(capture["ordinal"], index as u64 + 1);
        assert_eq!(capture["sessionId"], "managed");
    }
    for event in &snapshot.events {
        assert!(captures.iter().any(|capture| {
            capture["eventSequence"] == event.sequence && capture["frame"] == event.frame
        }));
    }
    assert!(!serde_json::to_string(&snapshot).unwrap().contains("secret"));
    assert!(
        !std::fs::read_to_string(root.path().join("captured.jsonl"))
            .unwrap()
            .contains("secret")
    );
    session.close().await.unwrap();
}
#[tokio::test]
async fn blocked_capture_cannot_be_observed_or_acknowledged_early() {
    let root = tempfile::tempdir().unwrap();
    let (processor, control) = Capture::new(root.path(), Behavior::BlockActivity);
    let session =
        AcpSession::start_with_processor(plan(root.path()), limits(root.path()), processor)
            .await
            .unwrap();
    configured(&session).await;
    session
        .submit("turn".into(), prompt("hello"))
        .await
        .unwrap();
    entered(&control).await;
    let before = session.snapshot(0).unwrap();
    assert!(
        !before
            .events
            .iter()
            .any(|event| event.frame["type"] == "activity")
    );
    assert!(session.acknowledge(before.sequence + 1).is_err());
    assert!(
        !captured(root.path())
            .iter()
            .any(|value| value["frame"]["type"] == "activity")
    );
    control.release.notify_one();
    let after = wait(&session, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    assert!(
        after
            .events
            .iter()
            .any(|event| event.frame["update"]["text"] == "[redacted reply]")
    );
    assert_eq!(control.abandoned.load(Ordering::SeqCst), 0);
    session.close().await.unwrap();
}
#[tokio::test]
async fn terminal_capture_failure_keeps_the_original_request_unknown_without_raw_fallback() {
    let root = tempfile::tempdir().unwrap();
    let (processor, _) = Capture::new(root.path(), Behavior::FailRecord);
    let session =
        AcpSession::start_with_processor(plan(root.path()), limits(root.path()), processor)
            .await
            .unwrap();
    configured(&session).await;
    session
        .submit("turn".into(), prompt("hello"))
        .await
        .unwrap();
    let snapshot = wait(&session, |s| s.closed).await;
    assert_eq!(snapshot.failure, Some("capture_failed"));
    assert_eq!(snapshot.unresolved_requests, ["turn"]);
    assert!(
        !snapshot
            .events
            .iter()
            .any(|event| event.frame["type"] == "record")
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
async fn processing_cannot_remap_protocol_identity_or_permission_options() {
    for field in [
        "sessionId",
        "requestId",
        "sequence",
        "recordTask",
        "recordStatus",
        "optionId",
        "optionKind",
    ] {
        let root = tempfile::tempdir().unwrap();
        let (processor, _) = Capture::new(root.path(), Behavior::Tamper(field));
        let session =
            AcpSession::start_with_processor(plan(root.path()), limits(root.path()), processor)
                .await
                .unwrap();
        configured(&session).await;
        session
            .submit(
                "turn".into(),
                prompt(if matches!(field, "optionId" | "optionKind") {
                    "permission"
                } else {
                    "hello"
                }),
            )
            .await
            .unwrap();
        let snapshot = wait(&session, |s| s.closed).await;
        assert_eq!(
            snapshot.failure,
            Some("processed_identity_changed"),
            "{field}"
        );
        assert_eq!(snapshot.unresolved_requests, ["turn"]);
        session.close().await.unwrap();
    }
}
#[tokio::test]
async fn startup_cancellation_drains_processing_before_reaping_and_exposes_no_session() {
    let root = tempfile::tempdir().unwrap();
    let (processor, control) = Capture::new(root.path(), Behavior::BlockFacts);
    let (cancel, signal) = tokio::sync::watch::channel(false);
    let task = tokio::spawn(AcpSession::start_cancellable_with_processor(
        startup_frames(root.path()),
        limits(root.path()),
        signal,
        processor,
    ));
    entered(&control).await;
    let pid = frames(root.path())[0]["pid"].as_u64().unwrap() as u32;
    cancel.send_replace(true);
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!task.is_finished());
    assert!(!stopped(pid));
    control.release.notify_one();
    assert_eq!(task.await.unwrap().err(), Some(Error("startup_cancelled")));
    assert_eq!(control.abandoned.load(Ordering::SeqCst), 0);
    assert_eq!(captured(root.path()).len(), 2);
    reaped(pid).await;
}
#[tokio::test]
async fn dropping_the_start_wait_does_not_abandon_inflight_processing_or_the_worker() {
    let root = tempfile::tempdir().unwrap();
    let (processor, control) = Capture::new(root.path(), Behavior::BlockFacts);
    let task = tokio::spawn(AcpSession::start_with_processor(
        startup_frames(root.path()),
        limits(root.path()),
        processor,
    ));
    entered(&control).await;
    let pid = frames(root.path())[0]["pid"].as_u64().unwrap() as u32;
    task.abort();
    assert!(task.await.err().unwrap().is_cancelled());
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!stopped(pid));
    assert_eq!(control.abandoned.load(Ordering::SeqCst), 0);
    control.release.notify_one();
    reaped(pid).await;
    assert_eq!(control.abandoned.load(Ordering::SeqCst), 0);
    assert_eq!(captured(root.path()).len(), 2);
}
#[tokio::test]
async fn close_waits_for_admitted_processing_and_never_replays_an_interrupted_turn() {
    let root = tempfile::tempdir().unwrap();
    let (processor, control) = Capture::new(root.path(), Behavior::BlockActivity);
    let session =
        AcpSession::start_with_processor(plan(root.path()), limits(root.path()), processor)
            .await
            .unwrap();
    configured(&session).await;
    session
        .submit("turn".into(), prompt("hello"))
        .await
        .unwrap();
    entered(&control).await;
    let owner = session.clone();
    let closing = tokio::spawn(async move { owner.close().await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!closing.is_finished());
    control.release.notify_one();
    closing.await.unwrap().unwrap();
    assert_eq!(control.abandoned.load(Ordering::SeqCst), 0);
    assert!(session.snapshot(0).unwrap().closed);
    assert_eq!(session.snapshot(0).unwrap().unresolved_requests, ["turn"]);
    assert_eq!(
        frames(root.path())
            .iter()
            .filter(|frame| frame["type"] == "prompt")
            .count(),
        1
    );
}
#[tokio::test]
async fn transformed_startup_facts_remain_bounded_and_failure_reaps_the_worker() {
    let root = tempfile::tempdir().unwrap();
    let (processor, _) = Capture::new(root.path(), Behavior::OversizeFacts);
    let mut options = limits(root.path());
    options.event_byte_limit = 1024;
    let result = AcpSession::start_with_processor(plan(root.path()), options, processor).await;
    assert_eq!(result.err(), Some(Error("output_coverage_lost")));
    reaped(frames(root.path())[0]["pid"].as_u64().unwrap() as u32).await;
}

#[tokio::test]
async fn startup_processing_cannot_replace_the_provider_identity_or_fall_back() {
    let root = tempfile::tempdir().unwrap();
    let (processor, _) = Capture::new(root.path(), Behavior::Tamper("factsId"));
    let result =
        AcpSession::start_with_processor(plan(root.path()), limits(root.path()), processor).await;
    assert_eq!(result.err(), Some(Error("processed_identity_changed")));
    let sent = frames(root.path());
    assert_eq!(
        sent.iter().filter(|frame| frame["type"] == "open").count(),
        1
    );
    assert!(!sent.iter().any(|frame| frame["type"] == "prompt"));
    reaped(sent[0]["pid"].as_u64().unwrap() as u32).await;
}
