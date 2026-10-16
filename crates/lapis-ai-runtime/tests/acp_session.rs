#![cfg(unix)]
#[path = "support/acp_session.rs"]
mod support;
use lapis_ai_runtime::{AcpSession, Error, SessionRequest};
use serde_json::json;
use std::time::Duration;
use support::*;

#[tokio::test]
async fn warm_open_are_lazy_and_launch_policy_never_comes_from_request_data() {
    let root = tempfile::tempdir().unwrap();
    let session = start(root.path(), "opaque-one").await;
    assert_eq!(session.id(), "opaque-one");
    assert_eq!(session.facts()["adapterSessionId"], "provider-opaque-one");
    let log = frames(root.path());
    assert_eq!(log[0]["args"], json!(["serve", "--control-version", "3"]));
    assert_eq!(
        log[0]["root"],
        root.path()
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(log[0]["launchRoot"], log[0]["root"]);
    assert_eq!(
        log.iter().filter(|frame| frame["type"] == "prompt").count(),
        0
    );
    for field in [
        "executable",
        "cwd",
        "environment",
        "tools",
        "sessionId",
        "version",
    ] {
        let mut value = json!({"type":"status"});
        value[field] = json!("override");
        assert!(serde_json::from_value::<SessionRequest>(value).is_err());
    }
    session.close().await.unwrap();
    assert!(session.snapshot(0).unwrap().closed);
    reaped(log[0]["pid"].as_u64().unwrap() as u32).await;
}

#[tokio::test]
async fn mode_selection_is_required_and_authored_text_stays_separate_from_context() {
    let root = tempfile::tempdir().unwrap();
    let session = start(root.path(), "model").await;
    assert_eq!(
        session.submit("early".into(), prompt("hello")).await,
        Err(Error("execution_mode_required"))
    );
    session
        .submit(
            "unsupported".into(),
            SessionRequest::Configure {
                model: Some("unsupported".into()),
                effort: None,
                mode: Some("ask".into()),
            },
        )
        .await
        .unwrap();
    wait(&session, |s| {
        s.events
            .iter()
            .any(|event| event.frame["type"] == "configured")
    })
    .await;
    assert_eq!(
        session.submit("still-early".into(), prompt("hello")).await,
        Err(Error("execution_mode_required"))
    );
    configured(&session).await;
    session
        .submit("turn".into(), prompt("authored"))
        .await
        .unwrap();
    wait(&session, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    let log = frames(root.path());
    let sent = log.iter().find(|frame| frame["type"] == "prompt").unwrap();
    assert_eq!(sent["text"], "authored");
    assert_eq!(sent["context"], "private context");
    let configure = log
        .iter()
        .find(|frame| frame["requestId"] == "configure")
        .unwrap();
    assert_eq!(configure["model"], "fixture-model");
    assert_eq!(configure["effort"], "high");
    session.close().await.unwrap();
}

#[tokio::test]
async fn overlapping_turns_and_request_reuse_are_rejected_before_writing() {
    let root = tempfile::tempdir().unwrap();
    let session = start(root.path(), "busy").await;
    configured(&session).await;
    session.submit("once".into(), prompt("hang")).await.unwrap();
    assert_eq!(
        session.submit("twice".into(), prompt("hello")).await,
        Err(Error("session_busy"))
    );
    assert_eq!(
        session.submit("once".into(), prompt("hello")).await,
        Err(Error("duplicate_request"))
    );
    session
        .submit("status".into(), SessionRequest::Status {})
        .await
        .unwrap();
    session
        .submit(
            "cancel".into(),
            SessionRequest::Cancel {
                target: "once".into(),
            },
        )
        .await
        .unwrap();
    wait(&session, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    session.submit("next".into(), prompt("next")).await.unwrap();
    wait(&session, |s| {
        s.events
            .iter()
            .filter(|event| event.frame["type"] == "record")
            .count()
            == 2
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
        2
    );
    session.close().await.unwrap();
}

#[tokio::test]
async fn exact_permission_choices_reject_semantic_fallback_unknown_and_stale_answers() {
    let root = tempfile::tempdir().unwrap();
    let session = start(root.path(), "permissions").await;
    configured(&session).await;
    session
        .submit("turn".into(), prompt("permission"))
        .await
        .unwrap();
    wait(&session, |s| {
        s.events
            .iter()
            .any(|event| event.frame["type"] == "permission")
    })
    .await;
    for option in ["allow_once", "allow-once", "unknown"] {
        assert_eq!(
            session
                .submit(
                    format!("bad-{option}"),
                    SessionRequest::Permission {
                        permission_id: "permission-exact".into(),
                        option_id: Some(option.into())
                    }
                )
                .await,
            Err(Error("stale_or_invalid_permission"))
        );
    }
    session
        .submit(
            "answer".into(),
            SessionRequest::Permission {
                permission_id: "permission-exact".into(),
                option_id: Some("provider:allow/once".into()),
            },
        )
        .await
        .unwrap();
    wait(&session, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    assert_eq!(
        session
            .submit(
                "stale".into(),
                SessionRequest::Permission {
                    permission_id: "permission-exact".into(),
                    option_id: None
                }
            )
            .await,
        Err(Error("stale_or_invalid_permission"))
    );
    let log = frames(root.path());
    let answers: Vec<_> = log
        .iter()
        .filter(|frame| frame["type"] == "permission")
        .collect();
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0]["optionId"], "provider:allow/once");
    session.close().await.unwrap();
}

#[tokio::test]
async fn detached_consumers_reconnect_in_order_and_acknowledged_gaps_are_explicit() {
    let root = tempfile::tempdir().unwrap();
    let session = start(root.path(), "reconnect").await;
    configured(&session).await;
    let subscriber = session.subscribe().unwrap();
    drop(subscriber);
    session
        .submit("turn".into(), prompt("hello"))
        .await
        .unwrap();
    let snapshot = wait(&session, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    assert_eq!(
        snapshot
            .events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        (1..=snapshot.sequence).collect::<Vec<_>>()
    );
    assert_eq!(session.snapshot(2).unwrap().events[0].sequence, 3);
    assert!(session.acknowledge(snapshot.sequence + 1).is_err());
    session.acknowledge(snapshot.sequence).unwrap();
    assert_eq!(session.snapshot(0).err(), Some(Error("event_coverage_gap")));
    assert!(
        session
            .snapshot(snapshot.sequence)
            .unwrap()
            .events
            .is_empty()
    );
    session.close().await.unwrap();
}

#[tokio::test]
async fn overflow_latches_failure_even_after_retained_events_are_acknowledged() {
    let root = tempfile::tempdir().unwrap();
    let mut limits = options(root.path(), "overflow");
    limits.event_limit = 8;
    let session = AcpSession::start(plan(root.path()), limits).await.unwrap();
    configured(&session).await;
    session
        .submit("turn".into(), prompt("overflow"))
        .await
        .unwrap();
    let snapshot = wait(&session, |s| s.closed).await;
    assert_eq!(snapshot.failure, Some("output_coverage_lost"));
    assert_eq!(snapshot.unresolved_requests, ["turn"]);
    session.acknowledge(snapshot.sequence).unwrap();
    assert_eq!(
        session.snapshot(snapshot.sequence).unwrap().failure,
        Some("output_coverage_lost")
    );
    assert!(
        session
            .submit("retry".into(), prompt("hello"))
            .await
            .is_err()
    );
    let _ = session.close().await;
    reaped(frames(root.path())[0]["pid"].as_u64().unwrap() as u32).await;
}

#[tokio::test]
async fn mismatched_session_request_and_sequence_retire_the_worker() {
    for (input, code) in [
        ("wrong-session", "session_scope_mismatch"),
        ("wrong-request", "uncorrelated_response"),
        ("wrong-order", "event_order_invalid"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let session = start(root.path(), "fenced").await;
        configured(&session).await;
        session.submit("turn".into(), prompt(input)).await.unwrap();
        let snapshot = wait(&session, |s| s.closed).await;
        assert_eq!(snapshot.failure, Some(code));
        assert_eq!(snapshot.unresolved_requests, ["turn"]);
        let _ = session.close().await;
        reaped(frames(root.path())[0]["pid"].as_u64().unwrap() as u32).await;
    }
}

#[tokio::test]
async fn timeouts_and_process_exit_report_unknown_without_prompt_replay() {
    for text in ["hang", "exit"] {
        let root = tempfile::tempdir().unwrap();
        let session = start(root.path(), "unknown").await;
        configured(&session).await;
        session.submit("once".into(), prompt(text)).await.unwrap();
        let snapshot = wait(&session, |s| s.closed).await;
        assert_eq!(
            snapshot.failure,
            Some(if text == "hang" {
                "outcome_unknown"
            } else {
                "helper_output_closed"
            })
        );
        assert_eq!(snapshot.unresolved_requests, ["once"]);
        let _ = session.close().await;
        assert_eq!(
            frames(root.path())
                .iter()
                .filter(|frame| frame["type"] == "prompt")
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn last_owner_drop_and_cancelled_close_wait_do_not_orphan_processes() {
    for cancel_wait in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let session = start(root.path(), "drop").await;
        let pid = frames(root.path())[0]["pid"].as_u64().unwrap() as u32;
        if cancel_wait {
            let mut close = Box::pin(session.close());
            tokio::select! { biased; _ = &mut close => {}, _ = tokio::task::yield_now() => {} }
            drop(close);
        }
        drop(session);
        reaped(pid).await;
    }
}

#[tokio::test]
async fn canonical_paths_block_escape_and_explicit_resume_never_falls_back() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let mut denied = options(root.path(), "escape");
    denied.directory = outside.path().into();
    assert_eq!(
        AcpSession::start(plan(root.path()), denied).await.err(),
        Some(Error("session_directory_denied"))
    );
    std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
    let mut linked = options(root.path(), "escape-link");
    linked.directory = root.path().join("escape");
    assert_eq!(
        AcpSession::start(plan(root.path()), linked).await.err(),
        Some(Error("session_directory_denied"))
    );
    assert!(!root.path().join("frames.jsonl").exists());
    let mut resume = options(root.path(), "resume");
    resume.resume = Some("unsupported".into());
    assert_eq!(
        AcpSession::start(plan(root.path()), resume).await.err(),
        Some(Error("resume_unsupported"))
    );
    assert_eq!(
        frames(root.path())
            .iter()
            .filter(|frame| frame["type"] == "open")
            .count(),
        1
    );
    reaped(frames(root.path())[0]["pid"].as_u64().unwrap() as u32).await;
}

#[tokio::test]
async fn independent_conversations_run_in_distinct_workers_and_do_not_share_shutdown() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let first = start(a.path(), "first").await;
    let second = start(b.path(), "second").await;
    configured(&first).await;
    configured(&second).await;
    first
        .submit("same-id".into(), prompt("hang"))
        .await
        .unwrap();
    second
        .submit("same-id".into(), prompt("hello"))
        .await
        .unwrap();
    wait(&second, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    first.close().await.unwrap();
    assert!(!second.snapshot(0).unwrap().closed);
    assert_ne!(frames(a.path())[0]["pid"], frames(b.path())[0]["pid"]);
    second
        .submit("after-other-close".into(), prompt("hello"))
        .await
        .unwrap();
    wait(&second, |s| {
        s.events
            .iter()
            .filter(|event| event.frame["type"] == "record")
            .count()
            == 2
    })
    .await;
    second.close().await.unwrap();
}

#[tokio::test]
async fn invalid_input_never_dispatches_and_large_requests_do_not_consume_identity() {
    let root = tempfile::tempdir().unwrap();
    let session = start(root.path(), "bounds").await;
    for id in ["", "native-open", "bad\nidentity"] {
        assert_eq!(
            session.submit(id.into(), SessionRequest::Status {}).await,
            Err(Error("invalid_request_identity"))
        );
    }
    assert_eq!(
        session
            .submit("large".into(), prompt(&"x".repeat(1_048_576)))
            .await,
        Err(Error("request_limit"))
    );
    assert_eq!(
        session
            .submit(
                "empty".into(),
                SessionRequest::Prompt {
                    text: " ".into(),
                    context: None,
                    max_duration_ms: 1
                }
            )
            .await,
        Err(Error("invalid_prompt"))
    );
    session
        .submit("large".into(), SessionRequest::Status {})
        .await
        .unwrap();
    wait(&session, |s| {
        s.events.iter().any(|event| event.frame["type"] == "status")
    })
    .await;
    assert!(
        !frames(root.path())
            .iter()
            .any(|frame| frame["type"] == "prompt")
    );
    session.close().await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
}
