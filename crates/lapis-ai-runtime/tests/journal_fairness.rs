#![cfg(unix)]
#[path = "support/acp_session.rs"]
mod support;
use lapis_ai_runtime::{AcpSession, Error, SessionRequest};
use std::time::Duration;
use support::*;

#[tokio::test]
async fn a_consuming_subscriber_preserves_a_burst_through_a_small_journal() {
    let root = tempfile::tempdir().unwrap();
    let mut limits = options(root.path(), "journal-burst");
    limits.event_limit = 8;
    let session = AcpSession::start(plan(root.path()), limits).await.unwrap();
    configured(&session).await;
    let mut cursor = session.snapshot(0).unwrap().sequence;
    session.acknowledge(cursor).unwrap();
    session
        .submit(
            "turn".into(),
            SessionRequest::Prompt {
                text: "overflow".into(),
                context: None,
                max_duration_ms: 3000,
            },
        )
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(4), async {
        let mut text = Vec::new();
        let mut cursors = Vec::new();
        loop {
            tokio::time::sleep(Duration::from_millis(16)).await;
            let snapshot = session.snapshot(cursor)?;
            if let Some(reason) = snapshot.failure {
                return Err(Error(reason));
            }
            let mut complete = false;
            for event in &snapshot.events {
                cursors.push(event.sequence);
                if event.frame["type"] == "activity" {
                    text.push(event.frame["update"]["text"].as_str().unwrap().to_owned());
                }
                complete |= event.frame["type"] == "record";
            }
            cursor = snapshot.sequence;
            session.acknowledge(cursor)?;
            if complete {
                return Ok((text, cursors));
            }
        }
    })
    .await;
    let cleanup = session.close().await;
    let (text, cursors) = result
        .expect("Consumer stalled")
        .expect("Active consumer lost output");
    cleanup.unwrap();
    assert_eq!(text, (0..80).map(|n| n.to_string()).collect::<Vec<_>>());
    assert!(cursors.windows(2).all(|pair| pair[1] == pair[0] + 1));
    assert_eq!(
        frames(root.path())
            .iter()
            .filter(|frame| frame["type"] == "prompt")
            .count(),
        1
    );
}

#[tokio::test]
async fn close_interrupts_a_full_journal_handoff_and_retains_the_unknown_request() {
    let root = tempfile::tempdir().unwrap();
    let mut limits = options(root.path(), "journal-close");
    limits.event_limit = 8;
    let session = AcpSession::start(plan(root.path()), limits).await.unwrap();
    configured(&session).await;
    session
        .submit("turn".into(), prompt("overflow"))
        .await
        .unwrap();
    wait(&session, |snapshot| snapshot.sequence == 8).await;
    session.close().await.unwrap();
    let snapshot = session.snapshot(0).unwrap();
    assert!(snapshot.closed);
    assert_eq!(snapshot.failure, None);
    assert_eq!(snapshot.unresolved_requests, ["turn"]);
    reaped(frames(root.path())[0]["pid"].as_u64().unwrap() as u32).await;
}

#[tokio::test]
async fn the_original_request_deadline_precedes_the_capacity_handoff_budget() {
    let root = tempfile::tempdir().unwrap();
    let mut limits = options(root.path(), "journal-deadline");
    limits.event_limit = 8;
    limits.request_timeout = Duration::from_millis(60);
    let session = AcpSession::start(plan(root.path()), limits).await.unwrap();
    configured(&session).await;
    session
        .submit(
            "turn".into(),
            SessionRequest::Prompt {
                text: "overflow".into(),
                context: None,
                max_duration_ms: 1,
            },
        )
        .await
        .unwrap();
    let snapshot = wait(&session, |snapshot| snapshot.closed).await;
    let cleanup = session.close().await;
    assert_eq!(snapshot.failure, Some("outcome_unknown"));
    assert_eq!(snapshot.unresolved_requests, ["turn"]);
    cleanup.unwrap();
}
