use crate::{
    Config, Error, ToolCall, ToolError, ToolHandler, calls::Calls, framing, protocol,
    writer::Writer,
};
use serde_json::{Value, json};
use std::{collections::HashSet, sync::Arc};
use tokio::io::{AsyncRead, AsyncWrite, BufReader};

/// Serve one host-owned MCP binding. No process, listener or execution authority
/// is allocated here. Closing or dropping this future cancels its call futures.
pub async fn serve<R, W>(
    input: R,
    output: W,
    handler: Arc<dyn ToolHandler>,
    config: Config,
) -> Result<(), Error>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let tools = handler.tools();
    protocol::validate(&config, &tools)?;
    let mut output = Writer::new(output, &config);
    let mut input = BufReader::new(input);
    let mut calls = Calls::default();
    let mut seen = HashSet::new();
    let mut initialized = false;
    let mut count = 0;
    let outcome = async {
        loop {
            // A completed call may interrupt a partially consumed input line.
            // Retain that read future instead of discarding its accumulated bytes.
            let reading = framing::read(&mut input, config.frame_bytes);
            tokio::pin!(reading);
            let frame = loop {
                tokio::select! {
                    failure = output.ended() => return Err(failure),
                    response = calls.next(), if calls.len() > 0 => {
                        output.send(response?)?;
                    },
                    frame = &mut reading => break frame?,
                }
            };
            let Some(frame) = frame else {
                return Ok(());
            };
            count += 1;
            if count > config.maximum_requests {
                return Err(Error::RequestLimit);
            }
            let request = match serde_json::from_slice::<protocol::Request>(&frame) {
                Ok(request) if request.jsonrpc == "2.0" => request,
                _ => {
                    let valid_json = serde_json::from_slice::<Value>(&frame).is_ok();
                    let error = if valid_json {
                        protocol::error(Value::Null, -32600, "Invalid request")
                    } else {
                        protocol::error(Value::Null, -32700, "Invalid JSON")
                    };
                    output.send(error)?;
                    continue;
                }
            };
            let Some(id) = request.id else {
                if request.method == "notifications/cancelled"
                    && let Some(key) = protocol::identity(&request.params["requestId"])
                {
                    calls.cancel(&key);
                }
                continue;
            };
            let Some(key) = protocol::identity(&id) else {
                let error = protocol::error(Value::Null, -32600, "Invalid request identity");
                output.send(error)?;
                continue;
            };
            if !seen.insert(key.clone()) {
                return Err(Error::DuplicateRequest);
            }
            let response = match request.method.as_str() {
                "initialize" if !initialized => {
                    initialized = true;
                    protocol::result(
                        id,
                        json!({
                            "protocolVersion":"2025-06-18",
                            "capabilities":{"tools":{"listChanged":false}},
                            "serverInfo":{"name":config.name,"version":config.version}
                        }),
                    )
                }
                "ping" => protocol::result(id, json!({})),
                _ if !initialized => protocol::error(id, -32002, "Initialize first"),
                "tools/list" => protocol::result(id, json!({"tools":tools})),
                "tools/call" => {
                    let name = request.params["name"].as_str();
                    let arguments = request
                        .params
                        .get("arguments")
                        .cloned()
                        .unwrap_or(json!({}));
                    match (name, arguments.is_object()) {
                        (Some(name), true) if !tools.iter().any(|tool| tool.name == name) => {
                            protocol::result(id, protocol::failure(ToolError::Unavailable))
                        }
                        (Some(_), true) if calls.len() >= config.maximum_calls => {
                            protocol::error(id, -32000, "Tool bridge is at capacity")
                        }
                        (Some(name), true) => {
                            calls.start(
                                key,
                                ToolCall {
                                    request_id: id,
                                    name: name.into(),
                                    arguments,
                                },
                                handler.clone(),
                                &config,
                            );
                            continue;
                        }
                        _ => protocol::result(id, protocol::failure(ToolError::InvalidInput)),
                    }
                }
                _ => protocol::error(id, -32601, "Unsupported method"),
            };
            output.send(response)?;
        }
    }
    .await;
    calls.stop().await;
    output.stop().await;
    outcome
}
