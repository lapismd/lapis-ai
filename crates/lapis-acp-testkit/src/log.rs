//! Reading the fixture agent's append-only record log.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The fixture's record log.
///
/// The fixture appends one JSON object per line: a launch record first, then one
/// record per frame it received, plus records for any descendant it spawned and
/// any permission answer it was given. A test reads this to assert what the agent
/// actually saw, rather than inferring it from the harness's own view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureLog {
    path: PathBuf,
}

impl FixtureLog {
    /// Wrap an existing or future log path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The wrapped path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Every record, in write order.
    ///
    /// A trailing partial line is ignored rather than reported, because a test
    /// that kills the fixture can catch it mid-write. Any other unparseable line
    /// is an error, so a corrupted log cannot masquerade as a shorter one.
    pub fn records(&self) -> io::Result<Vec<Value>> {
        let text = fs::read_to_string(&self.path)?;
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        let mut records = Vec::with_capacity(lines.len());
        for (index, line) in lines.iter().enumerate() {
            match serde_json::from_str::<Value>(line) {
                Ok(value) => records.push(value),
                Err(error) => {
                    if index + 1 == lines.len() {
                        break;
                    }
                    return Err(io::Error::other(format!(
                        "{} line {}: {error}",
                        self.path.display(),
                        index + 1
                    )));
                }
            }
        }
        Ok(records)
    }

    /// The launch record, which is always the first one written.
    pub fn launch(&self) -> io::Result<Value> {
        self.records()?
            .into_iter()
            .next()
            .ok_or_else(|| io::Error::other(format!("{} is empty", self.path.display())))
    }

    /// The process id of a descendant the fixture spawned, when it spawned one.
    pub fn descendant(&self) -> io::Result<Option<u32>> {
        Ok(self
            .records()?
            .iter()
            .find_map(|record| record.get("descendant"))
            .and_then(Value::as_u64)
            .map(|pid| pid as u32))
    }

    /// The permission answer the fixture received, when it asked for one.
    pub fn permission_answer(&self) -> io::Result<Option<Value>> {
        Ok(self
            .records()?
            .iter()
            .find_map(|record| record.get("permissionAnswer"))
            .cloned())
    }

    /// How many times a given method reached the fixture.
    ///
    /// Counting dispatches is how a test proves a crashed turn was never
    /// silently re-sent: the harness must report the crash, not retry it.
    pub fn method_count(&self, method: &str) -> io::Result<usize> {
        Ok(self
            .records()?
            .iter()
            .filter(|record| record.get("method").and_then(Value::as_str) == Some(method))
            .count())
    }

    /// The crash marker the fixture recorded, when it crashed on purpose.
    pub fn crashed(&self) -> io::Result<Option<String>> {
        Ok(self
            .records()?
            .iter()
            .find_map(|record| record.get("crashed"))
            .and_then(Value::as_str)
            .map(str::to_owned))
    }

    /// Report whether any record contains `needle` in its serialized form.
    ///
    /// This is how a test proves that a secret marker never reached the agent, or
    /// that a prompt did.
    pub fn contains(&self, needle: &str) -> io::Result<bool> {
        Ok(self
            .records()?
            .iter()
            .any(|record| record.to_string().contains(needle)))
    }
}
