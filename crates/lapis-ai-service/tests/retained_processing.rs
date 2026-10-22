#![cfg(unix)]
#[path = "support/processing.rs"]
mod fixture;
#[path = "support/retained_processing.rs"]
mod retained;
mod support;
use fixture::{
    BLOCK_ACTIVITY, BLOCK_FACTS, CAPTURE, CHANGE_IN_FACTORY, captured, entered, stopped,
};
use lapis_ai_service::Error;
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use support::{all_stopped, configure, context, prompt, wait};

#[tokio::test]
async fn retained_capture_survives_renderer_suspension_without_granting_renderer_operations() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = retained::setup(root.path(), BLOCK_ACTIVITY);
    let ctx = context("one");
    let info = host.open(&ctx, "original", "fixture", None).await.unwrap();
    let id = &info.session_id;
    configure(&host, &ctx, id).await;
    let subscription = host.attach(&ctx, id, 0, Arc::new(|_| Ok(()))).unwrap();
    host.submit(&ctx, id, "turn".into(), prompt())
        .await
        .unwrap();
    entered(&policy.inner).await;
    policy.suspended.store(true, Ordering::Release);
    policy.inner.control.release.notify_one();
    retained::terminal_captured(root.path(), id).await;
    assert_eq!(host.targets(&ctx).err(), Some(Error("renderer_suspended")));
    assert_eq!(host.list(&ctx).err(), Some(Error("renderer_suspended")));
    assert_eq!(
        host.snapshot(&ctx, id, 0).err(),
        Some(Error("renderer_suspended"))
    );
    assert_eq!(
        host.attach(&ctx, id, 0, Arc::new(|_| Ok(()))).err(),
        Some(Error("renderer_suspended"))
    );
    assert_eq!(
        host.detach(&ctx, id, &subscription),
        Err(Error("renderer_suspended"))
    );
    assert_eq!(
        host.acknowledge(&ctx, id, &subscription, 0),
        Err(Error("renderer_suspended"))
    );
    assert_eq!(host.close(&ctx, id).await, Err(Error("renderer_suspended")));
    assert_eq!(
        host.submit(&ctx, id, "forbidden".into(), prompt()).await,
        Err(Error("renderer_suspended"))
    );
    assert_eq!(
        host.open(&ctx, "forbidden", "fixture", None).await.err(),
        Some(Error("renderer_suspended"))
    );
    assert_eq!(policy.inner.control.calls.lock().unwrap().len(), 1);
    assert_eq!(retained::prompt_count(root.path()), 1);
    assert!(!all_stopped(root.path(), "one"));
    policy.suspended.store(false, Ordering::Release);
    let snapshot = wait(&host, &ctx, id, |s| {
        s.events.iter().any(|e| e.frame["type"] == "record")
    })
    .await;
    assert!(snapshot.failure.is_none());
    assert!(
        snapshot
            .events
            .iter()
            .any(|e| e.frame["update"]["content"]["text"] == "reply-filtered")
    );
    assert_eq!(host.list(&ctx).unwrap()[0].session_id, *id);
    host.shutdown().await.unwrap();
    assert!(all_stopped(root.path(), "one"));
    assert_eq!(policy.inner.control.abandoned.load(Ordering::Acquire), 0);
}

#[tokio::test]
async fn native_grant_cannot_replace_current_admission_during_factory_or_startup() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = retained::setup(root.path(), CAPTURE);
    policy.suspended.store(true, Ordering::Release);
    assert_eq!(
        host.open(&context("one"), "open", "fixture", None)
            .await
            .err(),
        Some(Error("renderer_suspended"))
    );
    assert!(policy.binding.lock().unwrap().is_none());
    assert!(!root.path().join("one").exists());
    host.shutdown().await.unwrap();

    let root = tempfile::tempdir().unwrap();
    let (host, _) = retained::setup(root.path(), CHANGE_IN_FACTORY);
    assert_eq!(
        host.open(&context("one"), "open", "fixture", None)
            .await
            .err(),
        Some(Error("admission_revoked"))
    );
    assert!(!root.path().join("one/log.jsonl").exists());
    host.shutdown().await.unwrap();

    let root = tempfile::tempdir().unwrap();
    let (host, policy) = retained::setup(root.path(), BLOCK_FACTS);
    let owner = host.clone();
    let opening =
        tokio::spawn(async move { owner.open(&context("one"), "open", "fixture", None).await });
    entered(&policy.inner).await;
    policy.suspended.store(true, Ordering::Release);
    policy.inner.control.release.notify_one();
    assert_eq!(
        opening.await.unwrap().err(),
        Some(Error("admission_revoked"))
    );
    assert!(all_stopped(root.path(), "one"));
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn wrong_native_generation_is_rejected_before_the_callback() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = retained::setup(root.path(), CAPTURE);
    let ctx = context("one");
    let info = host.open(&ctx, "open", "fixture", None).await.unwrap();
    configure(&host, &ctx, &info.session_id).await;
    let count = captured(root.path(), "one", &info.session_id).len();
    policy.foreign_generation.store(true, Ordering::Release);
    host.submit(&ctx, &info.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    stopped(root.path(), "one").await;
    let snapshot = host.snapshot(&ctx, &info.session_id, 0).unwrap();
    assert!(snapshot.closed);
    assert!(snapshot.failure.is_some());
    assert_eq!(captured(root.path(), "one", &info.session_id).len(), count);
    assert!(
        !snapshot
            .events
            .iter()
            .any(|e| e.frame["requestId"] == "turn")
    );
    assert_eq!(retained::prompt_count(root.path()), 1);
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn native_grant_is_rechecked_after_a_drained_callback() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = retained::setup(root.path(), BLOCK_ACTIVITY);
    let ctx = context("one");
    let info = host.open(&ctx, "open", "fixture", None).await.unwrap();
    configure(&host, &ctx, &info.session_id).await;
    host.submit(&ctx, &info.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    entered(&policy.inner).await;
    policy.granted.store(false, Ordering::Release);
    policy.inner.control.release.notify_one();
    stopped(root.path(), "one").await;
    let snapshot = host.snapshot(&ctx, &info.session_id, 0).unwrap();
    assert!(snapshot.closed);
    assert!(
        !snapshot
            .events
            .iter()
            .any(|e| e.frame["type"] == "activity" && e.frame["requestId"] == "turn")
    );
    assert!(
        captured(root.path(), "one", &info.session_id)
            .iter()
            .any(|r| r["frame"]["type"] == "activity" && r["frame"]["requestId"] == "turn")
    );
    assert_eq!(policy.inner.control.abandoned.load(Ordering::Acquire), 0);
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn native_grant_cannot_keep_removed_or_disabled_registry_slots_alive() {
    for disable in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (host, policy) = retained::setup(root.path(), BLOCK_ACTIVITY);
        let ctx = context("one");
        let info = host.open(&ctx, "open", "fixture", None).await.unwrap();
        configure(&host, &ctx, &info.session_id).await;
        host.submit(&ctx, &info.session_id, "turn".into(), prompt())
            .await
            .unwrap();
        entered(&policy.inner).await;
        let owner = host.clone();
        let retiring = tokio::spawn(async move {
            if disable {
                owner.disable().await
            } else {
                owner.revoke("one", Some("vault-0")).await
            }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!retiring.is_finished());
        assert!(!host.list(&ctx).is_ok_and(|sessions| !sessions.is_empty()));
        assert!(!all_stopped(root.path(), "one"));
        policy.inner.control.release.notify_one();
        retiring.await.unwrap().unwrap();
        assert!(all_stopped(root.path(), "one"));
        host.enable().unwrap();
        assert!(host.list(&ctx).unwrap().is_empty());
        assert!(host.snapshot(&ctx, &info.session_id, 0).is_err());
        assert_eq!(policy.inner.control.abandoned.load(Ordering::Acquire), 0);
        assert_eq!(retained::prompt_count(root.path()), 1);
        host.shutdown().await.unwrap();
    }
}
