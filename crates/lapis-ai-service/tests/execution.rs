#![cfg(unix)]
mod support;
use lapis_ai_service::{
    AcpHost, Config, ExecutionBackend, ExecutionFuture, ExecutionSession, HostExecution,
    runtime::{self, PreparedLaunch, SessionOptions, SessionProcessor},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use support::*;
use tokio::sync::watch;

#[derive(Default)]
struct SelectedBackend {
    starts: AtomicUsize,
    refuse: AtomicBool,
    substitute: AtomicBool,
}
impl ExecutionBackend for SelectedBackend {
    fn start(
        &self,
        launch: PreparedLaunch,
        mut options: SessionOptions,
        cancelled: watch::Receiver<bool>,
        processor: Option<Box<dyn SessionProcessor>>,
    ) -> ExecutionFuture<'_, Arc<dyn ExecutionSession>> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            if self.refuse.load(Ordering::SeqCst) {
                return Err(runtime::Error("backend_unavailable"));
            }
            if self.substitute.load(Ordering::SeqCst) {
                options.handle = "substituted".into();
            }
            HostExecution
                .start(launch, options, cancelled, processor)
                .await
        })
    }
}

#[tokio::test]
async fn selected_backend_uses_real_host_sessions_and_supervised_retirement() {
    let root = tempfile::tempdir().unwrap();
    let (original, policy) = setup(root.path());
    original.shutdown().await.unwrap();
    let backend = Arc::new(SelectedBackend::default());
    let host = Arc::new(AcpHost::with_backend(policy, Config::default(), backend.clone()).unwrap());
    let session = host
        .open(&context("one"), "open", "fixture", None)
        .await
        .unwrap();
    assert_eq!(backend.starts.load(Ordering::SeqCst), 1);
    assert_eq!(
        host.list(&context("one")).unwrap()[0].session_id,
        session.session_id
    );
    assert_eq!(host.list(&context("two")).unwrap().len(), 0);
    assert!(
        host.close(&context("two"), &session.session_id)
            .await
            .is_err()
    );
    host.revoke("one", None).await.unwrap();
    assert!(all_stopped(root.path(), "one"));
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn backend_refusal_never_falls_back_to_host_or_replays_the_open() {
    let root = tempfile::tempdir().unwrap();
    let (original, policy) = setup(root.path());
    original.shutdown().await.unwrap();
    let backend = Arc::new(SelectedBackend::default());
    backend.refuse.store(true, Ordering::SeqCst);
    let host = Arc::new(AcpHost::with_backend(policy, Config::default(), backend.clone()).unwrap());
    assert_eq!(
        host.open(&context("one"), "refused", "fixture", None)
            .await
            .err(),
        Some(lapis_ai_service::Error("backend_unavailable"))
    );
    backend.refuse.store(false, Ordering::SeqCst);
    assert_eq!(
        host.open(&context("one"), "refused", "fixture", None)
            .await
            .err(),
        Some(lapis_ai_service::Error("duplicate_request"))
    );
    assert_eq!(backend.starts.load(Ordering::SeqCst), 1);
    assert!(!root.path().join("one/log.jsonl").exists());
    assert!(host.list(&context("one")).unwrap().is_empty());
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn denied_scope_and_target_never_reach_the_execution_backend() {
    let root = tempfile::tempdir().unwrap();
    let (original, policy) = setup(root.path());
    original.shutdown().await.unwrap();
    let backend = Arc::new(SelectedBackend::default());
    let host = Arc::new(AcpHost::with_backend(policy, Config::default(), backend.clone()).unwrap());
    assert!(
        host.open(&context("foreign"), "open", "fixture", None)
            .await
            .is_err()
    );
    assert!(
        host.open(&context("one"), "open", "foreign", None)
            .await
            .is_err()
    );
    assert_eq!(backend.starts.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn substituted_worker_identity_is_retired_before_exposure() {
    let root = tempfile::tempdir().unwrap();
    let (original, policy) = setup(root.path());
    original.shutdown().await.unwrap();
    let backend = Arc::new(SelectedBackend::default());
    backend.substitute.store(true, Ordering::SeqCst);
    let host = Arc::new(AcpHost::with_backend(policy, Config::default(), backend).unwrap());
    assert_eq!(
        host.open(&context("one"), "open", "fixture", None)
            .await
            .err(),
        Some(lapis_ai_service::Error("invalid_execution_identity"))
    );
    assert!(all_stopped(root.path(), "one"));
    assert!(host.list(&context("one")).unwrap().is_empty());
    host.shutdown().await.unwrap();
}
