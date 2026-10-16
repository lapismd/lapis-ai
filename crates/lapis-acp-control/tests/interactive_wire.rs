//! Untrusted control input cannot broaden launch authority or lose partial frames.
use lapis_acp_control::interactive_wire::parse;
use lapis_acp_control::{FrameSource, LineSource};
use tokio::io::{AsyncWriteExt, BufReader, duplex};
#[test]
fn launch_paths_and_unadvertised_fields_are_refused() {
    for key in ["cwd", "executable", "env", "tools"] {
        let text = format!(r#"{{"version":2,"type":"open","requestId":"one","{key}":"unsafe"}}"#);
        assert_eq!(parse(&text).unwrap_err(), "invalid_request");
    }
    assert_eq!(
        parse(r#"{"version":3,"type":"open","requestId":"one"}"#).unwrap_err(),
        "unsupported_control_version"
    );
}
#[tokio::test]
async fn partial_frames_survive_cancelled_reads() {
    let (mut input, output) = duplex(256);
    let mut source = LineSource::new(BufReader::new(output));
    input.write_all(b"partial").await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), source.next_line())
            .await
            .is_err()
    );
    input.write_all(b" frame\n").await.unwrap();
    assert_eq!(
        source.next_line().await.unwrap().unwrap(),
        "partial frame\n"
    );
}
#[tokio::test]
async fn endless_control_input_is_bounded_before_a_newline() {
    let (mut input, output) = duplex(8192);
    let mut source = LineSource::new(BufReader::new(output));
    let writer = tokio::spawn(async move {
        let _ = input.write_all(&vec![b'x'; 2_097_153]).await;
    });
    assert!(source.next_line().await.is_err());
    writer.abort();
}

#[test]
fn prompt_context_is_optional_bounded_data_without_launch_authority() {
    use lapis_acp_control::interactive_wire::Command;
    let valid = r#"{"version":2,"type":"prompt","requestId":"one","text":"authored","context":"<context>facts</context>","maxDurationMs":1000}"#;
    assert!(matches!(
        parse(valid).unwrap(),
        Command::Prompt {
            context: Some(_),
            ..
        }
    ));
    for context in [
        serde_json::json!(12),
        serde_json::json!("x".repeat(512 * 1024 + 1)),
        serde_json::json!("bad\0context"),
    ] {
        let mut value: serde_json::Value = serde_json::from_str(valid).unwrap();
        value["context"] = context;
        assert_eq!(parse(&value.to_string()).unwrap_err(), "invalid_request");
    }
    let mut value: serde_json::Value = serde_json::from_str(valid).unwrap();
    value["cwd"] = serde_json::json!("/unsafe");
    assert_eq!(parse(&value.to_string()).unwrap_err(), "invalid_request");
}

#[test]
fn context_only_prompt_requires_nonempty_prepared_content() {
    let mut value = serde_json::json!({"version":2,"type":"prompt","requestId":"comments","text":"","context":"<comments>Review this</comments>","maxDurationMs":1000});
    assert!(parse(&value.to_string()).is_ok());
    for context in [
        serde_json::Value::Null,
        serde_json::json!(""),
        serde_json::json!(" \n "),
    ] {
        value["context"] = context;
        assert_eq!(parse(&value.to_string()).unwrap_err(), "invalid_request");
    }
}
