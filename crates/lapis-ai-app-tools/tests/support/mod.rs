use lapis_ai_app_tools::{
    Config, Error, Tool, ToolCall, ToolError, ToolFuture, ToolHandler, serve,
};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, WriteHalf},
    sync::{Notify, Semaphore, watch},
    task::JoinHandle,
};

pub struct Handler {
    pub tools: Mutex<Vec<Tool>>,
    pub calls: Mutex<Vec<Value>>,
    pub blocked: AtomicBool,
    pub active: AtomicUsize,
    pub dropped: AtomicUsize,
    pub cancellation_seen: AtomicBool,
    pub entered: Notify,
    pub release: Semaphore,
}
impl Default for Handler {
    fn default() -> Self {
        Self {
            tools: Mutex::new(vec![Tool {
                name: "notes.read".into(),
                description: "Read a scoped note".into(),
                input_schema: json!({"type":"object"}),
            }]),
            calls: Mutex::new(vec![]),
            blocked: AtomicBool::new(false),
            active: AtomicUsize::new(0),
            dropped: AtomicUsize::new(0),
            cancellation_seen: AtomicBool::new(false),
            entered: Notify::new(),
            release: Semaphore::new(0),
        }
    }
}
struct Guard<'a>(&'a Handler, watch::Receiver<bool>);
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
        self.0.dropped.fetch_add(1, Ordering::SeqCst);
        self.0
            .cancellation_seen
            .store(*self.1.borrow(), Ordering::SeqCst);
    }
}
impl ToolHandler for Handler {
    fn tools(&self) -> Vec<Tool> {
        self.tools.lock().unwrap().clone()
    }
    fn call(&self, call: ToolCall, cancelled: watch::Receiver<bool>) -> ToolFuture<'_> {
        Box::pin(async move {
            self.active.fetch_add(1, Ordering::SeqCst);
            let _guard = Guard(self, cancelled);
            self.calls
                .lock()
                .unwrap()
                .push(json!({"id":call.request_id,"name":call.name,"arguments":call.arguments}));
            self.entered.notify_one();
            if self.blocked.load(Ordering::SeqCst) {
                self.release.acquire().await.unwrap().forget();
            }
            match call.arguments["error"].as_str() {
                Some("unknown") => return Err(ToolError::Unknown),
                Some("invalid") => return Err(ToolError::InvalidInput),
                Some("unavailable") => return Err(ToolError::Unavailable),
                _ => {}
            }
            if let Some(bytes) = call.arguments["bytes"].as_u64() {
                return Ok(json!({"content":[{"type":"text","text":"x".repeat(bytes as usize)}]}));
            }
            Ok(
                json!({"content":[{"type":"text","text":"Captured note"}],"structuredContent":call.arguments}),
            )
        })
    }
}
pub struct Session {
    pub input: WriteHalf<DuplexStream>,
    output: BufReader<ReadHalf<DuplexStream>>,
    pub task: JoinHandle<Result<(), Error>>,
    pub handler: Arc<Handler>,
}
impl Session {
    pub fn new(config: Config) -> Self {
        let handler = Arc::new(Handler::default());
        let (client, server) = tokio::io::duplex(65_536);
        let (reader, writer) = tokio::io::split(server);
        let (output, input) = tokio::io::split(client);
        let task = tokio::spawn(serve(reader, writer, handler.clone(), config));
        Self {
            input,
            output: BufReader::new(output),
            task,
            handler,
        }
    }
    pub async fn raw(&mut self, bytes: &[u8]) {
        self.input.write_all(bytes).await.unwrap();
    }
    pub async fn send(&mut self, id: Value, method: &str, params: Value) {
        let mut bytes =
            serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
                .unwrap();
        bytes.push(b'\n');
        self.raw(&bytes).await;
    }
    pub async fn notify(&mut self, method: &str, params: Value) {
        let mut bytes =
            serde_json::to_vec(&json!({"jsonrpc":"2.0","method":method,"params":params})).unwrap();
        bytes.push(b'\n');
        self.raw(&bytes).await;
    }
    pub async fn next(&mut self) -> Value {
        let mut line = String::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.output.read_line(&mut line),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(!line.is_empty(), "bridge closed without a response");
        serde_json::from_str(&line).unwrap()
    }
    pub async fn initialize(&mut self) {
        self.send(
            json!("init"),
            "initialize",
            json!({"protocolVersion":"2025-06-18"}),
        )
        .await;
        assert_eq!(
            self.next().await["result"]["capabilities"]["tools"]["listChanged"],
            false
        );
    }
    pub async fn stop(mut self) -> Result<(), Error> {
        self.input.shutdown().await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), self.task)
            .await
            .unwrap()
            .unwrap()
    }
}
