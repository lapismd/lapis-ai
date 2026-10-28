use super::processed::{FrameBatch, TextIdentity};
use super::request::SessionRequest;
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
pub type BatchProcessorFuture<'a> = Pin<Box<dyn Future<Output = Result<FrameBatch>> + Send + 'a>>;
/// Immutable, validated outgoing input; it grants no launch or store authority.
pub struct RequestInput<'a> {
    pub session_id: &'a str,
    pub request_id: &'a str,
    pub request: &'a SessionRequest,
}
pub type RequestProcessorFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
/// One owned processor per worker, invoked serially before any frame is exposed.
///
/// Hosts own privacy and durable receipt recovery. Return only after required
/// capture is durable; never detach persistence work. Failure retires the worker
/// without raw fallback or replay. Identity and permission IDs must remain exact.
pub trait SessionProcessor: Send + 'static {
    /// Finish host-owned preparation before any provider command is sent.
    /// Existing processors preserve their outgoing-request behavior by default.
    fn prepare_request<'a>(&'a mut self, _input: RequestInput<'a>) -> RequestProcessorFuture<'a> {
        Box::pin(async { Ok(()) })
    }
    fn process<'a>(&'a mut self, _input: FrameInput<'a>) -> ProcessorFuture<'a> {
        Box::pin(async { Err(Error("single_frame_processing_unavailable")) })
    }
    /// Override for a captured deferred text group. The default preserves all
    /// existing single-frame processors without selecting a raw fallback.
    fn process_batch<'a>(&'a mut self, input: FrameInput<'a>) -> BatchProcessorFuture<'a> {
        Box::pin(async move { self.process(input).await.map(FrameBatch::single) })
    }
}

pub(super) struct Processing {
    session_id: String,
    ordinal: u64,
    byte_limit: usize,
    processor: Option<Box<dyn SessionProcessor>>,
    previous: Option<TextIdentity>,
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
            previous: None,
        }
    }
    pub async fn prepare(
        &mut self,
        frame: Value,
        publication: FramePublication,
    ) -> Result<FrameBatch> {
        let Some(processor) = self.processor.as_mut() else {
            return Ok(FrameBatch::single(frame));
        };
        self.ordinal = self
            .ordinal
            .checked_add(1)
            .ok_or(Error("processing_cursor_exhausted"))?;
        let output = processor
            .process_batch(FrameInput {
                session_id: &self.session_id,
                ordinal: self.ordinal,
                publication,
                frame: &frame,
            })
            .await?;
        if !identity_preserved(&frame, &output.frame) {
            return Err(Error("processed_identity_changed"));
        }
        if output.before.len() > 16
            || output.before.iter().any(|prefix| {
                !self
                    .previous
                    .as_ref()
                    .is_some_and(|previous| previous.accepts(prefix))
            })
        {
            return Err(Error("processed_batch_invalid"));
        }
        if output.size()? > self.byte_limit {
            return Err(Error("output_coverage_lost"));
        }
        self.previous = TextIdentity::of(&output.frame);
        Ok(output)
    }
    pub async fn prepare_request(&mut self, id: &str, request: &SessionRequest) -> Result<()> {
        if let Some(processor) = self.processor.as_mut() {
            processor
                .prepare_request(RequestInput {
                    session_id: &self.session_id,
                    request_id: id,
                    request,
                })
                .await?;
        }
        Ok(())
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
