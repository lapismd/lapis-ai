//! Bounded task document input.
//!
//! The read bound is separate from the execution deadline, so a stalled pipe
//! cannot consume the turn budget and a stalled agent cannot consume the read
//! budget. The size cap is enforced while reading rather than after, so an
//! endless stream cannot be buffered into memory first.

use std::path::Path;

use tokio::io::{AsyncRead, AsyncReadExt};

use crate::task::{MAX_TASK_BYTES, READ_TIMEOUT, TaskDocument, TaskRejection, code, parse};

/// Where a task document comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSource {
    /// Read from stdin, which the caller must close after the document.
    Stdin,
    /// Read from a path.
    Path(String),
}

impl TaskSource {
    /// Parse the conventional `-` spelling for stdin.
    pub fn parse(argument: &str) -> Self {
        if argument == "-" {
            Self::Stdin
        } else {
            Self::Path(argument.to_owned())
        }
    }
}

/// Read at most one capped document from a byte source, within a bound.
///
/// Returns the raw bytes rather than a parsed document so a caller can report a
/// size rejection before any decoding is attempted.
pub async fn read_bytes<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, TaskRejection> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    let read = async {
        loop {
            let count = reader.read(&mut chunk).await?;
            if count == 0 {
                return Ok::<(), std::io::Error>(());
            }
            if buffer.len() + count > MAX_TASK_BYTES {
                return Err(std::io::Error::other("task document exceeds the size cap"));
            }
            buffer.extend_from_slice(&chunk[..count]);
        }
    };
    match tokio::time::timeout(READ_TIMEOUT, read).await {
        Err(_) => Err(TaskRejection::new(
            code::TASK_READ_TIMEOUT,
            format!("no complete document within {:?}", READ_TIMEOUT),
        )),
        Ok(Err(error)) => {
            if buffer.len() >= MAX_TASK_BYTES {
                Err(TaskRejection::new(
                    code::TASK_TOO_LARGE,
                    format!("document exceeds the {MAX_TASK_BYTES} byte cap"),
                ))
            } else {
                Err(TaskRejection::new(
                    code::INVALID_TASK_JSON,
                    error.to_string(),
                ))
            }
        }
        Ok(Ok(())) => Ok(buffer),
    }
}

/// Read and parse a document from any byte source.
pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> Result<TaskDocument, TaskRejection> {
    parse(&read_bytes(reader).await?)
}

/// Read and parse a document from a file.
pub async fn read_path(path: &str) -> Result<TaskDocument, TaskRejection> {
    let file = tokio::fs::File::open(Path::new(path))
        .await
        .map_err(|error| {
            TaskRejection::new(
                code::INVALID_TASK_JSON,
                format!("cannot read task: {error}"),
            )
        })?;
    let mut file = file;
    read(&mut file).await
}

/// Read and parse a document from stdin.
pub async fn read_stdin() -> Result<TaskDocument, TaskRejection> {
    let mut stdin = tokio::io::stdin();
    read(&mut stdin).await
}
