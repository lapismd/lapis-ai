//! Bounded stdio coding tools. The ACP agent must obtain permission before calling them.
mod files;
mod shell;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
fn tools() -> Value {
    let item = |name: &str, description: &str, properties: Value, required: &[&str], read: bool| json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":read,"destructiveHint":!read,"openWorldHint":name=="shell"}});
    json!([
        item(
            "read_file",
            "Read a UTF-8 file within this session workspace",
            json!({"path":{"type":"string"}}),
            &["path"],
            true
        ),
        item(
            "search",
            "Find literal text in workspace files",
            json!({"query":{"type":"string"}}),
            &["query"],
            true
        ),
        item(
            "write_file",
            "Write a UTF-8 file within this session workspace",
            json!({"path":{"type":"string"},"content":{"type":"string"}}),
            &["path", "content"],
            false
        ),
        item(
            "shell",
            "Run a command in the session workspace with a bounded deadline",
            json!({"command":{"type":"string"},"timeout_seconds":{"type":"integer","minimum":1,"maximum":120}}),
            &["command"],
            false
        )
    ])
}
#[tokio::main]
async fn main() {
    let Ok(root) = std::env::current_dir().and_then(|p| p.canonicalize()) else {
        return;
    };
    let mut input = BufReader::new(tokio::io::stdin());
    let mut output = tokio::io::stdout();
    let (sender, mut requests) = tokio::sync::mpsc::channel::<Vec<u8>>(1);
    let (shutdown, mut shutting) = tokio::sync::watch::channel(false);
    let closed = shutdown.clone();
    tokio::spawn(async move {
        loop {
            let mut line = Vec::new();
            let read = tokio::io::AsyncReadExt::take(&mut input, 524289)
                .read_until(b'\n', &mut line)
                .await;
            if !matches!(read,Ok(n) if n>0) || line.len() > 524288 || line.last() != Some(&b'\n') {
                break;
            }
            if sender.send(line).await.is_err() {
                break;
            }
        }
        closed.send_replace(true);
    });
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            let Ok(mut term) =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            else {
                return;
            };
            tokio::select! {_=term.recv()=>{},_=tokio::signal::ctrl_c()=>{}};
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        shutdown.send_replace(true);
    });
    loop {
        let line = tokio::select! {line=requests.recv()=>{let Some(line)=line else{break;};line},_=shutting.changed()=>break};
        let Ok(request) = serde_json::from_slice::<Value>(&line) else {
            break;
        };
        let Some(id) = request.get("id") else {
            continue;
        };
        let method = request["method"].as_str().unwrap_or("");
        let result = match method {
            "initialize" => {
                json!({"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"lapis-agent-tools","version":"0.1.0"}})
            }
            "ping" => json!({}),
            "tools/list" => json!({"tools":tools()}),
            "tools/call" => {
                let params = &request["params"];
                let args = &params["arguments"];
                let result = if params["name"] == "shell" {
                    tokio::select! {result=shell::run(&root,args)=>result,_=shutting.changed()=>break}
                } else {
                    files::call(&root, params["name"].as_str().unwrap_or(""), args)
                };
                match result {
                    Ok(text) => json!({"content":[{"type":"text","text":text}]}),
                    Err(code) => json!({"isError":true,"content":[{"type":"text","text":code}]}),
                }
            }
            _ => {
                let frame = json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"unsupported method"}});
                let _ = output.write_all(format!("{frame}\n").as_bytes()).await;
                continue;
            }
        };
        let response = json!({"jsonrpc":"2.0","id":id,"result":result});
        if output
            .write_all(format!("{response}\n").as_bytes())
            .await
            .is_err()
            || output.flush().await.is_err()
        {
            break;
        }
    }
}
