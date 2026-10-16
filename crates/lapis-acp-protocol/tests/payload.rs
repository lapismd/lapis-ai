//! Payload construction and parsing, exercised without a stream.

use lapis_acp_protocol::config_option::{self, ModelSwitch};
use lapis_acp_protocol::content::{EnvVar, McpServer};
use lapis_acp_protocol::envelope::{self, Inbound};
use lapis_acp_protocol::error::{DEFAULT_AGENT_ERROR_CODE, agent_error};
use lapis_acp_protocol::initialize::{self, ClientCapabilities, ClientInfo};
use lapis_acp_protocol::permission::{PermissionDecision, PermissionRequest};
use lapis_acp_protocol::session::{self, SystemPrompt, SystemPromptTransport};
use lapis_acp_protocol::update::SessionUpdate;
use lapis_acp_protocol::{Error, StopReason};
use serde_json::json;

#[test]
fn stop_reasons_round_trip_through_the_wire_spelling() {
    for reason in [
        StopReason::EndTurn,
        StopReason::Cancelled,
        StopReason::MaxTokens,
        StopReason::MaxTurnRequests,
        StopReason::Refusal,
    ] {
        assert_eq!(StopReason::parse(reason.as_str()), Some(reason));
    }
}

#[test]
fn stop_reason_parsing_tolerates_agent_casing() {
    assert_eq!(StopReason::parse("END_TURN"), Some(StopReason::EndTurn));
    assert_eq!(StopReason::parse("Cancelled"), Some(StopReason::Cancelled));
    assert_eq!(
        StopReason::parse("max_turn_requests"),
        Some(StopReason::MaxTurnRequests)
    );
    assert_eq!(StopReason::parse("done"), None);
    assert_eq!(StopReason::parse(""), None);
}

#[test]
fn a_prompt_result_must_carry_a_known_stop_reason() {
    assert_eq!(
        session::parse_stop_reason(&json!({"stopReason": "end_turn"})).unwrap(),
        StopReason::EndTurn
    );
    assert!(matches!(
        session::parse_stop_reason(&json!({})),
        Err(Error::Protocol(_))
    ));
    assert!(matches!(
        session::parse_stop_reason(&json!({"stopReason": "finished"})),
        Err(Error::Protocol(_))
    ));
}

#[test]
fn initialize_requests_version_two_and_advertises_no_client_facilities() {
    let params = initialize::params(&ClientInfo::lapis_acp(), ClientCapabilities::default());
    assert_eq!(params["protocolVersion"], json!(2));
    assert_eq!(params["clientInfo"]["name"], json!("lapis-acp"));
    assert_eq!(params["clientCapabilities"]["terminal"], json!(false));
    assert_eq!(
        params["clientCapabilities"]["fs"]["readTextFile"],
        json!(false)
    );
    assert_eq!(
        params["clientCapabilities"]["fs"]["writeTextFile"],
        json!(false)
    );
}

#[test]
fn initialize_accepts_a_version_one_agent_and_rejects_an_unknown_one() {
    let one = initialize::parse_result(&json!({"protocolVersion": 1})).unwrap();
    assert_eq!(one.protocol_version, 1);
    assert!(initialize::parse_result(&json!({"protocolVersion": 2})).is_ok());
    assert!(matches!(
        initialize::parse_result(&json!({"protocolVersion": 3})),
        Err(Error::Protocol(_))
    ));
    assert!(matches!(
        initialize::parse_result(&json!({})),
        Err(Error::Protocol(_))
    ));
}

#[test]
fn capability_probing_treats_an_absent_path_as_false() {
    let result = initialize::parse_result(&json!({
        "protocolVersion": 2,
        "agentInfo": {"name": "adapter", "version": "1.2.3"},
        "agentCapabilities": {"loadSession": true, "promptCapabilities": {"image": true}},
    }))
    .unwrap();
    assert_eq!(result.agent_name.as_deref(), Some("adapter"));
    assert_eq!(result.agent_version.as_deref(), Some("1.2.3"));
    assert!(result.advertises(&["loadSession"]));
    assert!(result.advertises(&["promptCapabilities", "image"]));
    assert!(!result.advertises(&["promptCapabilities", "audio"]));
    assert!(!result.advertises(&["missing"]));
}

#[test]
fn an_explicit_false_capability_is_not_advertised() {
    // Adapters send explicit false for facilities they do not support. Presence
    // is not advertisement, and treating it as such would send content the agent
    // has already refused.
    let result = initialize::parse_result(&json!({
        "protocolVersion": 2,
        "agentCapabilities": {
            "promptCapabilities": {"image": true, "audio": false, "embeddedContext": false},
            "loadSession": false,
            "mcpCapabilities": {"http": true, "stdio": false},
        },
    }))
    .unwrap();
    assert!(result.advertises(&["promptCapabilities", "image"]));
    assert!(!result.advertises(&["promptCapabilities", "audio"]));
    assert!(!result.advertises(&["promptCapabilities", "embeddedContext"]));
    assert!(!result.advertises(&["loadSession"]));
    assert!(result.advertises(&["mcpCapabilities", "http"]));
    assert!(!result.advertises(&["mcpCapabilities", "stdio"]));
    assert!(!result.advertises(&["mcpCapabilities", "sse"]));
}

#[test]
fn mcp_servers_always_serialize_all_four_schema_fields() {
    let server = McpServer::new("tools", "/bin/echo")
        .arg("--json")
        .env("TOKEN", "value");
    let value = json!([server]);
    assert_eq!(
        value,
        json!([{
            "name": "tools",
            "command": "/bin/echo",
            "args": ["--json"],
            "env": [{"name": "TOKEN", "value": "value"}],
        }])
    );
    let bare = json!([McpServer::new("empty", "cmd")]);
    assert_eq!(bare[0]["args"], json!([]));
    assert_eq!(bare[0]["env"], json!([]));
    assert_eq!(EnvVar::new("a", "b").name, "a");
}

#[test]
fn system_prompt_transport_decides_placement_without_clobbering_meta() {
    let base = |transport| session::SessionNewRequest {
        cwd: "/work",
        mcp_servers: vec![],
        system_prompt: Some(SystemPrompt {
            transport,
            text: "standing",
        }),
        session_title: Some("title"),
    };
    let field = session::new_params(&base(SystemPromptTransport::Field));
    assert_eq!(field["systemPrompt"], json!("standing"));
    assert_eq!(field["_meta"]["sessionTitle"], json!("title"));
    assert!(field["_meta"].get("systemPrompt").is_none());

    let meta = session::new_params(&base(SystemPromptTransport::Meta));
    assert_eq!(meta["_meta"]["systemPrompt"], json!("standing"));
    assert_eq!(meta["_meta"]["sessionTitle"], json!("title"));
    assert!(meta.get("systemPrompt").is_none());

    let append = session::new_params(&base(SystemPromptTransport::MetaAppend));
    assert_eq!(
        append["_meta"]["systemPrompt"],
        json!({"append": "standing"})
    );
    assert_eq!(append["_meta"]["sessionTitle"], json!("title"));
}

#[test]
fn a_session_request_omits_meta_entirely_when_nothing_needs_it() {
    let request = session::SessionNewRequest {
        cwd: "/work",
        mcp_servers: vec![],
        system_prompt: None,
        session_title: None,
    };
    let params = session::new_params(&request);
    assert_eq!(params["cwd"], json!("/work"));
    assert_eq!(params["mcpServers"], json!([]));
    assert!(params.get("_meta").is_none());
}

#[test]
fn prompt_and_cancel_params_match_the_wire_contract() {
    assert_eq!(
        session::prompt_text_params("s-1", &["first", "second"]),
        json!({
            "sessionId": "s-1",
            "prompt": [{"type": "text", "text": "first"}, {"type": "text", "text": "second"}],
        })
    );
    assert_eq!(session::cancel_params("s-1"), json!({"sessionId": "s-1"}));
}

#[test]
fn session_new_requires_a_string_identifier() {
    assert_eq!(
        session::parse_new_result(&json!({"sessionId": "abc"}))
            .unwrap()
            .id,
        "abc"
    );
    assert!(matches!(
        session::parse_new_result(&json!({})),
        Err(Error::Protocol(_))
    ));
    assert!(matches!(
        session::parse_new_result(&json!({"sessionId": 7})),
        Err(Error::Protocol(_))
    ));
}

#[test]
fn model_switching_prefers_the_stable_config_option_surface() {
    let stable = json!({
        "sessionId": "s",
        "configOptions": [
            {"category": "model", "configId": "model", "value": "old",
             "options": [{"value": "old"}, {"value": "chosen"}]},
            {"category": "thought_level", "id": "effort", "value": "low"},
        ],
        "models": {"currentModelId": "old", "availableModels": [{"modelId": "chosen"}]},
    });
    assert_eq!(
        config_option::resolve_model_switch(&stable, "chosen"),
        Some(ModelSwitch::ConfigOption {
            config_id: "model".to_owned(),
            option_value: "chosen".to_owned(),
        })
    );
    assert_eq!(
        config_option::thought_level_config_id(&stable).as_deref(),
        Some("effort")
    );
    assert_eq!(config_option::model_options(&stable).len(), 1);
    assert!(config_option::model_state(&stable).is_some());
    assert!(config_option::offers_model(&stable, "chosen"));
    assert!(!config_option::offers_model(&stable, "absent"));
}

#[test]
fn model_switching_falls_back_to_the_unstable_surface() {
    let unstable = json!({
        "models": {"availableModels": [{"modelId": "only"}]},
    });
    assert_eq!(
        config_option::resolve_model_switch(&unstable, "only"),
        Some(ModelSwitch::SetModel {
            model_id: "only".to_owned(),
        })
    );
    assert_eq!(
        config_option::resolve_model_switch(&unstable, "other"),
        None
    );
    assert_eq!(
        config_option::resolve_model_switch(&json!({}), "only"),
        None
    );
}

#[test]
fn a_model_match_comes_from_the_nested_candidates_not_the_current_value() {
    // "old" is the value the session started with; selecting it again is not a
    // switch, and matching it would hide a model the adapter never offered.
    let result = json!({
        "configOptions": [{"category": "model", "configId": "model", "value": "old",
                           "options": [{"value": "old"}, {"value": "chosen"}]}],
    });
    assert!(config_option::offers_model(&result, "chosen"));
    assert!(config_option::offers_model(&result, "old"));
    assert!(!config_option::offers_model(&result, "invented"));
    // An option with no identifier cannot be addressed, so it offers nothing.
    let anonymous = json!({
        "configOptions": [{"category": "model", "value": "old",
                           "options": [{"value": "chosen"}]}],
    });
    assert!(!config_option::offers_model(&anonymous, "chosen"));
}

#[test]
fn the_cached_catalogue_mirror_agrees_with_full_resolution() {
    let result = json!({
        "configOptions": [{"category": "model", "configId": "model",
                           "options": [{"value": "stable"}]}],
        "models": {"availableModels": [{"modelId": "unstable"}]},
    });
    let options = config_option::model_options(&result);
    let models = config_option::model_state(&result);
    for model_id in ["stable", "unstable"] {
        assert!(config_option::model_in_catalog(
            &options,
            models.as_ref(),
            model_id
        ));
        assert!(config_option::offers_model(&result, model_id));
    }
    assert!(!config_option::model_in_catalog(
        &options,
        models.as_ref(),
        "absent"
    ));
    assert!(!config_option::model_in_catalog(&[], None, "stable"));
    assert!(config_option::model_in_catalog(
        &[],
        models.as_ref(),
        "unstable"
    ));
}

#[test]
fn config_option_params_use_the_adapter_supplied_identifier() {
    assert_eq!(
        config_option::set_config_option_params("s", "effort", "high"),
        json!({"sessionId": "s", "configId": "effort", "value": "high"})
    );
    assert_eq!(
        config_option::set_model_params("s", "m"),
        json!({"sessionId": "s", "modelId": "m"})
    );
}

#[test]
fn classification_separates_responses_from_agent_requests() {
    assert_eq!(
        envelope::classify(&json!({"id": 1, "result": {}})),
        Inbound::Response {
            id: json!(1),
            result: json!({})
        }
    );
    assert!(matches!(
        envelope::classify(&json!({"id": 1, "error": {"code": -1}})),
        Inbound::Failure { .. }
    ));
    assert!(matches!(
        envelope::classify(&json!({"method": "session/update"})),
        Inbound::Notification { .. }
    ));
    // An id alongside a method is an agent request, never a response to us.
    assert!(matches!(
        envelope::classify(&json!({"id": 1, "method": "session/request_permission"})),
        Inbound::Request { .. }
    ));
    assert_eq!(
        envelope::classify(&json!({"result": {}})),
        Inbound::Malformed
    );
}

#[test]
fn response_ids_match_only_the_same_json_type() {
    assert!(envelope::matches_id(&json!(4), 4));
    assert!(!envelope::matches_id(&json!("4"), 4));
    assert!(!envelope::matches_id(&json!(5), 4));
}

#[test]
fn notifications_never_carry_an_id() {
    let frame = envelope::notification("session/cancel", json!({"sessionId": "s"}));
    assert!(frame.get("id").is_none());
    assert_eq!(frame["jsonrpc"], json!("2.0"));
    let request = envelope::request(9, "session/new", json!({}));
    assert_eq!(request["id"], json!(9));
}

#[test]
fn agent_errors_preserve_the_code_and_substitute_a_missing_message() {
    match agent_error(&json!({"code": -32000, "message": "boom"})) {
        Error::AgentError { code, message } => {
            assert_eq!(code, -32000);
            assert_eq!(message, "boom");
        }
        other => panic!("unexpected {other:?}"),
    }
    match agent_error(&json!({"data": {"detail": "kept"}})) {
        Error::AgentError { code, message } => {
            assert_eq!(code, DEFAULT_AGENT_ERROR_CODE);
            assert!(message.contains("kept"), "{message}");
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn permission_options_are_selected_by_kind_never_by_identifier() {
    let request = PermissionRequest::from_parts(
        json!(7),
        json!({"options": [
            {"optionId": "a", "name": "Allow once", "kind": "allow_once"},
            {"optionId": "b", "name": "Reject once", "kind": "reject_once"},
        ]}),
    )
    .unwrap();
    assert_eq!(request.id, json!(7));
    assert_eq!(
        request
            .option_with_kind("allow_once")
            .map(|o| o.option_id.as_str()),
        Some("a")
    );
    assert_eq!(
        request
            .option_with_kind("reject_once")
            .unwrap()
            .name
            .as_deref(),
        Some("Reject once")
    );
    assert!(request.option_with_kind("allow_always").is_none());
    assert_eq!(request.first_selectable().unwrap().option_id, "a");
    assert_eq!(request.raw["options"].as_array().unwrap().len(), 2);
}

#[test]
fn a_permission_request_without_options_is_a_protocol_error() {
    assert!(matches!(
        PermissionRequest::from_parts(json!(1), json!({})),
        Err(Error::Protocol(_))
    ));
}

#[test]
fn permission_responses_encode_the_selected_or_cancelled_outcome() {
    assert_eq!(
        permission_response(json!(3), PermissionDecision::Select("a".to_owned())),
        json!({"jsonrpc": "2.0", "id": 3,
               "result": {"outcome": {"outcome": "selected", "optionId": "a"}}})
    );
    assert_eq!(
        permission_response(json!(3), PermissionDecision::Cancel),
        json!({"jsonrpc": "2.0", "id": 3,
               "result": {"outcome": {"outcome": "cancelled"}}})
    );
}

fn permission_response(id: serde_json::Value, decision: PermissionDecision) -> serde_json::Value {
    lapis_acp_protocol::permission::response(&id, &decision)
}

#[test]
fn session_updates_are_classified_and_never_dropped() {
    let params = |update| json!({"sessionId": "s", "update": update});
    assert!(matches!(
        lapis_acp_protocol::update::parse(&params(json!({
            "sessionUpdate": "agent_message_chunk",
            "content": {"type": "text", "text": "hello"},
        }))),
        SessionUpdate::AgentMessageChunk { text: Some(text), .. } if text == "hello"
    ));
    assert!(matches!(
        lapis_acp_protocol::update::parse(&params(json!({
            "sessionUpdate": "tool_call", "title": "read", "kind": "read", "toolCallId": "t1",
        }))),
        SessionUpdate::ToolCall { tool_call_id: Some(id), .. } if id == "t1"
    ));
    assert!(matches!(
        lapis_acp_protocol::update::parse(&params(json!({
            "sessionUpdate": "tool_call_update", "toolCallId": "t1", "status": "completed",
        }))),
        SessionUpdate::ToolCallUpdate { status: Some(status), .. } if status == "completed"
    ));
    let unknown = lapis_acp_protocol::update::parse(&params(json!({"sessionUpdate": "future"})));
    match unknown {
        SessionUpdate::Unknown { kind, raw } => {
            assert_eq!(kind, "future");
            assert_eq!(raw["sessionUpdate"], json!("future"));
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        lapis_acp_protocol::update::kind(&json!({"update": {}})),
        "unknown"
    );
}

#[test]
fn only_tool_activity_counts_as_progress() {
    let params = |update| json!({"update": update});
    let tool = lapis_acp_protocol::update::parse(&params(json!({"sessionUpdate": "tool_call"})));
    let text =
        lapis_acp_protocol::update::parse(&params(json!({"sessionUpdate": "agent_message_chunk"})));
    assert!(tool.is_activity());
    assert!(!text.is_activity());
    assert_eq!(tool.raw()["sessionUpdate"], json!("tool_call"));
}

#[test]
fn context_blocks_are_capability_gated_and_preserve_authored_bytes() {
    let authored = "Please edit this file\n";
    let context = "<agent_context>repository facts</agent_context>";
    let embedded = session::prompt_context_params("s", authored, Some(context), true);
    assert_eq!(embedded["prompt"][0]["type"], "resource");
    assert_eq!(embedded["prompt"][0]["resource"]["text"], context);
    assert_eq!(embedded["prompt"][0]["resource"]["mimeType"], "text/plain");
    assert_eq!(
        embedded["prompt"][1],
        json!({"type":"text","text":authored})
    );
    let fallback = session::prompt_context_params("s", authored, Some(context), false);
    assert_eq!(
        fallback["prompt"],
        json!([{"type":"text","text":context},{"type":"text","text":authored}])
    );
    assert_eq!(
        session::prompt_context_params("s", authored, None, true),
        session::prompt_text_params("s", &[authored])
    );
}
