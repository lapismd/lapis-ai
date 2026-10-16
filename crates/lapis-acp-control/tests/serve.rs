//! The control loop driven through the transport seam, in process.
//!
//! These tests exist because the seam is the claim. Driving `serve` over an
//! in-memory pair proves the loop depends only on `FrameSource` and `FrameSink`
//! and not on stdio, which is what makes a Unix socket or an in-process link a
//! transport addition rather than a rewrite. It is also far faster than spawning
//! the binary for every scheduler assertion.

use std::path::{Path, PathBuf};
use std::time::Duration;

use lapis_acp_agent::definition::{CommandSpec, McpTransport, RunnerDefinition};
use lapis_acp_agent::{Catalog, Tier};
use lapis_acp_control::invocation::InvocationConfig;
use lapis_acp_control::protocol::{
    CONTROL_VERSION, cancel_frame, runners_frame, scoped_submit_frame, shutdown_frame, status_frame,
};
use lapis_acp_control::serve::{ServeConfig, ServeExit, serve};
use lapis_acp_control::task::document_value;
use lapis_acp_control::transport::{LineSink, LineSource};
use lapis_acp_control::{DEFAULT_SCOPE, ServeLimits};
use lapis_acp_protocol::EnvVar;
use lapis_acp_protocol::session::SystemPromptTransport;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, duplex, split};

const CAPACITY: usize = 256 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(45);

/// One in-memory control session: the peer end plus the loop's join handle.
struct Session {
    stream: BufReader<DuplexStream>,
    handle: tokio::task::JoinHandle<ServeExit>,
    dir: PathBuf,
}

impl Session {
    /// Start the loop over one half of a duplex pair and keep the other half.
    async fn start(label: &str, mode: &str, limits: ServeLimits) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "lapis-acp-serve-{label}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("scratch directory");

        // A duplex endpoint is bidirectional, exactly like stdio. Splitting it
        // gives the loop a read half for its source and a write half for its sink,
        // which is the whole reason the seam is two traits rather than one.
        let (peer_side, serve_side) = duplex(CAPACITY);
        let (serve_read, serve_write) = split(serve_side);
        let source = LineSource::new(BufReader::new(serve_read));
        let sink = LineSink::shared(serve_write);
        let config = configuration(&dir, mode, limits);
        let handle = tokio::spawn(async move {
            let mut source = source;
            serve(&mut source, sink, config).await
        });

        Self {
            stream: BufReader::new(peer_side),
            handle,
            dir,
        }
    }

    async fn send(&mut self, frame: Value) {
        let mut line = frame.to_string();
        line.push('\n');
        self.stream
            .get_mut()
            .write_all(line.as_bytes())
            .await
            .expect("write");
        self.stream.get_mut().flush().await.expect("flush");
    }

    async fn recv(&mut self) -> Value {
        let mut line = String::new();
        let read = tokio::time::timeout(READ_TIMEOUT, self.stream.read_line(&mut line))
            .await
            .expect("the loop stopped answering")
            .expect("read");
        assert!(read > 0, "the control stream closed");
        serde_json::from_str(line.trim())
            .unwrap_or_else(|error| panic!("not a frame ({error}): {line:?}"))
    }

    async fn recv_type(&mut self, wanted: &str) -> Value {
        loop {
            let frame = self.recv().await;
            if frame["type"] == json!(wanted) {
                return frame;
            }
        }
    }

    /// Close the peer end and report how the loop exited.
    ///
    /// Dropping the stream is what produces end of input, so the whole reader has
    /// to go rather than a borrowed handle to it.
    async fn finish(self) -> ServeExit {
        let Self {
            stream,
            handle,
            dir,
        } = self;
        drop(stream);
        let exit = tokio::time::timeout(READ_TIMEOUT, handle)
            .await
            .expect("the loop did not exit")
            .expect("join");
        let _ = std::fs::remove_dir_all(&dir);
        exit
    }
}

fn configuration(dir: &Path, mode: &str, limits: ServeLimits) -> ServeConfig {
    let definition = RunnerDefinition::try_from_parts(
        "fixture".to_owned(),
        "Fixture".to_owned(),
        vec![CommandSpec::with_args(
            lapis_acp_testkit::python(),
            &[lapis_acp_testkit::fixture_script().to_string_lossy()],
        )],
        vec![
            EnvVar::new("JJ_ACP_FIXTURE_MODE", mode),
            EnvVar::new(
                "JJ_ACP_FIXTURE_LOG",
                dir.join("agent.log").to_string_lossy(),
            ),
        ],
        McpTransport::Stdio,
    )
    .expect("valid runner");
    let mut catalog = Catalog::empty();
    catalog
        .extend(Tier::ConfigFile, vec![definition])
        .expect("extends");
    ServeConfig {
        limits,
        invocation: InvocationConfig {
            mcp_servers: vec![],
            catalog,
            configured_runner: None,
            cwd: dir.to_path_buf(),
            idle: Duration::from_secs(30),
            absolute_cap: Duration::from_secs(60),
            cancel_grace: Duration::from_secs(2),
            reap: Duration::from_secs(2),
            configuration: Default::default(),
            system_prompt: None,
            prompt_transport: SystemPromptTransport::Field,
            inherited_env: Vec::new(),
        },
    }
}

fn one_worker() -> ServeLimits {
    ServeLimits {
        workers: 1,
        queue_capacity: 8,
    }
}

#[tokio::test]
async fn the_ready_frame_announces_sizing_and_the_catalogue() {
    let mut session = Session::start("ready", "normal", one_worker()).await;
    let ready = session.recv().await;
    assert_eq!(ready["type"], json!("ready"));
    assert_eq!(ready["version"], json!(CONTROL_VERSION));
    assert_eq!(ready["workers"], json!(1));
    assert_eq!(ready["queueCapacity"], json!(8));
    let runners = ready["runners"].as_array().expect("runners");
    assert_eq!(runners.len(), 1, "{runners:?}");
    assert_eq!(runners[0]["id"], json!("fixture"));
    assert_eq!(runners[0]["available"], json!(true));
    assert_eq!(session.finish().await, ServeExit::Eof);
}

#[tokio::test]
async fn a_submission_completes_and_the_loop_keeps_serving() {
    let mut session = Session::start("submit", "normal", one_worker()).await;
    let _ = session.recv().await;
    session
        .send(scoped_submit_frame(
            "r1",
            DEFAULT_SCOPE,
            &document_value("task-1", "fixture", "work", 60_000),
        ))
        .await;
    let queued = session.recv_type("queued").await;
    assert_eq!(queued["scope"], json!(DEFAULT_SCOPE));
    let record = session.recv_type("record").await;
    assert_eq!(record["requestId"], json!("r1"));
    assert_eq!(record["record"]["status"], json!("completed"));

    session.send(runners_frame("r2")).await;
    let listing = session.recv_type("runners").await;
    assert_eq!(listing["requestId"], json!("r2"));

    session.send(shutdown_frame("r3")).await;
    let ack = session.recv_type("ack").await;
    assert_eq!(ack["requestId"], json!("r3"));
    assert_eq!(session.finish().await, ServeExit::Shutdown);
}

#[tokio::test]
async fn one_scope_serializes_while_distinct_scopes_overlap() {
    let mut session = Session::start(
        "scopes",
        "turn-hang",
        ServeLimits {
            workers: 2,
            queue_capacity: 8,
        },
    )
    .await;
    let _ = session.recv().await;

    // Two submissions in one scope: the second must wait even though a worker is
    // free, because the scope is the unit of serialization.
    session
        .send(scoped_submit_frame(
            "a1",
            "same",
            &document_value("t-a1", "fixture", "x", 60_000),
        ))
        .await;
    let _ = session.recv_type("queued").await;
    session
        .send(scoped_submit_frame(
            "a2",
            "same",
            &document_value("t-a2", "fixture", "x", 60_000),
        ))
        .await;
    let _ = session.recv_type("queued").await;
    session.send(status_frame("s1")).await;
    let status = session.recv_type("status").await;
    assert_eq!(status["running"], json!(["same"]));
    assert_eq!(status["queued"], json!(1));

    // A different scope runs immediately on the second worker.
    session
        .send(scoped_submit_frame(
            "b1",
            "other",
            &document_value("t-b1", "fixture", "x", 60_000),
        ))
        .await;
    let _ = session.recv_type("queued").await;
    session.send(status_frame("s2")).await;
    let status = session.recv_type("status").await;
    let mut running: Vec<String> = status["running"]
        .as_array()
        .expect("running")
        .iter()
        .map(|value| value.as_str().expect("scope").to_owned())
        .collect();
    running.sort();
    assert_eq!(running, vec!["other", "same"]);
    assert_eq!(status["queued"], json!(1));

    // Releasing the first scope lets its queued sibling run.
    session.send(cancel_frame("c1", "a1")).await;
    let first = session.recv_type("record").await;
    assert_eq!(first["requestId"], json!("a1"));
    assert_eq!(first["record"]["status"], json!("cancelled"));
    session.send(status_frame("s3")).await;
    let status = session.recv_type("status").await;
    assert_eq!(
        status["queued"],
        json!(0),
        "the queued sibling never started"
    );

    session.send(cancel_frame("c2", "a2")).await;
    let _ = session.recv_type("record").await;
    session.send(cancel_frame("c3", "b1")).await;
    let _ = session.recv_type("record").await;
    let _ = session.finish().await;
}

#[tokio::test]
async fn a_crashed_agent_releases_its_slot_without_re_dispatch() {
    let mut session = Session::start("crash", "crash-turn", one_worker()).await;
    let _ = session.recv().await;
    session
        .send(scoped_submit_frame(
            "r1",
            "s",
            &document_value("task-crash", "fixture", "crash", 60_000),
        ))
        .await;
    let record = session.recv_type("record").await;
    assert_eq!(record["record"]["status"], json!("failed"));
    assert_eq!(record["record"]["error"], json!("agent_exited"));

    // The slot came back, so a second submission in the same scope still runs.
    session
        .send(scoped_submit_frame(
            "r2",
            "s",
            &document_value("task-after", "fixture", "crash", 60_000),
        ))
        .await;
    let second = session.recv_type("record").await;
    assert_eq!(second["requestId"], json!("r2"));

    session.send(status_frame("s1")).await;
    let status = session.recv_type("status").await;
    assert_eq!(status["running"], json!([]), "the scope was never released");
    assert_eq!(status["queued"], json!(0));

    let log = lapis_acp_testkit::FixtureLog::new(session.dir.join("agent.log"));
    assert_eq!(
        log.method_count("session/prompt").expect("log"),
        2,
        "a crashed turn was re-dispatched"
    );
    let _ = session.finish().await;
}

#[tokio::test]
async fn a_malformed_frame_never_stops_the_loop() {
    let mut session = Session::start("malformed", "normal", one_worker()).await;
    let _ = session.recv().await;
    for line in ["not json at all\n", "{\"type\":\"runners\"}\n", "\n"] {
        session
            .stream
            .get_mut()
            .write_all(line.as_bytes())
            .await
            .expect("write");
    }
    session.stream.get_mut().flush().await.expect("flush");
    let first = session.recv_type("error").await;
    assert_eq!(first["code"], json!("invalid_frame"));
    let second = session.recv_type("error").await;
    assert_eq!(second["code"], json!("invalid_frame"));
    // Still serving afterwards.
    session.send(status_frame("ok")).await;
    let status = session.recv_type("status").await;
    assert_eq!(status["requestId"], json!("ok"));
    let _ = session.finish().await;
}
