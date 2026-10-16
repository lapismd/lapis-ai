//! The one-shot task contract, exercised against the real binary.

mod common;

use common::{Scratch, as_refs, invoke, record, run_args, task, write_catalog, write_task};
use serde_json::{Value, json};

fn references(args: &[String]) -> Vec<&str> {
    as_refs(args)
}

#[tokio::test]
async fn a_prepared_task_completes_with_one_terminal_record() {
    let scratch = Scratch::new("normal");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let path = write_task(
        &scratch,
        &task("task-001", "inspect the working copy", 60_000),
    );
    let run = invoke(&references(&run_args(&catalog, &path)), None).await;

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(
        record(&run),
        json!({"version": 1, "taskId": "task-001", "status": "completed",
               "stopReason": "end_turn"})
    );
}

#[tokio::test]
async fn file_and_stdin_input_agree() {
    let scratch = Scratch::new("parity");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let document = task("task-parity", "same document both ways", 60_000);

    let from_file = write_task(&scratch, &document);
    let file_run = invoke(&references(&run_args(&catalog, &from_file)), None).await;
    assert_eq!(file_run.code, Some(0), "stderr: {}", file_run.stderr);

    let mut args = run_args(&catalog, &from_file);
    // Replace the path with the stdin spelling.
    let position = args.iter().position(|a| a == "--task").expect("flag");
    args[position + 1] = "-".to_owned();
    let stdin_run = invoke(&references(&args), Some(&document.to_string())).await;

    assert_eq!(stdin_run.code, Some(0), "stderr: {}", stdin_run.stderr);
    assert_eq!(record(&stdin_run), record(&file_run));
}

#[tokio::test]
async fn a_refusal_is_a_normal_completion_with_its_own_stop_reason() {
    let scratch = Scratch::new("refusal");
    let catalog = write_catalog(
        &scratch,
        "normal",
        &[("JJ_ACP_FIXTURE_STOP_REASON", "refusal")],
    );
    let path = write_task(&scratch, &task("task-refusal", "decline this", 60_000));
    let run = invoke(&references(&run_args(&catalog, &path)), None).await;

    // Exit 0 means the turn returned normally, which is not proof the task
    // succeeded; the caller must read the stop reason.
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    let value = record(&run);
    assert_eq!(value["status"], json!("completed"));
    assert_eq!(value["stopReason"], json!("refusal"));
}

#[tokio::test]
async fn an_agent_error_never_echoes_the_agent_message() {
    let scratch = Scratch::new("error");
    let catalog = write_catalog(&scratch, "error", &[]);
    let path = write_task(&scratch, &task("task-error", "provoke a failure", 60_000));
    let run = invoke(&references(&run_args(&catalog, &path)), None).await;

    assert_eq!(run.code, Some(1), "stderr: {}", run.stderr);
    let value = record(&run);
    assert_eq!(value["status"], json!("failed"));
    assert_eq!(value["error"], json!("agent_error"));
    assert!(value.get("stopReason").is_none());
    // The fixture's error message is a marker proving untrusted agent text is
    // kept off stdout and out of the record.
    assert!(
        !run.stdout.contains("SECRET_TASK_TEXT"),
        "agent text reached stdout: {}",
        run.stdout
    );
    assert!(
        run.stderr.contains("SECRET_TASK_TEXT"),
        "the operator diagnostic should carry the detail: {}",
        run.stderr
    );
}

#[tokio::test]
async fn the_prompt_and_credentials_never_reach_the_terminal_record() {
    let scratch = Scratch::new("sanitize");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let marker = "PROMPT-MARKER-7f3a9c";
    let path = write_task(&scratch, &task("task-sanitize", marker, 60_000));
    let run = invoke(&references(&run_args(&catalog, &path)), None).await;

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert!(
        !run.stdout.contains(marker),
        "prompt text reached stdout: {}",
        run.stdout
    );
    assert_eq!(
        record(&run).as_object().expect("object").len(),
        4,
        "the record schema grew a field that could carry content"
    );
    // The fixture did receive the prompt, so the marker's absence from stdout is
    // sanitization rather than the task never having run.
    let log = std::fs::read_to_string(scratch.log()).expect("fixture log");
    assert!(log.contains(marker), "the agent never saw the prompt");
}

#[tokio::test]
async fn the_fixture_receives_the_harness_reserved_variables() {
    let scratch = Scratch::new("reserved");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let path = write_task(&scratch, &task("task-reserved", "check injection", 60_000));
    let run = invoke(&references(&run_args(&catalog, &path)), None).await;
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);

    let log = std::fs::read_to_string(scratch.log()).expect("fixture log");
    let launch: Value =
        serde_json::from_str(log.lines().next().expect("launch record")).expect("json");
    assert_eq!(launch["runner"], json!("fixture"));
    assert_eq!(launch["taskId"], json!("task-reserved"));
    let reserved: Vec<String> = launch["reserved"]
        .as_array()
        .expect("reserved")
        .iter()
        .map(|value| value.as_str().expect("string").to_owned())
        .collect();
    assert!(
        reserved.contains(&"JJ_ACP_TASK_RUNNER".to_owned()),
        "{reserved:?}"
    );
    assert!(
        reserved.contains(&"JJ_ACP_TASK_ID".to_owned()),
        "{reserved:?}"
    );
}

#[tokio::test]
async fn invalid_documents_are_refused_before_any_agent_starts() {
    let scratch = Scratch::new("invalid");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let cases: Vec<(&str, Value, &str)> = vec![
        (
            "version",
            json!({"version": 2, "taskId": "t", "agent": "fixture",
                           "prompt": "p", "maxDurationMs": 1000}),
            "unsupported_task_version",
        ),
        (
            "unknown field",
            json!({"version": 1, "taskId": "t", "agent": "fixture",
                                 "prompt": "p", "maxDurationMs": 1000, "cwd": "/etc"}),
            "invalid_task_json",
        ),
        (
            "blank prompt",
            json!({"version": 1, "taskId": "t", "agent": "fixture",
                                "prompt": "   ", "maxDurationMs": 1000}),
            "invalid_prompt",
        ),
        (
            "zero duration",
            json!({"version": 1, "taskId": "t", "agent": "fixture",
                                 "prompt": "p", "maxDurationMs": 0}),
            "invalid_duration",
        ),
        (
            "control character",
            json!({"version": 1, "taskId": "a\u{7}", "agent": "fixture",
                                     "prompt": "p", "maxDurationMs": 1000}),
            "invalid_task_id",
        ),
    ];
    for (label, document, expected) in cases {
        let path = write_task(&scratch, &document);
        let run = invoke(&references(&run_args(&catalog, &path)), None).await;
        assert_eq!(run.code, Some(2), "{label}: stderr {}", run.stderr);
        let value = record(&run);
        assert_eq!(value["status"], json!("invalid"), "{label}");
        assert_eq!(value["error"], json!(expected), "{label}");
        assert!(
            !scratch.log().exists()
                || std::fs::read_to_string(scratch.log()).is_ok_and(|t| t.is_empty()),
            "{label}: an agent was started for a refused document"
        );
    }
}

#[tokio::test]
async fn an_unknown_runner_is_refused_without_pinning_one() {
    // No --runner here, so resolution rather than the mismatch guard decides.
    let scratch = Scratch::new("unknown-runner");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let document = json!({"version": 1, "taskId": "t-unknown", "agent": "no-such-runner",
                          "prompt": "p", "maxDurationMs": 5000});
    let path = write_task(&scratch, &document);
    let mut args = run_args(&catalog, &path);
    let position = args.iter().position(|a| a == "--runner").expect("flag");
    // Drop the pinned runner and its value.
    args.remove(position + 1);
    args.remove(position);
    let run = invoke(&references(&args), None).await;

    assert_eq!(run.code, Some(2), "stderr: {}", run.stderr);
    let value = record(&run);
    assert_eq!(value["status"], json!("invalid"));
    assert_eq!(value["error"], json!("invalid_runner"));
}

#[tokio::test]
async fn duplicate_and_trailing_documents_are_refused() {
    let scratch = Scratch::new("strict");
    let catalog = write_catalog(&scratch, "normal", &[]);

    let duplicated = scratch.join("duplicate.json");
    std::fs::write(
        &duplicated,
        r#"{"version":1,"taskId":"t","agent":"fixture","prompt":"p","maxDurationMs":1000,"agent":"other"}"#,
    )
    .expect("write");
    let run = invoke(&references(&run_args(&catalog, &duplicated)), None).await;
    assert_eq!(run.code, Some(2), "stderr: {}", run.stderr);
    assert_eq!(record(&run)["error"], json!("invalid_task_json"));

    let trailing = scratch.join("trailing.json");
    std::fs::write(
        &trailing,
        r#"{"version":1,"taskId":"t","agent":"fixture","prompt":"p","maxDurationMs":1000}
{"version":1,"taskId":"u","agent":"fixture","prompt":"q","maxDurationMs":1000}"#,
    )
    .expect("write");
    let run = invoke(&references(&run_args(&catalog, &trailing)), None).await;
    assert_eq!(run.code, Some(2), "stderr: {}", run.stderr);
    assert_eq!(record(&run)["error"], json!("invalid_task_json"));
}

#[tokio::test]
async fn an_oversized_document_is_refused_without_being_buffered() {
    let scratch = Scratch::new("oversize");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let padding = "x".repeat(1024 * 1024 + 64);
    let path = write_task(&scratch, &task("task-large", &padding, 60_000));
    let run = invoke(&references(&run_args(&catalog, &path)), None).await;
    assert_eq!(run.code, Some(2), "stderr: {}", run.stderr);
    assert_eq!(record(&run)["error"], json!("task_too_large"));
}

#[tokio::test]
async fn a_task_addressed_to_another_runner_is_refused() {
    let scratch = Scratch::new("mismatch");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let mut document = task("task-mismatch", "run this", 60_000);
    document["agent"] = json!("codex");
    let path = write_task(&scratch, &document);
    let run = invoke(&references(&run_args(&catalog, &path)), None).await;

    assert_eq!(run.code, Some(2), "stderr: {}", run.stderr);
    let value = record(&run);
    assert_eq!(value["status"], json!("invalid"));
    assert_eq!(value["error"], json!("runner_mismatch"));
    // The identifier is known because the document parsed, so it is reported.
    assert_eq!(value["taskId"], json!("task-mismatch"));
}

#[tokio::test]
async fn an_unresolvable_runner_reports_a_spawn_failure() {
    let scratch = Scratch::new("missing");
    let catalog_path = scratch.join("catalog.json");
    std::fs::write(
        &catalog_path,
        json!({"runners": [{"id": "absent", "label": "Absent",
                            "command": "lapis-acp-definitely-not-installed-9f3a",
                            "installHint": "install it first"}]})
        .to_string(),
    )
    .expect("catalog write");
    let document = json!({"version": 1, "taskId": "task-absent", "agent": "absent",
                          "prompt": "p", "maxDurationMs": 5000});
    let path = write_task(&scratch, &document);
    let mut args = run_args(&catalog_path, &path);
    let position = args.iter().position(|a| a == "--runner").expect("flag");
    args[position + 1] = "absent".to_owned();
    let run = invoke(&references(&args), None).await;

    assert_eq!(run.code, Some(1), "stderr: {}", run.stderr);
    assert_eq!(record(&run)["error"], json!("agent_spawn_failed"));
    // Guidance reaches the operator, not the controlling application's parser.
    assert!(run.stderr.contains("install it first"), "{}", run.stderr);
}

#[tokio::test]
async fn a_deadline_produces_timed_out_and_exit_124() {
    let scratch = Scratch::new("deadline");
    let catalog = write_catalog(&scratch, "turn-hang", &[]);
    let path = write_task(&scratch, &task("task-deadline", "never answer", 1500));
    let mut args = run_args(&catalog, &path);
    args.extend(["--cancel-grace".to_owned(), "1".to_owned()]);
    args.extend(["--reap-timeout".to_owned(), "1".to_owned()]);
    let run = invoke(&references(&args), None).await;

    assert_eq!(run.code, Some(124), "stderr: {}", run.stderr);
    let value = record(&run);
    assert_eq!(value["status"], json!("timed_out"));
    assert_eq!(value["error"], json!("deadline_exceeded"));
    assert!(value.get("stopReason").is_none());
}

#[tokio::test]
async fn an_idle_window_that_never_sees_a_frame_times_out_separately() {
    let scratch = Scratch::new("idle");
    let catalog = write_catalog(&scratch, "startup-hang", &[]);
    let path = write_task(&scratch, &task("task-idle", "never initialize", 60_000));
    let mut args = run_args(&catalog, &path);
    let position = args
        .iter()
        .position(|a| a == "--idle-timeout")
        .expect("flag");
    args[position + 1] = "1".to_owned();
    let run = invoke(&references(&args), None).await;

    assert_eq!(run.code, Some(124), "stderr: {}", run.stderr);
    assert_eq!(record(&run)["error"], json!("idle_timeout"));
}

#[tokio::test]
async fn a_missing_task_file_is_refused() {
    let scratch = Scratch::new("absent-file");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let run = invoke(
        &references(&run_args(&catalog, &scratch.join("nope.json"))),
        None,
    )
    .await;
    assert_eq!(run.code, Some(2), "stderr: {}", run.stderr);
    let value = record(&run);
    assert_eq!(value["status"], json!("invalid"));
    assert!(
        value["taskId"].is_null(),
        "no identifier was ever validated"
    );
}
