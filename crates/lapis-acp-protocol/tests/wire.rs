//! Request and response exchange: negotiation, session setup, prompting,
//! correlation and cancellation, driven over an in-memory pipe.

use std::time::Duration;

use lapis_acp_protocol::budget::TurnBudget;
use lapis_acp_protocol::handler::RecordingHandler;
use lapis_acp_protocol::initialize::{ClientCapabilities, ClientInfo};
use lapis_acp_protocol::permission::PermissionDecision;
use lapis_acp_protocol::update::SessionUpdate;
use lapis_acp_protocol::{Error, StopReason};
use serde_json::json;

mod common;

use common::{Harness, response, session_request};

#[tokio::test]
async fn initialize_negotiates_and_records_the_selected_version() {
    let mut harness = Harness::new();
    assert_eq!(harness.connection.negotiated_version(), None);
    harness
        .send(response(
            1,
            json!({"protocolVersion": 2, "agentInfo": {"name": "adapter"}}),
        ))
        .await;
    let mut handler = RecordingHandler::default();
    let negotiated = harness
        .connection
        .initialize(
            &mut handler,
            &ClientInfo::lapis_acp(),
            ClientCapabilities::default(),
        )
        .await
        .expect("initialize");
    assert_eq!(negotiated.protocol_version, 2);
    assert_eq!(negotiated.agent_name.as_deref(), Some("adapter"));
    assert_eq!(harness.connection.negotiated_version(), Some(2));
    let request = harness.frame().await;
    assert_eq!(request["method"], json!("initialize"));
    assert_eq!(request["id"], json!(1));
    assert_eq!(request["params"]["protocolVersion"], json!(2));
}

#[tokio::test]
async fn initialize_rejects_a_version_the_client_cannot_speak() {
    let mut harness = Harness::new();
    harness
        .send(response(1, json!({"protocolVersion": 7})))
        .await;
    let mut handler = RecordingHandler::default();
    let error = harness
        .connection
        .initialize(
            &mut handler,
            &ClientInfo::lapis_acp(),
            ClientCapabilities::default(),
        )
        .await
        .expect_err("version 7 must be refused");
    assert!(matches!(error, Error::Protocol(_)), "{error}");
    assert_eq!(harness.connection.negotiated_version(), None);
}

#[tokio::test]
async fn session_new_returns_the_identifier_and_retains_the_raw_result() {
    let mut harness = Harness::new();
    harness
        .send(response(
            1,
            json!({"sessionId": "s-9", "configOptions": [{"category": "model"}]}),
        ))
        .await;
    let mut handler = RecordingHandler::default();
    let request = session_request();
    let session = harness
        .connection
        .session_new(&mut handler, &request)
        .await
        .expect("session/new");
    assert_eq!(session.id, "s-9");
    assert_eq!(session.raw["configOptions"].as_array().unwrap().len(), 1);
    let request = harness.frame().await;
    assert_eq!(request["method"], json!("session/new"));
    assert_eq!(request["params"]["cwd"], json!("/work"));
    assert_eq!(request["params"]["systemPrompt"], json!("standing"));
}

#[tokio::test]
async fn a_prompt_streams_updates_and_then_returns_one_stop_reason() {
    let mut harness = Harness::new();
    harness
        .send(
            json!({"jsonrpc": "2.0", "method": "session/update", "params": {
            "sessionId": "s", "update": {"sessionUpdate": "agent_thought_chunk",
                                          "content": {"type": "text", "text": "thinking"}}}}),
        )
        .await;
    harness
        .send(
            json!({"jsonrpc": "2.0", "method": "session/update", "params": {
            "sessionId": "s", "update": {"sessionUpdate": "tool_call",
                                          "toolCallId": "t1", "title": "read", "kind": "read"}}}),
        )
        .await;
    harness
        .send(
            json!({"jsonrpc": "2.0", "method": "session/update", "params": {
            "sessionId": "s", "update": {"sessionUpdate": "agent_message_chunk",
                                          "content": {"type": "text", "text": "answer"}}}}),
        )
        .await;
    harness
        .send(response(1, json!({"stopReason": "end_turn"})))
        .await;

    let mut handler = RecordingHandler::default();
    let reason = harness
        .connection
        .session_prompt(
            &mut handler,
            "s",
            &["do the work"],
            TurnBudget::fixed(Duration::from_secs(5)),
        )
        .await
        .expect("prompt");
    assert_eq!(reason, StopReason::EndTurn);
    assert_eq!(handler.updates.len(), 3);
    assert!(matches!(
        handler.updates[0],
        SessionUpdate::AgentThoughtChunk { .. }
    ));
    assert!(handler.updates[1].is_activity());
    assert_eq!(
        handler.updates[2],
        SessionUpdate::AgentMessageChunk {
            text: Some("answer".to_owned()),
            raw: json!({"sessionUpdate": "agent_message_chunk",
                        "content": {"type": "text", "text": "answer"}})
        }
    );
    let request = harness.frame().await;
    assert_eq!(request["method"], json!("session/prompt"));
    assert_eq!(request["params"]["prompt"][0]["text"], json!("do the work"));
}

#[tokio::test]
async fn an_unrecognized_stop_reason_fails_rather_than_defaulting() {
    let mut harness = Harness::new();
    harness
        .send(response(1, json!({"stopReason": "finished"})))
        .await;
    let mut handler = RecordingHandler::default();
    let error = harness
        .connection
        .session_prompt(
            &mut handler,
            "s",
            &["x"],
            TurnBudget::fixed(Duration::from_secs(5)),
        )
        .await
        .expect_err("unknown stop reason");
    assert!(matches!(error, Error::Protocol(_)), "{error}");
}

#[tokio::test]
async fn cancellation_is_a_notification_and_carries_no_id() {
    let mut harness = Harness::new();
    harness
        .connection
        .session_cancel("s-1")
        .await
        .expect("cancel");
    let frame = harness.frame().await;
    assert_eq!(frame["method"], json!("session/cancel"));
    assert_eq!(frame["params"], json!({"sessionId": "s-1"}));
    assert!(
        frame.get("id").is_none(),
        "a notification must not carry an id"
    );
}

#[tokio::test]
async fn request_ids_increase_and_stay_correlated() {
    let mut harness = Harness::new();
    harness.send(response(1, json!({"a": 1}))).await;
    harness.send(response(2, json!({"b": 2}))).await;
    let mut handler = RecordingHandler::default();
    let first = harness
        .connection
        .call(&mut handler, "session/set_model", json!({}))
        .await
        .unwrap();
    let second = harness
        .connection
        .session_set_config_option(&mut handler, "s", "effort", "high")
        .await
        .unwrap();
    assert_eq!(first, json!({"a": 1}));
    assert_eq!(second, json!({"b": 2}));
    assert_eq!(harness.frame().await["id"], json!(1));
    let second_frame = harness.frame().await;
    assert_eq!(second_frame["id"], json!(2));
    assert_eq!(second_frame["method"], json!("session/set_config_option"));
    assert_eq!(
        second_frame["params"],
        json!({"sessionId": "s", "configId": "effort", "value": "high"})
    );
}

#[tokio::test]
async fn a_deferred_permission_answer_can_be_sent_outside_the_read_loop() {
    let mut harness = Harness::new();
    let id = json!("perm-1");
    harness
        .connection
        .answer_permission(&id, &PermissionDecision::Select("allow".to_owned()))
        .await
        .expect("answer");
    let frame = harness.frame().await;
    assert_eq!(frame["id"], id);
    assert_eq!(
        frame["result"]["outcome"],
        json!({"outcome": "selected", "optionId": "allow"})
    );
}

#[tokio::test]
async fn a_response_to_an_abandoned_request_never_satisfies_a_later_one() {
    let mut harness = Harness::new();
    harness.send(response(99, json!({"stale": true}))).await;
    harness.send(response(1, json!({"fresh": true}))).await;
    let mut handler = RecordingHandler::default();
    let result = harness
        .connection
        .call(&mut handler, "session/set_model", json!({}))
        .await
        .expect("call");
    assert_eq!(result, json!({"fresh": true}));
}

#[tokio::test]
async fn an_agent_error_response_carries_its_code_to_the_caller() {
    let mut harness = Harness::new();
    harness
        .send(
            json!({"jsonrpc": "2.0", "id": 1, "error": {"code": -32602, "message": "bad params"}}),
        )
        .await;
    let mut handler = RecordingHandler::default();
    let error = harness
        .connection
        .call(&mut handler, "session/new", json!({}))
        .await
        .expect_err("agent error");
    match error {
        Error::AgentError { code, message } => {
            assert_eq!(code, -32602);
            assert_eq!(message, "bad params");
        }
        other => panic!("unexpected {other:?}"),
    }
}
