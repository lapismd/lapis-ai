//! Serial commands for bounded sessions on an initialized worker.
use crate::{
    fleet_wire::Request,
    interactive_handler::{Events, Handler, Permissions},
    interactive_session::{classify, report_value},
    interactive_wire::Command,
    invocation::InvocationConfig,
    warm_worker::WarmWorker,
};
use lapis_acp_protocol::{budget::TurnBudget, session::Session};
use lapis_acp_runner::{SessionConfiguration, TurnStatus};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, watch};

pub struct Job {
    pub request: Request,
    pub cancel: watch::Receiver<bool>,
}
pub type Requests = Arc<Mutex<HashMap<String, (Option<String>, watch::Sender<bool>)>>>;
struct Scope {
    session: Session,
    mode_ready: bool,
}
struct Worker {
    warm: Option<WarmWorker>,
    scopes: HashMap<String, Scope>,
    opened: usize,
    known_sessions: std::collections::HashSet<String>,
}
impl Worker {
    async fn execute(
        &mut self,
        request: Request,
        config: &InvocationConfig,
        handler: &mut Handler,
        cancel: &mut watch::Receiver<bool>,
    ) -> Result<(), &'static str> {
        let id = handler.request_id.clone();
        match request {
            Request::Warm(_) => {
                if self.warm.is_none() {
                    self.warm = Some(WarmWorker::start(config, handler, cancel).await?);
                }
                handler.events.emit(
                    "warmed",
                    &id,
                    json!({"capabilities":self.warm.as_ref().map(|w| &w.initialized.capabilities)}),
                );
            }
            Request::Release { scope, .. } => {
                if self.scopes.remove(&scope).is_none() {
                    return Err("session_not_open");
                }
                handler.events.emit("session_closed", &id, json!({}));
            }
            Request::Session {
                command: Command::Open {
                    load_session_id, ..
                },
                scope,
                cwd,
            } => {
                if self.scopes.contains_key(&scope) {
                    return Err("session_already_open");
                }
                if self.opened >= 64 {
                    return Err("session_limit");
                }
                let warm = self.warm.as_mut().ok_or("worker_not_warm")?;
                let session = warm
                    .open(
                        config,
                        handler,
                        cwd.as_deref().ok_or("invalid_session_directory")?,
                        load_session_id.as_deref(),
                        cancel,
                    )
                    .await?;
                handler.events.emit("opened", &id, json!({"adapterSessionId":session.id,"session":session.raw,"capabilities":warm.initialized.capabilities}));
                self.known_sessions.insert(session.id.clone());
                self.scopes.insert(
                    scope,
                    Scope {
                        session,
                        mode_ready: false,
                    },
                );
                self.opened += 1;
            }
            Request::Session {
                command:
                    Command::Configure {
                        model,
                        effort,
                        mode,
                        ..
                    },
                scope,
                ..
            } => {
                let session = self.scopes.get_mut(&scope).ok_or("session_not_open")?;
                let warm = self.warm.as_mut().ok_or("worker_not_warm")?;
                handler.adapter_session = Some(session.session.id.clone());
                let preferences = SessionConfiguration {
                    model,
                    effort,
                    mode: mode.clone(),
                };
                let report = tokio::select! {
                    result = lapis_acp_runner::apply(&mut warm.agent.connection, handler, &session.session.id, &session.session.raw, &preferences) => result.map_err(|e| classify(&e))?,
                    () = lapis_acp_runner::persistent::cancellation(cancel) => return Err("cancelled"),
                };
                session.mode_ready = mode.is_some() && report.mode.applied();
                handler.events.emit(
                    "configured",
                    &id,
                    json!({"configuration":report_value(&report),"modeReady":session.mode_ready}),
                );
            }
            Request::Session {
                command:
                    Command::Prompt {
                        text,
                        context,
                        max_duration_ms,
                        ..
                    },
                scope,
                ..
            } => {
                let session = self.scopes.get(&scope).ok_or("session_not_open")?;
                if !session.mode_ready {
                    return Err("execution_mode_required");
                }
                let warm = self.warm.as_mut().ok_or("worker_not_warm")?;
                handler.adapter_session = Some(session.session.id.clone());
                handler.events.emit(
                    "started",
                    &id,
                    json!({"adapterSessionId":session.session.id}),
                );
                let outcome = lapis_acp_runner::persistent::prompt_input(
                    &mut warm.agent,
                    handler,
                    &session.session.id,
                    lapis_acp_runner::persistent::PromptInput {
                        text: &text,
                        context: context.as_deref(),
                        embedded_context: warm
                            .initialized
                            .advertises(&["promptCapabilities", "embeddedContext"]),
                    },
                    TurnBudget::new(
                        config.idle,
                        Duration::from_millis(max_duration_ms).min(config.absolute_cap),
                    ),
                    config.cancel_grace,
                    cancel,
                )
                .await;
                let usable = matches!(
                    outcome.status,
                    TurnStatus::Completed | TurnStatus::Cancelled
                ) && !handler
                    .events
                    .coverage_lost
                    .load(std::sync::atomic::Ordering::Acquire);
                let status = match outcome.status {
                    TurnStatus::Completed => "completed",
                    TurnStatus::Cancelled => "cancelled",
                    _ => "outcome_unknown",
                };
                let quiescent = if usable {
                    true
                } else {
                    self.stop(config).await
                };
                handler.events.emit("record", &id, json!({"record":{"version":1,"taskId":id,"status":status,"stopReason":outcome.stop_reason,"failureKind":outcome.failure.as_ref().map(|f|format!("{:?}",f.kind)),"agentCode":outcome.failure.as_ref().and_then(|f|f.agent_code),"timeoutKind":outcome.timeout.map(|t|format!("{t:?}"))},"quiescent":quiescent,"sessionUsable":usable}));
            }
            _ => return Err("invalid_request"),
        }
        Ok(())
    }
    async fn stop(&mut self, config: &InvocationConfig) -> bool {
        self.scopes.clear();
        if let Some(warm) = self.warm.take() {
            warm.stop(config).await
        } else {
            true
        }
    }
}
pub async fn run(
    mut queue: mpsc::Receiver<Job>,
    config: InvocationConfig,
    events: Events,
    permissions: Permissions,
    requests: Requests,
    active: Arc<Mutex<Option<String>>>,
) {
    let mut worker = Worker {
        warm: None,
        scopes: HashMap::new(),
        opened: 0,
        known_sessions: Default::default(),
    };
    while let Some(mut job) = queue.recv().await {
        let id = job.request.id().to_owned();
        let scope = job.request.scope().map(str::to_owned);
        let mut scoped = events.clone();
        scoped.scope = job.request.scope().map(str::to_owned);
        if let Ok(mut value) = active.lock() {
            *value = Some(id.clone());
        }
        let mut handler = Handler {
            events: scoped,
            request_id: id.clone(),
            permissions: permissions.clone(),
            timeout: config.absolute_cap,
            pending_rpc: None,
            adapter_session: None,
            known_sessions: worker.known_sessions.clone(),
        };
        if *job.cancel.borrow() || *events.cancel.borrow() {
            handler.events.emit("record", &id, json!({"record":{"version":1,"taskId":id,"status":"cancelled"},"quiescent":true,"sessionUsable":worker.warm.is_some() && !*events.cancel.borrow()}));
        } else if let Err(code) = worker
            .execute(job.request, &config, &mut handler, &mut job.cancel)
            .await
        {
            if code == "agent_rejected"
                && let Some(scope) = scope
            {
                worker.scopes.remove(&scope);
            }
            if matches!(
                code,
                "timed_out" | "agent_exited" | "protocol_error" | "cancelled"
            ) {
                worker.stop(&config).await;
            }
            handler.events.emit("error", &id, json!({"code":code}));
        }
        if let Ok(mut map) = requests.lock() {
            map.remove(&id);
        }
        if let Ok(mut value) = active.lock() {
            *value = None;
        }
    }
    let stopped = worker.stop(&config).await;
    events.emit("closed", "close", json!({"confirmedStopped":stopped}));
}
