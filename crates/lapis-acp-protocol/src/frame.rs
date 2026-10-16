//! Bounded newline-delimited JSON framing.
//!
//! The byte cap is enforced while reading rather than after a line is
//! assembled, so an agent that emits endless bytes without a newline cannot
//! grow the buffer. A frame that exceeds the cap is a hard protocol error, not
//! a truncation.

use serde_json::Value;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::Error;

/// Default cap for one frame, matching the upstream harness.
pub const DEFAULT_MAX_FRAME_BYTES: usize = 10_000_000;

enum Step {
    Line(usize),
    Partial(usize),
    End,
}

/// Reads one JSON value per line from a buffered byte stream.
///
/// A line that is blank or that fails to parse is skipped and counted rather
/// than treated as fatal, because adapters do emit stray diagnostics on stdout.
/// The count is observable so a caller can detect an agent producing nothing but
/// noise. Skipping is not silent acceptance: a stream of only garbage yields no
/// frames and eventually trips the caller's idle bound.
#[derive(Debug)]
pub struct FrameReader<R> {
    inner: R,
    buffer: Vec<u8>,
    limit: usize,
    skipped: u64,
    partial: bool,
}

impl<R: AsyncBufRead + Unpin> FrameReader<R> {
    /// Wrap a buffered reader with the default frame cap.
    pub fn new(inner: R) -> Self {
        Self::with_limit(inner, DEFAULT_MAX_FRAME_BYTES)
    }

    /// Wrap a buffered reader with an explicit frame cap.
    pub fn with_limit(inner: R, limit: usize) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
            limit: limit.max(1),
            skipped: 0,
            partial: false,
        }
    }

    /// How many lines were skipped because they were blank or unparseable.
    pub fn skipped(&self) -> u64 {
        self.skipped
    }

    /// The configured frame cap in bytes.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Read the next frame, or `None` once the stream ends.
    pub async fn next(&mut self) -> Result<Option<Value>, Error> {
        loop {
            if !self.fill().await? {
                return Ok(None);
            }
            let text = String::from_utf8_lossy(&self.buffer);
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<Value>(trimmed) {
                Ok(value) => return Ok(Some(value)),
                Err(_) => self.skipped += 1,
            }
        }
    }

    /// Load one physical line into the buffer, reporting whether one was found.
    ///
    /// The cap is checked before bytes are appended, so the buffer can never
    /// exceed it even transiently. A trailing partial line at end of stream is
    /// still delivered, matching the upstream codec.
    ///
    /// The read is resumable. Bytes already consumed from the underlying reader
    /// are held in the buffer across calls, so a caller that cancels a pending
    /// read mid-frame and then reads again continues the same frame instead of
    /// silently losing part of it. That is what makes racing a turn against a
    /// cancellation signal safe.
    async fn fill(&mut self) -> Result<bool, Error> {
        if !self.partial {
            self.buffer.clear();
        }
        let limit = self.limit;
        loop {
            let step = {
                let available = self.inner.fill_buf().await?;
                if available.is_empty() {
                    Step::End
                } else if let Some(index) = available.iter().position(|byte| *byte == b'\n') {
                    if self.buffer.len() + index > limit {
                        return Err(Error::FrameTooLarge { limit });
                    }
                    self.buffer.extend_from_slice(&available[..index]);
                    Step::Line(index + 1)
                } else {
                    let length = available.len();
                    if self.buffer.len() + length > limit {
                        return Err(Error::FrameTooLarge { limit });
                    }
                    self.buffer.extend_from_slice(available);
                    Step::Partial(length)
                }
            };
            match step {
                Step::End => {
                    self.partial = false;
                    return Ok(!self.buffer.is_empty());
                }
                Step::Line(consume) => {
                    self.inner.consume(consume);
                    self.partial = false;
                    return Ok(true);
                }
                Step::Partial(consume) => {
                    self.inner.consume(consume);
                    self.partial = true;
                }
            }
        }
    }
}

/// Writes one JSON value per line to a byte stream.
#[derive(Debug)]
pub struct FrameWriter<W> {
    inner: W,
}

impl<W: AsyncWrite + Unpin> FrameWriter<W> {
    /// Wrap a byte sink.
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    /// Flush and close the write half.
    ///
    /// Closing stdin is how a client tells an agent the exchange is over: a
    /// well-behaved agent sees end of input and exits on its own, which is a
    /// cleaner stop than being killed and leaves no doubt about whether it
    /// finished writing.
    pub async fn close(&mut self) -> Result<(), Error> {
        self.inner.flush().await?;
        self.inner.shutdown().await?;
        Ok(())
    }

    /// Serialize and flush one frame.
    ///
    /// Serialization is compact and never contains a raw newline, so one value
    /// is always exactly one line and a reader can never see a split frame.
    pub async fn write(&mut self, frame: &Value) -> Result<(), Error> {
        let mut line = serde_json::to_vec(frame)?;
        line.push(b'\n');
        self.inner.write_all(&line).await?;
        self.inner.flush().await?;
        Ok(())
    }
}
