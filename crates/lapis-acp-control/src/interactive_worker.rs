//! Serial work on a retained interactive session.
use crate::interactive_handler::{Events, Handler, Permissions};
use crate::interactive_session::{WarmSession, report_value};
use crate::interactive_wire::Command;
use crate::invocation::InvocationConfig;
use lapis_acp_runner::{SessionConfiguration, TurnStatus};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

/// A command admitted into the bounded session queue.
pub struct Job {
    pub command: Command,
    pub cancel: watch::Receiver<bool>,
}
/// Active request cancellation owners.
pub type Requests = Arc<Mutex<HashMap<String, watch::Sender<bool>>>>;
/// Execute commands in order; no completed or uncertain command is retried.
pub async fn run(
    mut queue: mpsc::Receiver<Job>,
    config: InvocationConfig,
    events: Events,
    permissions: Permissions,
    requests: Requests,
    active: Arc<Mutex<Option<String>>>,
) {
    let mut session: Option<WarmSession> = None;
    while let Some(mut job) = queue.recv().await {
        let id = job.command.id().to_owned();
        if let Ok(mut value) = active.lock() {
            *value = Some(id.clone());
        }
        let mut handler = Handler {
            events: events.clone(),
            request_id: id.clone(),
            permissions: permissions.clone(),
            timeout: config.absolute_cap,
            pending_rpc: None,
            adapter_session: None,
            known_sessions: Default::default(),
        };
        if *job.cancel.borrow() || *events.cancel.borrow() {
            events
                .emit_wait(
                    "record",
                    &id,
                    json!({"record":{"version":1,"taskId":id,"status":"cancelled"},"quiescent":true,
                    "sessionUsable":session.is_some() && !*events.cancel.borrow()}),
                )
                .await;
        } else {
            match job.command {
                Command::Open {
                    load_session_id, ..
                } => {
                    if session.is_some() {
                        events
                            .emit_wait("error", &id, json!({"code":"session_already_open"}))
                            .await;
                    } else {
                        match WarmSession::open(
                            &config,
                            &mut handler,
                            load_session_id.as_deref(),
                            &mut job.cancel,
                        )
                        .await
                        {
                            Ok(value) => {
                                events.emit_wait("opened", &id, value.facts()).await;
                                session = Some(value);
                            }
                            Err(code) => events.emit_wait("error", &id, json!({"code":code})).await,
                        }
                    }
                }
                Command::Configure {
                    model,
                    effort,
                    mode,
                    ..
                } => {
                    if let Some(value) = session.as_mut() {
                        match value.configure(&SessionConfiguration{model,effort,mode},&mut handler).await {
                            Ok(report)=>events.emit_wait("configured",&id,json!({"configuration":report_value(&report),"modeReady":value.mode_ready})).await,
                            Err(code)=>{ events.emit_wait("error",&id,json!({"code":code})).await; if let Some(value)=session.take(){value.close(&config).await;} },
                        }
                    } else {
                        events
                            .emit_wait("error", &id, json!({"code":"session_not_open"}))
                            .await;
                    }
                }
                Command::Prompt {
                    text,
                    context,
                    max_duration_ms,
                    ..
                } => {
                    if let Some(value) = session.as_mut() {
                        if !value.mode_ready {
                            events
                                .emit_wait("error", &id, json!({"code":"execution_mode_required"}))
                                .await;
                        } else {
                            events
                                .emit_wait("started", &id, json!({"sessionId":value.session.id}))
                                .await;
                            let budget = lapis_acp_protocol::budget::TurnBudget::new(
                                config.idle,
                                Duration::from_millis(max_duration_ms).min(config.absolute_cap),
                            );
                            let outcome = lapis_acp_runner::persistent::prompt_input(
                                &mut value.agent,
                                &mut handler,
                                &value.session.id,
                                lapis_acp_runner::persistent::PromptInput { text: &text, context: context.as_deref(), embedded_context: value.initialized["promptCapabilities"]["embeddedContext"] == true },
                                budget,
                                config.cancel_grace,
                                &mut job.cancel,
                            )
                            .await;
                            let (status, code) = match outcome.status {
                                TurnStatus::Completed => ("completed", None),
                                TurnStatus::Cancelled => ("cancelled", None),
                                TurnStatus::TimedOut => ("timed_out", Some("timed_out")),
                                TurnStatus::Failed => ("failed", Some("agent_or_protocol_failure")),
                            };
                            let mut quiescent = matches!(
                                outcome.status,
                                TurnStatus::Completed | TurnStatus::Cancelled
                            ) && !events
                                .coverage_lost
                                .load(std::sync::atomic::Ordering::Acquire);
                            if !quiescent && let Some(value) = session.take() {
                                quiescent = value.close(&config).await;
                            }
                            events.emit_wait("record",&id,json!({"record":{"version":1,"taskId":id,"status":status,
                                "stopReason":outcome.stop_reason,"error":code},"quiescent":quiescent,"sessionUsable":session.is_some()})).await;
                        }
                    } else {
                        events
                            .emit_wait("error", &id, json!({"code":"session_not_open"}))
                            .await;
                    }
                }
                _ => {}
            }
        }
        if let Ok(mut map) = requests.lock() {
            map.remove(&id);
        }
        if let Ok(mut value) = active.lock() {
            *value = None;
        }
    }
    let stopped = if let Some(value) = session {
        value.close(&config).await
    } else {
        true
    };
    events
        .emit_wait("closed", "close", json!({"confirmedStopped":stopped}))
        .await;
}
