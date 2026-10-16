//! Resource bounds: frame cap, idle and absolute turn budgets, write timeout,
//! stream end and tolerance of non-frame noise on an agent's stdout.
//!
//! Every bound here protects against a malfunctioning or hostile agent. Each is
//! enforced before untrusted bytes are buffered, and each is reported as a
//! distinct outcome rather than a generic failure.

use std::time::Duration;

use lapis_acp_protocol::budget::{Limits, TurnBudget};
use lapis_acp_protocol::handler::RecordingHandler;
use lapis_acp_protocol::{Connection, Error, FrameReader};
use serde_json::json;
use tokio::io::{AsyncWriteExt, BufReader, duplex, split};

mod common;

use common::{CAPACITY, Harness, response};

#[tokio::test]
async fn an_oversized_frame_is_refused_before_it_can_be_buffered() {
    let mut harness = Harness::with_limits(Limits {
        max_frame_bytes: 32,
        ..Limits::default()
    });
    harness.send_line(&format!("{}\n", "x".repeat(4096))).await;
    let mut handler = RecordingHandler::default();
    let error = harness
        .connection
        .call(&mut handler, "initialize", json!({}))
        .await
        .expect_err("oversized frame");
    assert!(
        matches!(error, Error::FrameTooLarge { limit: 32 }),
        "{error}"
    );
}

#[tokio::test]
async fn unparseable_lines_are_skipped_and_counted_while_blanks_are_not() {
    let mut harness = Harness::new();
    harness.send_line("\n").await;
    harness.send_line("Loading model...\n").await;
    harness.send_line("   \n").await;
    harness.send_line("{truncated json\n").await;
    harness.send(response(1, json!({"ok": true}))).await;
    let mut handler = RecordingHandler::default();
    let result = harness
        .connection
        .call(&mut handler, "initialize", json!({}))
        .await
        .expect("call survives noise");
    assert_eq!(result, json!({"ok": true}));
    // Blank lines are frame separators rather than discarded frames, so only the
    // two genuinely unparseable lines are counted.
    assert_eq!(harness.connection.skipped_frames(), 2);
}

#[tokio::test(start_paused = true)]
async fn the_idle_bound_reports_silence_while_the_absolute_bound_caps_noise() {
    let mut idle = Harness::new();
    let mut handler = RecordingHandler::default();
    let error = idle
        .connection
        .session_prompt(
            &mut handler,
            "s",
            &["x"],
            TurnBudget::new(Duration::from_secs(2), Duration::from_secs(60)),
        )
        .await
        .expect_err("idle timeout");
    assert!(
        matches!(error, Error::IdleTimeout(d) if d == Duration::from_secs(2)),
        "{error}"
    );

    // An agent that streams forever still stops at the absolute cap.
    let (client, mut agent) = duplex(CAPACITY);
    let (reader, writer) = split(client);
    let mut connection = Connection::new(reader, writer);
    let noise = tokio::spawn(async move {
        let frame = serde_json::to_vec(&json!({
            "jsonrpc": "2.0", "method": "session/update",
            "params": {"update": {"sessionUpdate": "agent_message_chunk"}}
        }))
        .unwrap();
        loop {
            if agent.write_all(&frame).await.is_err() {
                return;
            }
            agent.write_all(b"\n").await.ok();
            agent.flush().await.ok();
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
    let mut noisy = RecordingHandler::default();
    let error = connection
        .session_prompt(
            &mut noisy,
            "s",
            &["x"],
            TurnBudget::new(Duration::from_secs(30), Duration::from_secs(3)),
        )
        .await
        .expect_err("absolute cap");
    assert!(
        matches!(error, Error::Timeout(d) if d == Duration::from_secs(3)),
        "{error}"
    );
    assert!(!noisy.updates.is_empty(), "the noise was never delivered");
    noise.abort();
}

#[tokio::test(start_paused = true)]
async fn an_agent_that_stops_reading_is_reported_as_a_write_timeout() {
    // A one-byte pipe fills immediately, so the write can never complete.
    let (client, _agent) = duplex(1);
    let (reader, writer) = split(client);
    let mut connection = Connection::with_limits(
        reader,
        writer,
        Limits {
            write_timeout: Duration::from_secs(1),
            ..Limits::default()
        },
    );
    let mut handler = RecordingHandler::default();
    let payload = "y".repeat(4096);
    let error = connection
        .call(&mut handler, "session/prompt", json!({"big": payload}))
        .await
        .expect_err("write timeout");
    assert!(
        matches!(error, Error::WriteTimeout(d) if d == Duration::from_secs(1)),
        "{error}"
    );
}

#[tokio::test]
async fn a_closed_agent_stream_is_reported_not_hung() {
    // Shut down only the agent's write side, so the client sees end of stream
    // while its own writes still have somewhere to go. Dropping the whole stream
    // would break the client's writes first and report a broken pipe, which is a
    // different and less useful diagnosis.
    let (client, mut agent) = duplex(CAPACITY);
    agent.shutdown().await.expect("agent shutdown");
    let _still_open = agent;
    let (reader, writer) = split(client);
    let mut connection = Connection::new(reader, writer);
    let mut handler = RecordingHandler::default();
    let error = connection
        .call(&mut handler, "initialize", json!({}))
        .await
        .expect_err("closed stream");
    assert!(matches!(error, Error::AgentExited), "{error}");
}

#[tokio::test]
async fn an_ended_stream_yields_no_further_frames() {
    let (client, agent) = duplex(CAPACITY);
    drop(agent);
    let mut reader = FrameReader::new(BufReader::new(client));
    assert!(reader.next().await.expect("read").is_none());

    // A partial line without a trailing newline is still delivered at the end.
    let (client, mut agent) = duplex(CAPACITY);
    agent
        .write_all(b"{\"jsonrpc\":\"2.0\"}")
        .await
        .expect("write");
    drop(agent);
    let mut partial = FrameReader::new(BufReader::new(client));
    assert_eq!(
        partial.next().await.expect("read"),
        Some(json!({"jsonrpc": "2.0"}))
    );
    assert!(partial.next().await.expect("read").is_none());
}

#[tokio::test]
async fn a_read_cancelled_mid_frame_resumes_instead_of_losing_bytes() {
    // This is the property that makes racing a turn against a cancellation
    // signal safe: dropping a pending read must not discard the bytes of a frame
    // that has already partly arrived.
    let (client, mut agent) = duplex(CAPACITY);
    let mut reader = FrameReader::new(BufReader::new(client));
    agent
        .write_all(b"{\"jsonrpc\": ")
        .await
        .expect("first half");
    agent.flush().await.expect("flush");

    let pending = tokio::time::timeout(Duration::from_millis(50), reader.next());
    assert!(
        pending.await.is_err(),
        "the read should still be waiting for the rest of the frame"
    );

    agent
        .write_all(b"\"2.0\", \"id\": 1, \"result\": {\"ok\": true}}\n")
        .await
        .expect("second half");
    agent.flush().await.expect("flush");
    assert_eq!(
        reader.next().await.expect("resumed read"),
        Some(json!({"jsonrpc": "2.0", "id": 1, "result": {"ok": true}}))
    );
}
