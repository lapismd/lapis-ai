use super::support;
use lapis_ai_service::{
    AcpHost, Config, Error, HostContext, HostPolicy, Launch, Result, SessionBinding, Target,
    runtime::{FrameInput, FramePublication, ProcessorFuture, SessionProcessor},
};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::Notify;

pub const CAPTURE: usize = 0;
pub const REJECT: usize = 1;
pub const CHANGE_IN_FACTORY: usize = 2;
pub const BLOCK_FACTS: usize = 3;
pub const BLOCK_ACTIVITY: usize = 4;
pub const CHANGE_IN_RESOLVE: usize = 5;
#[derive(Default)]
pub struct Control {
    pub mode: AtomicUsize,
    pub entered: Notify,
    pub release: Notify,
    pub abandoned: AtomicUsize,
    pub calls: Mutex<Vec<Value>>,
}
pub struct Policy {
    pub base: support::Policy,
    pub control: Arc<Control>,
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
    fn targets(&self, context: &HostContext) -> Result<Vec<Target>> {
        self.base.targets(context)
    }
    fn resolve(&self, context: &HostContext, target: &str, resume: Option<&str>) -> Result<Launch> {
        let launch = self.base.resolve(context, target, resume)?;
        let script = support::peer_script(&launch);
        let peer = std::fs::read_to_string(script).unwrap();
        support::write_peer(
            script,
            &peer
                .replace(
                    "emit('warmed',rid)",
                    "emit('activity',rid,update={'text':'bootstrap-secret'});emit('warmed',rid)",
                )
                .replace("session={}", "session={'label':'facts-secret'}")
                .replace("'text':'reply'", "'text':'reply-secret'"),
        );
        if self.control.mode.load(Ordering::Acquire) == CHANGE_IN_RESOLVE {
            self.base.generation.fetch_add(1, Ordering::AcqRel);
        }
        Ok(launch)
    }
    fn processor(
        &self,
        context: &HostContext,
        binding: &SessionBinding,
    ) -> Result<Option<Box<dyn SessionProcessor>>> {
        let metadata = json!({
            "window": context.window,
            "url": context.url,
            "admission": binding.admission,
            "request": binding.request_id,
            "target": binding.target_id,
            "session": binding.session_id,
        });
        self.control.calls.lock().unwrap().push(metadata.clone());
        match self.control.mode.load(Ordering::Acquire) {
            REJECT => return Err(Error("processor_unavailable")),
            CHANGE_IN_FACTORY => {
                self.base.generation.fetch_add(1, Ordering::AcqRel);
            }
            _ => {}
        }
        Ok(Some(Box::new(Capture {
            path: self
                .base
                .root
                .join(&context.window)
                .join(format!("{}.capture", binding.session_id)),
            metadata,
            control: self.control.clone(),
        })))
    }
}
struct Capture {
    path: PathBuf,
    metadata: Value,
    control: Arc<Control>,
}
struct Pending(Arc<Control>, bool);
impl Drop for Pending {
    fn drop(&mut self) {
        if !self.1 {
            self.0.abandoned.fetch_add(1, Ordering::AcqRel);
        }
    }
}
impl SessionProcessor for Capture {
    fn process<'a>(&'a mut self, input: FrameInput<'a>) -> ProcessorFuture<'a> {
        Box::pin(async move {
            let mut pending = Pending(self.control.clone(), false);
            let mode = self.control.mode.load(Ordering::Acquire);
            if self.metadata["window"] == "one"
                && ((mode == BLOCK_FACTS && input.publication == FramePublication::Facts)
                    || (mode == BLOCK_ACTIVITY
                        && input.frame["type"] == "activity"
                        && input.frame["requestId"] == "turn"))
            {
                self.control.entered.notify_one();
                self.control.release.notified().await;
            }
            let output: Value =
                serde_json::from_str(&input.frame.to_string().replace("secret", "filtered"))
                    .unwrap();
            let sequence = match input.publication {
                FramePublication::Facts => None,
                FramePublication::Event { sequence } => Some(sequence),
            };
            let value = json!({"binding": self.metadata, "ordinal": input.ordinal, "sequence": sequence, "frame": output});
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .unwrap();
            writeln!(file, "{value}").unwrap();
            file.sync_all().unwrap();
            pending.1 = true;
            Ok(output)
        })
    }
}
pub fn setup(root: &Path, mode: usize, limit: usize) -> (Arc<AcpHost>, Arc<Policy>) {
    let control = Arc::new(Control::default());
    control.mode.store(mode, Ordering::Release);
    let policy = Arc::new(Policy {
        base: support::Policy {
            root: root.into(),
            generation: AtomicU64::new(0),
            blocked: AtomicBool::new(false),
            bad_cleanup: AtomicBool::new(false),
            slow_shutdown: AtomicBool::new(false),
        },
        control,
    });
    (
        Arc::new(
            AcpHost::new(
                policy.clone(),
                Config {
                    maximum_sessions: limit,
                    ..Config::default()
                },
            )
            .unwrap(),
        ),
        policy,
    )
}
pub fn captured(root: &Path, window: &str, id: &str) -> Vec<Value> {
    std::fs::read_to_string(root.join(window).join(format!("{id}.capture")))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
pub async fn entered(policy: &Policy) {
    tokio::time::timeout(Duration::from_secs(5), policy.control.entered.notified())
        .await
        .unwrap();
}
pub async fn stopped(root: &Path, window: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !support::all_stopped(root, window) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
