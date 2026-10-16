//! The control transport abstraction.
//!
//! A persistent server has one reader and many writers: the loop reads frames
//! while spawned submissions emit records concurrently. Modelling that as one
//! bidirectional object would force the two sides to share a lock they do not both
//! need, so the seam is split into a source the loop owns and a sink it shares.
//!
//! An earlier shape of this trait modelled the one-shot path — read a task
//! document, emit a record — and nothing used it, because the persistent protocol
//! exchanges requests and responses rather than documents. A seam that does not fit
//! its only caller is worse than no seam, so this one is derived from what the loop
//! actually does and is exercised by it.
//!
//! No transport may add authentication. There is no peer to authenticate: the
//! process that spawned this one already decided to run it with this environment.

use std::io;
use std::sync::Arc;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::protocol::Response;

/// Reads inbound control frames, one line at a time.
///
/// Lines are returned raw rather than parsed, so a malformed frame is the loop's
/// problem to refuse and never the transport's to interpret. A transport that
/// dropped or rewrote a frame it disliked would hide the very thing a controlling
/// application needs to hear about.
pub trait FrameSource: Send {
    /// The next inbound line, or `None` once the peer is finished.
    fn next_line(&mut self)
    -> impl std::future::Future<Output = io::Result<Option<String>>> + Send;
}

/// Writes outbound control frames.
///
/// Implementations take `&self` and serialize internally, because several
/// submissions emit concurrently and a controlling application parses the stream
/// line by line. An interleaved frame is indistinguishable from a corrupt one, so
/// atomicity here is a correctness requirement rather than tidiness.
pub trait FrameSink: Send + Sync {
    /// Write one frame as a single line.
    fn send(&self, response: &Response)
    -> impl std::future::Future<Output = io::Result<()>> + Send;
}

/// A line-oriented source over any buffered byte stream.
#[derive(Debug)]
pub struct LineSource<R> {
    input: R,
    pending: Vec<u8>,
}

impl<R: AsyncBufRead + Unpin + Send> LineSource<R> {
    /// Wrap a buffered byte stream.
    pub fn new(input: R) -> Self {
        Self {
            input,
            pending: Vec::new(),
        }
    }
}

impl<R: AsyncBufRead + Unpin + Send> FrameSource for LineSource<R> {
    async fn next_line(&mut self) -> io::Result<Option<String>> {
        loop {
            let bytes = self.input.fill_buf().await?;
            if bytes.is_empty() {
                if self.pending.is_empty() {
                    return Ok(None);
                }
                return String::from_utf8(std::mem::take(&mut self.pending))
                    .map(Some)
                    .map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid control UTF-8")
                    });
            }
            let newline = bytes.iter().position(|byte| *byte == b'\n');
            let count = newline.map_or(bytes.len(), |position| position + 1);
            if self.pending.len() + count > 2_097_152 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "control frame exceeds limit",
                ));
            }
            self.pending.extend_from_slice(&bytes[..count]);
            self.input.consume(count);
            if newline.is_some() {
                return String::from_utf8(std::mem::take(&mut self.pending))
                    .map(Some)
                    .map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid control UTF-8")
                    });
            }
        }
    }
}

/// A line-oriented sink over any byte stream, serialized by an internal lock.
#[derive(Debug)]
pub struct LineSink<W> {
    output: Mutex<W>,
}

impl<W: AsyncWrite + Unpin + Send> LineSink<W> {
    /// Write an explicitly versioned control value atomically.
    pub async fn send_value(&self, value: &serde_json::Value) -> io::Result<()> {
        let mut line = serde_json::to_vec(value)?;
        if line.len() > 1_048_576 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "control frame exceeds limit",
            ));
        }
        line.push(b'\n');
        let mut guard = self.output.lock().await;
        guard.write_all(&line).await?;
        guard.flush().await
    }

    /// Wrap a byte stream.
    pub fn new(output: W) -> Self {
        Self {
            output: Mutex::new(output),
        }
    }

    /// Share this sink so concurrent submissions can emit through it.
    pub fn shared(output: W) -> Arc<Self> {
        Arc::new(Self::new(output))
    }
}

impl<W: AsyncWrite + Unpin + Send> FrameSink for LineSink<W> {
    async fn send(&self, response: &Response) -> io::Result<()> {
        let mut line = response.to_line();
        line.push('\n');
        let mut guard = self.output.lock().await;
        guard.write_all(line.as_bytes()).await?;
        guard.flush().await
    }
}

/// Build the stdio source and sink pair.
///
/// Stdout carries frames and nothing else. Diagnostics that a human-facing path
/// would print to stderr travel as `diagnostic` frames instead, because a peer that
/// has to watch two streams to know what happened will eventually miss one.
pub fn stdio() -> (
    LineSource<tokio::io::BufReader<tokio::io::Stdin>>,
    Arc<LineSink<tokio::io::Stdout>>,
) {
    (
        LineSource::new(tokio::io::BufReader::new(tokio::io::stdin())),
        LineSink::shared(tokio::io::stdout()),
    )
}
