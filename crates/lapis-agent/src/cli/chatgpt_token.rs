//! Application-owned account-bound credential IPC; never opens a browser.
use lapis_agent::{auth::TokenSource, AgentError};
use serde_json::{json, Value};
use std::sync::Arc;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
pub fn source() -> Result<Option<Arc<dyn TokenSource>>, AgentError> {
    if std::env::var("BUZZ_AGENT_PROVIDER").ok().as_deref() != Some("chatgpt-plan") {
        return Ok(None);
    }
    let path = std::env::var_os("LAPIS_AGENT_TOKEN_SOCKET")
        .or_else(|| std::env::var_os("JJ_STORE_CHATGPT_SOCKET"))
        .ok_or_else(error)?;
    let capability = std::env::var("LAPIS_AGENT_TOKEN_CAPABILITY")
        .or_else(|_| std::env::var("JJ_STORE_CHATGPT_BROKER_TOKEN"))
        .map_err(|_| error())?;
    if !(32..=128).contains(&capability.len()) {
        return Err(error());
    }
    Ok(Some(Arc::new(Broker {
        path: path.into(),
        capability,
    })))
}
#[cfg(unix)]
struct Broker {
    path: std::path::PathBuf,
    capability: String,
}
fn error() -> AgentError {
    AgentError::LlmAuth("chatgpt_authentication_required".into())
}
#[cfg(unix)]
impl Broker {
    async fn acquire(&self, rejected: Option<&str>) -> Result<String, AgentError> {
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let mut stream = tokio::net::UnixStream::connect(&self.path)
                .await
                .map_err(|_| error())?;
            let mut bytes =
                serde_json::to_vec(&json!({"capability":self.capability,"rejected":rejected}))
                    .map_err(|_| error())?;
            bytes.push(b'\n');
            stream.write_all(&bytes).await.map_err(|_| error())?;
            let mut line = String::new();
            BufReader::new(stream)
                .take(65537)
                .read_line(&mut line)
                .await
                .map_err(|_| error())?;
            if line.len() > 65536 || !line.ends_with('\n') {
                return Err(error());
            }
            let value: Value = serde_json::from_str(&line).map_err(|_| error())?;
            value["access_token"]
                .as_str()
                .filter(|s| (8..=16384).contains(&s.len()))
                .map(str::to_owned)
                .ok_or_else(error)
        })
        .await
        .map_err(|_| error())?
    }
}
#[cfg(unix)]
#[async_trait::async_trait]
impl TokenSource for Broker {
    async fn bearer(&self) -> Result<String, AgentError> {
        self.acquire(None).await
    }
    async fn bearer_no_browser(&self) -> Result<String, AgentError> {
        self.acquire(None).await
    }
    async fn refresh_now(&self, rejected: &str) -> Result<String, AgentError> {
        self.acquire(Some(rejected)).await
    }
}

#[cfg(not(unix))]
pub fn source() -> Result<Option<Arc<dyn TokenSource>>, AgentError> {
    if std::env::var("BUZZ_AGENT_PROVIDER").ok().as_deref() == Some("chatgpt-plan") {
        Err(error())
    } else {
        Ok(None)
    }
}
