//! Shared in-memory agent harness for the protocol integration tests.
//!
//! Every test here drives a real [`Connection`] over a `tokio::io::duplex` pipe.
//! There is no subprocess, no model provider, no network endpoint and no
//! credential anywhere in this suite, which is what makes it runnable in every
//! lane. Items unused by a particular test binary are allowed, because each
//! binary includes this module whole.
#![allow(dead_code)]

use lapis_acp_protocol::Connection;
use lapis_acp_protocol::budget::Limits;
use lapis_acp_protocol::session::{SessionNewRequest, SystemPrompt, SystemPromptTransport};
use serde_json::{Value, json};
use tokio::io::{
    AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, WriteHalf, duplex, split,
};

/// Byte capacity of the in-memory pipe standing in for an agent's stdio.
pub const CAPACITY: usize = 64 * 1024;

/// One client connection paired with the scripted agent side of the pipe.
pub struct Harness {
    pub connection: Connection<ReadHalf<DuplexStream>, WriteHalf<DuplexStream>>,
    pub agent: BufReader<DuplexStream>,
}

impl Harness {
    pub fn new() -> Self {
        Self::with_limits(Limits::default())
    }

    pub fn with_limits(limits: Limits) -> Self {
        let (client, agent) = duplex(CAPACITY);
        let (reader, writer) = split(client);
        Self {
            connection: Connection::with_limits(reader, writer, limits),
            agent: BufReader::new(agent),
        }
    }

    /// Read the next frame the client wrote.
    pub async fn frame(&mut self) -> Value {
        let mut line = String::new();
        let read = self.agent.read_line(&mut line).await.expect("agent read");
        assert!(read > 0, "client wrote nothing");
        serde_json::from_str(line.trim()).unwrap_or_else(|err| panic!("{err}: {line}"))
    }

    /// Queue a frame for the client to read.
    pub async fn send(&mut self, frame: Value) {
        let mut bytes = serde_json::to_vec(&frame).unwrap();
        bytes.push(b'\n');
        self.agent
            .get_mut()
            .write_all(&bytes)
            .await
            .expect("agent write");
        self.agent.get_mut().flush().await.expect("agent flush");
    }

    /// Queue raw bytes, so a test can send something that is not a valid frame.
    pub async fn send_line(&mut self, line: &str) {
        self.agent
            .get_mut()
            .write_all(line.as_bytes())
            .await
            .expect("agent write");
        self.agent.get_mut().flush().await.expect("agent flush");
    }
}

/// A session request carrying standing instructions and no MCP servers.
pub fn session_request() -> SessionNewRequest<'static> {
    SessionNewRequest {
        cwd: "/work",
        mcp_servers: vec![],
        system_prompt: Some(SystemPrompt {
            transport: SystemPromptTransport::Field,
            text: "standing",
        }),
        session_title: None,
    }
}

/// A JSON-RPC success response to a client request id.
pub fn response(id: u64, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}
