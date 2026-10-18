use lapis_ai_runtime::{
    BatchProcessorFuture, Error, FrameBatch, FrameInput, FramePublication, SessionProcessor,
};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Notify;

#[derive(Clone, Copy)]
pub enum Mode {
    Capture,
    BlockFacts,
    BlockRecord,
    BadPrefix(&'static str),
    LargePrefix,
    TooMany,
}
#[derive(Default)]
pub struct Control {
    pub entered: Notify,
    pub release: Notify,
}
pub struct Capture {
    root: PathBuf,
    mode: Mode,
    pending: Option<Value>,
    control: Arc<Control>,
}
impl Capture {
    pub fn new(root: &Path, mode: Mode) -> (Box<Self>, Arc<Control>) {
        let control = Arc::new(Control::default());
        (
            Box::new(Self {
                root: root.into(),
                mode,
                pending: None,
                control: control.clone(),
            }),
            control,
        )
    }
}
impl SessionProcessor for Capture {
    fn process_batch<'a>(&'a mut self, input: FrameInput<'a>) -> BatchProcessorFuture<'a> {
        Box::pin(async move {
            let mut frame = input.frame.clone();
            let mut before = Vec::new();
            let text = frame["type"] == "activity" && frame["update"]["content"]["type"] == "text";
            if text {
                let mut safe = frame.clone();
                safe["update"]["content"]["text"] = json!("captured safe tail");
                safe.as_object_mut().unwrap().remove("sequence");
                self.pending = Some(safe);
                frame["update"]["content"]["text"] = json!("");
            } else if let Some(mut prefix) = self.pending.take() {
                if let Mode::BadPrefix(key) = self.mode {
                    match key {
                        "message" => prefix["update"]["messageId"] = json!("foreign"),
                        "kind" => prefix["type"] = json!("permission"),
                        "sequence" => prefix["sequence"] = json!(9),
                        _ => prefix[key] = json!("foreign"),
                    }
                }
                if matches!(self.mode, Mode::LargePrefix) {
                    prefix["update"]["content"]["text"] = json!("x".repeat(3500));
                }
                before.push(prefix);
            }
            if matches!(self.mode, Mode::TooMany) && frame["type"] == "record" {
                before = (0..17).map(|_| before[0].clone()).collect();
            }
            if (matches!(self.mode, Mode::BlockFacts)
                && input.publication == FramePublication::Facts)
                || (matches!(self.mode, Mode::BlockRecord) && frame["type"] == "record")
            {
                self.control.entered.notify_one();
                self.control.release.notified().await;
            }
            let batch = FrameBatch { frame, before };
            let value = json!({"ordinal":input.ordinal,"publication":match input.publication { FramePublication::Facts=>None, FramePublication::Event { sequence }=>Some(sequence) },"frame":batch.frame,"before":batch.before});
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.root.join("batches.jsonl"))
                .map_err(|_| Error("capture_failed"))?;
            writeln!(file, "{value}").map_err(|_| Error("capture_failed"))?;
            file.sync_all().map_err(|_| Error("capture_failed"))?;
            Ok(batch)
        })
    }
}
pub fn plan(root: &Path, startup: bool) -> lapis_ai_runtime::PreparedLaunch {
    let plan = super::acp_session::plan(root);
    let mut peer = std::fs::read_to_string(&plan.executable).unwrap().replace("update={'text':'reply'}", "update={'sessionUpdate':'agent_message_chunk','messageId':'answer','content':{'type':'text','text':'unprocessed reply'}}");
    if startup {
        peer = peer.replace("emit('queued',rid);emit('warmed',rid,capabilities={})", "emit('activity',rid,update={'sessionUpdate':'agent_message_chunk','messageId':'startup','content':{'type':'text','text':'unprocessed startup'}});emit('queued',rid);emit('warmed',rid,capabilities={})");
    }
    std::fs::write(&plan.executable, peer).unwrap();
    plan
}
pub fn captured(root: &Path) -> Vec<Value> {
    std::fs::read_to_string(root.join("batches.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
