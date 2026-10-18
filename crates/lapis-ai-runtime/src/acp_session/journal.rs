use super::processed::{FrameBatch, payload_size};
use crate::{Error, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::VecDeque;
use tokio::sync::watch;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    pub sequence: u64,
    pub frame: Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<Value>,
}
impl SessionEvent {
    /// Every safe frame at this cursor, in delivery order.
    pub fn frames(&self) -> impl Iterator<Item = &Value> {
        self.before
            .iter()
            .chain(std::iter::once(&self.frame))
            .chain(self.after.iter())
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub events: Vec<SessionEvent>,
    pub sequence: u64,
    pub acknowledged: u64,
    pub closed: bool,
    pub failure: Option<&'static str>,
    pub unresolved_requests: Vec<String>,
    pub cleanup_failed: bool,
}
pub struct Journal {
    events: VecDeque<SessionEvent>,
    bytes: usize,
    limit: usize,
    byte_limit: usize,
    sequence: u64,
    acknowledged: u64,
    closed: bool,
    failure: Option<&'static str>,
    unresolved: Vec<String>,
    changed: watch::Sender<u64>,
    cleanup_failed: bool,
}
impl Journal {
    pub fn new(limit: usize, byte_limit: usize) -> Self {
        Self {
            events: VecDeque::new(),
            bytes: 0,
            limit,
            byte_limit,
            sequence: 0,
            acknowledged: 0,
            closed: false,
            failure: None,
            unresolved: Vec::new(),
            changed: watch::channel(0).0,
            cleanup_failed: false,
        }
    }
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }
    pub fn cleanup_result(&self) -> Result<()> {
        if self.cleanup_failed {
            Err(Error("cleanup_unknown"))
        } else {
            Ok(())
        }
    }
    pub fn cleanup_failed(&mut self, reason: &'static str) {
        self.cleanup_failed = true;
        self.failure = self.failure.or(Some(reason));
        self.changed.send_modify(|revision| *revision += 1);
    }
    pub fn emit(&mut self, batch: FrameBatch) -> Result<()> {
        let size = batch.size()?;
        if self.events.len() >= self.limit || size > self.byte_limit.saturating_sub(self.bytes) {
            return Err(Error("output_coverage_lost"));
        }
        self.sequence = self.next_sequence()?;
        self.bytes += size;
        self.events.push_back(SessionEvent {
            sequence: self.sequence,
            frame: batch.frame,
            before: batch.before,
            after: Vec::new(),
        });
        self.changed.send_modify(|revision| *revision += 1);
        Ok(())
    }
    /// Startup owns this journal exclusively until all opened facts are ready.
    /// Appending a facts-triggered text flush cannot revise an exposed event.
    pub fn append_startup_tail(&mut self, frames: Vec<Value>) -> Result<()> {
        if frames.is_empty() {
            return Ok(());
        }
        let size = payload_size(frames.iter())?;
        if size > self.byte_limit.saturating_sub(self.bytes) {
            return Err(Error("output_coverage_lost"));
        }
        let event = self
            .events
            .back_mut()
            .ok_or(Error("processed_batch_invalid"))?;
        if frames.len() > 16 || !event.after.is_empty() {
            return Err(Error("processed_batch_invalid"));
        }
        event.after = frames;
        self.bytes += size;
        self.changed.send_modify(|revision| *revision += 1);
        Ok(())
    }
    pub fn next_sequence(&self) -> Result<u64> {
        self.sequence
            .checked_add(1)
            .ok_or(Error("event_cursor_exhausted"))
    }
    pub fn finish(&mut self, failure: Option<&'static str>, mut unresolved: Vec<String>) {
        if self.closed {
            return;
        }
        self.closed = true;
        self.failure = failure;
        unresolved.sort();
        self.unresolved = unresolved;
        self.changed.send_modify(|revision| *revision += 1);
    }
    pub fn snapshot(&self, after: u64) -> Result<SessionSnapshot> {
        if after < self.acknowledged {
            return Err(Error("event_coverage_gap"));
        }
        if after > self.sequence {
            return Err(Error("invalid_event_cursor"));
        }
        Ok(SessionSnapshot {
            events: self
                .events
                .iter()
                .filter(|event| event.sequence > after)
                .cloned()
                .collect(),
            sequence: self.sequence,
            acknowledged: self.acknowledged,
            closed: self.closed,
            failure: self.failure,
            unresolved_requests: self.unresolved.clone(),
            cleanup_failed: self.cleanup_failed,
        })
    }
    pub fn acknowledge(&mut self, sequence: u64) -> Result<()> {
        if sequence < self.acknowledged || sequence > self.sequence {
            return Err(Error("invalid_event_cursor"));
        }
        while self
            .events
            .front()
            .is_some_and(|event| event.sequence <= sequence)
        {
            if let Some(event) = self.events.pop_front() {
                self.bytes -= payload_size(event.frames())?;
            }
        }
        self.acknowledged = sequence;
        Ok(())
    }
}
