//! Interactive version-2 control with one session and bounded streams.
use crate::interactive_handler::{Events, Permissions};
use crate::interactive_wire::{Command, error, frame, parse};
use crate::interactive_worker::{Job, Requests};
use crate::invocation::InvocationConfig;
use crate::transport::{FrameSource, LineSink};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64},
};
use std::time::Duration;
use tokio::io::AsyncWrite;
use tokio::sync::{mpsc, watch};

/// Serve exactly one application-owned session until EOF, close or shutdown.
pub async fn serve<S: FrameSource, W: AsyncWrite + Unpin + Send + 'static>(
    source: &mut S,
    sink: Arc<LineSink<W>>,
    config: InvocationConfig,
    capacity: usize,
    mut shutdown: watch::Receiver<bool>,
) {
    let (output, mut responses) = mpsc::channel(256);
    let (cancel, _) = watch::channel(false);
    let events = Events {
        output,
        sequence: Arc::new(AtomicU64::new(0)),
        cancel,
        coverage_lost: Arc::new(AtomicBool::new(false)),
        scope: None,
        version: 2,
    };
    let permissions: Permissions = Arc::new(Mutex::new(HashMap::new()));
    let requests: Requests = Arc::new(Mutex::new(HashMap::new()));
    let active = Arc::new(Mutex::new(None));
    let (commands, queue) = mpsc::channel(capacity.clamp(1, 256));
    let mut worker = tokio::spawn(crate::interactive_worker::run(
        queue,
        config.clone(),
        events.clone(),
        permissions.clone(),
        requests.clone(),
        active.clone(),
    ));
    let mut commands = Some(commands);
    let mut seen = HashSet::new();
    let mut failed = events.cancel.subscribe();
    if !send(
        &sink,
        &frame(
            "ready",
            None,
            json!({"runners":crate::serve::runner_summaries(&config.catalog),
        "queueCapacity":capacity.clamp(1,256)}),
        ),
    )
    .await
    {
        return;
    }
    loop {
        tokio::select! {
            ()=lapis_acp_runner::persistent::cancellation(&mut failed),if commands.is_some()=> {
                terminate(&events,&requests);commands.take();
            },
            ()=lapis_acp_runner::persistent::cancellation(&mut shutdown),if commands.is_some()=> {
                terminate(&events,&requests);commands.take();
            },
            line=source.next_line(),if commands.is_some()=> {
                match line {
                    Ok(Some(line))=> match parse(&line) {
                        Ok(command)=>{
                            let id=command.id().to_owned();
                            let response=match command {
                                Command::Cancel{target,..}=> {
                                    match requests.lock().ok().and_then(|map|map.get(&target).cloned()) {
                                        Some(cancel)=>{let _=cancel.send(true);frame("ack",Some(&id),json!({}))},
                                        None=>error(Some(&id),"not_in_flight"),
                                    }
                                }
                                Command::Permission{permission_id,option_id,..}=> {
                                    decide(&permissions,&permission_id,option_id,&id)
                                }
                                Command::Status{..}=>frame("status",Some(&id),json!({
                                    "running":active.lock().ok().and_then(|value|value.clone()),
                                    "pending":requests.lock().map(|map|map.len()).unwrap_or(0)})),
                                Command::Close{..}=>{terminate(&events,&requests);commands.take();frame("ack",Some(&id),json!({}))},
                                command=>{
                                    if seen.contains(&id) {error(Some(&id),"duplicate_request")}
                                    else if seen.len()>=4096 {error(Some(&id),"request_limit")}
                                    else {
                                        let (cancel,receiver)=watch::channel(false);
                                        if let Ok(mut map)=requests.lock(){map.insert(id.clone(),cancel);}
                                        match commands.as_ref().map(|queue|queue.try_send(Job{command,cancel:receiver})) {
                                            Some(Ok(()))=>{
                                                seen.insert(id.clone());
                                                frame("queued",Some(&id),json!({}))
                                            },
                                            _=>{if let Ok(mut map)=requests.lock(){map.remove(&id);}error(Some(&id),"queue_full")},
                                        }
                                    }
                                }
                            };
                            if !send(&sink,&response).await {terminate(&events,&requests);commands.take();break;}
                        },
                        Err(code)=>{if !send(&sink,&error(None,code)).await {terminate(&events,&requests);commands.take();break;}}
                    },
                    _=>{terminate(&events,&requests);commands.take();},
                }
            },
            response=responses.recv()=> {
                let Some(response)=response else {break;};
                let closed=response["type"]=="closed";
                if matches!(response["type"].as_str(),Some("record"|"opened"|"configured"|"error"))
                    && let Some(id)=response["requestId"].as_str() {
                        if let Ok(mut map)=requests.lock(){map.remove(id);}
                        if let Ok(mut value)=active.lock() && value.as_deref()==Some(id){*value=None;}
                }
                if !send(&sink,&response).await {terminate(&events,&requests);commands.take();break;}
                if closed {break;}
            }
        }
    }
    terminate(&events, &requests);
    commands.take();
    drop(responses);
    if tokio::time::timeout(
        config.cancel_grace + config.reap + Duration::from_secs(2),
        &mut worker,
    )
    .await
    .is_err()
    {
        worker.abort();
        let _ = worker.await;
    }
}
fn decide(permissions: &Permissions, target: &str, option: Option<String>, id: &str) -> Value {
    let Ok(mut map) = permissions.lock() else {
        return error(Some(id), "permission_unavailable");
    };
    if !map.get(target).is_some_and(|pending| {
        option
            .as_ref()
            .is_none_or(|value| pending.options.contains(value))
    }) {
        return error(Some(id), "stale_or_invalid_permission");
    }
    if let Some(pending) = map.remove(target)
        && pending.answer.send(option).is_err()
    {
        return error(Some(id), "stale_or_invalid_permission");
    }
    frame("ack", Some(id), json!({}))
}
fn terminate(events: &Events, requests: &Requests) {
    let _ = events.cancel.send(true);
    if let Ok(map) = requests.lock() {
        for cancel in map.values() {
            let _ = cancel.send(true);
        }
    }
}
async fn send<W: AsyncWrite + Unpin + Send>(sink: &LineSink<W>, value: &Value) -> bool {
    matches!(
        tokio::time::timeout(Duration::from_secs(5), sink.send_value(value)).await,
        Ok(Ok(()))
    )
}
