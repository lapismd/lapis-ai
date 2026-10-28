use super::{
    AcpSession, Inner, SessionOptions, actor, bootstrap, journal,
    processing::{Processing, SessionProcessor},
    routing,
};
use crate::{Error, PreparedLaunch, Result};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot, watch};

pub async fn start(
    plan: PreparedLaunch,
    options: SessionOptions,
    mut cancel: watch::Receiver<bool>,
    processor: Option<Box<dyn SessionProcessor>>,
) -> Result<AcpSession> {
    options.validate()?;
    if *cancel.borrow() {
        return Err(Error("startup_cancelled"));
    }
    let (stop, signal) = watch::channel(false);
    let (mut reply, received) = oneshot::channel();
    // The owner survives cancellation of the caller's wait. In-flight processing
    // must finish before startup cleanup or the final worker handoff can finish.
    tokio::spawn(async move {
        let starting = build(plan, options, signal, processor);
        tokio::pin!(starting);
        let (result, cancelled) = tokio::select! {
            biased;
            _ = reply.closed() => { stop.send_replace(true); ((&mut starting).await, true) },
            _ = cancel.changed() => { stop.send_replace(true); ((&mut starting).await, true) },
            result = &mut starting => (result, false),
        };
        let result = if cancelled {
            match result {
                Ok(session) => match session.close().await {
                    Ok(()) => Err(Error("startup_cancelled")),
                    Err(_) => Err(Error("startup_cleanup_unknown")),
                },
                Err(error) => Err(error),
            }
        } else {
            result
        };
        if let Err(Ok(session)) = reply.send(result) {
            let _ = session.close().await;
        }
    });
    received
        .await
        .map_err(|_| Error("startup_cleanup_unknown"))?
}
async fn build(
    plan: PreparedLaunch,
    options: SessionOptions,
    mut cancel: watch::Receiver<bool>,
    processor: Option<Box<dyn SessionProcessor>>,
) -> Result<AcpSession> {
    let mut journal = journal::Journal::new(options.event_limit, options.event_byte_limit);
    let mut processing =
        Processing::new(options.handle.clone(), options.event_byte_limit, processor);
    let (client, facts, sequence) =
        bootstrap::start(plan, &options, &mut journal, &mut cancel, &mut processing).await?;
    let journal = Arc::new(Mutex::new(journal));
    let (commands, queue) = mpsc::channel(32);
    let (stop, cancel) = watch::channel(false);
    let (finished, done) = watch::channel(false);
    let id = options.handle.clone();
    let routing = routing::Routing::new(
        sequence,
        facts["adapterSessionId"]
            .as_str()
            .ok_or(Error("invalid_adapter_session_identity"))?
            .to_owned(),
    );
    tokio::spawn(
        actor::Actor {
            client,
            options,
            journal: journal.clone(),
            queue,
            stop: cancel,
            done: finished,
            routing,
            processing,
        }
        .run(),
    );
    Ok(AcpSession(Arc::new(Inner {
        id,
        facts,
        journal,
        commands,
        stop,
        done,
    })))
}
