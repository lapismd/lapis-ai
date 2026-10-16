//! The persistent control loop.
//!
//! One controlling application drives one harness over a pair of frame streams. The
//! loop reads frames, admits submissions into the scheduler, and emits responses.
//! It never blocks on a submission, so a `cancel`, a `status` query or a `shutdown`
//! is always serviceable while a turn is in flight — a peer that cannot interrupt a
//! running task has no way to bound it.
//!
//! A crashed agent is reported and its slot released. It is never re-dispatched: a
//! turn whose outcome is unknown may already have produced side effects, so running
//! it again is the controlling application's decision to make with its own durable
//! state, not this loop's to make automatically.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use lapis_acp_agent::Catalog;
use lapis_acp_agent::availability::PathProbe;
use serde_json::{Value, json};
use tokio::sync::{Semaphore, mpsc};

use crate::invocation::InvocationConfig;
use crate::protocol::{Request, Response, code_control, parse_request_text};
use crate::scheduler::{Completion, Job, ServeLimits, dispatch, drain};
use crate::transport::{FrameSink, FrameSource};

/// How the control loop ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServeExit {
    /// The controlling application asked for shutdown.
    Shutdown,
    /// The input stream ended.
    Eof,
    /// The input stream failed.
    InputFailed,
}

/// Everything the loop needs, assembled once at startup.
pub struct ServeConfig {
    /// Trusted launch configuration shared with the one-shot path, so a flag
    /// cannot mean one thing in `run` and something else in `serve`.
    pub invocation: InvocationConfig,
    /// Pool and queue sizing.
    pub limits: ServeLimits,
}

/// Summarize the enabled runners for a control frame.
///
/// Availability is probed, never installed, and an unavailable runner reports its
/// guidance so a controlling application can show an operator what to do.
pub fn runner_summaries(catalog: &Catalog) -> Vec<Value> {
    lapis_acp_agent::availability::probe_all(catalog.list(), &PathProbe)
        .into_iter()
        .map(|report| {
            json!({
                "id": report.runner_id,
                "label": report.label,
                "available": report.available(),
                "command": report.resolved.as_ref().map(|spec| spec.command.clone()),
                "args": report.resolved.as_ref().map(|spec| spec.args.clone()),
                "installHint": report.install_hint,
                "installInstructionsUrl": report.install_instructions_url,
            })
        })
        .collect()
}

/// Run the control loop until shutdown or end of input.
pub async fn serve<S, K>(source: &mut S, sink: Arc<K>, config: ServeConfig) -> ServeExit
where
    S: FrameSource,
    K: FrameSink + 'static,
{
    let limits = config.limits;
    let cleanup = config.invocation.cancel_grace + config.invocation.reap;
    let invocation = Arc::new(config.invocation);
    let slots = Arc::new(Semaphore::new(limits.workers.max(1)));
    let (done_tx, mut done_rx) = mpsc::unbounded_channel::<Completion>();

    let mut pending: VecDeque<Job> = VecDeque::new();
    let mut running: HashMap<String, crate::scheduler::Running> = HashMap::new();

    emit(
        &sink,
        &Response::Ready {
            runners: runner_summaries(&invocation.catalog),
            workers: limits.workers,
            queue_capacity: limits.queue_capacity,
        },
    )
    .await;

    loop {
        dispatch(&mut pending, &mut running, &slots, &invocation, &done_tx);
        tokio::select! {
            line = source.next_line() => match line {
                Ok(Some(line)) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if handle(&line, &sink, &limits, &invocation, &mut pending, &mut running).await {
                        break;
                    }
                }
                Ok(None) => {
                    drain(&running, &slots, &sink, limits.workers, cleanup).await;
                    return ServeExit::Eof;
                }
                Err(_) => {
                    drain(&running, &slots, &sink, limits.workers, cleanup).await;
                    return ServeExit::InputFailed;
                }
            },
            Some(completion) = done_rx.recv() => {
                // Released before the frames are emitted, so a peer that reacts to
                // a record and then asks for status never sees a contradiction.
                running.remove(&completion.scope);
                for frame in &completion.frames {
                    emit(&sink, frame).await;
                }
            }
        }
    }
    drain(&running, &slots, &sink, limits.workers, cleanup).await;
    ServeExit::Shutdown
}

/// Handle one inbound frame. Returns true when the loop should stop.
#[allow(clippy::too_many_arguments)]
async fn handle<K: FrameSink>(
    line: &str,
    sink: &Arc<K>,
    limits: &ServeLimits,
    invocation: &Arc<InvocationConfig>,
    pending: &mut VecDeque<Job>,
    running: &mut HashMap<String, crate::scheduler::Running>,
) -> bool {
    match parse_request_text(line) {
        Err(rejection) => {
            emit(sink, &refusal(line, &rejection)).await;
            false
        }
        Ok(Request::Runners { request_id }) => {
            emit(
                sink,
                &Response::Runners {
                    request_id,
                    runners: runner_summaries(&invocation.catalog),
                },
            )
            .await;
            false
        }
        Ok(Request::Status { request_id }) => {
            let mut scopes: Vec<String> = running.keys().cloned().collect();
            scopes.sort();
            emit(
                sink,
                &Response::Status {
                    request_id,
                    workers: limits.workers,
                    queue_capacity: limits.queue_capacity,
                    queued: pending.len(),
                    running: scopes,
                },
            )
            .await;
            false
        }
        Ok(Request::Shutdown { request_id }) => {
            emit(sink, &Response::Ack { request_id }).await;
            true
        }
        Ok(Request::Cancel { request_id, target }) => {
            let matched = running
                .values()
                .find(|entry| entry.request_id == target)
                .map(|entry| entry.cancel.clone());
            match matched {
                Some(cancel) => {
                    let _ = cancel.send(true);
                    emit(sink, &Response::Ack { request_id }).await;
                }
                None => {
                    emit(
                        sink,
                        &Response::Error {
                            request_id: Some(request_id),
                            code: code_control::NOT_IN_FLIGHT.to_owned(),
                            reason: format!("no submission {target:?} is running"),
                        },
                    )
                    .await
                }
            }
            false
        }
        Ok(Request::Submit {
            request_id,
            scope,
            task,
        }) => {
            if pending.len() >= limits.queue_capacity {
                emit(
                    sink,
                    &Response::Error {
                        request_id: Some(request_id),
                        code: code_control::QUEUE_FULL.to_owned(),
                        reason: format!(
                            "{} submissions are already waiting",
                            limits.queue_capacity
                        ),
                    },
                )
                .await;
                return false;
            }
            let position = pending.len();
            pending.push_back(Job {
                request_id: request_id.clone(),
                scope: scope.clone(),
                task,
            });
            // Admission is acknowledged, so a peer can tell a submission that is
            // waiting from one that was silently dropped. A submission queued
            // behind its own scope is not refused: serializing a scope means the
            // second turn waits, not that it is rejected.
            emit(
                sink,
                &Response::Queued {
                    request_id,
                    scope,
                    position,
                },
            )
            .await;
            false
        }
    }
}

async fn emit<K: FrameSink>(sink: &Arc<K>, response: &Response) {
    // A write failure means the controlling application is gone; there is nobody
    // left to tell, so it is dropped rather than escalated.
    let _ = sink.send(response).await;
}

/// Build a refusal frame, recovering the correlation id when one is present.
///
/// A controlling application that sent a malformed frame still needs to correlate
/// the refusal, so the identifier is read from the raw text rather than from a parse
/// that already failed.
fn refusal(line: &str, rejection: &crate::task::TaskRejection) -> Response {
    let request_id = serde_json::from_str::<Value>(line)
        .ok()
        .and_then(|frame| {
            frame
                .get("requestId")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .filter(|value| !value.is_empty());
    if is_task_outcome(rejection.code) {
        // A task document problem is reported as a terminal record so a peer
        // handles one shape for every task outcome.
        return Response::Record {
            request_id: request_id.unwrap_or_default(),
            record: crate::record::TerminalRecord::invalid(rejection.code),
        };
    }
    Response::Error {
        request_id,
        code: rejection.code.to_owned(),
        reason: rejection.reason.clone(),
    }
}

fn is_task_outcome(code: &str) -> bool {
    use crate::task::code as task_code;
    matches!(
        code,
        task_code::INVALID_TASK_JSON
            | task_code::TASK_TOO_LARGE
            | task_code::UNSUPPORTED_TASK_VERSION
            | task_code::INVALID_TASK_ID
            | task_code::INVALID_PROMPT
            | task_code::INVALID_DURATION
            | task_code::INVALID_RUNNER
            | task_code::RUNNER_MISMATCH
    )
}
