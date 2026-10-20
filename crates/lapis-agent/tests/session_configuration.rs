//! Real ACP process, no prompts, credentials or external provider requests.
use serde_json::{json, Value};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

#[tokio::test]
async fn effort_is_advertised_session_owned_and_unknown_choices_do_not_mutate_it() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lapis-agent"))
        .env_clear()
        .env("BUZZ_AGENT_PROVIDER", "anthropic")
        .env("ANTHROPIC_API_KEY", "synthetic-configuration-test")
        .env("ANTHROPIC_MODEL", "claude-sonnet-4-5")
        .env("BUZZ_AGENT_NO_HINTS", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    async fn request(
        input: &mut tokio::process::ChildStdin,
        output: &mut BufReader<tokio::process::ChildStdout>,
        id: u64,
        method: &str,
        params: Value,
    ) -> Value {
        input
            .write_all(
                format!(
                    "{}\n",
                    json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        loop {
            let mut line = String::new();
            let read = tokio::time::timeout(Duration::from_secs(10), output.read_line(&mut line))
                .await
                .unwrap()
                .unwrap();
            assert!(read > 0, "agent closed before replying");
            let frame: Value = serde_json::from_str(&line).unwrap();
            if frame["id"] == id {
                return frame;
            }
        }
    }
    request(
        &mut input,
        &mut output,
        1,
        "initialize",
        json!({"protocolVersion":1,"clientCapabilities":{}}),
    )
    .await;
    let first = request(
        &mut input,
        &mut output,
        2,
        "session/new",
        json!({"cwd":"/tmp","mcpServers":[]}),
    )
    .await;
    let second = request(
        &mut input,
        &mut output,
        3,
        "session/new",
        json!({"cwd":"/tmp","mcpServers":[]}),
    )
    .await;
    let a = first["result"]["sessionId"].as_str().unwrap();
    let b = second["result"]["sessionId"].as_str().unwrap();
    let options = &first["result"]["configOptions"];
    assert!(options
        .as_array()
        .unwrap()
        .iter()
        .any(|option| option["category"] == "thought_level"));
    let changed = request(
        &mut input,
        &mut output,
        4,
        "session/set_config_option",
        json!({"sessionId":a,"configId":"reasoning_effort","value":"high"}),
    )
    .await;
    assert_eq!(
        changed["result"]["configOptions"][1]["currentValue"],
        "high"
    );
    let other = request(
        &mut input,
        &mut output,
        5,
        "session/set_mode",
        json!({"sessionId":b,"modeId":"ask"}),
    )
    .await;
    assert_eq!(
        other["result"]["configOptions"][1]["currentValue"],
        "default"
    );
    for (id, value) in [(6, "invented"), (7, "none")] {
        let denied = request(
            &mut input,
            &mut output,
            id,
            "session/set_config_option",
            json!({"sessionId":a,"configId":"reasoning_effort","value":value}),
        )
        .await;
        assert!(denied.get("error").is_some());
    }
    let retained = request(
        &mut input,
        &mut output,
        8,
        "session/set_mode",
        json!({"sessionId":a,"modeId":"ask"}),
    )
    .await;
    assert_eq!(
        retained["result"]["configOptions"][1]["currentValue"],
        "high"
    );
    drop(input);
    assert!(tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .unwrap()
        .unwrap()
        .success());
}
