use crate::{Error, Result};
use serde_json::Value;
use std::{future::Future, pin::Pin};

/// The destination of a validated frame. This type grants no process authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramePublication {
    Facts,
    Event { sequence: u64 },
}
/// Native-only input; ordinals include facts and are distinct from event cursors.
pub struct FrameInput<'a> {
    pub session_id: &'a str,
    pub ordinal: u64,
    pub publication: FramePublication,
    pub frame: &'a Value,
}
pub type ProcessorFuture<'a> = Pin<Box<dyn Future<Output = Result<Value>> + Send + 'a>>;
/// One owned processor per worker, invoked serially before any frame is exposed.
///
/// Hosts own privacy and durable receipt recovery. Return only after required
/// capture is durable; never detach persistence work. Failure retires the worker
/// without raw fallback or replay. Identity and permission IDs must remain exact.
pub trait SessionProcessor: Send + 'static {
    fn process<'a>(&'a mut self, input: FrameInput<'a>) -> ProcessorFuture<'a>;
}

pub(super) struct Processing {
    session_id: String,
    ordinal: u64,
    byte_limit: usize,
    processor: Option<Box<dyn SessionProcessor>>,
}
impl Processing {
    pub fn new(
        session_id: String,
        byte_limit: usize,
        processor: Option<Box<dyn SessionProcessor>>,
    ) -> Self {
        Self {
            session_id,
            ordinal: 0,
            byte_limit,
            processor,
        }
    }
    pub async fn prepare(&mut self, frame: Value, publication: FramePublication) -> Result<Value> {
        let Some(processor) = self.processor.as_mut() else {
            return Ok(frame);
        };
        self.ordinal = self
            .ordinal
            .checked_add(1)
            .ok_or(Error("processing_cursor_exhausted"))?;
        let output = processor
            .process(FrameInput {
                session_id: &self.session_id,
                ordinal: self.ordinal,
                publication,
                frame: &frame,
            })
            .await?;
        if !identity_preserved(&frame, &output) {
            return Err(Error("processed_identity_changed"));
        }
        if serde_json::to_vec(&output)
            .map_err(|_| Error("invalid_frame"))?
            .len()
            > self.byte_limit
        {
            return Err(Error("output_coverage_lost"));
        }
        Ok(output)
    }
}
fn identity_preserved(raw: &Value, output: &Value) -> bool {
    if !output.is_object()
        || [
            "version",
            "type",
            "requestId",
            "sessionId",
            "sequence",
            "adapterSessionId",
            "permissionId",
            "optionId",
            "modeReady",
            "sessionUsable",
            "quiescent",
            "running",
            "code",
        ]
        .iter()
        .any(|key| raw.get(key) != output.get(key))
        || ["version", "taskId", "status"]
            .iter()
            .any(|key| raw["record"].get(key) != output["record"].get(key))
    {
        return false;
    }
    let options = |frame: &Value| {
        frame["request"]["options"].as_array().map(|values| {
            values
                .iter()
                .map(|value| (value["optionId"].clone(), value["kind"].clone()))
                .collect::<Vec<_>>()
        })
    };
    options(raw) == options(output)
}
