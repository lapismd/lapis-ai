//! Session configuration applied through the real binary.
//!
//! The fixture logs every frame it receives, so these assert what was actually
//! sent to an adapter rather than what the harness believed it sent. That
//! distinction is the whole point: a configuration step that silently does nothing
//! looks identical to one that worked unless the far end is observed.

mod common;

use std::process::Stdio;

use common::{Scratch, as_refs, invoke, record, run_args, task, write_catalog, write_task};
use lapis_acp_testkit::FixtureLog;
use serde_json::Value;

/// The `session/set_config_option` frames the fixture received, in order.
fn config_frames(scratch: &Scratch) -> Vec<(String, String)> {
    let log = FixtureLog::new(scratch.log());
    log.records()
        .expect("log")
        .into_iter()
        .filter(|frame| {
            frame.get("method").and_then(Value::as_str) == Some("session/set_config_option")
        })
        .map(|frame| {
            let params = &frame["params"];
            (
                params["configId"].as_str().unwrap_or_default().to_owned(),
                params["value"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn model_frames(scratch: &Scratch) -> Vec<String> {
    let log = FixtureLog::new(scratch.log());
    log.records()
        .expect("log")
        .into_iter()
        .filter(|frame| frame.get("method").and_then(Value::as_str) == Some("session/set_model"))
        .map(|frame| {
            frame["params"]["modelId"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

/// Run a task with extra launch flags appended.
async fn configured(scratch: &Scratch, mode: &str, extra: &[&str]) -> common::Run {
    let catalog = write_catalog(scratch, mode, &[]);
    let path = write_task(scratch, &task("task-config", "configure me", 60_000));
    let mut arguments = run_args(&catalog, &path);
    arguments.extend(extra.iter().map(|value| value.to_string()));
    invoke(&as_refs(&arguments), None).await
}

#[tokio::test]
async fn a_requested_model_effort_and_mode_all_reach_the_adapter() {
    let scratch = Scratch::new("config-applied");
    let run = configured(
        &scratch,
        "normal",
        &[
            "--model",
            "chosen",
            "--effort",
            "high",
            "--mode",
            "bypassPermissions",
        ],
    )
    .await;

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(record(&run)["status"], serde_json::json!("completed"));
    assert_eq!(
        config_frames(&scratch),
        vec![
            ("model".to_owned(), "chosen".to_owned()),
            ("effort".to_owned(), "high".to_owned()),
            ("mode".to_owned(), "bypassPermissions".to_owned()),
        ],
        "the adapter did not receive the requested settings in order"
    );
    // All three landed, so nothing is reported as a shortfall.
    assert!(
        !run.stderr.contains("its own default was kept"),
        "unexpected shortfall: {}",
        run.stderr
    );
}

#[tokio::test]
async fn a_value_the_adapter_does_not_offer_is_reported_and_does_not_fail_the_run() {
    let scratch = Scratch::new("config-unavailable");
    let run = configured(&scratch, "normal", &["--effort", "xhigh"]).await;

    // An unsupported preference is not a failed task: the session is still usable
    // on the adapter's own default, and failing the run would punish a caller for
    // asking a reasonable question of an adapter that happens not to answer it.
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(record(&run)["status"], serde_json::json!("completed"));
    assert!(
        run.stderr
            .contains("does not offer the requested effort value"),
        "stderr should explain the shortfall: {}",
        run.stderr
    );
    assert!(
        config_frames(&scratch).is_empty(),
        "nothing should have been sent for an unavailable value"
    );
}

#[tokio::test]
async fn a_mode_absent_from_available_modes_is_refused_even_though_the_option_lists_it() {
    let scratch = Scratch::new("config-gate");
    let run = configured(&scratch, "normal", &["--mode", "unlisted"]).await;

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert!(
        run.stderr.contains("does not list the requested mode"),
        "stderr: {}",
        run.stderr
    );
    assert!(
        config_frames(&scratch).is_empty(),
        "the two surfaces disagreed, so nothing should have been sent"
    );
}

#[tokio::test]
async fn a_value_offered_under_another_category_is_not_substituted() {
    let scratch = Scratch::new("config-category");
    let run = configured(&scratch, "normal", &["--mode", "plan"]).await;

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    // The fixture advertises `plan` under collaboration_mode, as one real adapter
    // does. Applying it as a session mode would be a guess about what the adapter
    // means, so the category is named in the diagnostic instead.
    assert!(
        run.stderr.contains("collaboration_mode"),
        "stderr should name the category: {}",
        run.stderr
    );
    assert!(config_frames(&scratch).is_empty());
}

#[tokio::test]
async fn an_unknown_model_is_reported_and_the_turn_still_completes() {
    let scratch = Scratch::new("config-model");
    let run = configured(&scratch, "normal", &["--model", "no-such-model"]).await;

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(record(&run)["status"], serde_json::json!("completed"));
    assert!(
        run.stderr
            .contains("does not offer the requested model value"),
        "stderr: {}",
        run.stderr
    );
    assert!(model_frames(&scratch).is_empty());
    assert!(config_frames(&scratch).is_empty());
}

#[tokio::test]
async fn configuration_arrives_by_environment_variable_too() {
    let scratch = Scratch::new("config-env");
    let catalog = write_catalog(&scratch, "normal", &[]);
    let path = write_task(&scratch, &task("task-env", "configure me", 60_000));
    let arguments = run_args(&catalog, &path);
    let mut command = std::process::Command::new(common::binary());
    command
        .args(as_refs(&arguments))
        .env("JJ_ACP_EFFORT", "high")
        .env("JJ_ACP_MODE", "bypassPermissions")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = command.output().expect("binary runs");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        config_frames(&scratch),
        vec![
            ("effort".to_owned(), "high".to_owned()),
            ("mode".to_owned(), "bypassPermissions".to_owned()),
        ]
    );
}

#[tokio::test]
async fn configuration_is_applied_before_the_turn_is_dispatched() {
    let scratch = Scratch::new("config-order");
    let run = configured(&scratch, "normal", &["--effort", "high"]).await;
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);

    let log = FixtureLog::new(scratch.log());
    let order: Vec<String> = log
        .records()
        .expect("log")
        .into_iter()
        .filter_map(|frame| {
            frame
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    let configure = order
        .iter()
        .position(|method| method == "session/set_config_option")
        .expect("configuration was sent");
    let prompt = order
        .iter()
        .position(|method| method == "session/prompt")
        .expect("prompt was sent");
    assert!(
        configure < prompt,
        "configuration must precede the turn: {order:?}"
    );
}
