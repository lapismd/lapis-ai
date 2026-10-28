//! Handoff-cap regression with mid-turn steer coverage.
//! Kept out of `regressions.rs` so that frozen architecture exception cannot grow.

use serde_json::{json, Value};

mod common;
use common::{approve_permission, openai_text, openai_tool_call, spawn_capturing_llm, Harness};

fn openai_text_with_usage(content: &str, prompt_tokens: u64) -> Value {
    let mut v = openai_text(content);
    v["usage"] = json!({
        "prompt_tokens": prompt_tokens,
        "completion_tokens": 1,
        "total_tokens": prompt_tokens + 1,
    });
    v
}

/// Within a single turn, the per-turn cap still bounds the number of handoffs.
/// A steer is injected while the run is active to prove the steer path does
/// not reset `handoff_attempts`. Drain waits for both the prompt result and the
/// steer RPC reply so prompt-complete races cannot drop the steer arm.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn handoff_cap_binds_within_a_single_turn() {
    let fake_mcp = env!("CARGO_BIN_EXE_fake-mcp");
    let tool_call_with_usage = {
        let mut v = openai_tool_call("tc-1", "test_tool", json!({}));
        v["usage"] = json!({
            "prompt_tokens": 950u64,
            "completion_tokens": 5,
            "total_tokens": 955,
        });
        v
    };
    let llm = spawn_capturing_llm(vec![
        openai_text_with_usage("seed", 950),
        openai_text("handoff-summary"),
        tool_call_with_usage,
        openai_text_with_usage("end_turn_text", 10),
    ])
    .await;

    let mut h = Harness::spawn_with_env(
        &llm.url,
        &[
            ("BUZZ_AGENT_MAX_CONTEXT_TOKENS", "1000"),
            ("BUZZ_AGENT_MAX_OUTPUT_TOKENS", "100"),
            ("BUZZ_AGENT_MAX_HANDOFFS", "1"),
            (
                "BUZZ_AGENT_MAX_HISTORY_BYTES",
                &(16 * 1024 * 1024).to_string(),
            ),
        ],
    )
    .await;

    h.send(
        "initialize",
        json!({"protocolVersion":1,"clientCapabilities":{}}),
    )
    .await;
    let _ = h.recv().await;
    h.send(
        "session/new",
        json!({
            "cwd": "/tmp",
            "mcpServers": [{
                "name": "cap_test",
                "command": fake_mcp,
                "args": [],
                "env": [{ "name": "FAKE_MCP_TOOL_COUNT", "value": "1" }],
            }],
        }),
    )
    .await;
    let r = h
        .recv_until(|v| v.get("result").is_some() || v.get("error").is_some())
        .await;
    let sid = r["result"]["sessionId"].as_str().unwrap().to_owned();

    let p1 = h
        .send(
            "session/prompt",
            json!({"sessionId": sid, "prompt": [{"type":"text","text":"seed"}]}),
        )
        .await;
    let _ = h.recv_until(|v| v["id"] == json!(p1)).await;

    let p2 = h
        .send(
            "session/prompt",
            json!({"sessionId": sid, "prompt": [{"type":"text","text":"do work"}]}),
        )
        .await;

    let mut run_id: Option<String> = None;
    let mut steer_id: i64 = -1;
    let mut steer_accepted = false;
    let mut prompt_done = false;
    loop {
        let v = h.recv().await;

        if run_id.is_none() {
            if let Some(rid) = v["params"]["update"]["_meta"]["goose"]["activeRunId"].as_str() {
                run_id = Some(rid.to_owned());
                steer_id = h
                    .send(
                        "_goose/unstable/session/steer",
                        json!({
                            "sessionId": sid,
                            "expectedRunId": rid,
                            "prompt": [{"type":"text","text":"STEER-CANARY: also consider the edge case"}],
                        }),
                    )
                    .await;
            }
        }

        if steer_id >= 0 && v["id"] == json!(steer_id) {
            assert!(
                v.get("result").is_some(),
                "steer must be accepted while the run is active; got: {v}"
            );
            assert_eq!(
                v["result"]["runId"].as_str(),
                run_id.as_deref(),
                "steer must reference the live run id"
            );
            steer_accepted = true;
            if prompt_done {
                break;
            }
            continue;
        }

        if v.get("method") == Some(&json!("session/request_permission")) {
            h.write(approve_permission(&v)).await;
            continue;
        }
        if v["id"] == json!(p2) {
            assert!(
                v.get("result").is_some(),
                "turn 2 must succeed even when cap blocks round-1 handoff; got: {v}"
            );
            prompt_done = true;
            if steer_accepted {
                break;
            }
            assert!(
                steer_id >= 0,
                "activeRunId was never observed before turn 2 completed"
            );
            continue;
        }
    }

    assert!(
        steer_accepted,
        "steer was never accepted during turn 2; the steer arm is missing coverage"
    );

    let count = llm.captured.lock().await.len();
    assert_eq!(
        count, 4,
        "expected 4 LLM requests (seed + summarize + tool-call + final); got {count}"
    );

    let stderr = h.stderr_text();
    assert!(
        stderr.contains("handoff cap reached"),
        "expected cap-reached WARN in stderr; got: {stderr}"
    );
    assert!(
        stderr.contains("reason=\"preflight\""),
        "expected reason=\"preflight\" field in cap WARN; got: {stderr}"
    );
    assert!(
        stderr.contains("handoff_attempts="),
        "expected handoff_attempts field in cap WARN; got: {stderr}"
    );
    assert!(
        stderr.contains("max_handoffs="),
        "expected max_handoffs field in cap WARN; got: {stderr}"
    );

    h.shutdown().await;
}
