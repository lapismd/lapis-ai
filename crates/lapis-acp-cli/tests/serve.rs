//! The persistent control protocol, driven over the real binary's stdio.
//!
//! These tests are the controlling application. They write frames, read frames,
//! and assert that stdout never carries anything else — which is the property that
//! makes the protocol parseable rather than merely documented.

mod common;

use std::process::Stdio;
use std::time::Duration;

use common::{Scratch, task, write_catalog};

fn scoped_submit(request_id: &str, scope: &str, task: &Value) -> Value {
    scoped_submit_frame(request_id, scope, task)
}
use lapis_acp_control::protocol::{
    CONTROL_VERSION, cancel_frame, runners_frame, scoped_submit_frame, shutdown_frame,
    status_frame, submit_frame,
};
use lapis_acp_testkit::FixtureLog;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const READ_TIMEOUT: Duration = Duration::from_secs(45);

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Server {
    async fn start(scratch: &Scratch, mode: &str, extra: &[&str]) -> Self {
        let catalog = write_catalog(scratch, mode, &[]);
        let mut command = Command::new(common::binary());
        command
            .arg("serve")
            .arg("--catalog")
            .arg(&catalog)
            .arg("--runner")
            .arg("fixture")
            .arg("--idle-timeout")
            .arg("30")
            .arg("--max-turn-duration")
            .arg("60")
            .arg("--cancel-grace")
            .arg("2")
            .arg("--reap-timeout")
            .arg("2")
            .args(extra)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn().expect("serve starts");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut server = Self {
            child,
            stdin,
            stdout,
        };
        // Every session begins with the ready frame, so consuming it here means a
        // test that reads next sees its own reply rather than the greeting.
        let ready = server.recv().await;
        assert_eq!(ready["type"], json!("ready"), "{ready}");
        assert_eq!(ready["version"], json!(CONTROL_VERSION));
        server
    }

    async fn send(&mut self, frame: Value) {
        let mut line = frame.to_string();
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await.expect("write");
        self.stdin.flush().await.expect("flush");
    }

    async fn recv(&mut self) -> Value {
        let mut line = String::new();
        let read = tokio::time::timeout(READ_TIMEOUT, self.stdout.read_line(&mut line))
            .await
            .expect("harness stopped answering")
            .expect("stdout read");
        assert!(read > 0, "the control stream closed unexpectedly");
        serde_json::from_str(line.trim())
            .unwrap_or_else(|error| panic!("stdout carried a non-frame line ({error}): {line:?}"))
    }

    /// Read frames until one of the wanted type arrives, collecting the rest.
    async fn recv_type(&mut self, wanted: &str) -> (Value, Vec<Value>) {
        let mut skipped = Vec::new();
        loop {
            let frame = self.recv().await;
            if frame["type"] == json!(wanted) {
                return (frame, skipped);
            }
            skipped.push(frame);
        }
    }

    async fn stop(self) -> (Option<i32>, String) {
        drop(self.stdin);
        let output = self.child.wait_with_output().await.expect("exit");
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        )
    }
}

#[tokio::test]
async fn startup_announces_the_resolved_runners() {
    let scratch = Scratch::new("serve-ready");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let mut command = Command::new(common::binary());
    command
        .args(["serve", "--catalog"])
        .arg(&catalog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().expect("serve starts");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
    let mut line = String::new();
    tokio::time::timeout(READ_TIMEOUT, stdout.read_line(&mut line))
        .await
        .expect("ready frame")
        .expect("read");
    let ready: Value = serde_json::from_str(line.trim()).expect("json frame");
    assert_eq!(ready["type"], json!("ready"));
    assert_eq!(ready["version"], json!(1));
    let runners = ready["runners"].as_array().expect("runners");
    // Five compiled-in runners plus the catalog's fixture entry.
    assert_eq!(runners.len(), 6, "{runners:?}");
    let ids: Vec<&str> = runners
        .iter()
        .map(|runner| runner["id"].as_str().expect("id"))
        .collect();
    assert!(ids.contains(&"fixture"), "{ids:?}");
    assert!(ids.contains(&"cursor"), "{ids:?}");
    drop(stdout);
    child.kill().await.ok();
}

#[tokio::test]
async fn a_submitted_task_returns_exactly_one_record() {
    let scratch = Scratch::new("serve-submit");
    let mut server = Server::start(&scratch, "normal", &[]).await;
    server
        .send(submit_frame(
            "req-1",
            &task("task-serve-1", "do the work", 60_000),
        ))
        .await;
    let (record, skipped) = server.recv_type("record").await;
    assert_eq!(record["requestId"], json!("req-1"));
    assert_eq!(record["record"]["status"], json!("completed"));
    assert_eq!(record["record"]["taskId"], json!("task-serve-1"));
    assert_eq!(record["record"]["stopReason"], json!("end_turn"));
    assert_eq!(record["version"], json!(CONTROL_VERSION));
    // Admission is acknowledged and diagnostics are frames too, so nothing that
    // is not a frame may share the stream.
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert_eq!(skipped[0]["type"], json!("queued"));
    assert_eq!(skipped[0]["requestId"], json!("req-1"));
    assert_eq!(skipped[0]["scope"], json!("default"));
    server.send(shutdown_frame("req-2")).await;
    let (ack, _) = server.recv_type("ack").await;
    assert_eq!(ack["requestId"], json!("req-2"));
    let (code, stderr) = server.stop().await;
    assert_eq!(code, Some(0), "stderr: {stderr}");
}

#[tokio::test]
async fn submissions_in_one_scope_serialize_rather_than_overlap() {
    let scratch = Scratch::new("serve-scope-serial");
    // One worker, one scope: the second submission must wait for the first, and
    // both must be observable while that is true.
    let mut server = Server::start(&scratch, "turn-hang", &["--workers", "1"]).await;
    server
        .send(scoped_submit(
            "req-a",
            "same",
            &task("task-a", "hang", 60_000),
        ))
        .await;
    let (queued_a, _) = server.recv_type("queued").await;
    assert_eq!(queued_a["position"], json!(0));
    server
        .send(scoped_submit(
            "req-b",
            "same",
            &task("task-b", "hang", 60_000),
        ))
        .await;
    let (queued_b, _) = server.recv_type("queued").await;
    // Position counts submissions already waiting, not those already running: the
    // first was dispatched immediately, so the second is at the head of an empty
    // queue even though it cannot start yet.
    assert_eq!(queued_b["position"], json!(0));
    assert_eq!(queued_b["scope"], json!("same"));

    server.send(status_frame("req-status")).await;
    let (status, _) = server.recv_type("status").await;
    assert_eq!(status["queued"], json!(1));
    assert_eq!(status["running"], json!(["same"]));
    assert_eq!(status["workers"], json!(1));

    // Cancelling the first releases the scope and the second runs, so the records
    // arrive in submission order and never overlap.
    server.send(cancel_frame("req-c1", "req-a")).await;
    let (ack, _) = server.recv_type("ack").await;
    assert_eq!(ack["requestId"], json!("req-c1"));
    let (first, _) = server.recv_type("record").await;
    assert_eq!(first["requestId"], json!("req-a"));
    assert_eq!(first["record"]["status"], json!("cancelled"));

    server.send(status_frame("req-status-2")).await;
    let (status, _) = server.recv_type("status").await;
    assert_eq!(
        status["queued"],
        json!(0),
        "the queued job should have started"
    );
    assert_eq!(status["running"], json!(["same"]));

    server.send(cancel_frame("req-c2", "req-b")).await;
    let (second, _) = server.recv_type("record").await;
    assert_eq!(second["requestId"], json!("req-b"));
    assert_eq!(second["record"]["status"], json!("cancelled"));
    let _ = server.stop().await;
}

#[tokio::test]
async fn submissions_in_distinct_scopes_run_concurrently() {
    let scratch = Scratch::new("serve-scope-parallel");
    let mut server = Server::start(&scratch, "turn-hang", &["--workers", "2"]).await;
    server
        .send(scoped_submit(
            "req-p",
            "scope-p",
            &task("task-p", "hang", 60_000),
        ))
        .await;
    let _ = server.recv_type("queued").await;
    server
        .send(scoped_submit(
            "req-q",
            "scope-q",
            &task("task-q", "hang", 60_000),
        ))
        .await;
    let _ = server.recv_type("queued").await;

    server.send(status_frame("req-status")).await;
    let (status, _) = server.recv_type("status").await;
    // Both scopes are in flight at once, which is the whole point of naming them.
    let mut running: Vec<String> = status["running"]
        .as_array()
        .expect("running")
        .iter()
        .map(|value| value.as_str().expect("scope").to_owned())
        .collect();
    running.sort();
    assert_eq!(running, vec!["scope-p", "scope-q"]);
    assert_eq!(status["queued"], json!(0));

    server.send(cancel_frame("req-cp", "req-p")).await;
    let (first, _) = server.recv_type("record").await;
    assert_eq!(first["requestId"], json!("req-p"));
    server.send(cancel_frame("req-cq", "req-q")).await;
    let (second, _) = server.recv_type("record").await;
    assert_eq!(second["requestId"], json!("req-q"));
    let _ = server.stop().await;
}

#[tokio::test]
async fn a_full_queue_is_refused_rather_than_grown() {
    let scratch = Scratch::new("serve-queue-full");
    let mut server = Server::start(
        &scratch,
        "turn-hang",
        &["--workers", "1", "--queue-capacity", "1"],
    )
    .await;
    // One runs, one waits, the third has nowhere to go.
    server
        .send(scoped_submit(
            "req-1",
            "s1",
            &task("task-1", "hang", 60_000),
        ))
        .await;
    let _ = server.recv_type("queued").await;
    server
        .send(scoped_submit(
            "req-2",
            "s2",
            &task("task-2", "hang", 60_000),
        ))
        .await;
    let _ = server.recv_type("queued").await;
    server
        .send(scoped_submit(
            "req-3",
            "s3",
            &task("task-3", "hang", 60_000),
        ))
        .await;
    let (error, _) = server.recv_type("error").await;
    assert_eq!(error["requestId"], json!("req-3"));
    assert_eq!(error["code"], json!("queue_full"));

    // Refusing one submission must not disturb the ones already admitted.
    server.send(status_frame("req-status")).await;
    let (status, _) = server.recv_type("status").await;
    assert_eq!(status["queued"], json!(1));
    assert_eq!(status["queueCapacity"], json!(1));
    let _ = server.stop().await;
}

#[tokio::test]
async fn a_crashed_agent_releases_its_slot_and_is_never_re_dispatched() {
    let scratch = Scratch::new("serve-crash");
    let mut server = Server::start(&scratch, "crash-turn", &["--workers", "1"]).await;
    server
        .send(scoped_submit(
            "req-crash",
            "s",
            &task("task-crash", "crash", 60_000),
        ))
        .await;
    let _ = server.recv_type("queued").await;
    let (record, _) = server.recv_type("record").await;
    assert_eq!(record["record"]["status"], json!("failed"));
    assert_eq!(record["record"]["error"], json!("agent_exited"));

    // The slot came back: a second submission in the same scope still runs, which
    // is what proves the crash released it rather than leaking the worker.
    server
        .send(scoped_submit(
            "req-after",
            "s",
            &task("task-after", "crash", 60_000),
        ))
        .await;
    let _ = server.recv_type("queued").await;
    let (second, _) = server.recv_type("record").await;
    assert_eq!(second["requestId"], json!("req-after"));

    // And the loop is still serving.
    server.send(status_frame("req-status")).await;
    let (status, _) = server.recv_type("status").await;
    assert_eq!(status["queued"], json!(0));
    assert_eq!(status["running"], json!([]));

    // Neither turn was re-dispatched after its agent died.
    let log = FixtureLog::new(scratch.log());
    assert_eq!(
        log.method_count("session/prompt").expect("log"),
        2,
        "a crashed turn was retried"
    );
    let _ = server.stop().await;
}

#[tokio::test]
async fn cancelling_a_submission_that_is_not_in_flight_is_refused() {
    let scratch = Scratch::new("serve-cancel-unknown");
    let mut server = Server::start(&scratch, "normal", &[]).await;
    server.send(cancel_frame("req-x", "no-such-request")).await;
    let (error, _) = server.recv_type("error").await;
    assert_eq!(error["requestId"], json!("req-x"));
    assert_eq!(error["code"], json!("not_in_flight"));
    // The loop survives a refused cancel and still serves work.
    server
        .send(submit_frame("req-y", &task("task-y", "still here", 60_000)))
        .await;
    let (record, _) = server.recv_type("record").await;
    assert_eq!(record["record"]["status"], json!("completed"));
    let _ = server.stop().await;
}

#[tokio::test]
async fn a_runner_listing_is_answered_without_starting_an_agent() {
    let scratch = Scratch::new("serve-runners");
    let mut server = Server::start(&scratch, "normal", &[]).await;
    server.send(runners_frame("req-list")).await;
    let (listing, _) = server.recv_type("runners").await;
    assert_eq!(listing["requestId"], json!("req-list"));
    let runners = listing["runners"].as_array().expect("runners");
    assert!(runners.len() >= 6, "{runners:?}");
    assert!(
        runners
            .iter()
            .all(|runner| runner["available"].is_boolean()),
        "{runners:?}"
    );
    assert!(
        !scratch.log().exists(),
        "listing runners must not start an agent"
    );
    let _ = server.stop().await;
}

#[tokio::test]
async fn malformed_and_unsupported_frames_are_refused_and_the_loop_continues() {
    let scratch = Scratch::new("serve-bad-frames");
    let mut server = Server::start(&scratch, "normal", &[]).await;

    // A frame with no version at all is malformed rather than merely unsupported,
    // because there is nothing to negotiate against.
    server
        .send(json!({"type": "runners", "requestId": "no-version"}))
        .await;
    let (error, _) = server.recv_type("error").await;
    assert_eq!(error["code"], json!("invalid_frame"));
    assert_eq!(error["requestId"], json!("no-version"));

    server
        .send(json!({"version": 99, "type": "runners", "requestId": "wrong-version"}))
        .await;
    let (error, _) = server.recv_type("error").await;
    assert_eq!(error["code"], json!("unsupported_control_version"));
    assert_eq!(error["requestId"], json!("wrong-version"));

    server
        .send(json!({"version": 1, "type": "teleport", "requestId": "unknown-kind"}))
        .await;
    let (error, _) = server.recv_type("error").await;
    assert_eq!(error["code"], json!("unknown_request"));

    server
        .send(json!({"version": 1, "type": "cancel", "requestId": "no-target"}))
        .await;
    let (error, _) = server.recv_type("error").await;
    assert_eq!(error["code"], json!("invalid_request"));

    // Raw garbage that is not JSON at all is refused rather than crashing the loop.
    server
        .stdin
        .write_all(b"this is not json\n")
        .await
        .expect("write");
    server.stdin.flush().await.expect("flush");
    let (error, _) = server.recv_type("error").await;
    assert_eq!(error["code"], json!("invalid_frame"));

    // And the loop still serves real work afterwards.
    server.send(runners_frame("req-after")).await;
    let (listing, _) = server.recv_type("runners").await;
    assert_eq!(listing["requestId"], json!("req-after"));
    let _ = server.stop().await;
}

#[tokio::test]
async fn an_invalid_task_document_is_reported_as_a_record_not_an_error() {
    let scratch = Scratch::new("serve-bad-task");
    let mut server = Server::start(&scratch, "normal", &[]).await;
    // One shape for every task outcome, so a controlling application does not have
    // to handle a refusal and a failure as different frame types.
    let mut document = task("task-bad", "p", 60_000);
    document["cwd"] = json!("/etc");
    server.send(submit_frame("req-bad", &document)).await;
    let (record, _) = server.recv_type("record").await;
    assert_eq!(record["requestId"], json!("req-bad"));
    assert_eq!(record["record"]["status"], json!("invalid"));
    assert_eq!(record["record"]["error"], json!("invalid_task_json"));
    assert!(
        !scratch.log().exists(),
        "an invalid document must not start an agent"
    );
    let _ = server.stop().await;
}

#[tokio::test]
async fn end_of_input_ends_the_loop_cleanly() {
    let scratch = Scratch::new("serve-eof");
    let server = Server::start(&scratch, "normal", &[]).await;
    // Closing stdin is how a controlling application says it is finished.
    let (code, stderr) = server.stop().await;
    assert_eq!(code, Some(0), "stderr: {stderr}");
}
