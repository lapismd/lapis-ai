use lapis_acp_control::interactive_handler::{Events, Handler};
use lapis_acp_protocol::{Connection, TurnBudget};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncWriteExt, duplex, split},
    sync::{mpsc, watch},
};

#[tokio::test]
async fn packed_protocol_updates_reach_an_active_bounded_control_consumer_in_order() {
    let (output, mut receive) = mpsc::channel(8);
    let (cancel, _) = watch::channel(false);
    let lost = Arc::new(AtomicBool::new(false));
    let events = Events {
        output,
        cancel,
        sequence: Arc::new(AtomicU64::new(0)),
        coverage_lost: lost.clone(),
        scope: Some("scope".into()),
        version: 3,
    };
    let mut handler = Handler {
        events,
        request_id: "turn".into(),
        permissions: Arc::new(Mutex::new(HashMap::new())),
        timeout: Duration::from_secs(5),
        pending_rpc: None,
        adapter_session: Some("adapter".into()),
        known_sessions: Default::default(),
    };
    let consumer = tokio::spawn(async move {
        let mut frames = Vec::new();
        while let Some(frame) = receive.recv().await {
            frames.push(frame);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        frames
    });
    let (client, mut adapter) = duplex(256 * 1024);
    let writer = tokio::spawn(async move {
        let mut bytes = Vec::new();
        for n in 0..1000 {
            let mut frame = json!({"jsonrpc":"2.0","method":"session/update","params":{
                "sessionId":"adapter","update":{"sessionUpdate":"agent_message_chunk",
                "content":{"type":"text","text":n.to_string()}}}})
            .to_string()
            .into_bytes();
            frame.push(b'\n');
            bytes.extend(frame);
        }
        bytes.extend(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"stopReason\":\"end_turn\"}}\n");
        adapter.write_all(&bytes).await.unwrap();
        // Keep the response pipe alive until the client finishes.
        adapter
    });
    let (read, write) = split(client);
    let result = Connection::new(read, write)
        .call_within(
            &mut handler,
            "session/prompt",
            json!({"sessionId":"adapter","prompt":[]}),
            TurnBudget::fixed(Duration::from_secs(5)),
        )
        .await;
    drop(handler);
    let frames = consumer.await.unwrap();
    let _adapter = writer.await.unwrap();
    result.unwrap();
    assert!(
        !lost.load(Ordering::Acquire),
        "Active output consumer lost coverage"
    );
    assert_eq!(frames.len(), 1000);
    for (n, frame) in frames.iter().enumerate() {
        assert_eq!(frame["sequence"], n + 1);
        assert_eq!(frame["update"]["content"]["text"], n.to_string());
    }
}

fn one_frame_queue() -> (Events, mpsc::Receiver<serde_json::Value>) {
    let (output, receive) = mpsc::channel(1);
    let (cancel, _) = watch::channel(false);
    (
        Events {
            output,
            cancel,
            sequence: Arc::new(AtomicU64::new(0)),
            coverage_lost: Arc::new(AtomicBool::new(false)),
            scope: Some("scope".into()),
            version: 3,
        },
        receive,
    )
}

#[tokio::test]
async fn stalled_async_output_latches_unknown_and_never_reports_completed() {
    let (events, mut receive) = one_frame_queue();
    events.emit("activity", "turn", json!({"update":{}}));
    events
        .emit_wait("activity", "turn", json!({"update":{}}))
        .await;
    assert!(events.coverage_lost.load(Ordering::Acquire));
    assert!(*events.cancel.borrow());
    receive.recv().await.unwrap();
    events
        .emit_wait(
            "record",
            "turn",
            json!({"record":{"status":"completed"},"sessionUsable":true}),
        )
        .await;
    let terminal = receive.recv().await.unwrap();
    assert_eq!(terminal["record"]["status"], "outcome_unknown");
    assert_eq!(terminal["record"]["error"], "activity_coverage_lost");
    assert_eq!(terminal["sessionUsable"], false);
}

#[tokio::test]
async fn cancellation_of_a_pending_output_handoff_cannot_hide_coverage_loss() {
    let (events, mut receive) = one_frame_queue();
    events.emit("activity", "turn", json!({"update":{}}));
    assert!(
        tokio::time::timeout(
            Duration::from_millis(10),
            events.emit_wait("activity", "turn", json!({"update":{}}))
        )
        .await
        .is_err()
    );
    assert!(events.coverage_lost.load(Ordering::Acquire));
    assert!(*events.cancel.borrow());
    assert_eq!(receive.recv().await.unwrap()["sequence"], 1);
    assert!(receive.try_recv().is_err());
}
