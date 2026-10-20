#![cfg(unix)]
#[path = "support/acp_session.rs"]
mod fixture;
use fixture::{configured, frames, options, plan, prompt, reaped, stopped, wait};
use lapis_ai_runtime::{
    AcpSession, Error, FrameInput, ProcessorFuture, RequestInput, RequestProcessorFuture,
    SessionProcessor, SessionRequest,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::sync::Notify;

type PreparedCall = (String, String, String, Option<String>);
#[derive(Default)]
struct Control {
    entered: Notify,
    release: Notify,
    calls: Mutex<Vec<PreparedCall>>,
    abandoned: AtomicUsize,
}
struct Gate {
    control: Arc<Control>,
    reject: bool,
}
struct Pending(Arc<Control>, bool);
impl Drop for Pending {
    fn drop(&mut self) {
        if !self.1 {
            self.0.abandoned.fetch_add(1, Ordering::AcqRel);
        }
    }
}
impl SessionProcessor for Gate {
    fn process<'a>(&'a mut self, input: FrameInput<'a>) -> ProcessorFuture<'a> {
        Box::pin(async move { Ok(input.frame.clone()) })
    }
    fn prepare_request<'a>(&'a mut self, input: RequestInput<'a>) -> RequestProcessorFuture<'a> {
        Box::pin(async move {
            if let SessionRequest::Prompt { text, context, .. } = input.request {
                let mut pending = Pending(self.control.clone(), false);
                self.control.calls.lock().unwrap().push((
                    input.session_id.into(),
                    input.request_id.into(),
                    text.clone(),
                    context.clone(),
                ));
                self.control.entered.notify_one();
                self.control.release.notified().await;
                pending.1 = true;
                if self.reject {
                    return Err(Error("authored_write_unknown"));
                }
            }
            Ok(())
        })
    }
}
async fn entered(control: &Control) {
    tokio::time::timeout(Duration::from_secs(5), control.entered.notified())
        .await
        .unwrap();
}
async fn session(
    root: &std::path::Path,
    reject: bool,
    timeout: Duration,
) -> (AcpSession, Arc<Control>) {
    let control = Arc::new(Control::default());
    let processor = Box::new(Gate {
        control: control.clone(),
        reject,
    });
    let mut options = options(root, "managed");
    options.request_timeout = timeout;
    (
        AcpSession::start_with_processor(plan(root), options, processor)
            .await
            .unwrap(),
        control,
    )
}
fn prompts(root: &std::path::Path) -> usize {
    frames(root)
        .iter()
        .filter(|frame| frame["type"] == "prompt")
        .count()
}

#[tokio::test]
async fn validated_authored_request_must_finish_native_preparation_before_provider_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let (session, control) = session(root.path(), false, Duration::from_secs(3)).await;
    assert_eq!(
        session
            .submit("before-mode".into(), prompt("unadmitted"))
            .await,
        Err(Error("execution_mode_required"))
    );
    assert!(control.calls.lock().unwrap().is_empty());
    configured(&session).await;
    let previous = session.snapshot(0).unwrap().sequence;
    let owner = session.clone();
    let submitting = tokio::spawn(async move {
        owner
            .submit("authored".into(), prompt("Authored text"))
            .await
    });
    entered(&control).await;
    assert!(!submitting.is_finished());
    assert_eq!(prompts(root.path()), 0);
    assert_eq!(session.snapshot(0).unwrap().sequence, previous);
    assert_eq!(
        *control.calls.lock().unwrap(),
        [(
            "managed".into(),
            "authored".into(),
            "Authored text".into(),
            Some("private context".into())
        )]
    );
    control.release.notify_one();
    submitting.await.unwrap().unwrap();
    wait(&session, |snapshot| {
        snapshot
            .events
            .iter()
            .any(|event| event.frame["type"] == "record")
    })
    .await;
    assert_eq!(prompts(root.path()), 1);
    assert_eq!(
        session
            .submit("authored".into(), prompt("another input"))
            .await,
        Err(Error("duplicate_request"))
    );
    assert_eq!(control.calls.lock().unwrap().len(), 1);
    session.close().await.unwrap();
    assert_eq!(control.abandoned.load(Ordering::Acquire), 0);
}

#[tokio::test]
async fn failed_request_preparation_keeps_the_original_request_unknown_and_never_dispatches() {
    let root = tempfile::tempdir().unwrap();
    let (session, control) = session(root.path(), true, Duration::from_secs(3)).await;
    configured(&session).await;
    let owner = session.clone();
    let submitting = tokio::spawn(async move {
        owner
            .submit("authored".into(), prompt("Authored text"))
            .await
    });
    entered(&control).await;
    control.release.notify_one();
    assert_eq!(
        submitting.await.unwrap(),
        Err(Error("authored_write_unknown"))
    );
    let snapshot = wait(&session, |snapshot| snapshot.closed).await;
    assert_eq!(snapshot.failure, Some("authored_write_unknown"));
    assert_eq!(snapshot.unresolved_requests, ["authored"]);
    assert_eq!(prompts(root.path()), 0);
    session.close().await.unwrap();
    reaped(frames(root.path())[0]["pid"].as_u64().unwrap() as u32).await;
}

#[tokio::test]
async fn cancelled_submit_wait_and_close_drain_preparation_without_late_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let (session, control) = session(root.path(), false, Duration::from_secs(3)).await;
    configured(&session).await;
    let owner = session.clone();
    let submitting = tokio::spawn(async move {
        owner
            .submit("authored".into(), prompt("Authored text"))
            .await
    });
    entered(&control).await;
    let pid = frames(root.path())[0]["pid"].as_u64().unwrap() as u32;
    submitting.abort();
    assert!(submitting.await.unwrap_err().is_cancelled());
    let owner = session.clone();
    let closing = tokio::spawn(async move { owner.close().await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!closing.is_finished());
    assert!(!stopped(pid));
    assert_eq!(control.abandoned.load(Ordering::Acquire), 0);
    control.release.notify_one();
    closing.await.unwrap().unwrap();
    assert_eq!(prompts(root.path()), 0);
    assert_eq!(
        session.snapshot(0).unwrap().unresolved_requests,
        ["authored"]
    );
    assert_eq!(control.abandoned.load(Ordering::Acquire), 0);
    reaped(pid).await;
}

#[tokio::test]
async fn expired_request_preparation_drains_then_refuses_provider_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let (session, control) = session(root.path(), false, Duration::from_millis(100)).await;
    configured(&session).await;
    let owner = session.clone();
    let submitting = tokio::spawn(async move {
        owner
            .submit("authored".into(), prompt("Authored text"))
            .await
    });
    entered(&control).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!submitting.is_finished());
    control.release.notify_one();
    assert_eq!(submitting.await.unwrap(), Err(Error("outcome_unknown")));
    let snapshot = wait(&session, |snapshot| snapshot.closed).await;
    assert_eq!(snapshot.unresolved_requests, ["authored"]);
    assert_eq!(prompts(root.path()), 0);
    assert_eq!(control.abandoned.load(Ordering::Acquire), 0);
    session.close().await.unwrap();
}
