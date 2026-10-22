#![cfg(unix)]
mod support;
use lapis_ai_service::{Config, Error};
use std::sync::atomic::Ordering;
use support::*;

#[tokio::test]
async fn lazy_host_refuses_untrusted_origins_and_targets_before_any_launch() {
    let dir = tempfile::tempdir().unwrap();
    let (host, _) = setup(dir.path());
    assert_eq!(host.targets(&context("one")).unwrap().len(), 1);
    assert!(host.list(&context("one")).unwrap().is_empty());
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
    let mut untrusted = context("one");
    untrusted.url = "https://evil.invalid".into();
    assert_eq!(host.targets(&untrusted).err(), Some(Error("denied")));
    assert_eq!(
        host.open(&untrusted, "denied", "fixture", None).await.err(),
        Some(Error("denied"))
    );
    assert_eq!(
        host.open(&context("one"), "unknown", "/bin/sh", None)
            .await
            .err(),
        Some(Error("denied"))
    );
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn windows_cannot_observe_submit_close_or_attach_each_others_workers() {
    let dir = tempfile::tempdir().unwrap();
    let (host, _) = setup(dir.path());
    let one = context("one");
    let two = context("two");
    let first = host.open(&one, "open", "fixture", None).await.unwrap();
    let second = host.open(&two, "open", "fixture", None).await.unwrap();
    assert_eq!(host.list(&one).unwrap().len(), 1);
    assert_eq!(host.list(&two).unwrap().len(), 1);
    assert!(host.snapshot(&two, &first.session_id, 0).is_err());
    assert!(
        host.submit(&two, &first.session_id, "bad".into(), prompt())
            .await
            .is_err()
    );
    assert!(host.close(&two, &first.session_id).await.is_err());
    assert!(
        host.attach(&two, &first.session_id, 0, std::sync::Arc::new(|_| Ok(())))
            .is_err()
    );
    host.revoke("one", None).await.unwrap();
    assert!(all_stopped(dir.path(), "one"));
    configure(&host, &two, &second.session_id).await;
    host.submit(&two, &second.session_id, "turn".into(), prompt())
        .await
        .unwrap();
    wait(&host, &two, &second.session_id, |s| {
        s.events.iter().any(|event| event.frame["type"] == "record")
    })
    .await;
    host.shutdown().await.unwrap();
    assert!(all_stopped(dir.path(), "two"));
}

#[tokio::test]
async fn admission_generation_changes_fence_handles_and_scoped_revocation_preserves_new_workers() {
    let dir = tempfile::tempdir().unwrap();
    let (host, policy) = setup(dir.path());
    let ctx = context("one");
    let first = host.open(&ctx, "open", "fixture", None).await.unwrap();
    policy.generation.store(1, Ordering::Release);
    assert!(host.list(&ctx).unwrap().is_empty());
    assert!(host.snapshot(&ctx, &first.session_id, 0).is_err());
    let second = host.open(&ctx, "open", "fixture", None).await.unwrap();
    host.revoke("one", Some("vault-0")).await.unwrap();
    assert_eq!(host.list(&ctx).unwrap()[0].session_id, second.session_id);
    host.shutdown().await.unwrap();
    assert!(all_stopped(dir.path(), "one"));
}

#[tokio::test]
async fn disable_and_reenable_never_resurrect_old_handles_or_repeat_open_requests() {
    let dir = tempfile::tempdir().unwrap();
    let (host, _) = setup(dir.path());
    let ctx = context("one");
    let first = host.open(&ctx, "open", "fixture", None).await.unwrap();
    assert_eq!(
        host.open(&ctx, "open", "fixture", None).await.err(),
        Some(Error("duplicate_request"))
    );
    host.disable().await.unwrap();
    assert!(all_stopped(dir.path(), "one"));
    assert_eq!(host.targets(&ctx).err(), Some(Error("disabled")));
    host.enable().unwrap();
    assert!(host.list(&ctx).unwrap().is_empty());
    assert!(host.snapshot(&ctx, &first.session_id, 0).is_err());
    assert_eq!(
        host.open(&ctx, "open", "fixture", None).await.err(),
        Some(Error("duplicate_request"))
    );
    host.open(&ctx, "new", "fixture", None).await.unwrap();
    host.shutdown().await.unwrap();
    assert!(host.enable().is_err());
}

#[tokio::test]
async fn revocation_cancels_startup_and_waits_for_process_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let (host, policy) = setup(dir.path());
    policy.blocked.store(true, Ordering::Release);
    let owner = host.clone();
    let opening =
        tokio::spawn(async move { owner.open(&context("one"), "open", "fixture", None).await });
    wait_start(dir.path(), "one").await;
    host.revoke("one", Some("vault-0")).await.unwrap();
    assert!(opening.await.unwrap().is_err());
    assert!(all_stopped(dir.path(), "one"));
    assert!(host.list(&context("one")).unwrap().is_empty());
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancelled_open_wait_remains_registry_owned_and_shutdown_drains_startup() {
    let dir = tempfile::tempdir().unwrap();
    let (host, policy) = setup(dir.path());
    policy.blocked.store(true, Ordering::Release);
    let owner = host.clone();
    let opening =
        tokio::spawn(async move { owner.open(&context("one"), "open", "fixture", None).await });
    wait_start(dir.path(), "one").await;
    opening.abort();
    let _ = opening.await;
    host.shutdown().await.unwrap();
    assert!(all_stopped(dir.path(), "one"));
    assert!(host.enable().is_err());
}

#[tokio::test]
async fn invalid_resource_limits_are_rejected_without_startup() {
    let dir = tempfile::tempdir().unwrap();
    let (_, policy) = setup(dir.path());
    let config = Config {
        maximum_sessions: 0,
        ..Config::default()
    };
    assert!(lapis_ai_service::AcpHost::new(policy, config).is_err());
}

#[tokio::test]
async fn lost_open_acknowledgement_is_recoverable_from_the_owned_registry() {
    use std::future::Future;
    let dir = tempfile::tempdir().unwrap();
    let (host, _) = setup(dir.path());
    let ctx = context("one");
    let mut opening = Box::pin(host.open(&ctx, "once", "fixture", None));
    std::future::poll_fn(|cx| {
        assert!(opening.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    drop(opening);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while host.list(&ctx).unwrap().is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let recovered = host.list(&ctx).unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].request_id, "once");
    assert_eq!(
        host.open(&ctx, "once", "fixture", None).await.err(),
        Some(Error("duplicate_request"))
    );
    assert_eq!(
        log(dir.path(), "one")
            .iter()
            .filter(|frame| frame["type"] == "open")
            .count(),
        1
    );
    host.shutdown().await.unwrap();
    assert!(all_stopped(dir.path(), "one"));
}

#[tokio::test]
async fn startup_slots_count_towards_the_limit_and_revocation_releases_capacity() {
    let dir = tempfile::tempdir().unwrap();
    let (_, policy) = setup(dir.path());
    policy.blocked.store(true, Ordering::Release);
    let host = std::sync::Arc::new(
        lapis_ai_service::AcpHost::new(
            policy.clone(),
            Config {
                maximum_sessions: 1,
                ..Config::default()
            },
        )
        .unwrap(),
    );
    let owner = host.clone();
    let opening =
        tokio::spawn(async move { owner.open(&context("one"), "first", "fixture", None).await });
    wait_start(dir.path(), "one").await;
    assert_eq!(
        host.open(&context("two"), "second", "fixture", None)
            .await
            .err(),
        Some(Error("session_limit"))
    );
    host.revoke("one", None).await.unwrap();
    assert!(opening.await.unwrap().is_err());
    policy.blocked.store(false, Ordering::Release);
    host.open(&context("two"), "second", "fixture", None)
        .await
        .unwrap();
    host.shutdown().await.unwrap();
    assert!(all_stopped(dir.path(), "one"));
    assert!(all_stopped(dir.path(), "two"));
}

#[tokio::test]
async fn provider_resume_identity_requires_native_admission_before_launch() {
    let dir = tempfile::tempdir().unwrap();
    let (host, _) = setup(dir.path());
    let result = host
        .open(
            &context("one"),
            "resume",
            "fixture",
            Some("provider-from-another-vault".into()),
        )
        .await;
    host.shutdown().await.unwrap();
    assert_eq!(result.err(), Some(Error("resume_not_admitted")));
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
}

#[tokio::test]
async fn native_resume_binding_is_resolved_only_for_its_admitted_vault() {
    let dir = tempfile::tempdir().unwrap();
    let (host, policy) = setup(dir.path());
    let ctx = context("one");
    let info = host
        .open(&ctx, "resume", "fixture", Some("owned-binding".into()))
        .await
        .unwrap();
    assert_eq!(info.facts["adapterSessionId"], "owned-provider-session");
    let frames = log(dir.path(), "one");
    let open = frames.iter().find(|frame| frame["type"] == "open").unwrap();
    assert_eq!(open["loadSessionId"], "owned-provider-session");
    policy.generation.store(1, Ordering::Release);
    assert_eq!(
        host.open(&ctx, "other-vault", "fixture", Some("owned-binding".into()))
            .await
            .err(),
        Some(Error("resume_not_admitted"))
    );
    host.shutdown().await.unwrap();
    assert!(all_stopped(dir.path(), "one"));
}

#[tokio::test]
async fn startup_cleanup_failure_is_not_reported_as_successful_revocation() {
    let dir = tempfile::tempdir().unwrap();
    let (host, policy) = setup(dir.path());
    policy.blocked.store(true, Ordering::Release);
    policy.bad_cleanup.store(true, Ordering::Release);
    let owner = host.clone();
    let opening =
        tokio::spawn(async move { owner.open(&context("one"), "open", "fixture", None).await });
    wait_start(dir.path(), "one").await;
    let result = host.revoke("one", None).await;
    assert!(opening.await.unwrap().is_err());
    assert!(all_stopped(dir.path(), "one"));
    assert_eq!(result, Err(Error("cleanup_unknown")));
}
