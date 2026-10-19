use crate::Error;
use serde_json::Value;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

pub(crate) fn fits(value: &impl serde::Serialize, limit: usize) -> bool {
    struct Count(usize, usize);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.0 + bytes.len() >= self.1 {
                return Err(std::io::Error::other("frame_limit"));
            }
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Count(0, limit), value).is_ok()
}

/// Never allocate the complete untrusted line before applying its bound.
pub(crate) async fn read<R: AsyncBufRead + Unpin>(
    input: &mut R,
    limit: usize,
) -> Result<Option<Vec<u8>>, Error> {
    let mut frame = Vec::new();
    loop {
        let available = input.fill_buf().await?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err(Error::IncompleteFrame)
            };
        }
        let end = available.iter().position(|byte| *byte == b'\n');
        let count = end.map_or(available.len(), |index| index + 1);
        if frame.len() + count > limit {
            return Err(Error::FrameLimit);
        }
        frame.extend_from_slice(&available[..count]);
        input.consume(count);
        if end.is_some() {
            return Ok(Some(frame));
        }
    }
}

pub(crate) async fn write<W: AsyncWrite + Unpin>(
    output: &mut W,
    value: &Value,
    limit: usize,
) -> Result<(), Error> {
    // Do not allocate an unbounded encoded copy of a host result either.
    struct Bounded(Vec<u8>, usize);
    impl std::io::Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.0.len() + bytes.len() >= self.1 {
                return Err(std::io::Error::other("frame_limit"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut frame = Bounded(Vec::new(), limit);
    serde_json::to_writer(&mut frame, value).map_err(|_| Error::FrameLimit)?;
    frame.0.push(b'\n');
    output.write_all(&frame.0).await?;
    output.flush().await?;
    Ok(())
}
