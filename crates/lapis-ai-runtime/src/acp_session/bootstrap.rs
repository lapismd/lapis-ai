use super::{SessionOptions, journal::Journal};
use crate::{AcpClient, Error, PreparedLaunch, Result};
use serde_json::Value;
use tokio::time::Instant;

pub async fn start(
    mut plan: PreparedLaunch,
    options: &SessionOptions,
    journal: &mut Journal,
) -> Result<(AcpClient, Value, u64)> {
    let root = plan
        .directory
        .canonicalize()
        .map_err(|_| Error("invalid_launch_root"))?;
    let cwd = options
        .directory
        .canonicalize()
        .map_err(|_| Error("invalid_session_directory"))?;
    if !root.is_dir() || !cwd.is_dir() || !cwd.starts_with(&root) {
        return Err(Error("session_directory_denied"));
    }
    plan.directory = root.clone();
    plan.override_environment("JJ_ACP_CWD", root.to_string_lossy())?;
    let mut client = AcpClient::start_prepared(plan).await?;
    let commands = client.commands();
    commands.warm("native-warm").await?;
    let (_, mut sequence) =
        receive(&mut client, options, "native-warm", "warmed", journal, 0).await?;
    commands
        .open(
            "native-open",
            &options.handle,
            &cwd,
            options.resume.as_deref(),
        )
        .await?;
    let (facts, next) = receive(
        &mut client,
        options,
        "native-open",
        "opened",
        journal,
        sequence,
    )
    .await?;
    sequence = next;
    if !facts["adapterSessionId"]
        .as_str()
        .is_some_and(super::request::label)
    {
        return Err(Error("invalid_adapter_session_identity"));
    }
    Ok((client, facts, sequence))
}
async fn receive(
    client: &mut AcpClient,
    options: &SessionOptions,
    id: &str,
    expected: &str,
    journal: &mut Journal,
    mut sequence: u64,
) -> Result<(Value, u64)> {
    let deadline = Instant::now() + options.request_timeout;
    loop {
        let frame = client
            .next(deadline.saturating_duration_since(Instant::now()))
            .await?;
        if frame["version"] != 3 || frame["requestId"] != id {
            return Err(Error("uncorrelated_response"));
        }
        if expected == "opened" && frame["sessionId"] != options.handle {
            return Err(Error("session_scope_mismatch"));
        }
        if let Some(value) = frame.get("sequence") {
            sequence = value
                .as_u64()
                .filter(|value| *value > sequence)
                .ok_or(Error("event_order_invalid"))?;
        }
        match frame["type"].as_str() {
            Some(kind) if kind == expected => return Ok((frame, sequence)),
            Some("queued") => {}
            Some("activity") => journal.emit(frame)?,
            Some("error") => {
                return Err(Error(match frame["code"].as_str() {
                    Some("resume_unsupported") => "resume_unsupported",
                    Some("agent_rejected") => "agent_rejected",
                    _ => "session_open_failed",
                }));
            }
            _ => return Err(Error("unexpected_session_response")),
        }
    }
}
