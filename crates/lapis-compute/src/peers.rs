//! Explicit paired-machine admission over authenticated encrypted QUIC.
use crate::{Config, Shared};
use iroh::{Endpoint, EndpointAddr, RelayMode, SecretKey};
use serde_json::{Value, json};
use std::sync::Arc;
const ALPN: &[u8] = b"jj-store/compute/1";
pub async fn bind(secret: &str) -> anyhow::Result<Endpoint> {
    let bytes: [u8; 32] = hex::decode(secret)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid identity"))?;
    let relays = std::env::var("JJ_STORE_COMPUTE_RELAYS").unwrap_or_default();
    let mode = if relays.is_empty() {
        RelayMode::Disabled
    } else {
        RelayMode::custom(
            relays
                .split(',')
                .map(str::parse)
                .collect::<Result<Vec<_>, _>>()?,
        )
    };
    let port = std::env::var("JJ_STORE_COMPUTE_PORT")
        .unwrap_or_else(|_| "0".into())
        .parse::<u16>()?;
    Ok(Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(SecretKey::from_bytes(&bytes))
        .alpns(vec![ALPN.to_vec()])
        .relay_mode(mode)
        .bind_addr(std::net::SocketAddr::from(([0, 0, 0, 0], port)))?
        .bind()
        .await?)
}
pub fn invitation(endpoint: &Endpoint) -> Value {
    serde_json::to_value(endpoint.addr()).unwrap_or(Value::Null)
}
pub async fn revoke_connections(shared: &Shared) {
    let config = shared.config.read().await;
    let mut connections = shared.connections.lock().await;
    connections.retain(|id, connection| {
        let keep = config.sharing && config.peers.contains_key(id);
        if !keep {
            connection.close(1u32.into(), b"sharing disabled or peer revoked");
        }
        keep
    });
}
pub fn validate_config(config: &Config) -> anyhow::Result<()> {
    anyhow::ensure!(config.peers.len() <= 16, "too many peers");
    for (id, value) in &config.peers {
        let address: EndpointAddr = serde_json::from_value(value.clone())?;
        anyhow::ensure!(
            address.id.to_string() == *id && !address.addrs.is_empty() && address.addrs.len() <= 16,
            "invalid invitation"
        );
    }
    Ok(())
}
pub async fn listen(shared: Arc<Shared>) {
    while let Some(incoming) = shared.endpoint.accept().await {
        let Ok(permit) = shared.peer_slots.clone().try_acquire_owned() else {
            incoming.refuse();
            continue;
        };
        let shared = shared.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let Ok(Ok(connection)) =
                tokio::time::timeout(std::time::Duration::from_secs(10), incoming).await
            else {
                return;
            };
            let id = connection.remote_id().to_string();
            {
                let config = shared.config.read().await;
                if !config.sharing || !config.peers.contains_key(&id) {
                    connection.close(1u32.into(), b"unpaired");
                    return;
                }
            }
            {
                let mut connections = shared.connections.lock().await;
                if let Some(old) = connections.insert(id.clone(), connection.clone()) {
                    old.close(1u32.into(), b"replaced");
                }
            }
            while let Ok(Ok((mut send, mut recv))) =
                tokio::time::timeout(std::time::Duration::from_secs(60), connection.accept_bi())
                    .await
            {
                let read = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    recv.read_to_end(263168),
                )
                .await;
                let Ok(Ok(bytes)) = read else { break };
                let Ok(request) = serde_json::from_slice::<Value>(&bytes) else {
                    break;
                };
                {
                    let config = shared.config.read().await;
                    if !config.sharing || !config.peers.contains_key(&id) {
                        break;
                    }
                }
                let path = request["path"].as_str().unwrap_or("");
                let body = request["body"].to_string();
                let result = tokio::select! {
                    result=tokio::time::timeout(std::time::Duration::from_secs(300),crate::gateway::infer(&shared,path,body.as_bytes()))=>result,
                    _=connection.closed()=>break,
                };
                let result = match result {
                    Ok(Ok((status, body))) => json!({"status":status,"body":body}),
                    _ => json!({"status":503,"body":"inference unavailable"}),
                };
                {
                    let config = shared.config.read().await;
                    if !config.sharing || !config.peers.contains_key(&id) {
                        break;
                    }
                }
                let bytes = result.to_string();
                if send.write_all(bytes.as_bytes()).await.is_err() {
                    break;
                }
                let _ = send.finish();
            }
            connection.close(0u32.into(), b"closed");
            let mut connections = shared.connections.lock().await;
            if connections
                .get(&id)
                .is_some_and(|c| c.stable_id() == connection.stable_id())
            {
                connections.remove(&id);
            }
        });
    }
}
pub async fn infer(
    shared: &Shared,
    id: &str,
    path: &str,
    body: &[u8],
) -> anyhow::Result<(u16, String)> {
    let peer = shared
        .config
        .read()
        .await
        .peers
        .get(id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("unpaired"))?;
    let address: EndpointAddr = serde_json::from_value(peer)?;
    anyhow::ensure!(address.id.to_string() == id, "identity mismatch");
    let _permit = shared
        .slots
        .try_acquire()
        .map_err(|_| anyhow::anyhow!("compute busy"))?;
    let connection = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        shared.endpoint.connect(address, ALPN),
    )
    .await??;
    let (mut send, mut recv) = connection.open_bi().await?;
    let request = json!({"path":path,"body":serde_json::from_slice::<Value>(body)?}).to_string();
    send.write_all(request.as_bytes()).await?;
    send.finish()?;
    let bytes = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        recv.read_to_end(2 * 1024 * 1024 + 4096),
    )
    .await??;
    connection.close(0u32.into(), b"complete");
    anyhow::ensure!(
        shared.config.read().await.peers.contains_key(id),
        "peer revoked"
    );
    let response: Value = serde_json::from_slice(&bytes)?;
    Ok((
        response["status"].as_u64().unwrap_or(503) as u16,
        response["body"]
            .as_str()
            .unwrap_or("inference unavailable")
            .to_owned(),
    ))
}
