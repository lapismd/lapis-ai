use crate::{Error, Result};
use serde_json::Value;

/// Native-owned text projection. The primary frame retains its control identity.
pub struct FrameBatch {
    pub frame: Value,
    pub before: Vec<Value>,
}
impl FrameBatch {
    pub fn single(frame: Value) -> Self {
        Self {
            frame,
            before: Vec::new(),
        }
    }
    /// Keep every frame returned by an injected capture owner, in order.
    pub fn from_frames(mut frames: Vec<Value>) -> Result<Self> {
        let frame = frames.pop().ok_or(Error("processed_batch_invalid"))?;
        Ok(Self {
            frame,
            before: frames,
        })
    }
    pub(super) fn size(&self) -> Result<usize> {
        payload_size(self.before.iter().chain(std::iter::once(&self.frame)))
    }
}
pub(super) fn payload_size<'a>(mut frames: impl Iterator<Item = &'a Value>) -> Result<usize> {
    frames.try_fold(0usize, |total, frame| {
        let bytes = serde_json::to_vec(frame)
            .map_err(|_| Error("invalid_frame"))?
            .len();
        total
            .checked_add(bytes)
            .ok_or(Error("output_coverage_lost"))
    })
}

/// Only sanitized text identity is retained; no source text or private prefix.
#[derive(PartialEq)]
pub(super) struct TextIdentity {
    header: Vec<Option<Value>>,
    kind: Value,
    message: Option<Value>,
}
impl TextIdentity {
    pub fn of(frame: &Value) -> Option<Self> {
        if frame["type"] != "activity"
            || !matches!(
                frame["update"]["sessionUpdate"].as_str(),
                Some("agent_message_chunk" | "agent_thought_chunk")
            )
            || frame["update"]["content"]["type"] != "text"
            || !frame["update"]["content"]["text"].is_string()
        {
            return None;
        }
        Some(Self {
            header: [
                "version",
                "type",
                "requestId",
                "sessionId",
                "adapterSessionId",
            ]
            .iter()
            .map(|key| frame.get(key).cloned())
            .collect(),
            kind: frame["update"]["sessionUpdate"].clone(),
            message: frame["update"].get("messageId").cloned(),
        })
    }
    pub fn accepts(&self, frame: &Value) -> bool {
        frame.get("sequence").is_none() && Self::of(frame).is_some_and(|value| value == *self)
    }
}
