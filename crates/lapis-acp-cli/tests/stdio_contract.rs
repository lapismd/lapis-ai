//! The stdout contract: records only, diagnostics never.
//!
//! A controlling application parses stdout unconditionally, so anything else
//! written there breaks it. These tests pin what stdout may contain on every
//! path, including the failure paths where a stray diagnostic is most likely.

mod common;

use common::{Scratch, as_refs, invoke, record, run_args, task, write_catalog, write_task};
use serde_json::json;

#[tokio::test]
async fn an_invalid_command_line_leaves_stdout_empty() {
    let run = invoke(&["nonsense-command"], None).await;
    assert_eq!(run.code, Some(2));
    assert!(
        run.stdout.is_empty(),
        "usage reached stdout: {:?}",
        run.stdout
    );
    assert!(run.stderr.contains("unknown command"), "{}", run.stderr);
    assert!(run.stderr.contains("Usage:"), "{}", run.stderr);
}

#[tokio::test]
async fn a_missing_task_flag_is_a_usage_refusal_on_stderr() {
    let run = invoke(&["run"], None).await;
    assert_eq!(run.code, Some(2));
    assert!(run.stdout.is_empty(), "stdout: {:?}", run.stdout);
    assert!(run.stderr.contains("--task"), "{}", run.stderr);
}

#[tokio::test]
async fn a_failed_invocation_still_emits_exactly_one_record() {
    let scratch = Scratch::new("stdio-failure");
    let catalog_path = scratch.join("catalog.json");
    std::fs::write(
        &catalog_path,
        json!({"runners": [{"id": "absent", "label": "Absent",
                            "command": "lapis-acp-definitely-not-installed-4b2c"}]})
        .to_string(),
    )
    .expect("catalog write");
    let path = write_task(
        &scratch,
        &json!({"version": 1, "taskId": "t", "agent": "absent",
                "prompt": "p", "maxDurationMs": 5000}),
    );
    let mut args = run_args(&catalog_path, &path);
    let position = args.iter().position(|a| a == "--runner").expect("flag");
    args[position + 1] = "absent".to_owned();
    let run = invoke(&as_refs(&args), None).await;

    assert_eq!(run.code, Some(1));
    // Exactly one parseable line, and the diagnostics went elsewhere.
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {:?}", run.stdout);
    assert_eq!(record(&run)["error"], json!("agent_spawn_failed"));
    assert!(!run.stderr.is_empty(), "diagnostics should be on stderr");
}

#[tokio::test]
async fn a_successful_run_writes_nothing_but_the_record() {
    let scratch = Scratch::new("stdio-clean");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let path = write_task(&scratch, &task("t-clean", "clean stdout", 60_000));
    let run = invoke(&as_refs(&run_args(&catalog, &path)), None).await;

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {:?}", run.stdout);
    let value = record(&run);
    assert_eq!(value["status"], json!("completed"));
    assert_eq!(value["taskId"], json!("t-clean"));
    // The fixture's own stderr is inherited, so it must not have leaked upward.
    assert!(
        !run.stdout.contains("fixture output"),
        "agent text reached stdout: {}",
        run.stdout
    );
}

#[tokio::test]
async fn help_and_version_are_the_only_non_record_stdout_output() {
    // This carve-out is deliberate and documented: these commands exist to print
    // to stdout, and neither can run alongside a task.
    let help = invoke(&["--help"], None).await;
    assert_eq!(help.code, Some(0));
    assert!(help.stdout.contains("Usage:"), "{}", help.stdout);

    let version = invoke(&["--version"], None).await;
    assert_eq!(version.code, Some(0));
    assert!(
        version.stdout.starts_with("lapis-acp "),
        "{}",
        version.stdout
    );
}

#[tokio::test]
async fn the_agents_command_emits_one_json_object_per_line() {
    let run = invoke(&["agents"], None).await;
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    let rows: Vec<serde_json::Value> = run
        .stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is JSON"))
        .collect();
    assert!(
        rows.len() >= 4,
        "expected the compiled-in runners: {rows:?}"
    );
    let ids: Vec<&str> = rows
        .iter()
        .map(|row| row["id"].as_str().expect("id"))
        .collect();
    for expected in ["codex", "claude", "opencode", "goose"] {
        assert!(ids.contains(&expected), "{expected} missing from {ids:?}");
    }
    for row in &rows {
        assert!(row["available"].is_boolean(), "{row}");
    }
}
