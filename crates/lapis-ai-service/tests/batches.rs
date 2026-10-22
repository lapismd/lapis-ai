#![cfg(unix)]
#[path = "support/batches.rs"]
mod batch;
mod support;
use std::{
    sync::{Arc, Mutex, atomic::Ordering},
    time::Duration,
};
use support::{configure, context, prompt, wait};
#[tokio::test]
async fn native_factory_carries_complete_text_groups_through_channel_and_acknowledgement() {
    let root = tempfile::tempdir().unwrap();
    let (host, _) = batch::setup(root.path());
    let ctx = context("one");
    let info = host.open(&ctx, "open", "fixture", None).await.unwrap();
    configure(&host, &ctx, &info.session_id).await;
    let messages = Arc::new(Mutex::new(Vec::new()));
    let sink = messages.clone();
    let token = host
        .attach(
            &ctx,
            &info.session_id,
            0,
            Arc::new(move |value| {
                sink.lock().unwrap().push(value);
                Ok(())
            }),
        )
        .unwrap();
    host.submit(&ctx, &info.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    let snapshot = wait(&host, &ctx, &info.session_id, |s| {
        s.events.iter().any(|e| e.frame["type"] == "record")
    })
    .await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while !messages
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.snapshot.sequence == snapshot.sequence)
        {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let event = snapshot.events.last().unwrap();
    assert_eq!(event.frames().count(), 2);
    assert_eq!(event.before[0]["update"]["content"]["text"], "safe tail");
    let encoded = serde_json::to_value(&*messages.lock().unwrap()).unwrap();
    assert!(!encoded.to_string().contains("raw reply"));
    assert!(encoded.as_array().unwrap().iter().any(|m| {
        m["snapshot"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["before"].as_array().is_some_and(|b| b.len() == 1))
    }));
    host.acknowledge(&ctx, &info.session_id, &token, snapshot.sequence)
        .unwrap();
    assert!(host.snapshot(&ctx, &info.session_id, 0).is_err());
    host.shutdown().await.unwrap();
    assert!(support::all_stopped(root.path(), "one"));
}
#[tokio::test]
async fn changed_native_scope_rejects_a_blocked_batch_without_affecting_another_window() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = batch::setup(root.path());
    policy.control.block.store(true, Ordering::Release);
    let one = context("one");
    let two = context("two");
    let first = host.open(&one, "open", "fixture", None).await.unwrap();
    let second = host.open(&two, "open", "fixture", None).await.unwrap();
    configure(&host, &one, &first.session_id).await;
    configure(&host, &two, &second.session_id).await;
    host.submit(&one, &first.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), policy.control.entered.notified())
        .await
        .unwrap();
    let before = host.snapshot(&one, &first.session_id, 0).unwrap();
    assert!(before.events.iter().all(|e| e.before.is_empty()));
    policy.base.generation.store(1, Ordering::Release);
    policy.control.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !support::all_stopped(root.path(), "one") {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(host.snapshot(&one, &first.session_id, 0).is_err());
    host.submit(&two, &second.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    let after = wait(&host, &two, &second.session_id, |s| {
        s.events.iter().any(|e| e.frame["type"] == "record")
    })
    .await;
    assert_eq!(after.events.last().unwrap().before.len(), 1);
    host.shutdown().await.unwrap();
}
