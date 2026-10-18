use lapis_ai_runtime::{Error, FrameInput, FramePublication, ProcessorFuture, SessionProcessor};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::Notify;

#[derive(Clone, Copy)]
pub enum Behavior {
    Record,
    BlockFacts,
    BlockActivity,
    FailRecord,
    Tamper(&'static str),
    OversizeFacts,
}
#[derive(Default)]
pub struct Control {
    pub entered: Notify,
    pub release: Notify,
    pub completed: AtomicUsize,
    pub abandoned: AtomicUsize,
}
struct Pending(Arc<Control>, bool);
impl Drop for Pending {
    fn drop(&mut self) {
        if !self.1 {
            self.0.abandoned.fetch_add(1, Ordering::SeqCst);
        }
    }
}
pub struct Capture {
    root: PathBuf,
    control: Arc<Control>,
    behavior: Behavior,
}
impl Capture {
    pub fn new(root: &Path, behavior: Behavior) -> (Box<Self>, Arc<Control>) {
        let control = Arc::new(Control::default());
        (
            Box::new(Self {
                root: root.into(),
                control: control.clone(),
                behavior,
            }),
            control,
        )
    }
}
impl SessionProcessor for Capture {
    fn process<'a>(&'a mut self, input: FrameInput<'a>) -> ProcessorFuture<'a> {
        Box::pin(async move {
            let mut pending = Pending(self.control.clone(), false);
            let facts = input.publication == FramePublication::Facts;
            if (matches!(self.behavior, Behavior::BlockFacts) && facts)
                || (matches!(self.behavior, Behavior::BlockActivity)
                    && input.frame["type"] == "activity"
                    && input.frame["requestId"] == "turn")
            {
                self.control.entered.notify_one();
                self.control.release.notified().await;
            }
            pending.1 = true;
            if matches!(self.behavior, Behavior::FailRecord) && input.frame["type"] == "record" {
                return Err(Error("capture_failed"));
            }
            let mut output: Value = serde_json::from_str(
                &input
                    .frame
                    .to_string()
                    .replace("secret", "[redacted]")
                    .replace("reply", "[redacted reply]"),
            )
            .unwrap();
            if let Behavior::Tamper(field) = self.behavior {
                if facts && field == "factsId" {
                    output["adapterSessionId"] = json!("other");
                }
                if input.frame["type"] == "record" {
                    match field {
                        "recordTask" => output["record"]["taskId"] = json!("other"),
                        "recordStatus" => output["record"]["status"] = json!("outcome_unknown"),
                        "sequence" => output["sequence"] = json!(999),
                        _ => output[field] = json!("other"),
                    }
                }
                if input.frame["type"] == "permission" {
                    if field == "optionId" {
                        output["request"]["options"][0]["optionId"] = json!("forged-option");
                    }
                    if field == "optionKind" {
                        output["request"]["options"][0]["kind"] = json!("allow_always");
                    }
                }
            }
            if matches!(self.behavior, Behavior::OversizeFacts) && facts {
                output["session"]["label"] = json!("x".repeat(4096));
            }
            let event_sequence = match input.publication {
                FramePublication::Facts => None,
                FramePublication::Event { sequence } => Some(sequence),
            };
            let captured = json!({
                "sessionId": input.session_id,
                "ordinal": input.ordinal,
                "eventSequence": event_sequence,
                "frame": output,
            });
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.root.join("captured.jsonl"))
                .unwrap();
            writeln!(file, "{captured}").unwrap();
            file.sync_all().unwrap();
            self.control.completed.fetch_add(1, Ordering::SeqCst);
            Ok(output)
        })
    }
}
pub fn captured(root: &Path) -> Vec<Value> {
    std::fs::read_to_string(root.join("captured.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
pub fn startup_frames(root: &Path) -> lapis_ai_runtime::PreparedLaunch {
    let plan = super::acp_session::plan(root);
    let peer = std::fs::read_to_string(&plan.executable).unwrap();
    let peer = peer
        .replace(
            "emit('queued',rid);emit('warmed',rid,capabilities={})",
            "emit('activity',rid,update={'text':'startup-secret'});emit('queued',rid);emit('warmed',rid,capabilities={})",
        )
        .replace(
            "session={'models':['fixture-model']}",
            "session={'models':['fixture-model'],'label':'facts-secret'}",
        );
    std::fs::write(&plan.executable, peer).unwrap();
    plan
}
