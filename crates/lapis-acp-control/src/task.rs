//! The prepared task document, version 1.
//!
//! A task document is untrusted executable input. It cannot change the working
//! directory, the executable, the tools or the credentials, all of which come
//! only from trusted launch configuration. Every rejection carries a stable code
//! so a controlling application can act without parsing prose, and no rejection
//! quotes the document back, because the document may contain anything.

use std::time::Duration;

use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// The only task document version this build accepts.
pub const TASK_VERSION: u64 = 1;

/// Maximum size of a task document, whitespace included.
pub const MAX_TASK_BYTES: usize = 1024 * 1024;

/// Maximum size of a task identifier.
pub const MAX_TASK_ID_BYTES: usize = 256;

/// Bound on reading the document, separate from the execution deadline.
pub const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Stable, sanitized rejection codes.
pub mod code {
    /// The document was malformed, oversized, duplicated or carried unknown fields.
    pub const INVALID_TASK_JSON: &str = "invalid_task_json";
    /// The document exceeded the size cap.
    pub const TASK_TOO_LARGE: &str = "task_too_large";
    /// The document named a version this build does not accept.
    pub const UNSUPPORTED_TASK_VERSION: &str = "unsupported_task_version";
    /// The identifier was blank, overlong or carried control characters.
    pub const INVALID_TASK_ID: &str = "invalid_task_id";
    /// The runner named by the document is unknown or not enabled.
    pub const INVALID_RUNNER: &str = "invalid_runner";
    /// The document named a runner other than the configured one.
    pub const RUNNER_MISMATCH: &str = "runner_mismatch";
    /// The prompt was blank.
    pub const INVALID_PROMPT: &str = "invalid_prompt";
    /// The duration was absent or zero.
    pub const INVALID_DURATION: &str = "invalid_duration";
    /// Reading the document did not finish inside its bound.
    pub const TASK_READ_TIMEOUT: &str = "task_read_timeout";
    /// The configured launch settings were unusable.
    pub const INVALID_LAUNCH_CONFIGURATION: &str = "invalid_launch_configuration";
    /// The agent executable could not be spawned.
    pub const AGENT_SPAWN_FAILED: &str = "agent_spawn_failed";
    /// The agent closed its stream before answering.
    pub const AGENT_EXITED: &str = "agent_exited";
    /// The agent returned an error for the turn.
    pub const AGENT_ERROR: &str = "agent_error";
    /// The protocol exchange was violated.
    pub const PROTOCOL_ERROR: &str = "protocol_error";
    /// The agent stopped reading its stdin.
    pub const WRITE_TIMEOUT: &str = "write_timeout";
    /// Cleanup could not confirm the agent stopped.
    pub const CLEANUP_UNCONFIRMED: &str = "cleanup_unconfirmed";
}

/// A validated task document.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskDocument {
    /// Document version; must equal [`TASK_VERSION`].
    pub version: u64,
    /// Correlation identifier. Not a deduplication key and not a claim.
    pub task_id: String,
    /// Runner the task is addressed to.
    pub agent: String,
    /// Prepared prompt text.
    pub prompt: String,
    /// Requested absolute cap in milliseconds.
    pub max_duration_ms: u64,
}

/// A rejected document, carrying a stable code and an operator-facing reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRejection {
    /// The stable code a controlling application switches on.
    pub code: &'static str,
    /// A reason for stderr. It never quotes the document.
    pub reason: String,
}

impl TaskRejection {
    /// Build a rejection from a stable code and an operator-facing reason.
    pub fn new(code: &'static str, reason: impl Into<String>) -> Self {
        Self {
            code,
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for TaskRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.reason)
    }
}

/// The top-level keys a version 1 document may carry.
const ALLOWED_KEYS: [&str; 5] = ["version", "taskId", "agent", "prompt", "maxDurationMs"];

/// Collect top-level object keys, preserving duplicates.
///
/// A `Value` cannot represent a duplicate key, so ambiguity has to be detected
/// before decoding. Two parsers could otherwise disagree about which value wins,
/// and a validator that accepted what an executor then interpreted differently
/// would be worse than one that refused both.
struct KeyCollector;

impl<'de> serde::de::Visitor<'de> for KeyCollector {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Vec<String>, A::Error> {
        let mut keys = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            keys.push(key);
            let ignored: serde::de::IgnoredAny = map.next_value()?;
            let _ = ignored;
        }
        Ok(keys)
    }
}

fn collect_keys(bytes: &[u8]) -> Result<Vec<String>, TaskRejection> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    deserializer
        .deserialize_map(KeyCollector)
        .map_err(|error| TaskRejection::new(code::INVALID_TASK_JSON, error.to_string()))
}

fn valid_task_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_TASK_ID_BYTES
        && !value.chars().any(char::is_control)
}

/// Parse and validate a task document.
pub fn parse(bytes: &[u8]) -> Result<TaskDocument, TaskRejection> {
    if bytes.len() > MAX_TASK_BYTES {
        return Err(TaskRejection::new(
            code::TASK_TOO_LARGE,
            format!(
                "{} bytes exceeds the {MAX_TASK_BYTES} byte cap",
                bytes.len()
            ),
        ));
    }
    let keys = collect_keys(bytes)?;
    for key in &keys {
        if !ALLOWED_KEYS.contains(&key.as_str()) {
            return Err(TaskRejection::new(
                code::INVALID_TASK_JSON,
                format!("unknown field {key:?}"),
            ));
        }
    }
    let mut seen: Vec<&str> = Vec::new();
    for key in &keys {
        if seen.contains(&key.as_str()) {
            return Err(TaskRejection::new(
                code::INVALID_TASK_JSON,
                format!("duplicate field {key:?}"),
            ));
        }
        seen.push(key);
    }

    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let document = TaskDocument::deserialize(&mut deserializer)
        .map_err(|error| TaskRejection::new(code::INVALID_TASK_JSON, error.to_string()))?;
    // `end` rejects a trailing document, so one file cannot smuggle two tasks.
    deserializer
        .end()
        .map_err(|error| TaskRejection::new(code::INVALID_TASK_JSON, error.to_string()))?;

    if document.version != TASK_VERSION {
        return Err(TaskRejection::new(
            code::UNSUPPORTED_TASK_VERSION,
            format!("version {} is not {TASK_VERSION}", document.version),
        ));
    }
    if !valid_task_id(&document.task_id) {
        return Err(TaskRejection::new(
            code::INVALID_TASK_ID,
            format!(
                "taskId must be nonblank, at most {MAX_TASK_ID_BYTES} bytes and free of control characters"
            ),
        ));
    }
    if document.agent.trim().is_empty() {
        return Err(TaskRejection::new(code::INVALID_RUNNER, "agent is blank"));
    }
    if document.prompt.trim().is_empty() {
        return Err(TaskRejection::new(code::INVALID_PROMPT, "prompt is blank"));
    }
    if document.max_duration_ms == 0 {
        return Err(TaskRejection::new(
            code::INVALID_DURATION,
            "maxDurationMs must be positive",
        ));
    }
    Ok(document)
}

/// Parse a task document from its JSON text.
pub fn parse_text(text: &str) -> Result<TaskDocument, TaskRejection> {
    parse(text.as_bytes())
}

/// The requested cap as a duration.
pub fn requested_cap(document: &TaskDocument) -> Duration {
    Duration::from_millis(document.max_duration_ms)
}

/// Build a document value, for tests and for callers assembling a task.
pub fn document_value(task_id: &str, agent: &str, prompt: &str, max_duration_ms: u64) -> Value {
    serde_json::json!({
        "version": TASK_VERSION,
        "taskId": task_id,
        "agent": agent,
        "prompt": prompt,
        "maxDurationMs": max_duration_ms,
    })
}
