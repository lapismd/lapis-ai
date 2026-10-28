//! ACP configuration over existing provider effort support; never changes auth.
use super::*;
use crate::config::{Provider, ThinkingEffort};

pub(super) fn options(provider: Provider, effort: Option<ThinkingEffort>) -> Value {
    let values = [
        "default", "none", "minimal", "low", "medium", "high", "xhigh", "max",
    ];
    let choices: Vec<_> = values
        .into_iter()
        .filter(|value| {
            !matches!(provider, Provider::Anthropic) || !matches!(*value, "none" | "minimal")
        })
        .map(|value| json!({"value":value,"name":value}))
        .collect();
    json!([
        {"id":"mode","name":"Execution mode","category":"mode","type":"select","currentValue":"ask","options":[{"value":"ask","name":"Ask before tools"}]},
        {"id":"reasoning_effort","name":"Reasoning effort","category":"thought_level","type":"select","currentValue":effort.map(ThinkingEffort::openai_effort_str).unwrap_or("default"),"options":choices}
    ])
}

pub(super) async fn configure(
    app: &Arc<App>,
    method: &str,
    id: Value,
    params: Value,
    wire_tx: &WireSender,
) {
    let result = {
        let mut sessions = app.sessions.lock().await;
        match sessions.get_mut(params["sessionId"].as_str().unwrap_or("")) {
            Some(session) if !session.busy => apply(app.cfg.provider, session, method, &params),
            _ => Err("unknown or busy session"),
        }
    };
    match result {
        Ok(options) => wire::send(wire_tx, wire::ok(id, json!({"configOptions":options}))).await,
        Err(reason) => reject(wire_tx, id, INVALID_PARAMS, reason).await,
    }
}
fn apply(
    provider: Provider,
    session: &mut Session,
    method: &str,
    params: &Value,
) -> Result<Value, &'static str> {
    if method == "session/set_mode" || params["configId"] == "mode" {
        let value = if method == "session/set_mode" {
            &params["modeId"]
        } else {
            &params["value"]
        };
        if value != "ask" {
            return Err("unsupported execution mode");
        }
    } else if params["configId"] == "reasoning_effort" {
        let value = params["value"].as_str().unwrap_or("");
        let offered = options(provider, session.effective_effort)[1]["options"]
            .as_array()
            .is_some_and(|choices| choices.iter().any(|choice| choice["value"] == value));
        if !offered {
            return Err("unsupported reasoning effort");
        }
        session.effective_effort = if value == "default" {
            None
        } else {
            Some(serde_json::from_value(json!(value)).map_err(|_| "unsupported reasoning effort")?)
        };
    } else {
        return Err("unsupported configuration option");
    }
    Ok(options(provider, session.effective_effort))
}
