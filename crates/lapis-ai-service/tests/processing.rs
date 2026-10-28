#![cfg(unix)]
#[path = "support/processing.rs"]
mod fixture;
mod support;
use fixture::*;
use lapis_ai_service::{Error, StreamMessage};
use serde_json::Value;
use std::{
    sync::{Arc, Mutex, atomic::Ordering},
    time::Duration,
};
use support::{all_stopped, configure, context, log, prompt, wait};

fn turn_activity(frame: &Value) -> bool {
    frame["type"] == "activity" && frame["requestId"] == "turn"
}
async fn delivered(queue: &Arc<Mutex<Vec<StreamMessage>>>, sequence: u64) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !queue
            .lock()
            .unwrap()
            .iter()
            .any(|value| value.snapshot.sequence >= sequence)
        {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn factory_receives_exact_native_binding_once_and_processes_startup_and_channel_events() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = fixture::setup(root.path(), CAPTURE, 4);
    let ctx = context("one");
    let info = host
        .open(&ctx, "original-open", "fixture", None)
        .await
        .unwrap();
    let binding = policy.control.calls.lock().unwrap()[0].clone();
    assert_eq!(binding["session"], info.session_id);
    assert_eq!(binding["request"], "original-open");
    assert_eq!(binding["target"], "fixture");
    assert_eq!(binding["admission"], "vault-0");
    assert_eq!(binding["window"], "one");
    assert_eq!(binding["url"], "tauri://localhost/");
    assert_eq!(info.facts["session"]["label"], "facts-filtered");
    assert_eq!(captured(root.path(), "one", &info.session_id).len(), 2);
    assert_eq!(
        host.open(&ctx, "original-open", "fixture", None)
            .await
            .err(),
        Some(Error("duplicate_request"))
    );
    assert_eq!(policy.control.calls.lock().unwrap().len(), 1);
    let queue = Arc::new(Mutex::new(Vec::new()));
    let sink = queue.clone();
    host.attach(
        &ctx,
        &info.session_id,
        0,
        Arc::new(move |message| {
            sink.lock().unwrap().push(message);
            Ok(())
        }),
    )
    .unwrap();
    configure(&host, &ctx, &info.session_id).await;
    host.submit(&ctx, &info.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    let snapshot = wait(&host, &ctx, &info.session_id, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    delivered(&queue, snapshot.sequence).await;
    let records = captured(root.path(), "one", &info.session_id);
    for message in queue.lock().unwrap().iter() {
        for event in &message.snapshot.events {
            assert!(records.iter().any(
                |record| record["sequence"] == event.sequence && record["frame"] == event.frame
            ));
            assert!(!event.frame.to_string().contains("secret"));
        }
    }
    host.shutdown().await.unwrap();
    assert!(all_stopped(root.path(), "one"));
}

#[tokio::test]
async fn rejected_factory_never_launches_or_falls_back_and_keeps_request_identity() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = fixture::setup(root.path(), REJECT, 1);
    let ctx = context("one");
    assert_eq!(
        host.open(&ctx, "open", "fixture", None).await.err(),
        Some(Error("processor_unavailable"))
    );
    assert!(!root.path().join("one/log.jsonl").exists());
    assert!(host.list(&ctx).unwrap().is_empty());
    assert_eq!(
        host.open(&ctx, "open", "fixture", None).await.err(),
        Some(Error("duplicate_request"))
    );
    assert_eq!(policy.control.calls.lock().unwrap().len(), 1);
    policy.control.mode.store(CAPTURE, Ordering::Release);
    host.open(&ctx, "fresh", "fixture", None).await.unwrap();
    host.shutdown().await.unwrap();
    assert!(all_stopped(root.path(), "one"));
}

#[tokio::test]
async fn current_admission_is_checked_before_and_after_factory_construction() {
    for mode in [CHANGE_IN_RESOLVE, CHANGE_IN_FACTORY] {
        let root = tempfile::tempdir().unwrap();
        let (host, policy) = fixture::setup(root.path(), mode, 1);
        let ctx = context("one");
        assert_eq!(
            host.open(&ctx, "open", "fixture", None).await.err(),
            Some(Error("admission_revoked"))
        );
        assert!(!root.path().join("one/log.jsonl").exists());
        assert_eq!(
            policy.control.calls.lock().unwrap().len(),
            usize::from(mode == CHANGE_IN_FACTORY)
        );
        policy.control.mode.store(CAPTURE, Ordering::Release);
        host.open(&ctx, "open", "fixture", None).await.unwrap();
        assert_eq!(
            policy.control.calls.lock().unwrap().last().unwrap()["admission"],
            "vault-1"
        );
        host.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn unauthorized_origins_and_targets_never_allocate_a_processor() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = fixture::setup(root.path(), CAPTURE, 1);
    let mut foreign = context("one");
    foreign.url = "https://foreign.invalid".into();
    assert!(host.open(&foreign, "open", "fixture", None).await.is_err());
    assert!(
        host.open(&context("one"), "open", "/bin/sh", None)
            .await
            .is_err()
    );
    assert!(policy.control.calls.lock().unwrap().is_empty());
    assert!(std::fs::read_dir(root.path()).unwrap().next().is_none());
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn startup_revocation_drains_blocked_facts_and_retains_capacity_until_reaping() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = fixture::setup(root.path(), BLOCK_FACTS, 1);
    let owner = host.clone();
    let opening =
        tokio::spawn(async move { owner.open(&context("one"), "open", "fixture", None).await });
    entered(&policy).await;
    assert!(host.list(&context("one")).unwrap().is_empty());
    let owner = host.clone();
    let revoking = tokio::spawn(async move { owner.revoke("one", Some("vault-0")).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!revoking.is_finished());
    assert!(!all_stopped(root.path(), "one"));
    assert_eq!(
        host.open(&context("two"), "other", "fixture", None)
            .await
            .err(),
        Some(Error("session_limit"))
    );
    policy.control.release.notify_one();
    assert_eq!(
        opening.await.unwrap().err(),
        Some(Error("admission_revoked"))
    );
    revoking.await.unwrap().unwrap();
    assert!(all_stopped(root.path(), "one"));
    assert_eq!(policy.control.abandoned.load(Ordering::Acquire), 0);
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn channel_delivery_and_acknowledgement_cannot_overtake_native_processing() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = fixture::setup(root.path(), BLOCK_ACTIVITY, 2);
    let ctx = context("one");
    let info = host.open(&ctx, "open", "fixture", None).await.unwrap();
    let queue = Arc::new(Mutex::new(Vec::new()));
    let sink = queue.clone();
    let subscription = host
        .attach(
            &ctx,
            &info.session_id,
            0,
            Arc::new(move |message| {
                sink.lock().unwrap().push(message);
                Ok(())
            }),
        )
        .unwrap();
    configure(&host, &ctx, &info.session_id).await;
    host.submit(&ctx, &info.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    entered(&policy).await;
    let before = host.snapshot(&ctx, &info.session_id, 0).unwrap();
    assert!(
        !before
            .events
            .iter()
            .any(|event| turn_activity(&event.frame))
    );
    assert_eq!(
        host.acknowledge(&ctx, &info.session_id, &subscription, before.sequence + 1),
        Err(Error("invalid_event_cursor"))
    );
    assert!(
        !captured(root.path(), "one", &info.session_id)
            .iter()
            .any(|record| turn_activity(&record["frame"]))
    );
    policy.control.release.notify_one();
    let after = wait(&host, &ctx, &info.session_id, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    delivered(&queue, after.sequence).await;
    assert!(
        after
            .events
            .iter()
            .any(|event| event.frame["update"]["content"]["text"] == "reply-filtered")
    );
    host.acknowledge(&ctx, &info.session_id, &subscription, after.sequence)
        .unwrap();
    assert_eq!(
        host.snapshot(&ctx, &info.session_id, 0).err(),
        Some(Error("event_coverage_gap"))
    );
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn admission_change_during_processing_blocks_publication_without_stopping_another_window() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = fixture::setup(root.path(), BLOCK_ACTIVITY, 2);
    let one = context("one");
    let two = context("two");
    let first = host.open(&one, "open", "fixture", None).await.unwrap();
    let second = host.open(&two, "open", "fixture", None).await.unwrap();
    configure(&host, &one, &first.session_id).await;
    configure(&host, &two, &second.session_id).await;
    host.submit(&one, &first.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    entered(&policy).await;
    policy.base.generation.store(1, Ordering::Release);
    policy.control.release.notify_one();
    stopped(root.path(), "one").await;
    assert!(host.snapshot(&one, &first.session_id, 0).is_err());
    host.submit(&two, &second.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    wait(&host, &two, &second.session_id, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    host.shutdown().await.unwrap();
    assert!(all_stopped(root.path(), "two"));
}

#[tokio::test]
async fn cancelled_close_wait_cannot_release_processing_before_global_shutdown_drains_it() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = fixture::setup(root.path(), BLOCK_ACTIVITY, 1);
    let ctx = context("one");
    let info = host.open(&ctx, "open", "fixture", None).await.unwrap();
    configure(&host, &ctx, &info.session_id).await;
    host.submit(&ctx, &info.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    entered(&policy).await;
    let owner = host.clone();
    let closing = tokio::spawn(async move { owner.close(&context("one"), &info.session_id).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    closing.abort();
    let _ = closing.await;
    let owner = host.clone();
    let stopping = tokio::spawn(async move { owner.shutdown().await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!stopping.is_finished());
    policy.control.release.notify_one();
    stopping.await.unwrap().unwrap();
    assert_eq!(policy.control.abandoned.load(Ordering::Acquire), 0);
    assert!(all_stopped(root.path(), "one"));
    assert_eq!(
        log(root.path(), "one")
            .iter()
            .filter(|frame| frame["type"] == "prompt")
            .count(),
        1
    );
}

#[tokio::test]
async fn the_scoped_processor_does_not_keep_the_last_host_alive() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = fixture::setup(root.path(), BLOCK_ACTIVITY, 1);
    let ctx = context("one");
    let info = host.open(&ctx, "open", "fixture", None).await.unwrap();
    configure(&host, &ctx, &info.session_id).await;
    host.submit(&ctx, &info.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    entered(&policy).await;
    let weak = Arc::downgrade(&host);
    drop(host);
    assert!(weak.upgrade().is_none());
    assert!(!all_stopped(root.path(), "one"));
    policy.control.release.notify_one();
    stopped(root.path(), "one").await;
    assert_eq!(policy.control.abandoned.load(Ordering::Acquire), 0);
}
