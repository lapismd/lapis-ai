//! The versioned control protocol between a controlling application and a
//! persistent `lapis-acp`.
//!
//! Frames are newline-delimited JSON over the inherited stdin and stdout, so
//! there is no listener, no port, no token and no authentication: the process that
//! spawned this one already decided to run it with this environment. Stdout carries
//! frames and nothing else, which is what lets a controlling application parse it
//! unconditionally.
//!
//! Every frame carries an explicit integer version and an unsupported one is
//! refused before any agent is started, so a controlling application cannot
//! silently drive a harness whose contract it has misread.

use serde_json::{Value, json};

use crate::record::TerminalRecord;
use crate::task::{TaskDocument, TaskRejection, code, parse};

/// The control protocol version this build speaks.
pub const CONTROL_VERSION: u64 = 1;

/// Stable control-level rejection codes, distinct from task document codes.
pub mod code_control {
    /// The frame was not valid JSON or not an object.
    pub const INVALID_FRAME: &str = "invalid_frame";
    /// The frame named a version this build does not speak.
    pub const UNSUPPORTED_CONTROL_VERSION: &str = "unsupported_control_version";
    /// The frame named an unknown request type.
    pub const UNKNOWN_REQUEST: &str = "unknown_request";
    /// A required field was absent or the wrong type.
    pub const INVALID_REQUEST: &str = "invalid_request";
    /// The bounded queue was full, so the submission was not admitted.
    pub const QUEUE_FULL: &str = "queue_full";
    /// A cancel named a request that is not in flight.
    pub const NOT_IN_FLIGHT: &str = "not_in_flight";
}

/// A request from the controlling application.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Execute one prepared task document.
    Submit {
        /// Correlates the reply to this request.
        request_id: String,
        /// Caller-named serialization scope. Two submissions in one scope never
        /// overlap; distinct scopes may run concurrently. Omitted means the
        /// default scope, which preserves serial behaviour.
        scope: String,
        /// The validated task document.
        task: TaskDocument,
    },
    /// Ask for cancellation of an in-flight submission.
    Cancel {
        /// This request's own identifier.
        request_id: String,
        /// The submission to cancel.
        target: String,
    },
    /// List resolved runners with availability.
    Runners {
        /// Correlates the reply to this request.
        request_id: String,
    },
    /// Report pool and queue state.
    Status {
        /// Correlates the reply to this request.
        request_id: String,
    },
    /// Stop accepting work and exit once in-flight work has drained.
    Shutdown {
        /// Correlates the acknowledgement to this request.
        request_id: String,
    },
}

impl Request {
    /// The correlation identifier, whatever the variant.
    pub fn request_id(&self) -> &str {
        match self {
            Self::Submit { request_id, .. }
            | Self::Cancel { request_id, .. }
            | Self::Runners { request_id }
            | Self::Status { request_id }
            | Self::Shutdown { request_id } => request_id,
        }
    }
}

/// A frame sent back to the controlling application.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// Emitted once at startup, carrying the resolved runner list and sizing.
    Ready {
        /// One entry per enabled runner.
        runners: Vec<Value>,
        /// How many submissions may execute at once.
        workers: usize,
        /// How many may wait before a submission is refused.
        queue_capacity: usize,
    },
    /// A submission was admitted and is waiting or running.
    Queued {
        /// The submission it acknowledges.
        request_id: String,
        /// The scope it will run in.
        scope: String,
        /// How many submissions were already waiting.
        position: usize,
    },
    /// The terminal record for one submission.
    Record {
        /// The submission it answers.
        request_id: String,
        /// The record, exactly as the one-shot path emits it.
        record: TerminalRecord,
    },
    /// A runner listing.
    Runners {
        /// The request it answers.
        request_id: String,
        /// One entry per enabled runner.
        runners: Vec<Value>,
    },
    /// Pool and queue state.
    Status {
        /// The request it answers.
        request_id: String,
        /// Configured concurrency.
        workers: usize,
        /// Configured queue bound.
        queue_capacity: usize,
        /// Submissions admitted but not yet running.
        queued: usize,
        /// Scopes with a turn in flight.
        running: Vec<String>,
    },
    /// A request that carries no payload other than success.
    Ack {
        /// The request it answers.
        request_id: String,
    },
    /// A refusal. The reason is operator text and never carries task content.
    Error {
        /// The request it answers, when one could be identified.
        request_id: Option<String>,
        /// A stable code from a fixed set.
        code: String,
        /// Operator-facing explanation.
        reason: String,
    },
    /// Operator diagnostics, which the one-shot path would write to stderr.
    Diagnostic {
        /// The message.
        message: String,
    },
}

impl Response {
    /// Serialize to the single line written to the control stream.
    pub fn to_value(&self) -> Value {
        let base = match self {
            Self::Ready {
                runners,
                workers,
                queue_capacity,
            } => json!({
                "type": "ready", "runners": runners,
                "workers": workers, "queueCapacity": queue_capacity,
            }),
            Self::Queued {
                request_id,
                scope,
                position,
            } => json!({
                "type": "queued", "requestId": request_id,
                "scope": scope, "position": position,
            }),
            Self::Status {
                request_id,
                workers,
                queue_capacity,
                queued,
                running,
            } => json!({
                "type": "status", "requestId": request_id, "workers": workers,
                "queueCapacity": queue_capacity, "queued": queued, "running": running,
            }),
            Self::Record { request_id, record } => json!({
                "type": "record", "requestId": request_id,
                "record": serde_json::to_value(record).expect("record serializes"),
            }),
            Self::Runners {
                request_id,
                runners,
            } => {
                json!({"type": "runners", "requestId": request_id, "runners": runners})
            }
            Self::Ack { request_id } => json!({"type": "ack", "requestId": request_id}),
            Self::Error {
                request_id,
                code,
                reason,
            } => json!({
                "type": "error", "requestId": request_id, "code": code, "reason": reason,
            }),
            Self::Diagnostic { message } => json!({"type": "diagnostic", "message": message}),
        };
        let mut frame = base.as_object().expect("object").clone();
        frame.insert("version".to_owned(), json!(CONTROL_VERSION));
        Value::Object(frame)
    }

    /// Serialize to one line.
    pub fn to_line(&self) -> String {
        serde_json::to_string(&self.to_value()).expect("control frame serializes")
    }
}

/// Parse one inbound control frame.
///
/// A submission's task document is validated here rather than at execution, so a
/// controlling application learns about a malformed document immediately and can
/// correlate the refusal to the request it sent.
pub fn parse_request(frame: &Value) -> Result<Request, TaskRejection> {
    let version = frame.get("version").and_then(Value::as_u64);
    match version {
        Some(CONTROL_VERSION) => {}
        Some(other) => {
            return Err(TaskRejection::new(
                code_control::UNSUPPORTED_CONTROL_VERSION,
                format!("control version {other} is not {CONTROL_VERSION}"),
            ));
        }
        None => {
            return Err(TaskRejection::new(
                code_control::INVALID_FRAME,
                "frame is missing an integer version".to_owned(),
            ));
        }
    }
    let kind = frame.get("type").and_then(Value::as_str).ok_or_else(|| {
        TaskRejection::new(
            code_control::INVALID_FRAME,
            "frame is missing a type".to_owned(),
        )
    })?;
    let request_id = string_field(frame, "requestId").ok_or_else(|| {
        TaskRejection::new(
            code_control::INVALID_REQUEST,
            "frame is missing a string requestId".to_owned(),
        )
    })?;
    match kind {
        "status" => Ok(Request::Status { request_id }),
        "submit" => {
            let task = frame.get("task").ok_or_else(|| {
                TaskRejection::new(
                    code_control::INVALID_REQUEST,
                    "submit is missing a task document".to_owned(),
                )
            })?;
            // Reuse the one-shot validator by re-encoding, so the two entry points
            // cannot drift into accepting different documents.
            let encoded = serde_json::to_vec(task)
                .map_err(|error| TaskRejection::new(code::INVALID_TASK_JSON, error.to_string()))?;
            let document = parse(&encoded)?;
            Ok(Request::Submit {
                request_id,
                scope: string_field(frame, "scope")
                    .unwrap_or_else(|| crate::scheduler::DEFAULT_SCOPE.to_owned()),
                task: document,
            })
        }
        "cancel" => {
            let target = string_field(frame, "target").ok_or_else(|| {
                TaskRejection::new(
                    code_control::INVALID_REQUEST,
                    "cancel is missing a string target".to_owned(),
                )
            })?;
            Ok(Request::Cancel { request_id, target })
        }
        "runners" => Ok(Request::Runners { request_id }),
        "shutdown" => Ok(Request::Shutdown { request_id }),
        other => Err(TaskRejection::new(
            code_control::UNKNOWN_REQUEST,
            format!("unknown request type {other:?}"),
        )),
    }
}

/// Parse a control frame from its JSON text.
pub fn parse_request_text(text: &str) -> Result<Request, TaskRejection> {
    let frame: Value = serde_json::from_str(text)
        .map_err(|error| TaskRejection::new(code_control::INVALID_FRAME, error.to_string()))?;
    parse_request(&frame)
}

fn string_field(frame: &Value, key: &str) -> Option<String> {
    frame
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
}

/// Build a submit frame in the default scope.
pub fn submit_frame(request_id: &str, task: &Value) -> Value {
    scoped_submit_frame(request_id, crate::scheduler::DEFAULT_SCOPE, task)
}

/// Build a submit frame in a named scope.
pub fn scoped_submit_frame(request_id: &str, scope: &str, task: &Value) -> Value {
    json!({
        "version": CONTROL_VERSION, "type": "submit", "scope": scope,
        "requestId": request_id, "task": task,
    })
}

/// Build a status frame.
pub fn status_frame(request_id: &str) -> Value {
    json!({"version": CONTROL_VERSION, "type": "status", "requestId": request_id})
}

/// Build a cancel frame.
pub fn cancel_frame(request_id: &str, target: &str) -> Value {
    json!({
        "version": CONTROL_VERSION, "type": "cancel",
        "requestId": request_id, "target": target,
    })
}

/// Build a runner listing frame.
pub fn runners_frame(request_id: &str) -> Value {
    json!({"version": CONTROL_VERSION, "type": "runners", "requestId": request_id})
}

/// Build a shutdown frame.
pub fn shutdown_frame(request_id: &str) -> Value {
    json!({"version": CONTROL_VERSION, "type": "shutdown", "requestId": request_id})
}
