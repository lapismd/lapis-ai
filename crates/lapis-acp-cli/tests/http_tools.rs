//! Trusted HTTP declarations survive the actual launcher/provider boundary.
mod common;
use common::{Scratch, as_refs, invoke, record, run_args, task, write_task};
use serde_json::json;

#[tokio::test]
async fn mixed_tools_reach_the_provider_without_exposing_headers_in_cli_output() {
    let scratch = Scratch::new("http-tools");
    let log = scratch.join("tools.log");
    let fixture = lapis_acp_testkit::fixture_script().with_file_name("fleet_agent.py");
    let catalog = scratch.join("catalog.json");
    std::fs::write(
        &catalog,
        json!({"runners":[{
            "id":"fixture","label":"Fixture","command":lapis_acp_testkit::python(),
            "args":[fixture],"env":{"FLEET_LOG":scratch.log(),"FLEET_TOOLS_LOG":log}
        }]})
        .to_string(),
    )
    .unwrap();
    let declarations = json!([
        {"name":"stdio","command":"/native/bridge","args":[],"env":[]},
        {"type":"http","name":"cursor","url":"http://127.0.0.1:3210/mcp",
         "headers":[{"name":"Authorization","value":"Bearer cli-http-tool-sentinel"}]}
    ]);
    let tools = scratch.join("tools.json");
    std::fs::write(&tools, declarations.to_string()).unwrap();
    let task = write_task(&scratch, &task("http-tools", "ordinary prompt", 60_000));
    let mut arguments = run_args(&catalog, &task);
    arguments.extend(["--tools".into(), tools.to_string_lossy().into_owned()]);
    let run = invoke(&as_refs(&arguments), None).await;
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(record(&run)["status"], "completed");
    assert!(!run.stdout.contains("cli-http-tool-sentinel"));
    assert!(!run.stderr.contains("cli-http-tool-sentinel"));
    let received: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(log).unwrap()).unwrap();
    assert_eq!(received, declarations);
}
