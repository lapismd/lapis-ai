mod support;
use lapis_ai_app_tools::{Config, Error};
use serde_json::json;
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use support::{Handler, Session};

#[tokio::test]
async fn initialization_snapshot_and_exact_string_unsigned_and_negative_ids() {
    let mut session = Session::new(Config::default());
    session.send(json!(0), "tools/list", json!({})).await;
    assert_eq!(session.next().await["error"]["code"], -32002);
    session.initialize().await;
    session.handler.tools.lock().unwrap().clear();
    session.send(json!("list"), "tools/list", json!({})).await;
    let list = session.next().await;
    assert_eq!(list["result"]["tools"][0]["name"], "notes.read");
    for id in [json!(1), json!("1"), json!(u64::MAX), json!(-42)] {
        session
            .send(
                id.clone(),
                "tools/call",
                json!({"name":"notes.read","arguments":{"path":"Notes/Δ.md"}}),
            )
            .await;
        let response = session.next().await;
        assert_eq!(response["id"], id);
        assert_eq!(
            response["result"]["structuredContent"]["path"],
            "Notes/Δ.md"
        );
    }
    assert_eq!(session.handler.calls.lock().unwrap().len(), 4);
    session.stop().await.unwrap();
}

#[tokio::test]
async fn malformed_unknown_and_unlisted_requests_never_dispatch() {
    let mut session = Session::new(Config::default());
    session.raw(b"not-json\n").await;
    assert_eq!(session.next().await["error"]["code"], -32700);
    session.raw(b"[]\n").await;
    assert_eq!(session.next().await["error"]["code"], -32600);
    session.initialize().await;
    session
        .send(
            json!("method"),
            "files/write",
            json!({"secret":"UNTRUSTED"}),
        )
        .await;
    let response = session.next().await;
    assert_eq!(response["error"]["code"], -32601);
    assert!(!response.to_string().contains("UNTRUSTED"));
    session
        .send(
            json!("hidden"),
            "tools/call",
            json!({"name":"notes.delete","arguments":{}}),
        )
        .await;
    assert_eq!(
        session.next().await["result"]["_meta"]["lapis.md/outcome"],
        "rejected"
    );
    session
        .send(
            json!("invalid"),
            "tools/call",
            json!({"name":"notes.read","arguments":[]}),
        )
        .await;
    assert_eq!(session.next().await["result"]["isError"], true);
    for id in [
        json!(true),
        json!(1.5),
        json!("bad\nidentity"),
        json!(""),
        json!("x".repeat(257)),
    ] {
        session
            .send(id, "tools/call", json!({"name":"notes.read"}))
            .await;
        assert_eq!(session.next().await["error"]["code"], -32600);
    }
    assert!(session.handler.calls.lock().unwrap().is_empty());
    session.stop().await.unwrap();
}

#[tokio::test]
async fn duplicate_id_retires_without_replaying_a_completed_call() {
    let mut session = Session::new(Config::default());
    session.initialize().await;
    session
        .send(json!("once"), "tools/call", json!({"name":"notes.read"}))
        .await;
    assert_eq!(session.next().await["id"], "once");
    session
        .send(json!("once"), "tools/call", json!({"name":"notes.read"}))
        .await;
    let handler = session.handler.clone();
    assert!(matches!(
        session.task.await.unwrap(),
        Err(Error::DuplicateRequest)
    ));
    assert_eq!(handler.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn frame_budget_and_unterminated_input_retire_without_dispatch() {
    for (bytes, oversized) in [
        (vec![b'x'; 1025], true),
        (b"{\"jsonrpc\":\"2.0\"}".to_vec(), false),
    ] {
        let mut session = Session::new(Config {
            frame_bytes: 1024,
            ..Config::default()
        });
        session.raw(&bytes).await;
        let handler = session.handler.clone();
        let result = session.stop().await;
        assert!(if oversized {
            matches!(result, Err(Error::FrameLimit))
        } else {
            matches!(result, Err(Error::IncompleteFrame))
        });
        assert!(handler.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn request_budget_counts_notifications_and_never_queues_them() {
    let mut session = Session::new(Config {
        maximum_requests: 2,
        ..Config::default()
    });
    session.initialize().await;
    session.notify("notifications/initialized", json!({})).await;
    session.notify("notifications/unknown", json!({})).await;
    assert!(matches!(
        session.task.await.unwrap(),
        Err(Error::RequestLimit)
    ));
}

#[tokio::test]
async fn oversized_host_output_retires_without_partial_result_or_retry() {
    let mut session = Session::new(Config {
        frame_bytes: 1024,
        ..Config::default()
    });
    session.initialize().await;
    session
        .send(
            json!("large"),
            "tools/call",
            json!({"name":"notes.read","arguments":{"bytes":2048}}),
        )
        .await;
    let handler = session.handler.clone();
    assert!(matches!(
        session.task.await.unwrap(),
        Err(Error::FrameLimit)
    ));
    assert_eq!(handler.calls.lock().unwrap().len(), 1);
    assert_eq!(handler.active.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn host_rejections_and_unknown_effects_are_distinct() {
    let mut session = Session::new(Config::default());
    session.initialize().await;
    for error in ["invalid", "unavailable", "unknown"] {
        session
            .send(
                json!(error),
                "tools/call",
                json!({"name":"notes.read","arguments":{"error":error}}),
            )
            .await;
        assert_eq!(
            session.next().await["result"]["_meta"]["lapis.md/outcome"],
            if error == "unknown" {
                "unknown"
            } else {
                "rejected"
            }
        );
    }
    session.stop().await.unwrap();
}

#[tokio::test]
async fn invalid_budgets_or_snapshots_refuse_before_reading_input() {
    for fault in [
        "calls",
        "requests",
        "frame",
        "timeout",
        "identity",
        "duplicate",
        "schema",
        "large",
        "count",
    ] {
        let handler = Arc::new(Handler::default());
        let mut config = Config::default();
        match fault {
            "calls" => config.maximum_calls = 9,
            "requests" => config.maximum_requests = 4097,
            "frame" => config.frame_bytes = 1_048_577,
            "timeout" => config.call_timeout = Duration::ZERO,
            "identity" => config.name = "bad\nlabel".into(),
            "duplicate" => {
                let mut tools = handler.tools.lock().unwrap();
                let duplicate = tools[0].clone();
                tools.push(duplicate);
            }
            "schema" => handler.tools.lock().unwrap()[0].input_schema = json!([]),
            "large" => {
                handler.tools.lock().unwrap()[0].input_schema =
                    json!({"description":"x".repeat(65_536)})
            }
            "count" => {
                let mut tools = handler.tools.lock().unwrap();
                let tool = tools[0].clone();
                *tools = vec![tool; 257];
            }
            _ => unreachable!(),
        }
        let result = lapis_ai_app_tools::serve(
            tokio::io::empty(),
            tokio::io::sink(),
            handler.clone(),
            config,
        )
        .await;
        assert!(matches!(result, Err(Error::Configuration)), "fault {fault}");
        assert!(handler.calls.lock().unwrap().is_empty());
    }
}
