use crate::{CaptureResult, discovery::CodexSourceCandidate, source_file};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::{
    fs::File,
    io::{BufRead as _, BufReader, Read as _},
};

/// Raw records have no serialization wrapper or authority. Never send them to a renderer.
pub struct JsonlPage {
    pub records: Vec<Value>,
    pub complete: bool,
    pub bytes: u64,
    pub total_bytes: u64,
    pub sha256: [u8; 32],
}
pub struct JsonlSnapshot {
    candidate: CodexSourceCandidate,
    file: BufReader<File>,
    stamp: source_file::FileStamp,
    bytes: u64,
    hash: Sha256,
    failed: bool,
}
impl JsonlSnapshot {
    pub fn open(candidate: CodexSourceCandidate, maximum_bytes: u64) -> CaptureResult<Self> {
        if maximum_bytes == 0 || maximum_bytes > 64 * 1024 * 1024 || !candidate.revalidate()? {
            return Err("local JSONL source unavailable".into());
        }
        let (file, stamp) =
            source_file::open_regular(candidate.path(), Some(candidate.identity()))?;
        if stamp.len > maximum_bytes {
            return Err("local JSONL source exceeds its bound".into());
        }
        Ok(Self {
            candidate,
            file: BufReader::new(file),
            stamp,
            bytes: 0,
            hash: Sha256::new(),
            failed: false,
        })
    }
    fn check(&self) -> CaptureResult<()> {
        if self.failed
            || !self.candidate.revalidate()?
            || !source_file::unchanged(self.file.get_ref(), self.candidate.path(), &self.stamp)?
        {
            return Err("local JSONL source changed".into());
        }
        Ok(())
    }
    pub fn next(
        &mut self,
        maximum_records: usize,
        check: &dyn Fn() -> CaptureResult<()>,
    ) -> CaptureResult<JsonlPage> {
        let result = self.read(maximum_records, check);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn read(
        &mut self,
        maximum_records: usize,
        check: &dyn Fn() -> CaptureResult<()>,
    ) -> CaptureResult<JsonlPage> {
        if !(1..=256).contains(&maximum_records) {
            return Err("local JSONL page bound".into());
        }
        check()?;
        self.check()?;
        let mut records = Vec::new();
        let start = self.bytes;
        while records.len() < maximum_records
            && self.bytes < self.stamp.len
            && self.bytes - start < 256 * 1024
        {
            check()?;
            let mut bytes = Vec::new();
            (&mut self.file)
                .take(1024 * 1024 + 1)
                .read_until(b'\n', &mut bytes)?;
            if bytes.len() > 1024 * 1024 || bytes.last() != Some(&b'\n') {
                return Err("complete bounded JSONL record required".into());
            }
            self.bytes += bytes.len() as u64;
            self.hash.update(&bytes);
            records.push(
                serde_json::from_slice(&bytes).map_err(|_| "local JSONL record unavailable")?,
            );
        }
        self.check()?;
        check()?;
        Ok(JsonlPage {
            records,
            complete: self.bytes == self.stamp.len,
            bytes: self.bytes,
            total_bytes: self.stamp.len,
            sha256: self.hash.clone().finalize().into(),
        })
    }
}
