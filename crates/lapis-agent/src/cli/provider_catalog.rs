//! Bounded provider catalogs for configuration previews. Never prompts or signs in.
use lapis_agent::{
    config::{Config, Provider},
    AgentError,
};
use serde_json::{json, Value};
use std::{collections::BTreeSet, time::Duration};

pub fn run() {
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "unavailable")
        .and_then(|runtime| {
            runtime.block_on(async {
                tokio::time::timeout(Duration::from_secs(20), discover())
                    .await
                    .map_err(|_| "unavailable")?
            })
        });
    let value = match result {
        Ok(facts) => {
            json!({"status": if facts["models"]["availableModels"].as_array().is_some_and(|m| m.is_empty()) {"empty"} else {"ready"}, "facts":facts})
        }
        Err(status) => json!({"status":status,"facts":{}}),
    };
    println!("{value}");
}

async fn discover() -> Result<Value, &'static str> {
    let mut cfg = Config::from_env().map_err(|_| "authentication_required")?;
    // The provider's legacy curated fallback is inappropriate for a live availability picker.
    if matches!(cfg.provider, Provider::DatabricksV2) && cfg.databricks_model_filter.is_none() {
        cfg.databricks_model_filter = lapis_agent::config::DatabricksModelFilter::parse(Some("*"))
            .map_err(|_| "unavailable")?;
    }
    let models = if cfg.openai_api == lapis_agent::config::OpenAiApi::ChatGptPlan {
        let source = crate::chatgpt_token::source()
            .map_err(|_| "authentication_required")?
            .ok_or("authentication_required")?;
        lapis_agent::discover_subscription_models(source)
            .await
            .map_err(|_| "unavailable")?
    } else {
        match cfg.provider {
            Provider::Databricks | Provider::DatabricksV2 => {
                lapis_agent::discover_databricks_models(&cfg)
                    .await
                    .map_err(|e| {
                        if matches!(e, AgentError::LlmAuth(_)) {
                            "authentication_required"
                        } else {
                            "unavailable"
                        }
                    })?
                    .into_iter()
                    .map(|m| json!({"modelId":m.id,"name":m.name}))
                    .collect()
            }
            _ => discover_http(&cfg).await?,
        }
    };
    let options = vec![
        json!({"id":"mode","name":"Execution mode","category":"mode","type":"select","currentValue":"ask","options":[{"value":"ask","name":"Ask before tools"}]}),
    ];
    Ok(
        json!({"models":{"currentModelId":cfg.model,"availableModels":bounded_models(models)?},"configOptions":options,"modes":{"currentModeId":"ask","availableModes":[{"id":"ask","name":"Ask before tools"}]}}),
    )
}

async fn discover_http(cfg: &Config) -> Result<Vec<Value>, &'static str> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "unavailable")?;
    let anthropic = matches!(cfg.provider, Provider::Anthropic);
    let base = cfg.base_url.trim_end_matches('/');
    let url = if anthropic && !base.ends_with("/v1") {
        format!("{base}/v1/models")
    } else {
        format!("{base}/models")
    };
    let mut after: Option<String> = None;
    let mut models = Vec::new();
    let mut total_bytes = 0;
    let mut cursors = BTreeSet::new();
    for _ in 0..20 {
        let mut request = client.get(&url);
        if anthropic {
            request = request
                .header("x-api-key", &cfg.api_key)
                .header("anthropic-version", &cfg.anthropic_api_version)
                .query(&[("limit", "100")]);
            if let Some(after) = &after {
                request = request.query(&[("after_id", after)]);
            }
        } else {
            request = request.bearer_auth(&cfg.api_key);
        }
        let mut response = request.send().await.map_err(|_| "unavailable")?;
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err("authentication_required");
        }
        if !response.status().is_success() {
            return Err("unavailable");
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "unavailable")? {
            total_bytes += chunk.len();
            if total_bytes > 4 * 1024 * 1024 {
                return Err("unavailable");
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| "unavailable")?;
        models.extend(normalize(&value, cfg.provider, base)?);
        if models.len() > 1024 {
            return Err("unavailable");
        }
        if !anthropic || value["has_more"] != true {
            return bounded_models(models);
        }
        let cursor = value["last_id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("unavailable")?
            .to_owned();
        if !cursors.insert(cursor.clone()) {
            return Err("unavailable");
        }
        after = Some(cursor);
    }
    Err("unavailable")
}
fn normalize(value: &Value, provider: Provider, base: &str) -> Result<Vec<Value>, &'static str> {
    let rows = value["data"].as_array().ok_or("unavailable")?;
    let official = matches!(provider, Provider::OpenAi)
        && reqwest::Url::parse(base)
            .ok()
            .is_some_and(|u| u.host_str() == Some("api.openai.com"));
    let mut models = Vec::new();
    for row in rows {
        let id = row["id"].as_str().ok_or("unavailable")?;
        if official
            && (!id.starts_with("gpt-")
                && !id.starts_with("chatgpt-")
                && !id.starts_with("o1")
                && !id.starts_with("o3")
                && !id.starts_with("o4")
                || [
                    "audio",
                    "realtime",
                    "transcribe",
                    "tts",
                    "image",
                    "embedding",
                    "search",
                ]
                .iter()
                .any(|kind| id.contains(kind)))
        {
            continue;
        }
        if matches!(provider, Provider::OpenRouter)
            && !row["supported_parameters"]
                .as_array()
                .is_some_and(|p| p.iter().any(|v| v == "tools"))
        {
            continue;
        }
        let name = row["display_name"]
            .as_str()
            .or_else(|| row["name"].as_str())
            .unwrap_or(id);
        models.push(json!({"modelId":id,"name":name}));
    }
    Ok(models)
}
fn bounded_models(models: Vec<Value>) -> Result<Vec<Value>, &'static str> {
    if models.len() > 1024 {
        return Err("unavailable");
    }
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for model in models {
        for key in ["modelId", "name"] {
            let value = model[key].as_str().ok_or("unavailable")?;
            if value.is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
                return Err("unavailable");
            }
        }
        if seen.insert(model["modelId"].as_str().unwrap().to_owned()) {
            result.push(model);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_filters_preserve_exact_ids_and_tools_capabilities() {
        let rows =
            json!({"data":[{"id":"gpt-5"},{"id":"text-embedding-3-large"},{"id":"gpt-audio"}]});
        assert_eq!(
            normalize(&rows, Provider::OpenAi, "https://api.openai.com/v1")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            normalize(&rows, Provider::OpenAi, "http://localhost/v1")
                .unwrap()
                .len(),
            3
        );
        let rows = json!({"data":[{"id":"vendor/model","supported_parameters":["tools"]},{"id":"no-tools"}]});
        assert_eq!(
            normalize(&rows, Provider::OpenRouter, "https://openrouter.ai/api/v1").unwrap()[0]
                ["modelId"],
            "vendor/model"
        );
    }
    #[test]
    fn catalog_deduplicates_and_rejects_unbounded_or_malformed_ids() {
        let model = json!({"modelId":"x","name":"X"});
        assert_eq!(
            bounded_models(vec![model.clone(), model.clone()])
                .unwrap()
                .len(),
            1
        );
        assert!(bounded_models(vec![model; 1025]).is_err());
        assert!(bounded_models(vec![json!({"modelId":"x\nsecret","name":"X"})]).is_err());
        assert_eq!(bounded_models(vec![]).unwrap().len(), 0);
    }
}
