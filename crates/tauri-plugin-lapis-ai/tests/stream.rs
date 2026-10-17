#![cfg(unix)]
mod support;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use support::*;
use tauri_plugin_lapis_ai::{Error, StreamMessage};

async fn received(
    queue: &Arc<Mutex<Vec<StreamMessage>>>,
    predicate: impl Fn(&StreamMessage) -> bool,
) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if queue.lock().unwrap().iter().any(&predicate) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn reconnect_retains_events_and_only_the_current_subscription_can_acknowledge() {
    let dir = tempfile::tempdir().unwrap();
    let (host, _) = setup(dir.path());
    let ctx = context("one");
    let info = host.open(&ctx, "open", "fixture", None).await.unwrap();
    let id = &info.session_id;
    configure(&host, &ctx, id).await;
    let first = Arc::new(Mutex::new(Vec::new()));
    let sink = first.clone();
    let token = host
        .attach(
            &ctx,
            id,
            0,
            Arc::new(move |frame| {
                sink.lock().unwrap().push(frame);
                Ok(())
            }),
        )
        .unwrap();
    received(&first, |frame| !frame.snapshot.events.is_empty()).await;
    host.detach(&ctx, id, &token).unwrap();
    host.submit(&ctx, id, "turn".into(), prompt())
        .await
        .unwrap();
    let before = wait(&host, &ctx, id, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    assert_eq!(
        host.acknowledge(&ctx, id, &token, before.sequence),
        Err(Error("stale_subscription"))
    );
    let second = Arc::new(Mutex::new(Vec::new()));
    let sink = second.clone();
    let current = host
        .attach(
            &ctx,
            id,
            0,
            Arc::new(move |frame| {
                sink.lock().unwrap().push(frame);
                Ok(())
            }),
        )
        .unwrap();
    received(&second, |frame| {
        frame
            .snapshot
            .events
            .iter()
            .any(|event| event.frame["type"] == "record")
    })
    .await;
    assert_eq!(
        host.acknowledge(&ctx, id, &current, before.sequence + 1),
        Err(Error("invalid_event_cursor"))
    );
    host.acknowledge(&ctx, id, &current, before.sequence)
        .unwrap();
    assert_eq!(
        host.snapshot(&ctx, id, 0).err(),
        Some(Error("event_coverage_gap"))
    );
    assert!(
        host.snapshot(&ctx, id, before.sequence)
            .unwrap()
            .events
            .is_empty()
    );
    host.shutdown().await.unwrap();
    assert!(all_stopped(dir.path(), "one"));
}

#[tokio::test]
async fn replacing_or_failing_a_channel_does_not_stop_the_worker() {
    let dir = tempfile::tempdir().unwrap();
    let (host, _) = setup(dir.path());
    let ctx = context("one");
    let info = host.open(&ctx, "open", "fixture", None).await.unwrap();
    let id = &info.session_id;
    let first = host.attach(&ctx, id, 0, Arc::new(|_| Ok(()))).unwrap();
    let replacement = host.attach(&ctx, id, 0, Arc::new(|_| Err(()))).unwrap();
    assert_ne!(first, replacement);
    assert_eq!(
        host.detach(&ctx, id, &first),
        Err(Error("stale_subscription"))
    );
    configure(&host, &ctx, id).await;
    host.submit(&ctx, id, "turn".into(), prompt())
        .await
        .unwrap();
    let snapshot = wait(&host, &ctx, id, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    assert!(!snapshot.closed);
    let queue = Arc::new(Mutex::new(Vec::new()));
    let sink = queue.clone();
    host.attach(
        &ctx,
        id,
        0,
        Arc::new(move |frame| {
            sink.lock().unwrap().push(frame);
            Ok(())
        }),
    )
    .unwrap();
    received(&queue, |frame| frame.snapshot.sequence == snapshot.sequence).await;
    host.shutdown().await.unwrap();
}
