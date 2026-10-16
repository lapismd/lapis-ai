//! LapisMD extension: bounded OpenAI-compatible discovery without prompting.
use crate::config::Config;
use serde_json::{json, Value};
use std::{collections::BTreeSet, time::Duration};

pub async fn discover(
    config: &Config,
) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let mut response = client
        .get(format!("{}/models", config.base_url.trim_end_matches('/')))
        .bearer_auth(&config.api_key)
        .send()
        .await?
        .error_for_status()?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > 131072 {
            return Err("model catalog limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes)?;
    let rows = value["data"].as_array().ok_or("invalid model catalog")?;
    if rows.len() > 64 {
        return Err("model catalog limit".into());
    }
    let mut ids = BTreeSet::new();
    let mut models = Vec::new();
    for row in rows {
        let id = row["id"].as_str().ok_or("invalid model identity")?;
        if id.is_empty() || id.len() > 512 || id.chars().any(char::is_control) {
            return Err("invalid model identity".into());
        }
        if ids.insert(id) {
            models.push(json!({"modelId":id,"name":id}));
        }
    }
    if serde_json::to_vec(&models)?.len() > 16384 {
        return Err("model catalog limit".into());
    }
    Ok(models)
}
