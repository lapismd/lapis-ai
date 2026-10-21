//! Owned compute process: JSON control over stdio, authenticated inference over loopback.
mod engine;
mod failure;
mod gateway;
mod models;
mod peers;
mod progress;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::{Mutex, RwLock, Semaphore},
};
#[derive(Clone, Deserialize, Serialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub model: Option<String>,
    pub memory_gb: Option<u64>,
    pub sharing: bool,
    pub peers: BTreeMap<String, Value>,
    pub relay_urls: Vec<String>,
}
pub struct Shared {
    pub config: RwLock<Config>,
    pub engine: Mutex<Option<mesh_llm_sdk::EmbeddedNodeHandle>>,
    pub source: RwLock<Option<String>>,
    pub source_model: RwLock<Option<String>>,
    pub endpoint: iroh::Endpoint,
    pub connections: Mutex<BTreeMap<String, iroh::endpoint::Connection>>,
    pub slots: Semaphore,
    pub peer_slots: Arc<Semaphore>,
    pub token: String,
    pub client: reqwest::Client,
}
pub fn emit(value: Value) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}
fn main() {
    if let Some(code) = lapis_ai_config::launch::cli(lapis_ai_config::Component::Compute) {
        std::process::exit(code);
    }
    let thread = std::thread::Builder::new()
        .name("compute".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_stack_size(16 * 1024 * 1024)
                .build()
                .unwrap()
                .block_on(run())
        });
    if !matches!(
        thread.and_then(|t| t
            .join()
            .map_err(|_| std::io::Error::other("compute stopped"))),
        Ok(Ok(()))
    ) {
        emit(json!({"type":"error","code":"compute_unavailable"}));
        std::process::exit(1);
    }
}
async fn run() -> anyhow::Result<()> {
    progress::install();
    let token = std::env::var("JJ_STORE_COMPUTE_TOKEN")?;
    let key = std::env::var("JJ_STORE_COMPUTE_IDENTITY")?;
    let config = match std::env::var("LAPIS_AI_COMPUTE_CONFIG") {
        Ok(text) => serde_json::from_str(&text)?,
        Err(_) => Config::default(),
    };
    peers::validate_config(&config)?;
    let endpoint = peers::bind(&key).await?;
    let shared = Arc::new(Shared {
        config: RwLock::new(config),
        engine: Mutex::new(None),
        source: RwLock::new(None),
        source_model: RwLock::new(None),
        endpoint,
        connections: Mutex::new(BTreeMap::new()),
        slots: Semaphore::new(2),
        peer_slots: Arc::new(Semaphore::new(8)),
        token,
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()?,
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/v1", listener.local_addr()?);
    let server = tokio::spawn(axum::serve(listener, gateway::router(shared.clone())).into_future());
    let peer_task = tokio::spawn(peers::listen(shared.clone()));
    emit(
        json!({"type":"ready","endpoint":origin,"identity":shared.endpoint.id().to_string(),"invitation":peers::invitation(&shared.endpoint)}),
    );
    let mut input = BufReader::new(tokio::io::stdin());
    let mut operation: Option<tokio::task::JoinHandle<()>> = None;
    loop {
        let mut line = Vec::new();
        let mut bounded = tokio::io::AsyncReadExt::take(&mut input, 65537);
        let next = tokio::select! { next=bounded.read_until(b'\n',&mut line)=>next?, _=tokio::signal::ctrl_c()=>0 };
        if next == 0 || line.len() > 65536 || line.last() != Some(&b'\n') {
            break;
        }
        let request: Value = serde_json::from_slice(&line)?;
        let id = request["requestId"].as_str().unwrap_or("").to_owned();
        match request["action"].as_str().unwrap_or("") {
            "configure" => {
                let config: Config = serde_json::from_value(request["config"].clone())?;
                if operation.as_ref().is_some_and(|task| !task.is_finished()) {
                    emit(json!({"type":"error","requestId":id,"code":"compute_busy"}));
                    continue;
                }
                peers::validate_config(&config)?;
                *shared.config.write().await = config;
                peers::revoke_connections(&shared).await;
                emit(json!({"type":"configured","requestId":id}));
            }
            "prepare" | "download" | "start" | "import" | "remove" => {
                if operation.as_ref().is_some_and(|task| !task.is_finished()) {
                    emit(json!({"type":"error","requestId":id,"code":"compute_busy"}));
                    continue;
                }
                let path = request["path"].as_str().unwrap_or("").to_owned();
                let model = request["model"].as_str().unwrap_or("").to_owned();
                let shared = shared.clone();
                let action = request["action"].as_str().unwrap().to_owned();
                operation = Some(tokio::spawn(async move {
                    progress::begin(&id, &action);
                    let result = match action.as_str() {
                        "prepare" => engine::prepare().await,
                        "download" => engine::download(&shared).await,
                        "import" => models::import(&path).await.map(|model| {
                            emit(json!({"type":"imported","requestId":id,"model":model}));
                        }),
                        "remove" => {
                            if shared.engine.lock().await.is_some() {
                                Err(anyhow::anyhow!("model is running"))
                            } else {
                                models::remove(&model).await
                            }
                        }
                        _ => engine::start(&shared).await,
                    };
                    progress::end();
                    emit(match result {
                        Ok(()) => {
                            json!({"type":"completed","requestId":id,"action":action,"model":model})
                        }
                        Err(error) => {
                            let failure = failure::classify(&action, &error);
                            json!({"type":"error","requestId":id,"code":failure.code,"message":failure.message})
                        }
                    });
                }));
            }
            "stop" => break,
            _ => emit(json!({"type":"error","requestId":id,"code":"invalid_compute_action"})),
        }
    }
    if let Some(task) = operation {
        task.abort();
    }
    if let Some(engine) = shared.engine.lock().await.take() {
        let _ = tokio::time::timeout(Duration::from_secs(12), engine.stop()).await;
    }
    shared.endpoint.close().await;
    server.abort();
    peer_task.abort();
    Ok(())
}
use std::future::IntoFuture;

#[cfg(test)]
mod tests;
