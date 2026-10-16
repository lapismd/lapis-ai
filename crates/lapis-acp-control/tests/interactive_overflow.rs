use lapis_acp_control::interactive_handler::Events;
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use tokio::sync::{mpsc, watch};
#[tokio::test]
async fn output_overflow_never_reports_completed_or_reusable_session() {
    let (output, mut receiver) = mpsc::channel(1);
    let (cancel, _) = watch::channel(false);
    let events = Events {
        output,
        cancel,
        sequence: Arc::new(AtomicU64::new(0)),
        coverage_lost: Arc::new(AtomicBool::new(false)),
        scope: None,
        version: 2,
    };
    events.emit("activity", "turn", json!({"update":{}}));
    events.emit("activity", "turn", json!({"update":{}}));
    assert!(events.coverage_lost.load(Ordering::Acquire));
    assert!(*events.cancel.borrow());
    receiver.recv().await.unwrap();
    events.emit(
        "record",
        "turn",
        json!({"record":{"status":"completed"},"quiescent":true,"sessionUsable":true}),
    );
    let terminal = receiver.recv().await.unwrap();
    assert_eq!(terminal["record"]["status"], "outcome_unknown");
    assert_eq!(terminal["record"]["error"], "activity_coverage_lost");
    assert_eq!(terminal["sessionUsable"], false);
}
