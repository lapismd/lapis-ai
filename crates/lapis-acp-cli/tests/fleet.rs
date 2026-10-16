//! Warmup is independent of conversation creation and never sends a prompt.
mod common;
use common::Scratch;
use serde_json::{Value, json};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

#[tokio::test]
async fn warm_worker_opens_distinct_sessions_without_reinitializing() {
    let scratch = Scratch::new("fleet");
    let root = scratch.join("sessions");
    std::fs::create_dir_all(root.join("a")).unwrap();
    std::fs::create_dir_all(root.join("b")).unwrap();
    let log = scratch.log();
    let source = lapis_acp_testkit::fixture_script().with_file_name("fleet_agent.py");
    let fixture = scratch.join("fleet_agent.py");
    std::fs::write(&fixture, std::fs::read_to_string(source).unwrap().replace(
        "elif method == \"session/new\":",
        "elif method == \"session/load\":\n        print(json.dumps({\"jsonrpc\":\"2.0\",\"id\":frame[\"id\"],\"error\":{\"code\":-32603,\"message\":\"fixture rejected load\"}}), flush=True)\n        continue\n    elif method == \"session/new\":"
    )).unwrap();
    let tools = scratch.join("tools.json");
    let tools_log = scratch.join("tools.log");
    std::fs::write(
        &tools,
        json!([{"name":"coding","command":"/bin/false","args":[],"env":[]}]).to_string(),
    )
    .unwrap();
    let catalog = scratch.join("catalog.json");
    std::fs::write(
        &catalog,
        json!({"runners":[{"id":"fixture","label":"Fixture",
        "command":"python3","args":[fixture],"env":{"FLEET_LOG":log,"FLEET_TOOLS_LOG":tools_log}}]})
        .to_string(),
    )
    .unwrap();
    let mut child = Command::new(common::binary())
        .args([
            "serve",
            "--control-version",
            "3",
            "--runner",
            "fixture",
            "--catalog",
        ])
        .arg(catalog)
        .arg("--tools")
        .arg(tools)
        .arg("--cwd")
        .arg(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    async fn receive(output: &mut BufReader<tokio::process::ChildStdout>, kind: &str) -> Value {
        loop {
            let mut line = String::new();
            assert!(
                tokio::time::timeout(Duration::from_secs(10), output.read_line(&mut line))
                    .await
                    .unwrap()
                    .unwrap()
                    > 0
            );
            let value: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(value["version"], 3);
            assert!(
                value["type"] != "error" || kind == "error",
                "unexpected frame: {value}"
            );
            if value["type"] == kind {
                return value;
            }
        }
    }
    async fn send(input: &mut tokio::process::ChildStdin, value: Value) {
        input
            .write_all(format!("{value}\n").as_bytes())
            .await
            .unwrap();
    }
    receive(&mut output, "ready").await;
    send(
        &mut input,
        json!({"version":3,"type":"warm","requestId":"warm"}),
    )
    .await;
    receive(&mut output, "warmed").await;
    let before = std::fs::read_to_string(&log).unwrap();
    assert!(before.contains("initialize"));
    assert!(!before.contains("session/new"));
    assert!(!tools_log.exists());
    assert!(!before.contains("session/prompt"));
    for scope in ["a", "b"] {
        send(
            &mut input,
            json!({"version":3,"type":"open","requestId":format!("open-{scope}"),
            "sessionId":scope,"cwd":root.join(scope)}),
        )
        .await;
        let opened = receive(&mut output, "opened").await;
        assert_eq!(opened["sessionId"], scope);
        send(
            &mut input,
            json!({"version":3,"type":"configure","requestId":format!("cfg-{scope}"),
            "sessionId":scope,"mode":"ask"}),
        )
        .await;
        receive(&mut output, "configured").await;
        send(
            &mut input,
            json!({"version":3,"type":"prompt","requestId":format!("turn-{scope}"),
            "sessionId":scope,"text":if scope == "a" { scope } else { "" },"context":"<agent_context>exact repository facts</agent_context>","maxDurationMs":10000}),
        )
        .await;
        let activity = receive(&mut output, "activity").await;
        assert_eq!(activity["sessionId"], scope);
        assert_eq!(
            activity["update"]["content"]["text"],
            if scope == "a" { scope } else { "" }
        );
        assert_eq!(
            receive(&mut output, "record").await["record"]["status"],
            "completed"
        );
    }
    let capture = std::fs::read_to_string(&log).unwrap();
    let prompts = capture
        .lines()
        .filter_map(|line| line.strip_prefix("prompt-blocks:"))
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(prompts.len(), 2);
    for (index, blocks) in prompts.iter().enumerate() {
        assert_eq!(blocks[0]["type"], "resource");
        assert_eq!(
            blocks[0]["resource"]["text"],
            "<agent_context>exact repository facts</agent_context>"
        );
        assert_eq!(blocks[1]["text"], if index == 0 { "a" } else { "" });
    }
    send(&mut input, json!({"version":3,"type":"open","requestId":"rejected-load","sessionId":"missing","cwd":root.join("a"),"loadSessionId":"missing-rollout"})).await;
    let rejected = receive(&mut output, "error").await;
    assert_eq!(rejected["code"], "agent_rejected");
    assert_eq!(rejected["sessionId"], "missing");
    send(&mut input, json!({"version":3,"type":"configure","requestId":"still-usable","sessionId":"b","mode":"ask"})).await;
    assert_eq!(receive(&mut output, "configured").await["modeReady"], true);
    send(&mut input, json!({"version":3,"type":"prompt","requestId":"permission-turn","sessionId":"a","text":"permission","maxDurationMs":10000})).await;
    let permission = receive(&mut output, "permission").await;
    send(&mut input, json!({"version":3,"type":"permission","requestId":"wrong-scope","sessionId":"b","permissionId":permission["permissionId"],"optionId":"once"})).await;
    assert_eq!(
        receive(&mut output, "error").await["code"],
        "stale_or_invalid_permission"
    );
    send(&mut input, json!({"version":3,"type":"cancel","requestId":"wrong-cancel","sessionId":"b","target":"permission-turn"})).await;
    assert_eq!(receive(&mut output, "error").await["code"], "not_in_flight");
    send(&mut input, json!({"version":3,"type":"permission","requestId":"allow","sessionId":"a","permissionId":permission["permissionId"],"optionId":"once"})).await;
    assert_eq!(
        receive(&mut output, "record").await["record"]["status"],
        "completed"
    );
    send(&mut input, json!({"version":3,"type":"permission","requestId":"stale","sessionId":"a","permissionId":permission["permissionId"],"optionId":"once"})).await;
    assert_eq!(
        receive(&mut output, "error").await["code"],
        "stale_or_invalid_permission"
    );
    send(&mut input, json!({"version":3,"type":"open","requestId":"escape","sessionId":"escape","cwd":scratch.join("outside")})).await;
    assert_eq!(
        receive(&mut output, "error").await["code"],
        "invalid_session_directory"
    );
    send(&mut input, json!({"version":3,"type":"prompt","requestId":"foreign-turn","sessionId":"a","text":"foreign","maxDurationMs":10000})).await;
    assert_eq!(
        receive(&mut output, "record").await["record"]["status"],
        "outcome_unknown"
    );
    send(&mut input, json!({"version":3,"type":"prompt","requestId":"invalidated","sessionId":"b","text":"never replay","maxDurationMs":10000})).await;
    assert_eq!(
        receive(&mut output, "error").await["code"],
        "session_not_open"
    );
    send(
        &mut input,
        json!({"version":3,"type":"close","requestId":"stop"}),
    )
    .await;
    assert_eq!(
        receive(&mut output, "closed").await["confirmedStopped"],
        true
    );
    assert!(child.wait().await.unwrap().success());
    let declarations = std::fs::read_to_string(tools_log).unwrap();
    assert_eq!(declarations.lines().count(), 2);
    for line in declarations.lines() {
        let tools: Value = serde_json::from_str(line).unwrap();
        assert_eq!(tools[0]["name"], "coding");
        assert_eq!(tools[0]["command"], "/bin/false");
    }
    let log = std::fs::read_to_string(log).unwrap();
    assert_eq!(log.lines().filter(|line| *line == "initialize").count(), 1);
    assert_eq!(log.lines().filter(|line| *line == "session/new").count(), 2);
}
