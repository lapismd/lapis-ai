//! Agent Client Protocol wire contract.
//!
//! Owns the JSON-RPC framing, request/response correlation, capability
//! negotiation and session methods that `lapis-acp` speaks to an ACP agent
//! subprocess over stdio. This crate is the ported, decomposed derivative of
//! the upstream `buzz-acp` ACP client; the pin and per-region port map live in
//! `docs/upstream-lock.md`.
//!
//! The connection is generic over a byte stream pair rather than owning a child
//! process, so every behaviour here is testable against an in-memory pipe with
//! no subprocess, no model provider and no credential. Process ownership,
//! scheduling, credentials and transport policy all live above this crate.
//!
//! ```no_run
//! use lapis_acp_protocol::{Connection, ClientInfo, ClientCapabilities, RecordingHandler};
//!
//! # async fn demo(agent_out: impl tokio::io::AsyncRead + Unpin,
//! #               agent_in: impl tokio::io::AsyncWrite + Unpin)
//! #   -> Result<(), lapis_acp_protocol::Error> {
//! let mut handler = RecordingHandler::default();
//! let mut connection = Connection::new(agent_out, agent_in);
//! let negotiated = connection
//!     .initialize(&mut handler, &ClientInfo::lapis_acp(), ClientCapabilities::default())
//!     .await?;
//! assert!(negotiated.protocol_version >= 1);
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

pub mod budget;
pub mod client;
pub mod config_option;
pub mod content;
pub mod envelope;
pub mod error;
pub mod frame;
pub mod handler;
pub mod initialize;
pub mod permission;
pub mod session;
pub mod stop_reason;
pub mod thinking;
pub mod update;

pub use budget::{Limits, TurnBudget};
pub use client::Connection;
pub use config_option::{
    EffortResolution, ModeResolution, ModelSwitch, available_modes, model_in_catalog,
    model_options, model_state, offers_model, option_in_category, option_values, resolve_effort,
    resolve_mode, resolve_model_switch, thought_level_config_id,
};
pub use content::{EnvVar, McpServer, text_block, text_blocks};
pub use envelope::Inbound;
pub use error::Error;
pub use frame::{DEFAULT_MAX_FRAME_BYTES, FrameReader, FrameWriter};
pub use handler::{RecordingHandler, SessionHandler};
pub use initialize::{
    ClientCapabilities, ClientInfo, InitializeResult, PROTOCOL_VERSION, SUPPORTED_VERSIONS,
};
pub use permission::{PermissionDecision, PermissionOption, PermissionRequest};
pub use session::{Session, SessionNewRequest, SystemPrompt, SystemPromptTransport};
pub use stop_reason::StopReason;
pub use thinking::{
    EFFORT_KEYS, Thinking, advertised_effort, advertises_fast, badges, effort_matches,
    model_id_attrs, model_id_base,
};
pub use update::SessionUpdate;

/// The result type used throughout this crate.
pub type Result<T> = std::result::Result<T, Error>;
