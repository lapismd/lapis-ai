#![cfg(unix)]
mod common;
use common::Scratch;
use lapis_ai_runtime::{
    AcpSession, Environment, PreparedLaunch, SessionOptions, SessionRequest, SessionSnapshot,
};
use serde_json::{Value, json};
use std::time::Duration;

async fn start(scratch: &Scratch, handle: &str) -> AcpSession {
    start_with_fixture(scratch, handle, "fleet_agent.py").await
}
async fn start_with_fixture(scratch: &Scratch, handle: &str, name: &str) -> AcpSession {
    let root = scratch.join("vault");
    std::fs::create_dir_all(&root).unwrap();
    let fixture = lapis_acp_testkit::fixture_script().with_file_name(name);
    let catalog = scratch.join("catalog.json");
    std::fs::write(&catalog, json!({"runners":[{"id":"fixture","label":"Fixture","command":lapis_acp_testkit::python(),"args":[fixture],"env":{"FLEET_LOG":scratch.log()}}]}).to_string()).unwrap();
    let mut environment = Environment::default();
    for (key, value) in [
        ("PATH", std::env::var("PATH").unwrap()),
        ("JJ_ACP_RUNNER", "fixture".into()),
        ("JJ_ACP_CATALOG", catalog.to_string_lossy().into()),
        ("JJ_ACP_CANCEL_GRACE", "1".into()),
        ("JJ_ACP_REAP_TIMEOUT", "1".into()),
    ] {
        environment.0.insert(key.into(), value);
    }
    let plan = PreparedLaunch {
        executable: common::binary().into(),
        arguments: Vec::new(),
        directory: root.clone(),
        environment,
    };
    let mut options = SessionOptions::new(handle.into(), root);
    options.request_timeout = Duration::from_secs(5);
    AcpSession::start(plan, options).await.unwrap()
}
async fn until(session: &AcpSession, id: &str, kind: &str) -> SessionSnapshot {
    let mut changed = session.subscribe().unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let snapshot = session.snapshot(0).unwrap();
            if snapshot
                .events
                .iter()
                .any(|event| event.frame["requestId"] == id && event.frame["type"] == kind)
            {
                return snapshot;
            }
            assert!(
                !snapshot.closed,
                "unexpected closure: {:?}",
                snapshot.failure
            );
            changed.changed().await.unwrap();
        }
    })
    .await
    .unwrap()
}
async fn configure(session: &AcpSession) {
    session
        .submit(
            "configure".into(),
            SessionRequest::Configure {
                model: None,
                effort: None,
                mode: Some("ask".into()),
            },
        )
        .await
        .unwrap();
    let snapshot = until(session, "configure", "configured").await;
    assert!(
        snapshot
            .events
            .iter()
            .any(|event| event.frame["modeReady"] == true)
    );
}
fn prompt(text: &str) -> SessionRequest {
    SessionRequest::Prompt {
        text: text.into(),
        context: Some("<context>owned facts</context>".into()),
        max_duration_ms: 10_000,
    }
}

#[tokio::test]
async fn actual_control_v3_preserves_context_permission_options_and_follow_up() {
    let scratch = Scratch::new("owned-session");
    let session = start(&scratch, "opaque").await;
    let before = std::fs::read_to_string(scratch.log()).unwrap();
    assert_eq!(
        before.lines().filter(|line| *line == "initialize").count(),
        1
    );
    assert!(!before.contains("session/prompt"));
    assert_eq!(session.facts()["adapterSessionId"], "adapter-1");
    configure(&session).await;
    session
        .submit("turn".into(), prompt("permission"))
        .await
        .unwrap();
    let pending = until(&session, "turn", "permission").await;
    let permission = pending
        .events
        .iter()
        .find(|event| event.frame["type"] == "permission")
        .unwrap();
    assert_eq!(
        permission.frame["request"]["options"][0]["optionId"],
        "once"
    );
    let permission_id = permission.frame["permissionId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        session
            .submit(
                "invented".into(),
                SessionRequest::Permission {
                    permission_id: permission_id.clone(),
                    option_id: Some("allow_once".into())
                }
            )
            .await
            .is_err()
    );
    session
        .submit(
            "answer".into(),
            SessionRequest::Permission {
                permission_id,
                option_id: Some("once".into()),
            },
        )
        .await
        .unwrap();
    let result = until(&session, "turn", "record").await;
    let terminal = result
        .events
        .iter()
        .find(|event| event.frame["type"] == "record")
        .unwrap();
    assert_eq!(terminal.frame["record"]["status"], "completed");
    assert_eq!(terminal.frame["sessionUsable"], true);
    session
        .submit("follow-up".into(), prompt("follow-up"))
        .await
        .unwrap();
    until(&session, "follow-up", "record").await;
    session.close().await.unwrap();
    let log = std::fs::read_to_string(scratch.log()).unwrap();
    assert!(log.contains("decision:once"));
    assert_eq!(log.lines().filter(|line| *line == "session/new").count(), 1);
    let prompts: Vec<Value> = log
        .lines()
        .filter_map(|line| line.strip_prefix("prompt-blocks:"))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(prompts.len(), 2);
    assert_eq!(
        prompts[0][0]["resource"]["text"],
        "<context>owned facts</context>"
    );
    assert_eq!(prompts[0][1]["text"], "permission");
}

#[tokio::test]
async fn provider_scope_failure_is_explicit_and_does_not_retry_the_turn() {
    let scratch = Scratch::new("owned-foreign");
    let session = start(&scratch, "fenced").await;
    configure(&session).await;
    session
        .submit("turn".into(), prompt("foreign"))
        .await
        .unwrap();
    let terminal = until(&session, "turn", "record").await;
    let record = terminal
        .events
        .iter()
        .find(|event| event.frame["type"] == "record")
        .unwrap();
    assert_eq!(record.frame["record"]["status"], "outcome_unknown");
    assert_eq!(record.frame["sessionUsable"], false);
    assert!(
        session
            .submit("retry".into(), prompt("hello"))
            .await
            .is_err()
    );
    session.close().await.unwrap();
    let log = std::fs::read_to_string(scratch.log()).unwrap();
    assert_eq!(
        log.lines().filter(|line| *line == "session/prompt").count(),
        1
    );
}

#[tokio::test]
async fn closing_a_real_pending_permission_worker_does_not_affect_another_worker() {
    let one = Scratch::new("owned-first");
    let two = Scratch::new("owned-second");
    let first = start(&one, "first").await;
    let second = start(&two, "second").await;
    configure(&first).await;
    configure(&second).await;
    first
        .submit("turn".into(), prompt("permission"))
        .await
        .unwrap();
    until(&first, "turn", "permission").await;
    first.close().await.unwrap();
    let closed = first.snapshot(0).unwrap();
    assert!(closed.closed);
    assert_eq!(closed.unresolved_requests, ["turn"]);
    second.submit("turn".into(), prompt("hello")).await.unwrap();
    until(&second, "turn", "record").await;
    assert!(!second.snapshot(0).unwrap().closed);
    second.close().await.unwrap();
}

#[tokio::test]
async fn cancelled_startup_stops_the_real_provider_process_group() {
    let scratch = Scratch::new("owned-cancel-start");
    let root = scratch.join("vault");
    std::fs::create_dir_all(&root).unwrap();
    let pid_file = scratch.join("provider.pid");
    let fixture = scratch.join("blocked.py");
    std::fs::write(
        &fixture,
        format!(
            "import os,time\nopen({},'w').write(str(os.getpid()))\ntime.sleep(120)\n",
            json!(pid_file)
        ),
    )
    .unwrap();
    let catalog = scratch.join("catalog.json");
    std::fs::write(&catalog, json!({"runners":[{"id":"fixture","label":"Fixture","command":lapis_acp_testkit::python(),"args":[fixture],"env":{}}]}).to_string()).unwrap();
    let mut environment = Environment::default();
    for (key, value) in [
        ("PATH", std::env::var("PATH").unwrap()),
        ("JJ_ACP_RUNNER", "fixture".into()),
        ("JJ_ACP_CATALOG", catalog.to_string_lossy().into()),
        ("JJ_ACP_CANCEL_GRACE", "1".into()),
        ("JJ_ACP_REAP_TIMEOUT", "1".into()),
    ] {
        environment.0.insert(key.into(), value);
    }
    let launch = PreparedLaunch {
        executable: common::binary().into(),
        arguments: vec![],
        directory: root.clone(),
        environment,
    };
    let startup = tokio::spawn(AcpSession::start(
        launch,
        SessionOptions::new("cancelled".into(), root),
    ));
    tokio::time::timeout(Duration::from_secs(5), async {
        while !pid_file.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    startup.abort();
    let _ = startup.await;
    let stopped = tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if !std::process::Command::new("/bin/kill")
                .args(["-0", &pid])
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap()
                .success()
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .is_ok();
    if !stopped {
        // Bound the regression itself: never leave its deliberately blocked peer.
        let _ = std::process::Command::new("/bin/kill")
            .args(["-KILL", &pid])
            .status();
    }
    assert!(stopped, "cancelled startup orphaned the provider");
}

#[tokio::test]
async fn actual_cancellation_retires_permission_and_allows_an_explicit_follow_up() {
    let scratch = Scratch::new("owned-cancel");
    let session = start_with_fixture(&scratch, "cancel", "interactive_agent.py").await;
    configure(&session).await;
    session
        .submit(
            "turn".into(),
            SessionRequest::Prompt {
                text: "wait".into(),
                context: None,
                max_duration_ms: 10_000,
            },
        )
        .await
        .unwrap();
    let pending = until(&session, "turn", "permission").await;
    let permission_id = pending
        .events
        .iter()
        .find(|event| event.frame["type"] == "permission")
        .unwrap()
        .frame["permissionId"]
        .as_str()
        .unwrap()
        .to_owned();
    session
        .submit(
            "cancel".into(),
            SessionRequest::Cancel {
                target: "turn".into(),
            },
        )
        .await
        .unwrap();
    let terminal = until(&session, "turn", "record").await;
    let record = terminal
        .events
        .iter()
        .find(|event| event.frame["type"] == "record")
        .unwrap();
    assert_eq!(record.frame["record"]["status"], "cancelled");
    assert_eq!(record.frame["sessionUsable"], true);
    assert!(
        session
            .submit(
                "late-approval".into(),
                SessionRequest::Permission {
                    permission_id,
                    option_id: Some("allow".into())
                }
            )
            .await
            .is_err()
    );
    session
        .submit(
            "follow-up".into(),
            SessionRequest::Prompt {
                text: "hello".into(),
                context: None,
                max_duration_ms: 10_000,
            },
        )
        .await
        .unwrap();
    until(&session, "follow-up", "record").await;
    session.close().await.unwrap();
}
