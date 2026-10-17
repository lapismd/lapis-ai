#![cfg(unix)]
mod support;
use std::{sync::atomic::Ordering, time::Duration};
use support::*;
use tauri_plugin_lapis_ai::{AcpHost, Config, Error};

async fn stopped(root: &std::path::Path) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !all_stopped(root, "one") {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cancelled_close_wait_does_not_let_shutdown_report_success_before_reaping() {
    let dir = tempfile::tempdir().unwrap();
    let (host, policy) = setup(dir.path());
    policy.slow_shutdown.store(true, Ordering::Release);
    let info = host
        .open(&context("one"), "open", "fixture", None)
        .await
        .unwrap();
    let owner = host.clone();
    let closing = tokio::spawn(async move { owner.close(&context("one"), &info.session_id).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while !host.list(&context("one")).unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    closing.abort();
    let _ = closing.await;
    host.shutdown().await.unwrap();
    let reaped_on_return = all_stopped(dir.path(), "one");
    // Even a red run cleans up its delayed peer before failing the assertion.
    stopped(dir.path()).await;
    assert!(
        reaped_on_return,
        "shutdown returned while a previously closing worker was still alive"
    );
}

#[tokio::test]
async fn shutdown_awaits_a_window_revocation_that_is_already_in_progress() {
    let dir = tempfile::tempdir().unwrap();
    let (host, policy) = setup(dir.path());
    policy.slow_shutdown.store(true, Ordering::Release);
    host.open(&context("one"), "open", "fixture", None)
        .await
        .unwrap();
    let owner = host.clone();
    let revoking = tokio::spawn(async move { owner.revoke("one", None).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while !host.list(&context("one")).unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    host.shutdown().await.unwrap();
    let reaped_on_return = all_stopped(dir.path(), "one");
    revoking.await.unwrap().unwrap();
    stopped(dir.path()).await;
    assert!(
        reaped_on_return,
        "shutdown overtook cleanup started by window destruction"
    );
}

#[tokio::test]
async fn retiring_workers_hold_capacity_until_their_processes_are_reaped() {
    let dir = tempfile::tempdir().unwrap();
    let (_, policy) = setup(dir.path());
    policy.slow_shutdown.store(true, Ordering::Release);
    let host = std::sync::Arc::new(
        AcpHost::new(
            policy,
            Config {
                maximum_sessions: 1,
                ..Config::default()
            },
        )
        .unwrap(),
    );
    let info = host
        .open(&context("one"), "first", "fixture", None)
        .await
        .unwrap();
    let owner = host.clone();
    let closing = tokio::spawn(async move { owner.close(&context("one"), &info.session_id).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while !host.list(&context("one")).unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        host.open(&context("one"), "next", "fixture", None)
            .await
            .err(),
        Some(Error("session_limit"))
    );
    closing.await.unwrap().unwrap();
    host.open(&context("one"), "next", "fixture", None)
        .await
        .unwrap();
    host.shutdown().await.unwrap();
    assert!(all_stopped(dir.path(), "one"));
}

#[tokio::test]
async fn scoped_cleanup_failure_remains_observable_without_stopping_another_window() {
    let dir = tempfile::tempdir().unwrap();
    let (host, policy) = setup(dir.path());
    let other = host
        .open(&context("two"), "other", "fixture", None)
        .await
        .unwrap();
    policy.blocked.store(true, Ordering::Release);
    policy.bad_cleanup.store(true, Ordering::Release);
    let owner = host.clone();
    let opening = tokio::spawn(async move {
        owner
            .open(&context("one"), "blocked", "fixture", None)
            .await
    });
    wait_start(dir.path(), "one").await;
    assert_eq!(
        host.revoke("one", None).await,
        Err(Error("cleanup_unknown"))
    );
    assert!(opening.await.unwrap().is_err());
    assert_eq!(
        host.revoke("one", None).await,
        Err(Error("cleanup_unknown"))
    );
    configure(&host, &context("two"), &other.session_id).await;
    host.revoke("two", None).await.unwrap();
    assert!(all_stopped(dir.path(), "two"));
    assert_eq!(host.shutdown().await, Err(Error("cleanup_unknown")));
    assert!(all_stopped(dir.path(), "one"));
}
