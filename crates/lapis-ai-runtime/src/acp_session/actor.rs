use super::{
    SessionOptions,
    journal::Journal,
    processing::{FramePublication, Processing},
    request::SessionRequest,
    routing::Routing,
};
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
pub struct Actor {
    pub client: AcpClient,
    pub options: SessionOptions,
    pub journal: Arc<Mutex<Journal>>,
    pub queue: mpsc::Receiver<Envelope>,
    pub stop: watch::Receiver<bool>,
    pub done: watch::Sender<bool>,
    pub routing: Routing,
    pub processing: Processing,
}
impl Actor {
    pub async fn run(self) {
        let Self {
            mut client,
            options,
            journal,
            mut queue,
            mut stop,
            done,
            mut routing,
            mut processing,
        } = self;
        let commands = client.commands();
        let mut unresolved_override = None;
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
                    let emitted = async {
                        let sequence = {
                            journal.lock().map_err(|_| Error("session_unavailable"))?.next_sequence()?
                        };
                        let frame = processing.prepare(frame, FramePublication::Event { sequence }).await?;
                        journal.lock().map_err(|_| Error("session_unavailable"))?.emit(frame)
                    }.await;
                    if let Err(error) = emitted {
                        unresolved_override = Some(unresolved_before);
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
        queue.close();
        while let Ok(input) = queue.try_recv() {
            let _ = input.reply.send(Err(Error("session_closed")));
        }
        // Publish failure before cleanup so a lost event can never look like success.
        let unresolved =
            unresolved_override.unwrap_or_else(|| routing.pending.keys().cloned().collect());
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
}
