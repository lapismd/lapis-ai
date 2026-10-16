//! Independent Rust consumer: configure, warm, prompt and close an ACP helper.
use lapis_ai_config::SystemResolver;
use lapis_ai_runtime::{AcpClient, Config, ResourceBundle};
use std::{path::Path, time::Duration};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: native-session CONFIG.toml")?;
    let mut config = Config::load(Path::new(&path))?;
    let args: Vec<String> = std::env::args().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--bundle") {
        ResourceBundle::load(Path::new(
            args.get(index + 1).ok_or("bundle path required")?,
        ))?
        .configure(&mut config);
    }
    let mut client = AcpClient::start(&config, &SystemResolver).await?;
    let commands = client.commands();
    commands.warm("warm").await?;
    loop {
        let frame = client.next(Duration::from_secs(30)).await?;
        if frame["type"] == "warmed" {
            break;
        }
        if frame["type"] == "error" {
            return Err("warm failed".into());
        }
    }
    let cwd = config.acp.cwd.as_ref().ok_or("acp.cwd required")?;
    commands.open("open", "example", cwd, None).await?;
    loop {
        let frame = client.next(Duration::from_secs(30)).await?;
        if frame["type"] == "opened" {
            break;
        }
        if frame["type"] == "error" {
            return Err("open failed".into());
        }
    }
    commands
        .configure("configure", "example", serde_json::json!({"mode":"ask"}))
        .await?;
    loop {
        let frame = client.next(Duration::from_secs(30)).await?;
        if frame["type"] == "configured" && frame["modeReady"] == true {
            break;
        }
        if frame["type"] == "error" {
            return Err("configure failed".into());
        }
    }
    commands
        .prompt(
            "prompt",
            "example",
            "Reply READY",
            None,
            Duration::from_secs(180),
        )
        .await?;
    loop {
        let frame = client.next(Duration::from_secs(180)).await?;
        if frame["type"] == "permission" {
            let approve = std::env::args().any(|arg| arg == "--approve-tools");
            let options = frame
                .get("options")
                .or_else(|| frame.get("request").and_then(|v| v.get("options")))
                .and_then(serde_json::Value::as_array);
            let choice = options
                .and_then(|options| {
                    options.iter().find(|option| {
                        option["kind"] == if approve { "allow_once" } else { "reject_once" }
                    })
                })
                .and_then(|option| option["optionId"].as_str());
            commands
                .permission(
                    "decision",
                    "example",
                    frame["permissionId"]
                        .as_str()
                        .ok_or("missing permission identity")?,
                    choice,
                )
                .await?;
        }
        if frame["type"] == "record" {
            println!("{}", frame["record"]);
            break;
        }
        if frame["type"] == "error" {
            return Err("prompt failed".into());
        }
    }
    commands.close("close").await?;
    client.shutdown().await?;
    Ok(())
}
