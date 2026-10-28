use crate::{Config, Error, Tool, ToolError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashSet, time::Duration};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

pub(crate) fn identity(value: &Value) -> Option<String> {
    match value {
        Value::String(text)
            if !text.is_empty() && text.len() <= 256 && !text.chars().any(char::is_control) =>
        {
            Some(format!("s:{text}"))
        }
        Value::Number(number) if number.is_i64() || number.is_u64() => Some(format!("n:{number}")),
        _ => None,
    }
}

pub(crate) fn validate(config: &Config, tools: &[Tool]) -> Result<(), Error> {
    let label = |value: &str| {
        !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
    };
    if !label(&config.name)
        || !label(&config.version)
        || !(256..=1_048_576).contains(&config.frame_bytes)
        || !(1..=8).contains(&config.maximum_calls)
        || !(1..=4096).contains(&config.maximum_requests)
        || config.call_timeout.is_zero()
        || config.call_timeout > Duration::from_secs(60)
        || tools.len() > 256
    {
        return Err(Error::Configuration);
    }
    let mut names = HashSet::new();
    for tool in tools {
        if !label(&tool.name)
            || tool
                .name
                .bytes()
                .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')))
            || !names.insert(&tool.name)
            || tool.description.len() > 4096
            || !tool.input_schema.is_object()
        {
            return Err(Error::Configuration);
        }
    }
    // The snapshot must itself fit a single ordinary list response.
    #[derive(Serialize)]
    struct List<'a> {
        tools: &'a [Tool],
    }
    #[derive(Serialize)]
    struct Response<'a> {
        jsonrpc: &'static str,
        id: String,
        result: List<'a>,
    }
    let list = Response {
        jsonrpc: "2.0",
        id: "\\".repeat(256),
        result: List { tools },
    };
    if !crate::framing::fits(&list, config.frame_bytes.min(65_536)) {
        return Err(Error::Configuration);
    }
    Ok(())
}

pub(crate) fn result(id: Value, value: Value) -> Value {
    // Move the host payload; serializing through json! would clone it before
    // the bounded output writer could refuse an oversized result.
    Value::Object(serde_json::Map::from_iter([
        ("jsonrpc".into(), Value::String("2.0".into())),
        ("id".into(), id),
        ("result".into(), value),
    ]))
}
pub(crate) fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
pub(crate) fn failure(error: ToolError) -> Value {
    let (message, outcome) = match error {
        ToolError::Unavailable => ("Application tool is unavailable.", "rejected"),
        ToolError::InvalidInput => ("Application tool input is invalid.", "rejected"),
        ToolError::Unknown => (
            "Tool execution outcome is unknown. Do not replay automatically.",
            "unknown",
        ),
    };
    json!({"content":[{"type":"text","text":message}],"isError":true,"_meta":{"lapis.md/outcome":outcome}})
}
