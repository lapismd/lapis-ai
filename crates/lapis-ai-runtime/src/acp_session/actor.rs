use super::{SessionOptions, journal::Journal, request::SessionRequest, routing::Routing};
use crate::{AcpClient, Error, Result};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::{
    sync::{mpsc, oneshot, watch},
    time::Instant,
};

pub struct Envelope {
    pub id: String,
    pub request: SessionRequest,
    pub reply: oneshot::Sender<Result<()>>,
}
pub async fn run(
    mut client: AcpClient,
    options: SessionOptions,
    journal: Arc<Mutex<Journal>>,
    mut queue: mpsc::Receiver<Envelope>,
    mut stop: watch::Receiver<bool>,
    done: watch::Sender<bool>,
    mut routing: Routing,
) {
    let commands = client.commands();
    let failure = loop {
        let deadline = routing
            .pending
            .values()
            .map(|pending| pending.deadline)
            .min()
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(86_400));
        tokio::select! {
            biased;
            _ = stop.changed() => break None,
            _ = tokio::time::sleep_until(deadline), if !routing.pending.is_empty() => break Some("outcome_unknown"),
            frame = client.next(Duration::from_secs(86_400)) => {
                let frame = match frame { Ok(frame) => frame, Err(error) => break Some(error.0) };
                let unresolved_before = routing.pending.keys().cloned().collect();
                let reusable = match routing.receive(&options.handle, &frame) {
                    Ok(reusable) => reusable, Err(error) => break Some(error.0),
                };
                let emitted = journal.lock().map_err(|_| Error("session_unavailable")).and_then(|mut journal| journal.emit(frame));
                if let Err(error) = emitted {
                    if let Ok(mut journal) = journal.lock() { journal.finish(Some(error.0), unresolved_before); }
                    break Some(error.0);
                }
                if !reusable { break Some("worker_not_reusable"); }
            }
            input = queue.recv() => {
                let Some(input) = input else { break None; };
                if let Err(error) = routing.admit(&input.id, &input.request, options.request_timeout) {
                    let _ = input.reply.send(Err(error));
                    continue;
                }
                let result = input.request.send(&commands, &input.id, &options.handle).await;
                let failed = result.err();
                let _ = input.reply.send(result);
                if let Some(error) = failed { break Some(error.0); }
            }
        }
    };
    // Publish failure before cleanup so a lost event can never look like success.
    let unresolved = routing.pending.keys().cloned().collect();
    if let Ok(mut journal) = journal.lock() {
        journal.finish(failure, unresolved);
    }
    let result = client.shutdown().await;
    if let Err(error) = result
        && let Ok(mut journal) = journal.lock()
    {
        journal.cleanup_failed(error.0);
    }
    done.send_replace(true);
}
