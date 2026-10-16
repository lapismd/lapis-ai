// LapisMD: subscription transport regression fixtures for the maintained Buzz fork.
use super::*;
fn cfg() -> Config {
    Config::for_discovery(Provider::OpenAi, "fixture-only".into(), API.into(), None)
}
fn event(value: Value) -> Vec<u8> {
    format!("data: {}\r\n\r\n", value).into_bytes()
}
#[test]
fn fragmented_unicode_stream_requires_completed_terminal_and_preserves_output() {
    let output = json!([{ "type":"message","id":"msg-1","phase":"final_answer","content":[{"type":"output_text","text":"héllo"}] }]);
    let bytes = event(
        json!({"type":"response.completed","response":{"status":"completed","output":output}}),
    );
    let mut parser = Stream::default();
    for byte in bytes {
        parser.feed(&[byte]).unwrap();
    }
    assert_eq!(parser.finish().unwrap()["output"], output);
    let mut truncated = Stream::default();
    truncated
        .feed(b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}\n\n")
        .unwrap();
    assert!(truncated.finish().is_err());
}
#[test]
fn late_failure_and_incomplete_never_release_tool_calls() {
    for kind in ["response.failed", "response.incomplete", "error"] {
        let mut parser = Stream::default();
        parser.feed(&event(json!({"type":"response.output_item.done","item":{"type":"function_call","name":"write_file","arguments":"{}"}}))).unwrap();
        let error=parser.feed(&event(json!({"type":kind,"response":{"error":{"code":"subscription_sharing_usage_limit_exceeded"}}}))).unwrap_err();
        assert!(error
            .to_string()
            .contains("subscription_sharing_usage_limit_exceeded"));
    }
}
#[test]
fn tools_history_and_opaque_reasoning_are_replayed_once_in_exact_order() {
    let raw = json!([
        {"type":"reasoning","id":"reason-1","encrypted_content":"opaque","summary":[]},
        {"type":"function_call","id":"fc-1","call_id":"call-1","namespace":"functions","name":"read_file","arguments":"{}"}
    ]);
    let history = vec![
        HistoryItem::User("read".into()),
        HistoryItem::Assistant {
            text: "".into(),
            tool_calls: vec![],
            reasoning_details: Some(json!({"chatgpt_output":raw})),
        },
        HistoryItem::ToolResult(crate::types::ToolResult {
            provider_id: "call-1".into(),
            content: vec![ToolResultContent::Text("contents".into())],
            is_error: false,
        }),
    ];
    let body = request(
        &cfg(),
        "instructions",
        &history,
        &[ToolDef {
            name: "read_file".into(),
            description: "Read file".into(),
            input_schema: json!({"type":"object","properties":{}}),
        }],
        "chosen",
    );
    assert!(body.get("max_output_tokens").is_none());
    assert_eq!(body["stream"], true);
    assert_eq!(body["store"], false);
    assert_eq!(body["tools"][0]["type"], "namespace");
    assert_eq!(body["input"].as_array().unwrap().len(), 4);
    assert_eq!(body["input"][1], raw[0]);
    assert_eq!(body["input"][2], raw[1]);
    assert_eq!(body["input"][3]["call_id"], "call-1");
}
#[test]
fn trailing_truncation_or_malformed_terminal_is_not_success() {
    let mut parser = Stream::default();
    parser
        .feed(&event(
            json!({"type":"response.completed","response":{"status":"completed","output":[]}}),
        ))
        .unwrap();
    parser.feed(b"data: {").unwrap();
    assert!(parser.finish().is_err());
    let mut parser = Stream::default();
    assert!(parser
        .feed(&event(
            json!({"type":"response.completed","response":{"status":"incomplete","output":[]}})
        ))
        .is_err());
}
#[tokio::test]
async fn pre_stream_401_renews_once_tool_rounds_replay_and_uncertain_calls_never_retry() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Source(AtomicUsize);
    #[async_trait::async_trait]
    impl TokenSource for Source {
        async fn bearer(&self) -> Result<String, AgentError> {
            Ok("fixture-before".into())
        }
        async fn bearer_no_browser(&self) -> Result<String, AgentError> {
            self.bearer().await
        }
        async fn refresh_now(&self, rejected: &str) -> Result<String, AgentError> {
            assert_eq!(rejected, "fixture-before");
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok("fixture-renewed".into())
        }
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let saved = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let calls = count.clone();
    let requests = saved.clone();
    let output = json!([{ "type":"reasoning","id":"reason-one","summary":[],"encrypted_content":"opaque-fixture" },{"type":"function_call","id":"fc-one","namespace":"functions","call_id":"call-one","name":"read_file","arguments":"{}","status":"completed"}]);
    let returned = output.clone();
    let router=axum::Router::new().route("/v1/responses",axum::routing::post(move |headers:axum::http::HeaderMap,axum::Json(body):axum::Json<Value>|{let requests=requests.clone();let calls=calls.clone();let returned=returned.clone();async move{
        let call=calls.fetch_add(1,Ordering::SeqCst);requests.lock().unwrap().push(body);
        match call {
            0=>{assert_eq!(headers["authorization"],"Bearer fixture-before");(axum::http::StatusCode::UNAUTHORIZED,[("content-type","application/json")],"{}".into())},
            1=>{assert_eq!(headers["authorization"],"Bearer fixture-renewed");(axum::http::StatusCode::OK,[("content-type","text/event-stream")],String::from_utf8(event(json!({"type":"response.completed","response":{"status":"completed","output":returned}}))).unwrap())},
            2=>{let output=json!([{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Complete"}]}]);(axum::http::StatusCode::OK,[("content-type","text/event-stream")],String::from_utf8(event(json!({"type":"response.completed","response":{"status":"completed","output":output}}))).unwrap())},
            _=>(axum::http::StatusCode::OK,[("content-type","text/event-stream")],"data: {\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}\n\n".into()),
        }
    }}));
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let mut config = cfg();
    config.base_url = format!("http://{address}/v1");
    config.openai_api = OpenAiApi::ChatGptPlan;
    let auth = Arc::new(Source(AtomicUsize::new(0)));
    let llm = Llm::new(&config).unwrap().with_token_source(auth.clone());
    let mut history = vec![HistoryItem::User("Read".into())];
    let result = llm
        .complete(&config, "system", &history, &[], "account-model")
        .await
        .unwrap();
    assert_eq!(result.tool_calls.len(), 1);
    history.push(HistoryItem::Assistant {
        text: result.text,
        tool_calls: result.tool_calls,
        reasoning_details: result.reasoning_details,
    });
    history.push(HistoryItem::ToolResult(crate::types::ToolResult {
        provider_id: "call-one".into(),
        content: vec![ToolResultContent::Text("fixture file".into())],
        is_error: false,
    }));
    assert_eq!(
        llm.complete(&config, "system", &history, &[], "account-model")
            .await
            .unwrap()
            .text,
        "Complete"
    );
    {
        let requests = saved.lock().unwrap();
        assert_eq!(requests[0], requests[1]);
        assert_eq!(requests[2]["input"][1], output[0]);
        assert_eq!(requests[2]["input"][2], output[1]);
    }
    assert_eq!(auth.0.load(Ordering::SeqCst), 1);
    assert!(llm
        .complete(&config, "system", &history, &[], "account-model")
        .await
        .is_err());
    assert_eq!(count.load(Ordering::SeqCst), 4);
    server.abort();
}
