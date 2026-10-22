#![cfg(unix)]
mod support;
use lapis_ai_service::{AcpHost, Config, Error};
use std::{sync::Arc, time::Duration};
use support::*;

#[test]
fn attachment_and_last_owner_drop_use_the_owned_runtime_outside_its_context() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let (host, _) = runtime.block_on(async { setup(directory.path()) });
    let caller = context("one");
    let session = runtime
        .block_on(host.open(&caller, "open", "fixture", None))
        .unwrap();
    assert!(tokio::runtime::Handle::try_current().is_err());
    let (send, receive) = std::sync::mpsc::channel();
    let subscription = host
        .attach(
            &caller,
            &session.session_id,
            0,
            Arc::new(move |frame| send.send(frame).map_err(|_| ())),
        )
        .unwrap();
    let frame = receive.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(frame.session_id, session.session_id);
    assert_eq!(frame.subscription_id, subscription);
    host.acknowledge(
        &caller,
        &session.session_id,
        &subscription,
        frame.snapshot.sequence,
    )
    .unwrap();
    drop(host);
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !all_stopped(directory.path(), "one") {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    });
}

#[test]
fn implicit_construction_requires_a_live_runtime_without_launching() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let (host, policy) = runtime.block_on(async { setup(directory.path()) });
    runtime.block_on(host.shutdown()).unwrap();
    drop(host);
    assert_eq!(
        AcpHost::new(policy, Config::default()).err(),
        Some(Error("runtime_unavailable"))
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}
