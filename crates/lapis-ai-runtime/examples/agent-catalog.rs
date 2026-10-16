//! Native discovery and tool-free metadata through the reusable Rust client.
use lapis_ai_config::SystemResolver;
use lapis_ai_runtime::{AgentClient, Config};
use serde_json::json;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(std::path::Path::new(
        &std::env::args().nth(1).ok_or("configuration required")?,
    ))?;
    let models = AgentClient::models(&config, &SystemResolver).await?;
    let metadata=AgentClient::metadata(&config,&SystemResolver,json!({"version":1,"transcript":"User: Add a notes command. Assistant: Added the command."})).await?;
    assert_eq!(models["status"], "ready");
    assert!(metadata["title"].is_string() && metadata["summary"].is_string());
    println!("agent Rust discovery and metadata: passed");
    Ok(())
}
