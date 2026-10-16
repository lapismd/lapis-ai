#![cfg(unix)]
use lapis_ai_config::SystemResolver;
use lapis_ai_runtime::{AcpClient, Component, Config, Error, NativeProcess, PreparedLaunch};
use std::{os::unix::fs::PermissionsExt, path::Path, time::Duration};
fn helper(root: &Path, body: &str) -> std::path::PathBuf {
    let path = root.join("peer.py");
    std::fs::write(&path, format!("#!/usr/bin/env python3\n{body}")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}
#[tokio::test]
async fn isolated_processes_preserve_parent_environment_and_working_directory() {
    let parent = std::env::current_dir().unwrap();
    let inherited = std::env::var("LAPIS_TEST_INSTANCE").ok();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    for (root, label) in [(&a, "first"), (&b, "second")] {
        helper(
            root.path(),
            "import os,json,sys\nprint(json.dumps({'instance':os.environ['LAPIS_TEST_INSTANCE'],'cwd':os.getcwd()}),flush=True)\nsys.stdin.read()\n",
        );
        let text = format!(
            "version=1\n[runtime]\nagent='peer.py'\n[acp]\ncwd='.'\n[agent.environment]\nLAPIS_TEST_INSTANCE='{label}'"
        );
        let cfg = Config::parse(&text, root.path()).unwrap();
        let plan = PreparedLaunch::new(&cfg, Component::Agent, &SystemResolver).unwrap();
        let mut process = NativeProcess::spawn(plan).await.unwrap();
        let event = process.next(Duration::from_secs(5)).await.unwrap();
        assert_eq!(event["instance"], label);
        assert_eq!(
            event["cwd"],
            root.path()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .as_ref()
        );
        process.shutdown(Duration::from_secs(1)).await.unwrap();
    }
    assert_eq!(std::env::current_dir().unwrap(), parent);
    assert_eq!(std::env::var("LAPIS_TEST_INSTANCE").ok(), inherited);
}
#[tokio::test]
async fn malformed_readiness_and_lost_output_fail_without_replay() {
    let root = tempfile::tempdir().unwrap();
    helper(
        root.path(),
        "import json,sys\nprint(json.dumps({'type':'ready','version':999}),flush=True)\nsys.stdin.read()",
    );
    let cfg = Config::parse("version=1\n[runtime]\nacp='peer.py'", root.path()).unwrap();
    assert!(matches!(
        AcpClient::start(&cfg, &SystemResolver).await,
        Err(Error("invalid_acp_readiness"))
    ));
    helper(
        root.path(),
        "import sys\nprint('not json',flush=True)\nsys.stdin.read()",
    );
    let plan = PreparedLaunch::new(&cfg, Component::Acp, &SystemResolver).unwrap();
    let mut child = NativeProcess::spawn(plan).await.unwrap();
    assert_eq!(
        child.next(Duration::from_secs(3)).await,
        Err(Error("helper_protocol_failed"))
    );
    let _ = child.shutdown(Duration::from_secs(1)).await;
}
#[tokio::test]
async fn cancelled_shutdown_reaps_a_child_that_ignores_eof() {
    let root = tempfile::tempdir().unwrap();
    helper(
        root.path(),
        "import time,json\nprint(json.dumps({'pid':'ready'}),flush=True)\ntime.sleep(120)",
    );
    let cfg = Config::parse("version=1\n[runtime]\nagent='peer.py'", root.path()).unwrap();
    let mut child =
        NativeProcess::spawn(PreparedLaunch::new(&cfg, Component::Agent, &SystemResolver).unwrap())
            .await
            .unwrap();
    let pid = child.id().unwrap();
    child.next(Duration::from_secs(3)).await.unwrap();
    child.shutdown(Duration::from_millis(10)).await.unwrap();
    assert!(nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_err());
}
#[tokio::test]
async fn permission_response_preserves_the_exact_provider_option() {
    let root = tempfile::tempdir().unwrap();
    helper(
        root.path(),
        "import json,sys\nprint(json.dumps({'type':'ready','version':3}),flush=True)\nfor line in sys.stdin:\n print(line.strip(),flush=True)",
    );
    let cfg = Config::parse("version=1\n[runtime]\nacp='peer.py'", root.path()).unwrap();
    let mut client = AcpClient::start(&cfg, &SystemResolver).await.unwrap();
    client
        .commands()
        .permission(
            "decision",
            "session",
            "permission",
            Some("provider-specific-choice"),
        )
        .await
        .unwrap();
    let frame = client.next(Duration::from_secs(3)).await.unwrap();
    assert_eq!(frame["optionId"], "provider-specific-choice");
    assert_eq!(frame["sessionId"], "session");
    client.shutdown().await.unwrap();
}
#[tokio::test]
async fn oversized_prompt_is_rejected_locally_and_timeout_does_not_replay() {
    let root = tempfile::tempdir().unwrap();
    helper(
        root.path(),
        "import json,sys,time\nprint(json.dumps({'type':'ready','version':3}),flush=True)\nfor line in sys.stdin:\n frame=json.loads(line)\n print(json.dumps(frame),flush=True)\n time.sleep(.1)",
    );
    let config = Config::parse("version=1\n[runtime]\nacp='peer.py'", root.path()).unwrap();
    let mut client = AcpClient::start(&config, &SystemResolver).await.unwrap();
    let commands = client.commands();
    assert_eq!(
        commands
            .prompt(
                "large",
                "session",
                &"x".repeat(1_048_576),
                None,
                Duration::from_secs(1)
            )
            .await,
        Err(Error("request_limit"))
    );
    assert_eq!(
        client.next(Duration::from_millis(20)).await,
        Err(Error("outcome_unknown"))
    );
    commands
        .prompt(
            "once",
            "session",
            "hello",
            Some("context"),
            Duration::from_secs(7),
        )
        .await
        .unwrap();
    let frame = client.next(Duration::from_secs(3)).await.unwrap();
    assert_eq!(frame["requestId"], "once");
    assert_eq!(frame["maxDurationMs"], 7000);
    assert_eq!(frame["context"], "context");
    assert_eq!(
        client.next(Duration::from_millis(150)).await,
        Err(Error("outcome_unknown"))
    );
    client.shutdown().await.unwrap();
}
#[tokio::test]
async fn output_overflow_latches_coverage_loss() {
    let root = tempfile::tempdir().unwrap();
    helper(
        root.path(),
        "import json,sys\nfor i in range(1000): print(json.dumps({'sequence':i}),flush=True)\nsys.stdin.read()",
    );
    let config = Config::parse("version=1\n[runtime]\nagent='peer.py'", root.path()).unwrap();
    let mut child = NativeProcess::spawn(
        PreparedLaunch::new(&config, Component::Agent, &SystemResolver).unwrap(),
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        child.next(Duration::from_secs(1)).await,
        Err(Error("output_coverage_lost"))
    );
    assert_eq!(
        child.next(Duration::from_secs(1)).await,
        Err(Error("output_coverage_lost"))
    );
    // Closing the failed output pipe may make the helper exit with BrokenPipe.
    assert!(matches!(
        child.shutdown(Duration::from_secs(1)).await,
        Ok(()) | Err(Error("helper_failed"))
    ));
}
#[tokio::test]
async fn compute_control_bound_is_checked_before_writing_and_readiness_stays_loopback() {
    use lapis_ai_runtime::ComputeClient;
    struct Secrets;
    impl lapis_ai_runtime::Resolver for Secrets {
        fn environment(&self, _: &str) -> Option<String> {
            None
        }
        fn host_secret(&self, _: &str) -> Option<String> {
            Some("fixture-token-value".into())
        }
    }
    let root = tempfile::tempdir().unwrap();
    helper(
        root.path(),
        "import json,sys\nprint(json.dumps({'type':'ready','endpoint':'http://127.0.0.1:12345/v1'}),flush=True)\nfor line in sys.stdin:\n frame=json.loads(line)\n if frame.get('action')=='stop':break\n print(json.dumps({'type':'configured','requestId':frame['requestId']}),flush=True)",
    );
    let config=Config::parse("version=1\n[runtime]\ncompute='peer.py'\ndata_dir='state'\n[compute]\ntoken={host='token'}",root.path()).unwrap();
    let mut client = ComputeClient::start(&config, &Secrets).await.unwrap();
    assert_eq!(
        client
            .configure("large", serde_json::json!({"value":"x".repeat(65_536)}))
            .await,
        Err(Error("request_limit"))
    );
    assert_eq!(
        client
            .configure("small", serde_json::json!({}))
            .await
            .unwrap()["requestId"],
        "small"
    );
    client.shutdown().await.unwrap();
    helper(
        root.path(),
        "import json\nprint(json.dumps({'type':'ready','endpoint':'http://example.invalid/v1'}),flush=True)",
    );
    assert!(matches!(
        ComputeClient::start(&config, &Secrets).await,
        Err(Error("invalid_compute_readiness"))
    ));
}
#[tokio::test]
async fn agent_single_json_documents_need_no_trailing_newline_and_reject_extra_values() {
    use lapis_ai_runtime::AgentClient;
    let root = tempfile::tempdir().unwrap();
    helper(
        root.path(),
        "import json,sys\nif sys.argv[1]=='--metadata':\n value=json.load(sys.stdin)\n assert value['version']==1\n sys.stdout.write(json.dumps({'title':'Metadata','summary':'Fixture'}))\nelse:sys.stdout.write(json.dumps({'status':'ready'}))",
    );
    let config = Config::parse("version=1\n[runtime]\nagent='peer.py'", root.path()).unwrap();
    assert_eq!(
        AgentClient::models(&config, &SystemResolver).await.unwrap()["status"],
        "ready"
    );
    assert_eq!(
        AgentClient::metadata(
            &config,
            &SystemResolver,
            serde_json::json!({"version":1,"transcript":"Fixture"})
        )
        .await
        .unwrap()["title"],
        "Metadata"
    );
    helper(root.path(), "import sys\nsys.stdout.write('{}{}')");
    assert_eq!(
        AgentClient::models(&config, &SystemResolver).await,
        Err(Error("helper_protocol_failed"))
    );
}
