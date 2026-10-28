mod support;
use lapis_ai_app_tools::Config;
use serde_json::json;
use std::{sync::atomic::Ordering, time::Duration};
use support::Session;

#[tokio::test]
async fn inflight_completion_does_not_discard_a_partial_next_input_frame() {
    let mut session = Session::new(Config::default());
    session.initialize().await;
    session.handler.blocked.store(true, Ordering::SeqCst);
    session
        .send(json!("read"), "tools/call", json!({"name":"notes.read"}))
        .await;
    session.handler.entered.notified().await;
    session
        .raw(br#"{"jsonrpc":"2.0","id":"next","method":"pi"#)
        .await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    session.handler.release.add_permits(1);
    assert_eq!(session.next().await["id"], "read");
    session.raw(b"ng\"}\n").await;
    assert_eq!(session.next().await["id"], "next");
    session.stop().await.unwrap();
}

#[tokio::test]
async fn cancel_exact_call_preserves_other_work_and_reports_unknown_effects() {
    let mut session = Session::new(Config::default());
    session.initialize().await;
    session.handler.blocked.store(true, Ordering::SeqCst);
    session
        .send(json!("held"), "tools/call", json!({"name":"notes.read"}))
        .await;
    session.handler.entered.notified().await;
    session
        .send(
            json!("retained"),
            "tools/call",
            json!({"name":"notes.read"}),
        )
        .await;
    session.handler.entered.notified().await;
    session
        .notify("notifications/cancelled", json!({"requestId":"other"}))
        .await;
    session.send(json!("alive"), "ping", json!({})).await;
    assert_eq!(session.next().await["id"], "alive");
    assert_eq!(session.handler.active.load(Ordering::SeqCst), 2);
    session
        .notify("notifications/cancelled", json!({"requestId":"held"}))
        .await;
    let response = session.next().await;
    assert_eq!(response["id"], "held");
    assert_eq!(response["result"]["_meta"]["lapis.md/outcome"], "unknown");
    assert!(session.handler.cancellation_seen.load(Ordering::SeqCst));
    assert_eq!(session.handler.active.load(Ordering::SeqCst), 1);
    session.handler.release.add_permits(1);
    let retained = session.next().await;
    assert_eq!(retained["id"], "retained");
    assert_eq!(retained["result"]["content"][0]["text"], "Captured note");
    session.stop().await.unwrap();
}

#[tokio::test]
async fn capacity_refuses_without_queue_and_deadline_never_claims_no_effect() {
    let mut session = Session::new(Config {
        maximum_calls: 1,
        call_timeout: Duration::from_millis(150),
        ..Config::default()
    });
    session.initialize().await;
    session.handler.blocked.store(true, Ordering::SeqCst);
    session
        .send(json!("first"), "tools/call", json!({"name":"notes.read"}))
        .await;
    session.handler.entered.notified().await;
    session
        .send(json!("second"), "tools/call", json!({"name":"notes.read"}))
        .await;
    assert_eq!(
        session.next().await["error"]["message"],
        "Tool bridge is at capacity"
    );
    let expired = session.next().await;
    assert_eq!(expired["id"], "first");
    assert_eq!(expired["result"]["_meta"]["lapis.md/outcome"], "unknown");
    assert_eq!(session.handler.calls.lock().unwrap().len(), 1);
    assert_eq!(session.handler.active.load(Ordering::SeqCst), 0);
    assert!(session.handler.cancellation_seen.load(Ordering::SeqCst));
    session.stop().await.unwrap();
}

#[tokio::test]
async fn eof_and_owner_drop_release_all_owned_call_futures() {
    for abort in [false, true] {
        let mut session = Session::new(Config::default());
        session.initialize().await;
        session.handler.blocked.store(true, Ordering::SeqCst);
        session
            .send(json!("active"), "tools/call", json!({"name":"notes.read"}))
            .await;
        session.handler.entered.notified().await;
        let handler = session.handler.clone();
        if abort {
            session.task.abort();
            assert!(session.task.await.unwrap_err().is_cancelled());
        } else {
            session.stop().await.unwrap();
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while handler.active.load(Ordering::SeqCst) != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(handler.dropped.load(Ordering::SeqCst), 1);
        assert!(handler.cancellation_seen.load(Ordering::SeqCst));
    }
}

use std::{
    pin::Pin,
    sync::{Arc, atomic::AtomicBool},
    task::{Context, Poll},
};
use tokio::io::{AsyncWrite, AsyncWriteExt};
struct Blocked(Arc<AtomicBool>);
impl AsyncWrite for Blocked {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Pending
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Pending
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Pending
    }
}
impl Drop for Blocked {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn stalled_output_does_not_hold_call_or_eof_cleanup() {
    let (mut input, reader) = tokio::io::duplex(4096);
    let dropped = Arc::new(AtomicBool::new(false));
    let handler = Arc::new(support::Handler::default());
    handler.blocked.store(true, Ordering::SeqCst);
    let task = tokio::spawn(lapis_ai_app_tools::serve(
        reader,
        Blocked(dropped.clone()),
        handler.clone(),
        Config::default(),
    ));
    input.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":\"init\",\"method\":\"initialize\"}\n{\"jsonrpc\":\"2.0\",\"id\":\"call\",\"method\":\"tools/call\",\"params\":{\"name\":\"notes.read\"}}\n").await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), handler.entered.notified())
        .await
        .unwrap();
    input.shutdown().await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(handler.active.load(Ordering::SeqCst), 0);
    assert!(handler.cancellation_seen.load(Ordering::SeqCst));
    assert!(dropped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn stalled_output_capacity_retires_without_queuing_more_calls() {
    let (mut input, reader) = tokio::io::duplex(4096);
    let dropped = Arc::new(AtomicBool::new(false));
    let handler = Arc::new(support::Handler::default());
    handler.blocked.store(true, Ordering::SeqCst);
    let task = tokio::spawn(lapis_ai_app_tools::serve(
        reader,
        Blocked(dropped.clone()),
        handler.clone(),
        Config {
            maximum_calls: 1,
            ..Config::default()
        },
    ));
    input.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":\"init\",\"method\":\"initialize\"}\n{\"jsonrpc\":\"2.0\",\"id\":\"call\",\"method\":\"tools/call\",\"params\":{\"name\":\"notes.read\"}}\n").await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), handler.entered.notified())
        .await
        .unwrap();
    let burst = (0..8)
        .map(|id| format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"ping\"}}\n"))
        .collect::<String>();
    input.write_all(burst.as_bytes()).await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap(),
        Err(lapis_ai_app_tools::Error::OutputCapacity)
    ));
    assert_eq!(handler.calls.lock().unwrap().len(), 1);
    assert_eq!(handler.active.load(Ordering::SeqCst), 0);
    assert!(handler.cancellation_seen.load(Ordering::SeqCst));
    assert!(dropped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn stalled_output_deadline_is_observable_and_releases_the_writer() {
    let (mut input, reader) = tokio::io::duplex(4096);
    let dropped = Arc::new(AtomicBool::new(false));
    let task = tokio::spawn(lapis_ai_app_tools::serve(
        reader,
        Blocked(dropped.clone()),
        Arc::new(support::Handler::default()),
        Config {
            call_timeout: Duration::from_millis(50),
            ..Config::default()
        },
    ));
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":\"init\",\"method\":\"initialize\"}\n")
        .await
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap(),
        Err(lapis_ai_app_tools::Error::OutputTimeout)
    ));
    assert!(dropped.load(Ordering::SeqCst));
}
