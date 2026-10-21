//! Transient raw local-session reads. Hosts must admit sources and sanitize before display.
mod jsonl;
#[cfg(feature = "opencode-sqlite")]
mod opencode;
#[cfg(feature = "opencode-sqlite")]
mod sqlite_snapshot;
pub use jsonl::{JsonlPage, JsonlSnapshot};
#[cfg(feature = "opencode-sqlite")]
pub use opencode::{OpenCodeDatabase, SessionMetadata};
#[cfg(feature = "opencode-sqlite")]
pub use sqlite_snapshot::{OpenCodePage, OpenCodeSnapshot};
