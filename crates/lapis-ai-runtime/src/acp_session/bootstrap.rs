use super::{
    SessionOptions,
    journal::Journal,
    processing::{FramePublication, Processing},
};
use crate::{AcpClient, Error, PreparedLaunch, Result};
use serde_json::Value;
use tokio::time::Instant;

pub async fn start(
    mut plan: PreparedLaunch,
    options: &SessionOptions,
    journal: &mut Journal,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
    processing: &mut Processing,
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
    let mut client = AcpClient::start_prepared_cancellable(plan, cancel).await?;
    let result = initialize(&mut client, options, journal, &cwd, cancel, processing).await;
    match result {
        Ok((facts, sequence)) => Ok((client, facts, sequence)),
        Err(error) => {
            client
                .shutdown()
                .await
                .map_err(|_| Error("startup_cleanup_unknown"))?;
            Err(error)
        }
    }
}
async fn initialize(
    client: &mut AcpClient,
    options: &SessionOptions,
    journal: &mut Journal,
    cwd: &std::path::Path,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
    processing: &mut Processing,
) -> Result<(Value, u64)> {
    let mut events = StartupEvents {
        options,
        journal,
        cancel,
        processing,
    };
    let commands = client.commands();
    cancelled(events.cancel)?;
    tokio::select! {
        biased;
        _ = events.cancel.changed() => Err(Error("startup_cancelled")),
        result = commands.warm("native-warm") => result,
    }?;
    let (_, mut sequence) = events.receive(client, "native-warm", "warmed", 0).await?;
    tokio::select! {
        biased;
        _ = events.cancel.changed() => Err(Error("startup_cancelled")),
        result = commands.open(
            "native-open",
            &options.handle,
            cwd,
            options.resume.as_deref(),
        ) => result,
    }?;
    let (facts, next) = events
        .receive(client, "native-open", "opened", sequence)
        .await?;
    sequence = next;
    if !facts["adapterSessionId"]
        .as_str()
        .is_some_and(super::request::label)
    {
        return Err(Error("invalid_adapter_session_identity"));
    }
    if options
        .resume
        .as_ref()
        .is_some_and(|resume| facts["adapterSessionId"] != *resume)
    {
        return Err(Error("provider_resume_mismatch"));
    }
    let facts = events
        .processing
        .prepare(facts, FramePublication::Facts)
        .await?;
    cancelled(events.cancel)?;
    Ok((facts, sequence))
}
struct StartupEvents<'a> {
    options: &'a SessionOptions,
    journal: &'a mut Journal,
    cancel: &'a mut tokio::sync::watch::Receiver<bool>,
    processing: &'a mut Processing,
}
impl StartupEvents<'_> {
    async fn receive(
        &mut self,
        client: &mut AcpClient,
        id: &str,
        expected: &str,
        mut sequence: u64,
    ) -> Result<(Value, u64)> {
        let deadline = Instant::now() + self.options.request_timeout;
        loop {
            cancelled(self.cancel)?;
            let frame = tokio::select! {
                biased;
                _ = self.cancel.changed() => Err(Error("startup_cancelled")),
                frame = client.next(deadline.saturating_duration_since(Instant::now())) => frame,
            }?;
            if frame["version"] != 3 || frame["requestId"] != id {
                return Err(Error("uncorrelated_response"));
            }
            if expected == "opened" && frame["sessionId"] != self.options.handle {
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
                Some("activity") => {
                    let sequence = self.journal.next_sequence()?;
                    let frame = self
                        .processing
                        .prepare(frame, FramePublication::Event { sequence })
                        .await?;
                    cancelled(self.cancel)?;
                    self.journal.emit(frame)?;
                }
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
}
fn cancelled(cancel: &tokio::sync::watch::Receiver<bool>) -> Result<()> {
    if *cancel.borrow() || cancel.has_changed().is_err() {
        Err(Error("startup_cancelled"))
    } else {
        Ok(())
    }
}
