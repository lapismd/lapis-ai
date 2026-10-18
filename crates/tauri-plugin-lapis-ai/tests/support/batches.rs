use super::support;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri_plugin_lapis_ai::{
    AcpHost, Config, HostContext, HostPolicy, Launch, Result, SessionBinding, Target,
    runtime::{BatchProcessorFuture, FrameBatch, FrameInput, SessionProcessor},
};
use tokio::sync::Notify;
#[derive(Default)]
pub struct Control {
    pub block: AtomicBool,
    pub entered: Notify,
    pub release: Notify,
}
pub struct Policy {
    pub base: support::Policy,
    pub control: Arc<Control>,
}
impl HostPolicy for Policy {
    fn authorize(&self, context: &HostContext) -> Result<String> {
        let scope = self.base.authorize(context)?;
        Ok(if context.window == "two" {
            "vault-two".into()
        } else {
            scope
        })
    }
    fn targets(&self, context: &HostContext) -> Result<Vec<Target>> {
        self.base.targets(context)
    }
    fn resolve(&self, context: &HostContext, target: &str, resume: Option<&str>) -> Result<Launch> {
        let launch = self.base.resolve(context, target, resume)?;
        let peer=std::fs::read_to_string(&launch.prepared.executable).unwrap().replace("update={'content':{'text':'reply'}}", "update={'sessionUpdate':'agent_message_chunk','messageId':'answer','content':{'type':'text','text':'raw reply'}}");
        std::fs::write(&launch.prepared.executable, peer).unwrap();
        Ok(launch)
    }
    fn processor(
        &self,
        context: &HostContext,
        _binding: &SessionBinding,
    ) -> Result<Option<Box<dyn SessionProcessor>>> {
        Ok(Some(Box::new(Capture {
            control: self.control.clone(),
            window: context.window.clone(),
            pending: None,
        })))
    }
}
struct Capture {
    control: Arc<Control>,
    window: String,
    pending: Option<Value>,
}
impl SessionProcessor for Capture {
    fn process_batch<'a>(&'a mut self, input: FrameInput<'a>) -> BatchProcessorFuture<'a> {
        Box::pin(async move {
            let mut frame = input.frame.clone();
            let mut before = Vec::new();
            if frame["type"] == "activity" {
                let mut prefix = frame.clone();
                prefix.as_object_mut().unwrap().remove("sequence");
                prefix["update"]["content"]["text"] = json!("safe tail");
                self.pending = Some(prefix);
                frame["update"]["content"]["text"] = json!("");
            } else if let Some(prefix) = self.pending.take() {
                before.push(prefix);
            }
            if frame["type"] == "record"
                && self.window == "one"
                && self.control.block.load(Ordering::Acquire)
            {
                self.control.entered.notify_one();
                self.control.release.notified().await;
            }
            Ok(FrameBatch { frame, before })
        })
    }
}
pub fn setup(root: &Path) -> (Arc<AcpHost>, Arc<Policy>) {
    let policy = Arc::new(Policy {
        base: support::Policy {
            root: root.into(),
            generation: Default::default(),
            blocked: Default::default(),
            bad_cleanup: Default::default(),
            slow_shutdown: Default::default(),
        },
        control: Arc::default(),
    });
    (
        Arc::new(AcpHost::new(policy.clone(), Config::default()).unwrap()),
        policy,
    )
}
