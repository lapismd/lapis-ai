//! Driving the real fixture agent through the protocol crate.
//!
//! These tests spawn an actual subprocess and speak ACP to it over its stdio.
//! They need Python 3 and nothing else: no model provider, no network endpoint
//! and no credential, which is the property that lets every lane run them.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use lapis_acp_protocol::budget::TurnBudget;
use lapis_acp_protocol::handler::{RecordingHandler, SessionHandler};
use lapis_acp_protocol::initialize::{ClientCapabilities, ClientInfo};
use lapis_acp_protocol::permission::{PermissionDecision, PermissionRequest};
use lapis_acp_protocol::session::{SessionNewRequest, SystemPrompt, SystemPromptTransport};
use lapis_acp_protocol::{Connection, Error, StopReason};
use lapis_acp_testkit::{Fixture, FixtureLog, Mode};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A scratch directory that removes itself, so a failed test leaves no litter.
struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let sequence = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "lapis-acp-fixture-{label}-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("scratch directory");
        Self(path)
    }

    fn log(&self) -> FixtureLog {
        FixtureLog::new(self.0.join("agent.log"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn budget() -> TurnBudget {
    TurnBudget::new(Duration::from_secs(20), Duration::from_secs(60))
}

fn session_request() -> SessionNewRequest<'static> {
    SessionNewRequest {
        cwd: "/tmp",
        mcp_servers: vec![],
        system_prompt: Some(SystemPrompt {
            transport: SystemPromptTransport::Field,
            text: "standing instructions",
        }),
        session_title: None,
    }
}

/// Launch the fixture with the harness-reserved variables a real run supplies.
async fn launch(fixture: &Fixture) -> (Connection<ChildStdout, ChildStdin>, Child) {
    let spec = fixture.command();
    let mut command = Command::new(&spec.command);
    command
        .args(&spec.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // Inherited stderr keeps fixture diagnostics out of the protocol stream.
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    for variable in fixture.environment() {
        command.env(&variable.name, &variable.value);
    }
    command
        .env("JJ_ACP_TASK_RUNNER", "fixture")
        .env("JJ_ACP_TASK_ID", "task-42");
    let mut child = command.spawn().expect("fixture agent spawns");
    let stdout = child.stdout.take().expect("fixture stdout");
    let stdin = child.stdin.take().expect("fixture stdin");
    (Connection::new(stdout, stdin), child)
}

async fn negotiate<H: SessionHandler>(
    connection: &mut Connection<ChildStdout, ChildStdin>,
    handler: &mut H,
) -> String {
    connection
        .initialize(
            handler,
            &ClientInfo::lapis_acp(),
            ClientCapabilities::default(),
        )
        .await
        .expect("initialize");
    let request = session_request();
    connection
        .session_new(handler, &request)
        .await
        .expect("session/new")
        .id
}

fn log_path(scratch: &Scratch) -> PathBuf {
    scratch.log().path().to_path_buf()
}

fn wait_for_log(path: &Path) {
    for _ in 0..200 {
        if path.exists() && std::fs::metadata(path).is_ok_and(|m| m.len() > 0) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("fixture never wrote {}", path.display());
}

#[tokio::test]
async fn the_fixture_completes_a_turn_without_a_model_provider() {
    let scratch = Scratch::new("normal");
    let log = scratch.log();
    let fixture = Fixture::new(log_path(&scratch));
    let (mut connection, mut child) = launch(&fixture).await;
    let mut handler = RecordingHandler::default();

    let negotiated = connection
        .initialize(
            &mut handler,
            &ClientInfo::lapis_acp(),
            ClientCapabilities::default(),
        )
        .await
        .expect("initialize");
    assert_eq!(negotiated.protocol_version, 2);
    assert_eq!(negotiated.agent_name.as_deref(), Some("lapis-acp-fixture"));

    let request = session_request();
    let session = connection
        .session_new(&mut handler, &request)
        .await
        .expect("session/new");
    assert!(session.id.starts_with("fixture-"));
    // The advertised model and effort catalogues are discoverable from the raw
    // session result, which is why it is retained.
    assert_eq!(
        lapis_acp_protocol::config_option::thought_level_config_id(&session.raw).as_deref(),
        Some("effort")
    );

    let reason = connection
        .session_prompt(&mut handler, &session.id, &["prepared task text"], budget())
        .await
        .expect("prompt");
    assert_eq!(reason, StopReason::EndTurn);
    assert!(
        handler.updates.iter().any(|update| matches!(
            update,
            lapis_acp_protocol::SessionUpdate::AgentMessageChunk { .. }
        )),
        "the streamed chunk never reached the handler"
    );

    child.kill().await.ok();
    wait_for_log(log.path());
    let launch = log.launch().expect("launch record");
    assert_eq!(launch["mode"], "normal");
    assert_eq!(launch["runner"], "fixture");
    assert_eq!(launch["taskId"], "task-42");
    let reserved: Vec<String> = launch["reserved"]
        .as_array()
        .expect("reserved")
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();
    assert!(
        reserved.contains(&"JJ_ACP_TASK_RUNNER".to_owned()),
        "{reserved:?}"
    );
    assert!(
        reserved.contains(&"JJ_ACP_TASK_ID".to_owned()),
        "{reserved:?}"
    );
    // The fixture saw the prompt, which is what makes the log usable as evidence.
    assert!(log.contains("prepared task text").expect("log scan"));
}

#[tokio::test]
async fn a_reported_stop_reason_reaches_the_caller_unchanged() {
    let scratch = Scratch::new("refusal");
    let fixture = Fixture::new(log_path(&scratch)).with_stop_reason("refusal");
    let (mut connection, mut child) = launch(&fixture).await;
    let mut handler = RecordingHandler::default();
    let session_id = negotiate(&mut connection, &mut handler).await;
    let reason = connection
        .session_prompt(&mut handler, &session_id, &["x"], budget())
        .await
        .expect("prompt");
    assert_eq!(reason, StopReason::Refusal);
    child.kill().await.ok();
}

#[tokio::test]
async fn a_version_one_agent_is_accepted_and_recorded() {
    let scratch = Scratch::new("v1");
    let fixture = Fixture::new(log_path(&scratch)).with_protocol(1);
    let (mut connection, mut child) = launch(&fixture).await;
    let mut handler = RecordingHandler::default();
    let negotiated = connection
        .initialize(
            &mut handler,
            &ClientInfo::lapis_acp(),
            ClientCapabilities::default(),
        )
        .await
        .expect("initialize");
    assert_eq!(negotiated.protocol_version, 1);
    assert_eq!(connection.negotiated_version(), Some(1));
    child.kill().await.ok();
}

#[tokio::test]
async fn an_agent_error_arrives_typed_with_its_marker_intact() {
    let scratch = Scratch::new("error");
    let fixture = Fixture::new(log_path(&scratch)).with_mode(Mode::Error);
    let (mut connection, mut child) = launch(&fixture).await;
    let mut handler = RecordingHandler::default();
    let session_id = negotiate(&mut connection, &mut handler).await;
    let error = connection
        .session_prompt(&mut handler, &session_id, &["x"], budget())
        .await
        .expect_err("agent error");
    // The protocol layer preserves the agent's own text so a caller can act on
    // the code. Keeping that text out of a terminal record is the control layer's
    // job, and it is specified separately rather than assumed here.
    match error {
        Error::AgentError { code, message } => {
            assert_eq!(code, -32000);
            assert!(message.contains("SECRET_TASK_TEXT"), "{message}");
        }
        other => panic!("unexpected {other:?}"),
    }
    child.kill().await.ok();
}

/// Approves by semantic kind, the only way a caller should select an option.
struct Approver;

impl SessionHandler for Approver {
    fn permission(
        &mut self,
        request: PermissionRequest,
    ) -> impl std::future::Future<Output = PermissionDecision> + Send {
        let decision = match request.option_with_kind("allow_once") {
            Some(option) => PermissionDecision::Select(option.option_id.clone()),
            None => PermissionDecision::Cancel,
        };
        async move { decision }
    }
}

#[tokio::test]
async fn a_permission_request_is_answered_and_the_turn_completes() {
    let scratch = Scratch::new("permission");
    let log = scratch.log();
    let fixture = Fixture::new(log_path(&scratch)).with_mode(Mode::Permission);
    let (mut connection, mut child) = launch(&fixture).await;
    let mut handler = Approver;
    let session_id = negotiate(&mut connection, &mut handler).await;
    let reason = connection
        .session_prompt(&mut handler, &session_id, &["x"], budget())
        .await
        .expect("prompt after approval");
    assert_eq!(reason, StopReason::EndTurn);
    child.kill().await.ok();
    wait_for_log(log.path());
    let answer = log.permission_answer().expect("log").expect("answer");
    assert_eq!(
        answer["outcome"],
        serde_json::json!({"outcome": "selected", "optionId": "allow"})
    );
}

#[tokio::test]
async fn a_silent_agent_is_bounded_by_the_idle_window() {
    let scratch = Scratch::new("hang");
    let fixture = Fixture::new(log_path(&scratch)).with_mode(Mode::StartupHang);
    let (mut connection, mut child) = launch(&fixture).await;
    let mut handler = RecordingHandler::default();
    let error = connection
        .call_within(
            &mut handler,
            "initialize",
            serde_json::json!({"protocolVersion": 2}),
            TurnBudget::new(Duration::from_millis(400), Duration::from_secs(10)),
        )
        .await
        .expect_err("silent agent must be bounded");
    assert!(
        matches!(error, Error::IdleTimeout(_)),
        "expected an idle timeout, got {error}"
    );
    child.kill().await.ok();
}

#[tokio::test]
async fn killing_the_agent_alone_leaves_its_descendant_running() {
    let scratch = Scratch::new("descendant");
    let log = scratch.log();
    let fixture = Fixture::new(log_path(&scratch)).with_mode(Mode::Descendant);
    let (mut connection, mut child) = launch(&fixture).await;
    let mut handler = RecordingHandler::default();
    let _session_id = negotiate(&mut connection, &mut handler).await;
    wait_for_log(log.path());
    let pid = log.descendant().expect("log").expect("descendant pid");
    assert!(pid > 0, "the fixture did not record its descendant");
    assert!(
        process_exists(pid),
        "descendant should be running before the kill"
    );

    child.kill().await.expect("kill");
    // This is the hazard JAC-RUN-007 exists for: killing the direct child leaves
    // an orphaned descendant alive, so cleanup must own the process group rather
    // than the child. The assertion is the evidence, not a tautology.
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        process_exists(pid),
        "the descendant died with its parent, so process-group cleanup would be untestable"
    );
    terminate(pid);
}

/// Report whether a pid still exists.
fn process_exists(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Reap a descendant this test orphaned, so a run leaves nothing behind.
fn terminate(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
