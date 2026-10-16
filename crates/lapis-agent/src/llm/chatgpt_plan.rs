//! LapisMD maintained fork: stateless ChatGPT-plan Responses transport.
use super::*;
const API: &str = "https://api.openai.com/v1";
impl Llm {
    pub async fn subscription_models(&self) -> Result<Vec<Value>, AgentError> {
        let token = self.auth.bearer_no_browser().await?;
        let mut response = self
            .http
            .get(format!("{API}/models"))
            .bearer_auth(token)
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await
            .map_err(|_| failed("model_catalog_unavailable"))?;
        if !response.status().is_success() {
            return Err(failed("model_catalog_unavailable"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| failed("model_catalog_unavailable"))?
        {
            if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
                return Err(failed("model_catalog_limit"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| failed("model_catalog_invalid"))?;
        let rows = value["models"]
            .as_array()
            .ok_or_else(|| failed("model_catalog_invalid"))?;
        if rows.len() > 1024 {
            return Err(failed("model_catalog_limit"));
        }
        rows.iter()
            .filter(|r| r["visibility"] == "list")
            .map(|r| {
                let id = r["slug"]
                    .as_str()
                    .filter(|s| !s.is_empty() && s.len() <= 512 && !s.chars().any(char::is_control))
                    .ok_or_else(|| failed("model_catalog_invalid"))?;
                let name = r["display_name"]
                    .as_str()
                    .filter(|s| !s.is_empty() && s.len() <= 512 && !s.chars().any(char::is_control))
                    .ok_or_else(|| failed("model_catalog_invalid"))?;
                Ok(json!({"modelId":id,"name":name}))
            })
            .collect()
    }
    pub(super) async fn subscription_complete(
        &self,
        cfg: &Config,
        system: &str,
        history: &[HistoryItem],
        tools: &[ToolDef],
        model: &str,
    ) -> Result<LlmResponse, AgentError> {
        let body = request(cfg, system, history, tools, model);
        let value = self.subscription_post(cfg, &body).await?;
        let output = value["output"].clone();
        let mut result = parse_responses(value)?;
        result.reasoning_details = Some(json!({"chatgpt_output":output}));
        result.request_model = Some(model.into());
        Ok(result)
    }
    async fn subscription_post(&self, cfg: &Config, body: &Value) -> Result<Value, AgentError> {
        if serde_json::to_vec(body)
            .map_err(|_| failed("request_invalid"))?
            .len()
            > MAX_LLM_RESPONSE_BYTES
        {
            return Err(failed("request_limit"));
        }
        tokio::time::timeout(cfg.llm_timeout, self.subscription_send(cfg, body))
            .await
            .map_err(|_| failed("request_outcome_unknown"))?
    }
    async fn subscription_send(&self, cfg: &Config, body: &Value) -> Result<Value, AgentError> {
        #[cfg(not(test))]
        let endpoint = format!("{API}/responses");
        #[cfg(test)]
        let endpoint = format!("{}/responses", cfg.base_url.trim_end_matches('/'));
        let mut token = self.auth.bearer_no_browser().await?;
        for attempt in 0..2 {
            let response = self
                .http
                .post(&endpoint)
                .bearer_auth(&token)
                .json(body)
                .timeout(cfg.llm_timeout)
                .send()
                .await
                .map_err(|_| failed("request_outcome_unknown"))?;
            if response.status() == reqwest::StatusCode::UNAUTHORIZED && attempt == 0 {
                token = self.auth.refresh_now(&token).await?;
                continue;
            }
            return collect(response).await;
        }
        Err(failed("authentication_required"))
    }
}
fn request(
    cfg: &Config,
    system: &str,
    history: &[HistoryItem],
    tools: &[ToolDef],
    model: &str,
) -> Value {
    let mut body = responses_body(cfg, system, history, tools, model, cfg.thinking_effort);
    body.as_object_mut().unwrap().remove("max_output_tokens");
    body["store"] = json!(false);
    body["stream"] = json!(true);
    let mut input = Vec::new();
    for item in history {
        if let HistoryItem::Assistant {
            reasoning_details: Some(raw),
            ..
        } = item
        {
            if let Some(items) = raw["chatgpt_output"].as_array() {
                input.extend(items.iter().cloned());
                continue;
            }
        }
        if let Some(items) =
            responses_body(cfg, system, std::slice::from_ref(item), &[], model, None)["input"]
                .as_array()
        {
            input.extend(items.iter().cloned());
        }
    }
    body["input"] = json!(input);
    if let Some(tools) = body.get("tools").cloned() {
        body["tools"] = json!([{"type":"namespace","name":"functions","tools":tools}]);
    }
    body
}
fn failed(code: &str) -> AgentError {
    AgentError::Llm(format!("chatgpt_plan: {code}"))
}
async fn collect(mut response: reqwest::Response) -> Result<Value, AgentError> {
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| s.len() <= 128 && !s.chars().any(char::is_control))
        .unwrap_or("unknown")
        .to_owned();
    if !status.is_success() {
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| failed("request_outcome_unknown"))?
        {
            if bytes.len() + chunk.len() > MAX_LLM_ERROR_BODY_BYTES {
                break;
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        return Err(failed(&format!(
            "HTTP {} {} request {}",
            status.as_u16(),
            error_code(&value),
            request_id
        )));
    }
    if !response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.starts_with("text/event-stream"))
    {
        return Err(failed("invalid_stream_media_type"));
    }
    let mut parser = Stream::default();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| failed("stream_outcome_unknown"))?
    {
        parser.feed(&chunk)?;
    }
    parser.finish()
}
fn error_code(v: &Value) -> &str {
    v["error"]["code"]
        .as_str()
        .or_else(|| v["response"]["error"]["code"].as_str())
        .or_else(|| v["code"].as_str())
        .filter(|s| {
            s.len() <= 128
                && s.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
        })
        .unwrap_or("provider_error")
}
#[derive(Default)]
struct Stream {
    pending: Vec<u8>,
    data: Vec<u8>,
    total: usize,
    completed: Option<Value>,
}
impl Stream {
    fn feed(&mut self, bytes: &[u8]) -> Result<(), AgentError> {
        self.total = self
            .total
            .checked_add(bytes.len())
            .ok_or_else(|| failed("stream_limit"))?;
        if self.total > MAX_LLM_RESPONSE_BYTES {
            return Err(failed("stream_limit"));
        }
        self.pending.extend_from_slice(bytes);
        while let Some(at) = self.pending.iter().position(|b| *b == b'\n') {
            let line = self.pending.drain(..=at).collect::<Vec<_>>();
            let mut line = &line[..line.len() - 1];
            if line.ends_with(b"\r") {
                line = &line[..line.len() - 1];
            }
            if line.is_empty() {
                self.event()?;
            } else if let Some(data) = line.strip_prefix(b"data:") {
                if !self.data.is_empty() {
                    self.data.push(b'\n');
                }
                self.data
                    .extend_from_slice(data.strip_prefix(b" ").unwrap_or(data));
            }
        }
        Ok(())
    }
    fn event(&mut self) -> Result<(), AgentError> {
        if self.data.is_empty() {
            return Ok(());
        }
        if self.data == b"[DONE]" {
            self.data.clear();
            return Ok(());
        }
        let value: Value =
            serde_json::from_slice(&self.data).map_err(|_| failed("invalid_stream_event"))?;
        self.data.clear();
        match value["type"].as_str() {
            Some("response.completed") => {
                if self.completed.is_some()
                    || value["response"]["status"] != "completed"
                    || !value["response"]["output"].is_array()
                {
                    return Err(failed("invalid_completion"));
                }
                self.completed = Some(value["response"].clone());
            }
            Some("response.failed" | "response.incomplete" | "error") => {
                return Err(failed(error_code(&value)));
            }
            _ => {}
        }
        Ok(())
    }
    fn finish(mut self) -> Result<Value, AgentError> {
        if !self.pending.is_empty() || !self.data.is_empty() {
            return Err(failed("stream_outcome_unknown"));
        }
        self.completed
            .take()
            .ok_or_else(|| failed("stream_outcome_unknown"))
    }
}
#[cfg(test)]
mod tests;
