#![cfg(unix)]
mod support;
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tauri_plugin_lapis_ai::{
    AcpHost, Config, Error, HostContext, HostPolicy, Launch, Result, SessionBinding, Target,
    runtime::{
        FrameInput, ProcessorFuture, RequestInput, RequestProcessorFuture, SessionProcessor,
        SessionRequest,
    },
};
use tokio::sync::Notify;

#[derive(Default)]
struct Control {
    entered: Notify,
    release: Notify,
    bindings: Mutex<Vec<SessionBinding>>,
}
struct Policy {
    base: support::Policy,
    control: Arc<Control>,
}
impl HostPolicy for Policy {
    fn authorize(&self, context: &HostContext) -> Result<String> {
        let admission = self.base.authorize(context)?;
        Ok(if context.window == "two" {
            "vault-two".into()
        } else {
            admission
        })
    }
    fn authorize_processing(&self, _: &HostContext, binding: &SessionBinding) -> Result<String> {
        // Deliberately retain incoming capture after this renderer generation
        // changes. This grant must never authorize a new outgoing command.
        Ok(binding.admission.clone())
    }
    fn targets(&self, context: &HostContext) -> Result<Vec<Target>> {
        self.base.targets(context)
    }
    fn resolve(&self, context: &HostContext, target: &str, resume: Option<&str>) -> Result<Launch> {
        self.base.resolve(context, target, resume)
    }
    fn processor(
        &self,
        context: &HostContext,
        binding: &SessionBinding,
    ) -> Result<Option<Box<dyn SessionProcessor>>> {
        self.control.bindings.lock().unwrap().push(binding.clone());
        Ok(Some(Box::new(Gate {
            control: self.control.clone(),
            window: context.window.clone(),
        })))
    }
}
struct Gate {
    control: Arc<Control>,
    window: String,
}
impl SessionProcessor for Gate {
    fn process<'a>(&'a mut self, input: FrameInput<'a>) -> ProcessorFuture<'a> {
        Box::pin(async move { Ok(input.frame.clone()) })
    }
    fn prepare_request<'a>(&'a mut self, input: RequestInput<'a>) -> RequestProcessorFuture<'a> {
        Box::pin(async move {
            if self.window == "one" && matches!(input.request, SessionRequest::Prompt { .. }) {
                self.control.entered.notify_one();
                self.control.release.notified().await;
            }
            Ok(())
        })
    }
}
fn setup(root: &Path) -> (Arc<AcpHost>, Arc<Policy>) {
    let policy = Arc::new(Policy {
        base: support::Policy {
            root: root.into(),
            generation: AtomicU64::new(0),
            blocked: AtomicBool::new(false),
            bad_cleanup: AtomicBool::new(false),
            slow_shutdown: AtomicBool::new(false),
        },
        control: Arc::new(Control::default()),
    });
    (
        Arc::new(AcpHost::new(policy.clone(), Config::default()).unwrap()),
        policy,
    )
}

#[tokio::test]
async fn retained_incoming_capture_cannot_authorize_dispatch_after_current_admission_changes() {
    let root = tempfile::tempdir().unwrap();
    let (host, policy) = setup(root.path());
    let one = support::context("one");
    let two = support::context("two");
    let first = host.open(&one, "first", "fixture", None).await.unwrap();
    let second = host.open(&two, "second", "fixture", None).await.unwrap();
    support::configure(&host, &one, &first.session_id).await;
    support::configure(&host, &two, &second.session_id).await;
    let owner = host.clone();
    let id = first.session_id.clone();
    let sending = tokio::spawn(async move {
        owner
            .submit(
                &support::context("one"),
                &id,
                "turn".into(),
                support::prompt(),
            )
            .await
    });
    if tokio::time::timeout(Duration::from_secs(5), policy.control.entered.notified())
        .await
        .is_err()
    {
        let dispatched = support::log(root.path(), "one")
            .iter()
            .any(|frame| frame["type"] == "prompt");
        host.shutdown().await.unwrap();
        assert!(!dispatched, "Provider dispatched before native preparation");
        panic!("Native request preparation was not delegated");
    }
    assert!(
        !support::log(root.path(), "one")
            .iter()
            .any(|frame| frame["type"] == "prompt")
    );
    policy.base.generation.store(1, Ordering::Release);
    let binding = policy.control.bindings.lock().unwrap()[0].clone();
    assert_eq!(
        policy.authorize_processing(&one, &binding).unwrap(),
        "vault-0"
    );
    assert_eq!(policy.authorize(&one).unwrap(), "vault-1");
    policy.control.release.notify_one();
    assert_eq!(sending.await.unwrap(), Err(Error("admission_revoked")));
    assert!(
        !support::log(root.path(), "one")
            .iter()
            .any(|frame| frame["type"] == "prompt")
    );
    host.submit(&two, &second.session_id, "turn".into(), support::prompt())
        .await
        .unwrap();
    support::wait(&host, &two, &second.session_id, |snapshot| {
        snapshot
            .events
            .iter()
            .any(|event| event.frame["type"] == "record")
    })
    .await;
    host.shutdown().await.unwrap();
    assert!(support::all_stopped(root.path(), "one"));
    assert!(support::all_stopped(root.path(), "two"));
    assert_eq!(
        support::log(root.path(), "two")
            .iter()
            .filter(|frame| frame["type"] == "prompt")
            .count(),
        1
    );
}
