//! Agent-initiated traffic that arrives while a request is outstanding.
//!
//! An agent that asks for approval, or that calls a method this client does not
//! implement, must be answered rather than ignored: silence would deadlock the
//! turn. These tests prove the answer, and prove that the default answer to an
//! approval request is a refusal.

use std::time::Duration;

use lapis_acp_protocol::budget::TurnBudget;
use lapis_acp_protocol::handler::{RecordingHandler, SessionHandler};
use lapis_acp_protocol::permission::{PermissionDecision, PermissionRequest};
use serde_json::json;

mod common;

use common::{Harness, response};

#[tokio::test]
async fn an_unanswered_permission_request_is_cancelled_not_approved() {
    let mut harness = Harness::new();
    harness
        .send(
            json!({"jsonrpc": "2.0", "id": 42, "method": "session/request_permission",
                     "params": {"sessionId": "s", "options": [
                         {"optionId": "allow", "kind": "allow_once"},
                         {"optionId": "deny", "kind": "reject_once"}]}}),
        )
        .await;
    harness
        .send(response(1, json!({"stopReason": "end_turn"})))
        .await;
    let mut handler = RecordingHandler::default();
    harness
        .connection
        .session_prompt(
            &mut handler,
            "s",
            &["x"],
            TurnBudget::fixed(Duration::from_secs(5)),
        )
        .await
        .expect("prompt");
    assert_eq!(handler.permissions, 1);
    let _request = harness.frame().await;
    let answer = harness.frame().await;
    assert_eq!(answer["id"], json!(42));
    assert_eq!(answer["result"]["outcome"]["outcome"], json!("cancelled"));
    assert!(answer["result"]["outcome"].get("optionId").is_none());
}

/// A handler that approves by semantic kind, the way a caller must.
struct Approving;

impl SessionHandler for Approving {
    fn permission(
        &mut self,
        request: PermissionRequest,
    ) -> impl std::future::Future<Output = PermissionDecision> + Send {
        let decision = match request.option_with_kind("allow_once") {
            Some(option) => PermissionDecision::Select(option.option_id.clone()),
            None => PermissionDecision::Cancel,
        };
        async move { decision }
    }
}

#[tokio::test]
async fn an_explicit_approval_selects_the_option_by_kind() {
    let mut harness = Harness::new();
    harness
        .send(json!({"jsonrpc": "2.0", "id": 5, "method": "session/request_permission",
                     "params": {"options": [{"optionId": "vendor-specific", "kind": "allow_once"}]}}))
        .await;
    harness
        .send(response(1, json!({"stopReason": "end_turn"})))
        .await;
    let mut handler = Approving;
    harness
        .connection
        .session_prompt(
            &mut handler,
            "s",
            &["x"],
            TurnBudget::fixed(Duration::from_secs(5)),
        )
        .await
        .expect("prompt");
    let _request = harness.frame().await;
    let answer = harness.frame().await;
    assert_eq!(
        answer["result"]["outcome"],
        json!({"outcome": "selected", "optionId": "vendor-specific"})
    );
}

#[tokio::test]
async fn an_unimplemented_agent_request_is_answered_not_ignored() {
    let mut harness = Harness::new();
    harness
        .send(
            json!({"jsonrpc": "2.0", "id": 11, "method": "fs/read_text_file",
                     "params": {"path": "/etc/passwd"}}),
        )
        .await;
    harness
        .send(response(1, json!({"stopReason": "end_turn"})))
        .await;
    let mut handler = RecordingHandler::default();
    harness
        .connection
        .session_prompt(
            &mut handler,
            "s",
            &["x"],
            TurnBudget::fixed(Duration::from_secs(5)),
        )
        .await
        .expect("prompt");
    assert_eq!(handler.requests, vec!["fs/read_text_file".to_owned()]);
    let _request = harness.frame().await;
    let refusal = harness.frame().await;
    assert_eq!(refusal["id"], json!(11));
    assert_eq!(refusal["error"]["code"], json!(-32601));
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("fs/read_text_file")
    );
}

#[tokio::test]
async fn a_malformed_frame_is_reported_to_the_handler_and_skipped() {
    let mut harness = Harness::new();
    harness.send(json!({"jsonrpc": "2.0"})).await;
    harness.send(response(1, json!({"ok": true}))).await;
    let mut handler = RecordingHandler::default();
    harness
        .connection
        .call(&mut handler, "initialize", json!({}))
        .await
        .expect("call");
    assert_eq!(handler.malformed, 1);
}
