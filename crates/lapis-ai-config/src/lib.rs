//! Versioned, inert TOML configuration and explicit native launch preparation.
#![forbid(unsafe_code)]
mod compute_failure;
mod compute_inventory;
mod compute_model_name;
mod environment;
pub use compute_failure::ComputeFailure;
pub use compute_inventory::{ComputeInventory, ComputeModel};
pub mod launch;
mod validation;
pub use environment::{Environment, Resolver, SystemResolver};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub runtime: Runtime,
    #[serde(default)]
    pub acp: Acp,
    #[serde(default)]
    pub agent: Agent,
    #[serde(default)]
    pub compute: Compute,
    #[serde(skip)]
    pub directory: PathBuf,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Runtime {
    pub data_dir: Option<PathBuf>,
    pub acp: Option<PathBuf>,
    pub agent: Option<PathBuf>,
    pub compute: Option<PathBuf>,
    pub tools: Option<PathBuf>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Acp {
    pub runner: Option<String>,
    pub cwd: Option<PathBuf>,
    pub catalog: Option<PathBuf>,
    pub tools: Option<PathBuf>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub mode: Option<String>,
    pub control_version: Option<u64>,
    pub idle_timeout: Option<u64>,
    pub max_turn_duration: Option<u64>,
    pub cancel_grace: Option<u64>,
    pub reap_timeout: Option<u64>,
    pub workers: Option<usize>,
    pub queue_capacity: Option<usize>,
    pub system_prompt_file: Option<PathBuf>,
    pub environment: BTreeMap<String, EnvValue>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Agent {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub thinking_effort: Option<String>,
    pub environment: BTreeMap<String, EnvValue>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Compute {
    pub model: Option<String>,
    pub memory_gb: Option<u64>,
    pub sharing: bool,
    pub peers: BTreeMap<String, serde_json::Value>,
    pub relay_urls: Vec<String>,
    pub port: Option<u16>,
    pub identity: Option<SecretRef>,
    pub token: Option<SecretRef>,
    pub native_runtime: Option<PathBuf>,
    pub environment: BTreeMap<String, EnvValue>,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum EnvValue {
    Reference(SecretRef),
    Value(String),
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SecretRef {
    pub env: Option<String>,
    pub host: Option<String>,
}
impl std::fmt::Debug for EnvValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}
impl std::fmt::Debug for SecretRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretRef([redacted])")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Component {
    Acp,
    Agent,
    Compute,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(pub &'static str);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}
impl Config {
    pub fn parse(text: &str, directory: &Path) -> Result<Self, Error> {
        if text.len() > 1024 * 1024 {
            return Err(Error("configuration_limit"));
        }
        let mut config: Self =
            toml::from_str(text).map_err(|_| Error("invalid_toml_configuration"))?;
        config.directory = directory.to_path_buf();
        config.validate()?;
        config.resolve_paths();
        Ok(config)
    }
    pub fn load(path: &Path) -> Result<Self, Error> {
        let path = path
            .canonicalize()
            .map_err(|_| Error("configuration_unavailable"))?;
        let size = std::fs::metadata(&path)
            .map_err(|_| Error("configuration_unavailable"))?
            .len();
        if size > 1024 * 1024 {
            return Err(Error("configuration_limit"));
        }
        let text =
            std::fs::read_to_string(&path).map_err(|_| Error("configuration_unavailable"))?;
        Self::parse(&text, path.parent().ok_or(Error("configuration_path"))?)
    }
    pub fn executable(&self, component: Component) -> Option<&Path> {
        match component {
            Component::Acp => self.runtime.acp.as_deref(),
            Component::Agent => self.runtime.agent.as_deref(),
            Component::Compute => self.runtime.compute.as_deref(),
        }
    }
    pub fn compute_control(&self) -> serde_json::Value {
        serde_json::json!({"model":self.compute.model,"memory_gb":self.compute.memory_gb,"sharing":self.compute.sharing,"peers":self.compute.peers,"relay_urls":self.compute.relay_urls})
    }
}
