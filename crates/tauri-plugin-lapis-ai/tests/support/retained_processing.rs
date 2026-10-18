use super::{fixture, support};
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri_plugin_lapis_ai::{
    AcpHost, Config, Error, HostContext, HostPolicy, Launch, Result, SessionBinding, Target,
    runtime::SessionProcessor,
};

pub struct Policy {
    pub inner: Arc<fixture::Policy>,
    pub suspended: AtomicBool,
    pub granted: AtomicBool,
    pub foreign_generation: AtomicBool,
    pub binding: Mutex<Option<(HostContext, SessionBinding)>>,
}
impl HostPolicy for Policy {
    fn authorize(&self, context: &HostContext) -> Result<String> {
        if self.suspended.load(Ordering::Acquire) {
            return Err(Error("renderer_suspended"));
        }
        self.inner.authorize(context)
    }
    fn authorize_processing(
        &self,
        context: &HostContext,
        binding: &SessionBinding,
    ) -> Result<String> {
        let retained = self.binding.lock().unwrap();
        let exact = retained.as_ref().is_some_and(|(caller, original)| {
            caller.window == context.window
                && caller.url == context.url
                && original.session_id == binding.session_id
                && original.request_id == binding.request_id
                && original.target_id == binding.target_id
                && original.admission == binding.admission
        });
        if !exact || !self.granted.load(Ordering::Acquire) {
            return Err(Error("capture_revoked"));
        }
        if self.foreign_generation.load(Ordering::Acquire) {
            return Ok("replacement-generation".into());
        }
        Ok(binding.admission.clone())
    }
    fn targets(&self, context: &HostContext) -> Result<Vec<Target>> {
        self.inner.targets(context)
    }
    fn resolve(&self, context: &HostContext, target: &str, resume: Option<&str>) -> Result<Launch> {
        self.inner.resolve(context, target, resume)
    }
    fn processor(
        &self,
        context: &HostContext,
        binding: &SessionBinding,
    ) -> Result<Option<Box<dyn SessionProcessor>>> {
        *self.binding.lock().unwrap() = Some((context.clone(), binding.clone()));
        self.inner.processor(context, binding)
    }
}
pub fn setup(root: &Path, mode: usize) -> (Arc<AcpHost>, Arc<Policy>) {
    let (unused, inner) = fixture::setup(root, mode, 1);
    drop(unused);
    let policy = Arc::new(Policy {
        inner,
        suspended: AtomicBool::new(false),
        granted: AtomicBool::new(true),
        foreign_generation: AtomicBool::new(false),
        binding: Mutex::new(None),
    });
    let host = Arc::new(
        AcpHost::new(
            policy.clone(),
            Config {
                maximum_sessions: 1,
                ..Config::default()
            },
        )
        .unwrap(),
    );
    (host, policy)
}
pub async fn terminal_captured(root: &Path, id: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            // The native fixture is appending concurrently. Wait for its whole
            // terminal record rather than parsing an incomplete final append.
            let contents = std::fs::read_to_string(root.join("one").join(format!("{id}.capture")))
                .unwrap_or_default();
            if contents
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .any(|record| record["frame"]["type"] == "record")
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
pub fn prompt_count(root: &Path) -> usize {
    support::log(root, "one")
        .iter()
        .filter(|frame| frame["type"] == "prompt")
        .count()
}
