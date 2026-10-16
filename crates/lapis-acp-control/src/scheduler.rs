//! Submission scheduling: scope serialization, worker slots and bounded waiting.
//!
//! A scope has at most one turn in flight. Distinct scopes run concurrently up to
//! the worker limit, and a bounded queue absorbs bursts. The scope itself is opaque
//! here on purpose: upstream scoped by channel and thread because a relay has those
//! concepts, while a controlling application decides what a scope means and this
//! module only guarantees that two turns in the same scope never overlap.
//!
//! Selection is first-fit over the queue rather than strictly FIFO, because a job
//! whose scope is busy must not block an unrelated scope behind it. Order within a
//! scope is preserved, which is the guarantee that actually matters.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use crate::invocation::{InvocationConfig, invoke_with_probe};
use crate::protocol::Response;
use crate::task::TaskDocument;
use crate::transport::FrameSink;
use lapis_acp_agent::availability::PathProbe;
use tokio::sync::{Semaphore, mpsc, watch};

/// The scope a submission lands in when the caller does not name one.
///
/// Defaulting to one shared scope preserves the serial behaviour a controlling
/// application saw before scopes existed, so adopting them is opt-in rather than a
/// change in semantics for frames that did not ask for one.
pub const DEFAULT_SCOPE: &str = "default";

/// Pool and queue sizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServeLimits {
    /// How many submissions may execute at once.
    pub workers: usize,
    /// How many may wait. A full queue is refused rather than grown, because an
    /// unbounded queue turns backpressure into memory exhaustion.
    pub queue_capacity: usize,
}

impl Default for ServeLimits {
    fn default() -> Self {
        Self {
            workers: 1,
            queue_capacity: 16,
        }
    }
}

/// One accepted submission waiting for a scope and a worker.
pub struct Job {
    /// Correlates the terminal record to the submit frame.
    pub request_id: String,
    /// The serialization scope.
    pub scope: String,
    /// The validated task document.
    pub task: TaskDocument,
}

/// One running submission, and the handle that cancels it.
pub struct Running {
    /// The submission's correlation identifier.
    pub request_id: String,
    /// Signals cooperative cancellation.
    pub cancel: watch::Sender<bool>,
}

/// A finished submission, routed back through the loop.
///
/// The frames are emitted by the loop rather than by the task that produced them,
/// so the scope is released before a controlling application can observe the
/// record. Emitting directly from the task would let a peer see a record and then
/// ask for status that still listed the submission as running, which is a
/// contradiction it has no way to resolve.
pub struct Completion {
    /// The scope to release.
    pub scope: String,
    /// Diagnostics followed by the terminal record.
    pub frames: Vec<Response>,
}

/// Move every dispatchable job into a spawned task.
pub fn dispatch(
    pending: &mut VecDeque<Job>,
    running: &mut HashMap<String, Running>,
    slots: &Arc<Semaphore>,
    config: &Arc<InvocationConfig>,
    done_tx: &mpsc::UnboundedSender<Completion>,
) {
    let mut index = 0;
    while index < pending.len() {
        if running.contains_key(&pending[index].scope) {
            index += 1;
            continue;
        }
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            // No worker free. Everything behind this point waits too, so there is
            // nothing to gain by scanning further.
            return;
        };
        let job = pending.remove(index).expect("index was in range");
        let (cancel, receiver) = watch::channel(false);
        running.insert(
            job.scope.clone(),
            Running {
                request_id: job.request_id.clone(),
                cancel,
            },
        );
        let config = config.clone();
        let done_tx = done_tx.clone();
        tokio::spawn(async move {
            // The permit is held for the whole turn, so releasing it on drop is
            // what makes a crashed agent give its slot back.
            let _permit = permit;
            let Job {
                request_id,
                scope,
                task,
            } = job;
            let mut receiver = receiver;
            let invocation = invoke_with_probe(&config, task, &mut receiver, &PathProbe).await;
            let mut frames = Vec::with_capacity(invocation.diagnostics.len() + 1);
            for message in &invocation.diagnostics {
                frames.push(Response::Diagnostic {
                    message: message.clone(),
                });
            }
            frames.push(Response::Record {
                request_id,
                record: invocation.record,
            });
            let _ = done_tx.send(Completion { scope, frames });
        });
    }
}

/// Cancel running submissions and wait, within a bound, for their slots to return.
///
/// The bound is derived from the same cleanup intervals a single invocation uses,
/// so a shutdown cannot hang indefinitely on an agent that will not stop. When the
/// bound elapses first, that is reported rather than concealed: a peer that is
/// told the shutdown was clean when it was not has no way to detect an orphan.
pub async fn drain<K: FrameSink>(
    running: &HashMap<String, Running>,
    slots: &Arc<Semaphore>,
    sink: &Arc<K>,
    workers: usize,
    cleanup: Duration,
) {
    if running.is_empty() {
        return;
    }
    for entry in running.values() {
        let _ = entry.cancel.send(true);
    }
    let deadline = tokio::time::Instant::now() + cleanup;
    let wanted = workers.max(1);
    while slots.available_permits() < wanted && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    if slots.available_permits() < wanted {
        let _ = sink
            .send(&Response::Diagnostic {
                message: "shutdown did not wait for every running submission to finish".to_owned(),
            })
            .await;
    }
}
