//! Application-owned native helper clients. No durable orchestration or UI policy.
#![forbid(unsafe_code)]
mod acp;
mod agent;
mod compute;
mod launch;
mod process;
mod resources;
pub use acp::{AcpClient, AcpCommands};
pub use agent::AgentClient;
pub use compute::ComputeClient;
pub use lapis_ai_config::{Component, Config, Environment, Resolver};
pub use launch::PreparedLaunch;
pub use process::{NativeProcess, Sender};
pub use resources::ResourceBundle;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(pub &'static str);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}
impl From<lapis_ai_config::Error> for Error {
    fn from(e: lapis_ai_config::Error) -> Self {
        Self(e.0)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
