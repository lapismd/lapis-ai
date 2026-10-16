//! Descendant containment and crash reporting.
//!
//! These prove two properties that a passing happy-path suite cannot: that
//! cleanup reaches the processes an agent spawned, and that a crashed agent is
//! reported rather than silently retried. Re-dispatching a turn whose outcome is
//! unknown can execute its side effects twice, so "report and stop" is the
//! contract and the prompt dispatch count is the evidence.

mod common;

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use common::{Scratch, as_refs, invoke, record, run_args, task, write_catalog, write_task};
use lapis_acp_testkit::FixtureLog;
use serde_json::json;

/// Report whether a pid still exists, without depending on a signal crate.
fn process_exists(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Wait for a pid to disappear, allowing the kernel a moment to finish.
fn await_gone(pid: u32) -> bool {
    for _ in 0..100 {
        if !process_exists(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    !process_exists(pid)
}

/// Read the descendant pid the fixture recorded, proving it was really spawned.
fn descendant(scratch: &Scratch) -> u32 {
    let log = FixtureLog::new(scratch.log());
    for _ in 0..1500 {
        if let Ok(Some(pid)) = log.descendant() {
            return pid;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the fixture never recorded a descendant pid");
}

fn shortened(args: &[String], cancel_grace: &str, reap: &str) -> Vec<String> {
    let mut arguments = args.to_vec();
    arguments.extend(["--cancel-grace".to_owned(), cancel_grace.to_owned()]);
    arguments.extend(["--reap-timeout".to_owned(), reap.to_owned()]);
    arguments
}

#[tokio::test]
async fn a_forced_termination_takes_the_agents_descendant_with_it() {
    let scratch = Scratch::new("containment-forced");
    let catalog = write_catalog(&scratch, "descendant-turn", &[]);
    // The absolute deadline includes interpreter startup. Allow startup under
    // hosted-runner contention without changing the deadline/cleanup contract.
    let path = write_task(&scratch, &task("task-forced", "never answer", 10_000));
    let arguments = shortened(&run_args(&catalog, &path), "1", "1");

    // Start the run, then read the descendant pid while it is in flight.
    let child = tokio::process::Command::new(common::binary())
        .args(&arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("binary spawns");
    let pid = descendant(&scratch);
    assert!(process_exists(pid), "descendant {pid} should be running");

    let output = tokio::time::timeout(Duration::from_secs(14), child.wait_with_output())
        .await
        .expect("deadline and cleanup remain bounded")
        .expect("binary exits");
    assert_eq!(output.status.code(), Some(124), "deadline expected");
    let terminal: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one record");
    assert_eq!(terminal["status"], "timed_out");
    assert_eq!(terminal["error"], "deadline_exceeded");
    assert_eq!(
        FixtureLog::new(scratch.log())
            .method_count("session/prompt")
            .expect("log"),
        1,
        "the turn must start exactly once before its deadline"
    );

    // The whole point of process-group ownership: the descendant the agent
    // spawned does not outlive the turn that was killed.
    assert!(
        await_gone(pid),
        "descendant {pid} survived cleanup; the process group was not reached"
    );
}

#[tokio::test]
async fn a_completed_turn_still_does_not_leave_a_descendant_behind() {
    let scratch = Scratch::new("containment-clean");
    let catalog = write_catalog(&scratch, "descendant", &[]);
    let path = write_task(&scratch, &task("task-clean", "finish normally", 60_000));
    let arguments = shortened(&run_args(&catalog, &path), "3", "3");
    let run = invoke(&as_refs(&arguments), None).await;

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(record(&run)["status"], json!("completed"));

    let pid = descendant(&scratch);
    assert!(
        await_gone(pid),
        "descendant {pid} outlived a successful turn: {}",
        run.stderr
    );
    // A graceful exit must not be reported as an escalation.
    assert!(
        !run.stderr.contains("killed after the grace window"),
        "a clean turn should not need a fatal signal: {}",
        run.stderr
    );
}

#[tokio::test]
async fn an_agent_that_crashes_mid_turn_is_reported_and_never_re_dispatched() {
    let scratch = Scratch::new("crash-turn");
    let catalog = write_catalog(&scratch, "crash-turn", &[]);
    let path = write_task(&scratch, &task("task-crash", "crash on me", 60_000));
    let run = invoke(&as_refs(&run_args(&catalog, &path)), None).await;

    assert_eq!(run.code, Some(1), "stderr: {}", run.stderr);
    let value = record(&run);
    assert_eq!(value["status"], json!("failed"));
    assert_eq!(value["error"], json!("agent_exited"));
    assert_eq!(value["taskId"], json!("task-crash"));
    assert!(value.get("stopReason").is_none(), "{value}");

    // The crash is a distinct outcome, not a timeout and not a protocol error.
    let log = FixtureLog::new(scratch.log());
    assert_eq!(log.crashed().expect("log").as_deref(), Some("turn"));
    // Exactly one dispatch: an unknown outcome is reported, never retried,
    // because a retry could execute the same side effects a second time.
    assert_eq!(
        log.method_count("session/prompt").expect("log"),
        1,
        "the turn was re-dispatched after the crash"
    );
}

#[tokio::test]
async fn an_agent_that_crashes_during_startup_never_opens_a_session() {
    let scratch = Scratch::new("crash-startup");
    let catalog = write_catalog(&scratch, "crash-startup", &[]);
    let path = write_task(&scratch, &task("task-startup", "crash early", 60_000));
    let run = invoke(&as_refs(&run_args(&catalog, &path)), None).await;

    assert_eq!(run.code, Some(1), "stderr: {}", run.stderr);
    let value = record(&run);
    assert_eq!(value["status"], json!("failed"));
    assert_eq!(value["error"], json!("agent_exited"));

    let log = FixtureLog::new(scratch.log());
    assert_eq!(log.crashed().expect("log").as_deref(), Some("startup"));
    assert_eq!(log.method_count("session/new").expect("log"), 0);
    assert_eq!(log.method_count("session/prompt").expect("log"), 0);
}

#[tokio::test]
async fn a_crash_is_distinguished_from_a_bound_that_elapsed() {
    // Same refusal to answer, different cause, different record: a crash says the
    // agent is gone, a deadline says its state is unknown. Conflating them would
    // tell a controlling application it is safe to retry when it is not.
    let scratch = Scratch::new("crash-vs-deadline");
    let crash_catalog = write_catalog(&scratch, "crash-turn", &[]);
    let crash_task = write_task(&scratch, &task("task-crash-2", "x", 60_000));
    let crash = invoke(&as_refs(&run_args(&crash_catalog, &crash_task)), None).await;

    let hang_catalog = write_catalog(&scratch, "turn-hang", &[]);
    let hang_task = write_task(&scratch, &task("task-hang", "x", 1500));
    let hang = invoke(
        &as_refs(&shortened(&run_args(&hang_catalog, &hang_task), "1", "1")),
        None,
    )
    .await;

    assert_eq!(record(&crash)["status"], json!("failed"));
    assert_eq!(record(&hang)["status"], json!("timed_out"));
    assert_ne!(crash.code, hang.code);
    assert_eq!(crash.code, Some(1));
    assert_eq!(hang.code, Some(124));
}

#[test]
fn the_helpers_resolve_the_real_fixture_paths() {
    // Guards the test suite against silently pointing at a stale script.
    let script = lapis_acp_testkit::fixture_script();
    assert!(script.exists(), "missing {}", script.display());
    assert!(script.ends_with(Path::new("tests/fixtures/fixture_agent.py")));
}
