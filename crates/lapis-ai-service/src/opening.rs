use crate::{
    AcpHost, Error, HostContext, Result, SessionBinding, SessionInfo,
    policy::label,
    registry::{Reservation, Slot},
};
use lapis_ai_runtime::SessionOptions;
use std::sync::Arc;
use tokio::sync::{oneshot, watch};
use uuid::Uuid;
impl AcpHost {
    pub async fn open(
        self: &Arc<Self>,
        context: &HostContext,
        request: &str,
        target: &str,
        resume_binding: Option<String>,
    ) -> Result<SessionInfo> {
        if !label(request)
            || !label(target)
            || resume_binding
                .as_ref()
                .is_some_and(|binding| !label(binding))
        {
            return Err(Error("invalid_request_identity"));
        }
        let scope = self.scope(context)?;
        let launch = self
            .policy
            .resolve(context, target, resume_binding.as_deref())?;
        let id = Uuid::new_v4().to_string();
        let (cancel, cancelled) = watch::channel(false);
        let (done, completed) = watch::channel(None);
        {
            let mut state = self.state.lock().map_err(|_| Error("host_unavailable"))?;
            state.available()?;
            let key = (
                scope.window.clone(),
                scope.admission.clone(),
                request.into(),
            );
            if state.seen.contains(&key) {
                return Err(Error("duplicate_request"));
            }
            if state.resources() >= self.config.maximum_sessions {
                return Err(Error("session_limit"));
            }
            if state.seen.len() >= 4096 {
                return Err(Error("request_limit"));
            }
            state.seen.insert(key);
            state.slots.insert(
                id.clone(),
                Slot {
                    scope: scope.clone(),
                    request: request.into(),
                    target: target.into(),
                    session: None,
                    cancel,
                    subscription: None,
                    startup_done: Some(completed),
                },
            );
        }
        let mut reservation = Reservation::new(self.state.clone(), id.clone(), done);
        let binding = SessionBinding {
            session_id: id.clone(),
            request_id: request.into(),
            target_id: target.into(),
            admission: scope.admission.clone(),
        };
        let mut options = SessionOptions::new(id, launch.directory);
        options.resume = launch.resume;
        options.request_timeout = self.config.request_timeout;
        options.event_limit = self.config.event_limit;
        options.event_byte_limit = self.config.event_byte_limit;
        let (reply, received) = oneshot::channel();
        let host = self.clone();
        let context = context.clone();
        // The registry owns startup even if the invoking WebView loses its reply.
        // Revocation cancels this task cooperatively and waits for its guard.
        self.runtime.spawn(async move {
            let result = match crate::processing::create(&host, &context, &scope, &binding) {
                Ok(processor) => {
                    let started = host
                        .backend
                        .start(launch.prepared, options, cancelled, processor)
                        .await;
                    match started {
                        Ok(session) => {
                            let result = if session.id() != binding.session_id {
                                Err(Error("invalid_execution_identity"))
                            } else if host.scope(&context).is_ok_and(|current| current == scope) {
                                reservation.commit(session.clone())
                            } else {
                                Err(Error("admission_revoked"))
                            };
                            if result.is_err() && session.close().await.is_err() {
                                return finish(reservation, reply, Err(Error("cleanup_unknown")));
                            }
                            result
                        }
                        Err(error) => Err(error.into()),
                    }
                }
                Err(error) => Err(error),
            };
            finish(reservation, reply, result);
        });
        received.await.map_err(|_| Error("submission_unknown"))?
    }
}
fn finish(
    mut reservation: Reservation,
    reply: oneshot::Sender<Result<SessionInfo>>,
    result: Result<SessionInfo>,
) {
    let cleanup = if result
        .as_ref()
        .err()
        .is_some_and(|error| matches!(error.0, "startup_cleanup_unknown" | "cleanup_unknown"))
    {
        Err(Error("cleanup_unknown"))
    } else {
        Ok(())
    };
    reservation.finished(cleanup);
    drop(reservation);
    let _ = reply.send(result);
}
