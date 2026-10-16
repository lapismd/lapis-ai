//! Signal handling: bounded cleanup and the published signal exit codes.

mod common;

use std::time::Duration;

use common::{Scratch, run_args, task, write_catalog, write_task};
use serde_json::Value;
use tokio::process::Command;

/// Send a signal to a pid without depending on a signal crate.
fn signal(pid: u32, name: &str) {
    let status = std::process::Command::new("kill")
        .args([name, &pid.to_string()])
        .status()
        .expect("kill runs");
    assert!(status.success(), "could not signal {pid} with {name}");
}

/// Wait until the fixture has started, so the signal arrives mid-turn.
///
/// The bound is set so that only a genuine failure trips it.
///
/// A full workspace run starts roughly fifteen test binaries in parallel, several
/// of which spawn an interpreter subprocess each. Under that contention fixture
/// startup can take an order of magnitude longer than the 0.1 seconds it takes
/// uncontended, and a bound tuned to an idle machine turns this into an
/// intermittent failure whose cause has nothing to do with signal handling. A
/// flaky lifecycle test is worse than a slow one: it trains reviewers to re-run
/// instead of investigate. When startup works this returns almost immediately, so
/// the generous ceiling costs nothing except in the case it exists to catch.
fn await_agent_start(log: &std::path::Path) {
    const ATTEMPTS: u32 = 6000;
    const INTERVAL: Duration = Duration::from_millis(25);
    for attempt in 0..ATTEMPTS {
        if let Ok(text) = std::fs::read_to_string(log)
            && !text.trim().is_empty()
        {
            return;
        }
        std::thread::sleep(INTERVAL);
        if attempt + 1 == ATTEMPTS {
            panic!(
                "the fixture agent wrote nothing to {} within {:?}",
                log.display(),
                INTERVAL * ATTEMPTS
            );
        }
    }
}

async fn interrupted(signal_name: &str, expected: i32) {
    let scratch = Scratch::new("signal");
    let catalog = write_catalog(&scratch, "turn-hang", &[]);
    let path = write_task(&scratch, &task("task-signal", "never answer", 120_000));
    let mut arguments = run_args(&catalog, &path);
    // Both bounds must be long, not just the idle one. The absolute cap is the
    // shorter of the flag and the task document, so leaving it at the shared
    // default let a deadline end the turn before the signal arrived under load and
    // reported exit 124 instead of the signal's own code. The intent of this test
    // is that only the signal can end the turn, so both bounds say so.
    for (flag, value) in [("--idle-timeout", "240"), ("--max-turn-duration", "240")] {
        let position = arguments
            .iter()
            .position(|item| item == flag)
            .unwrap_or_else(|| panic!("{flag} is always present"));
        arguments[position + 1] = value.to_owned();
    }
    arguments.extend(["--cancel-grace".to_owned(), "5".to_owned()]);
    arguments.extend(["--reap-timeout".to_owned(), "5".to_owned()]);

    let child = Command::new(common::binary())
        .args(&arguments)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("binary spawns");
    let pid = child.id().expect("pid");
    await_agent_start(&scratch.log());

    signal(pid, signal_name);
    let output = child.wait_with_output().await.expect("binary exits");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert_eq!(
        output.status.code(),
        Some(expected),
        "{signal_name} should exit {expected}; stderr: {stderr}"
    );
    // Stdout still carries exactly one record after a signal, which is what lets
    // a controlling application parse the output of an interrupted run.
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {stdout:?}");
    let record: Value = serde_json::from_str(lines[0]).expect("json record");
    assert_eq!(record["status"], Value::String("cancelled".to_owned()));
    assert_eq!(record["taskId"], Value::String("task-signal".to_owned()));
    assert_eq!(record["error"], Value::String("interrupted".to_owned()));
    assert!(record.get("stopReason").is_none(), "{record}");
}

#[tokio::test]
async fn sigterm_produces_bounded_cleanup_and_exit_143() {
    interrupted("-TERM", 143).await;
}

#[tokio::test]
async fn sigint_produces_bounded_cleanup_and_exit_130() {
    interrupted("-INT", 130).await;
}
