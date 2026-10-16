//! Interactive consumer drives the actual binary; old v1 tests remain separate.
mod common;
use common::Scratch;
use serde_json::{Value, json};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};
struct Peer {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Peer {
    async fn start(scratch: &Scratch) -> Self {
        let fixture = lapis_acp_testkit::fixture_script().with_file_name("interactive_agent.py");
        let catalog = scratch.join("catalog.json");
        std::fs::write(
            &catalog,
            json!({"runners":[{"id":"fixture","label":"Fixture","command":"python3",
            "args":[fixture],"env":{}}]})
            .to_string(),
        )
        .unwrap();
        let mut child = Command::new(common::binary())
            .args([
                "serve",
                "--control-version",
                "2",
                "--runner",
                "fixture",
                "--catalog",
            ])
            .arg(catalog)
            .args(["--cancel-grace", "1", "--reap-timeout", "1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut peer = Self {
            child,
            input,
            output,
        };
        assert_eq!(peer.recv().await["version"], 2);
        peer
    }
    async fn send(&mut self, id: &str, kind: &str, mut body: Value) {
        body["version"] = json!(2);
        body["requestId"] = json!(id);
        body["type"] = json!(kind);
        self.input
            .write_all(format!("{body}\n").as_bytes())
            .await
            .unwrap();
    }
    async fn recv(&mut self) -> Value {
        let mut line = String::new();
        let n = tokio::time::timeout(Duration::from_secs(15), self.output.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        assert!(n > 0, "stream ended");
        serde_json::from_str(&line).unwrap()
    }
    async fn until(&mut self, kind: &str) -> Value {
        loop {
            let value = self.recv().await;
            if value["type"] == kind {
                return value;
            }
        }
    }
    async fn open(&mut self) {
        self.send("open", "open", json!({})).await;
        assert_eq!(self.until("opened").await["sessionId"], "warm-session");
        self.send("configure", "configure", json!({"mode":"ask"}))
            .await;
        assert_eq!(self.until("configured").await["modeReady"], true);
    }
    async fn close(mut self) {
        self.send("close", "close", json!({})).await;
        assert_eq!(self.until("closed").await["confirmedStopped"], true);
        drop(self.input);
        assert!(
            tokio::time::timeout(Duration::from_secs(5), self.child.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
    }
}
#[tokio::test]
async fn follow_up_reuses_the_session_and_streams_activity() {
    let scratch = Scratch::new("interactive-warm");
    let mut peer = Peer::start(&scratch).await;
    peer.open().await;
    for (id, expected) in [("one", "turn 1"), ("two", "turn 2")] {
        peer.send(id, "prompt", json!({"text":"hello","maxDurationMs":10000}))
            .await;
        let activity = peer.until("activity").await;
        assert_eq!(activity["requestId"], id);
        assert_eq!(activity["update"]["content"]["text"], expected);
        let record = peer.until("record").await;
        assert_eq!(record["record"]["status"], "completed");
        assert_eq!(record["sessionUsable"], true);
    }
    peer.close().await;
}
#[tokio::test]
async fn cancelling_a_queued_prompt_keeps_the_warm_session_usable() {
    let scratch = Scratch::new("interactive-queued-cancel");
    let mut peer = Peer::start(&scratch).await;
    peer.open().await;
    peer.send(
        "active",
        "prompt",
        json!({"text":"permission", "maxDurationMs":10000}),
    )
    .await;
    let permission = peer.until("permission").await;
    peer.send(
        "queued",
        "prompt",
        json!({"text":"hello", "maxDurationMs":10000}),
    )
    .await;
    assert_eq!(peer.until("queued").await["requestId"], "queued");
    peer.send("cancel-queued", "cancel", json!({"target":"queued"}))
        .await;
    peer.until("ack").await;
    peer.send(
        "allow",
        "permission",
        json!({"permissionId":permission["permissionId"], "optionId":"allow"}),
    )
    .await;
    assert_eq!(peer.until("record").await["requestId"], "active");
    let cancelled = peer.until("record").await;
    assert_eq!(cancelled["requestId"], "queued");
    assert_eq!(cancelled["record"]["status"], "cancelled");
    assert_eq!(cancelled["sessionUsable"], true);
    peer.send(
        "follow-up",
        "prompt",
        json!({"text":"hello", "maxDurationMs":10000}),
    )
    .await;
    let follow_up = peer.until("record").await;
    assert_eq!(follow_up["record"]["status"], "completed");
    assert_eq!(follow_up["sessionUsable"], true);
    peer.close().await;
}
#[tokio::test]
async fn permissions_validate_exact_options_and_expire_after_resolution() {
    let scratch = Scratch::new("interactive-permissions");
    let mut peer = Peer::start(&scratch).await;
    peer.open().await;
    peer.send(
        "turn",
        "prompt",
        json!({"text":"permission","maxDurationMs":10000}),
    )
    .await;
    let permission = peer.until("permission").await;
    let id = permission["permissionId"].as_str().unwrap();
    peer.send(
        "invalid",
        "permission",
        json!({"permissionId":id,"optionId":"invented"}),
    )
    .await;
    assert_eq!(
        peer.until("error").await["code"],
        "stale_or_invalid_permission"
    );
    peer.send(
        "allow",
        "permission",
        json!({"permissionId":id,"optionId":"allow"}),
    )
    .await;
    assert_eq!(peer.until("record").await["record"]["status"], "completed");
    peer.send(
        "late",
        "permission",
        json!({"permissionId":id,"optionId":"allow"}),
    )
    .await;
    assert_eq!(
        peer.until("error").await["code"],
        "stale_or_invalid_permission"
    );
    peer.close().await;
}
#[tokio::test]
async fn cancellation_remains_serviceable_during_permission_wait() {
    let scratch = Scratch::new("interactive-cancel");
    let mut peer = Peer::start(&scratch).await;
    peer.open().await;
    peer.send(
        "turn",
        "prompt",
        json!({"text":"wait","maxDurationMs":10000}),
    )
    .await;
    peer.until("permission").await;
    peer.send("cancel", "cancel", json!({"target":"turn"}))
        .await;
    assert_eq!(peer.until("record").await["record"]["status"], "cancelled");
    peer.close().await;
}
#[tokio::test]
async fn permission_wait_is_inside_the_absolute_deadline() {
    let scratch = Scratch::new("interactive-timeout");
    let mut peer = Peer::start(&scratch).await;
    peer.open().await;
    peer.send("turn", "prompt", json!({"text":"wait","maxDurationMs":100}))
        .await;
    assert_eq!(peer.until("record").await["record"]["status"], "timed_out");
    peer.close().await;
}
#[tokio::test]
async fn unsupported_modes_block_prompts_and_duplicates_never_run_again() {
    let scratch = Scratch::new("interactive-mode");
    let mut peer = Peer::start(&scratch).await;
    peer.send("open", "open", json!({})).await;
    peer.until("opened").await;
    peer.send("config", "configure", json!({"mode":"invented"}))
        .await;
    assert_eq!(peer.until("configured").await["modeReady"], false);
    peer.send(
        "turn",
        "prompt",
        json!({"text":"hello","maxDurationMs":10000}),
    )
    .await;
    assert_eq!(peer.until("error").await["code"], "execution_mode_required");
    peer.send(
        "turn",
        "prompt",
        json!({"text":"hello","maxDurationMs":10000}),
    )
    .await;
    assert_eq!(peer.until("error").await["code"], "duplicate_request");
    peer.close().await;
}
#[tokio::test]
async fn resume_is_explicit_and_does_not_send_a_prompt() {
    let scratch = Scratch::new("interactive-resume");
    let mut peer = Peer::start(&scratch).await;
    peer.send("load", "open", json!({"loadSessionId":"warm-session"}))
        .await;
    assert_eq!(peer.until("opened").await["sessionId"], "warm-session");
    peer.send("status", "status", json!({})).await;
    assert!(peer.until("status").await["running"].is_null());
    peer.close().await;
}
