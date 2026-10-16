//! Independent TOML-configured compute consumer, with explicit model lifecycle.
use lapis_ai_config::SystemResolver;
use lapis_ai_runtime::{ComputeClient, Config, ResourceBundle};
use serde_json::json;
use std::path::Path;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut config = Config::load(Path::new(args.get(1).ok_or("configuration required")?))?;
    if let Some(index) = args.iter().position(|arg| arg == "--bundle") {
        ResourceBundle::load(Path::new(args.get(index + 1).ok_or("bundle required")?))?
            .configure(&mut config);
    }
    let mut client = ComputeClient::start(&config, &SystemResolver).await?;
    client
        .prepare("prepare")
        .await
        .map_err(|error| format!("prepare: {error}"))?;
    client
        .download("download")
        .await
        .map_err(|error| format!("download: {error}"))?;
    client
        .start_model("start")
        .await
        .map_err(|error| format!("start: {error}"))?;
    let result = client.infer(false, json!({"model":"auto","messages":[{"role":"user","content":"Reply READY."}],"stream":false,"max_tokens":32,"chat_template_kwargs":{"enable_thinking":false}})).await?;
    let valid = result["choices"][0]["message"]["content"]
        .as_str()
        .is_some_and(|value| !value.is_empty());
    client.shutdown().await?;
    if !valid {
        return Err("empty native inference".into());
    }
    println!("native Rust compute client: passed");
    Ok(())
}
