//! Shared helpers for driving the real `lapis-acp` binary.
//!
//! Each test binary includes this module whole, so a helper one binary does not
//! use is allowed rather than warned about.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// The compiled binary under test.
pub fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_lapis-acp")
}

/// One completed invocation.
#[derive(Debug, Clone)]
pub struct Run {
    /// The process exit code.
    pub code: Option<i32>,
    /// Everything written to stdout.
    pub stdout: String,
    /// Everything written to stderr.
    pub stderr: String,
}

/// A scratch directory that removes itself on drop.
pub struct Scratch(PathBuf);

impl Scratch {
    /// Create a uniquely named scratch directory.
    pub fn new(label: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        loop {
            let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "lapis-acp-cli-{label}-{}-{serial}",
                std::process::id()
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("scratch directory: {error}"),
            }
        }
    }

    /// A path inside this scratch directory.
    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// The fixture's log path inside this scratch directory.
    pub fn log(&self) -> PathBuf {
        self.join("agent.log")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Write a catalog file declaring one runner backed by the fixture agent.
pub fn write_catalog(scratch: &Scratch, mode: &str, extra_env: &[(&str, &str)]) -> PathBuf {
    let mut env = json!({
        "JJ_ACP_FIXTURE_MODE": mode,
        "JJ_ACP_FIXTURE_LOG": scratch.log().to_string_lossy(),
    });
    for (name, value) in extra_env {
        env[name] = json!(value);
    }
    let document = json!({
        "runners": [{
            "id": "fixture",
            "label": "Fixture",
            "command": lapis_acp_testkit::python(),
            "args": [lapis_acp_testkit::fixture_script().to_string_lossy()],
            "env": env,
            "installHint": "Python 3 must be installed",
        }],
    });
    let path = scratch.join("catalog.json");
    std::fs::write(&path, document.to_string()).expect("catalog write");
    path
}

/// Write a task document to a file and return its path.
pub fn write_task(scratch: &Scratch, document: &Value) -> PathBuf {
    let path = scratch.join("task.json");
    std::fs::write(&path, document.to_string()).expect("task write");
    path
}

/// A valid version 1 task document addressed to the fixture runner.
pub fn task(task_id: &str, prompt: &str, max_duration_ms: u64) -> Value {
    json!({
        "version": 1,
        "taskId": task_id,
        "agent": "fixture",
        "prompt": prompt,
        "maxDurationMs": max_duration_ms,
    })
}

/// Run the binary, optionally feeding stdin, and capture both streams.
pub async fn invoke(args: &[&str], stdin: Option<&str>) -> Run {
    let mut command = Command::new(binary());
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().expect("binary spawns");
    if let Some(input) = stdin {
        let mut handle = child.stdin.take().expect("stdin");
        handle
            .write_all(input.as_bytes())
            .await
            .expect("stdin write");
        // Closing stdin is part of the contract for '-' input.
        drop(handle);
    } else {
        drop(child.stdin.take());
    }
    let output = child.wait_with_output().await.expect("binary exits");
    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

/// The single terminal record on stdout, failing when stdout is not exactly one
/// parseable JSON line.
pub fn record(run: &Run) -> Value {
    let trimmed = run.stdout.trim_end_matches('\n');
    assert!(
        !trimmed.contains('\n'),
        "stdout carried more than one line: {:?}",
        run.stdout
    );
    serde_json::from_str(trimmed)
        .unwrap_or_else(|error| panic!("stdout is not a JSON record ({error}): {:?}", run.stdout))
}

/// Arguments that run a task file against a fixture-mode catalog.
pub fn run_args(catalog: &Path, task: &Path) -> Vec<String> {
    vec![
        "run".to_owned(),
        "--task".to_owned(),
        task.to_string_lossy().into_owned(),
        "--catalog".to_owned(),
        catalog.to_string_lossy().into_owned(),
        "--runner".to_owned(),
        "fixture".to_owned(),
        "--idle-timeout".to_owned(),
        "20".to_owned(),
        "--max-turn-duration".to_owned(),
        "60".to_owned(),
    ]
}

/// Borrow a `Vec<String>` as the `&[&str]` shape `invoke` expects.
pub fn as_refs(arguments: &[String]) -> Vec<&str> {
    arguments.iter().map(String::as_str).collect()
}
