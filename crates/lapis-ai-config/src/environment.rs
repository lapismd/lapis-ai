use crate::{Component, Config, EnvValue, Error, SecretRef};
use std::{collections::BTreeMap, path::Path};
pub trait Resolver: Send + Sync {
    fn environment(&self, name: &str) -> Option<String>;
    fn host_secret(&self, _name: &str) -> Option<String> {
        None
    }
}
pub struct SystemResolver;
impl Resolver for SystemResolver {
    fn environment(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}
#[derive(Default)]
pub struct Environment(pub BTreeMap<String, String>);
impl std::fmt::Debug for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Environment")
            .field("keys", &self.0.keys().collect::<Vec<_>>())
            .finish()
    }
}
impl SecretRef {
    pub fn resolve(&self, resolver: &dyn Resolver) -> Result<String, Error> {
        let value = match (&self.env, &self.host) {
            (Some(name), None) => resolver.environment(name),
            (None, Some(name)) => resolver.host_secret(name),
            _ => None,
        }
        .filter(|v| !v.is_empty())
        .ok_or(Error("credential_reference_unavailable"))?;
        if value.len() > 65536 || value.contains('\0') {
            return Err(Error("invalid_credential_value"));
        }
        Ok(value)
    }
}
impl Config {
    pub fn environment(
        &self,
        component: Component,
        resolver: &dyn Resolver,
    ) -> Result<Environment, Error> {
        self.validate()?;
        let mut result = if component == Component::Acp
            && matches!(self.acp.runner.as_deref(), Some("local" | "lapis-agent"))
        {
            self.environment(Component::Agent, resolver)?.0
        } else {
            BTreeMap::new()
        };
        let map = match component {
            Component::Acp => &self.acp.environment,
            Component::Agent => &self.agent.environment,
            Component::Compute => &self.compute.environment,
        };
        for (key, value) in map {
            let value = match value {
                EnvValue::Value(v) => v.clone(),
                EnvValue::Reference(v) => v.resolve(resolver)?,
            };
            if value.contains('\0') {
                return Err(Error("invalid_environment_value"));
            }
            result.insert(key.clone(), value);
        }
        let mut put = |name: &str, value: Option<String>| {
            if let Some(v) = value {
                result.insert(name.into(), v);
            }
        };
        let path = |v: Option<&Path>| v.map(|p| p.to_string_lossy().into_owned());
        match component {
            Component::Acp => {
                for (name, value) in [
                    ("RUNNER", self.acp.runner.clone()),
                    ("MODEL", self.acp.model.clone()),
                    ("EFFORT", self.acp.effort.clone()),
                    ("MODE", self.acp.mode.clone()),
                    ("CWD", path(self.acp.cwd.as_deref())),
                    ("CATALOG", path(self.acp.catalog.as_deref())),
                    ("TOOLS", path(self.acp.tools.as_deref())),
                    (
                        "SYSTEM_PROMPT_FILE",
                        path(self.acp.system_prompt_file.as_deref()),
                    ),
                ] {
                    put(&format!("JJ_ACP_{name}"), value);
                }
                for (name, value) in [
                    ("IDLE_TIMEOUT", self.acp.idle_timeout),
                    ("MAX_TURN_DURATION", self.acp.max_turn_duration),
                    ("CANCEL_GRACE", self.acp.cancel_grace),
                    ("REAP_TIMEOUT", self.acp.reap_timeout),
                ] {
                    put(&format!("JJ_ACP_{name}"), value.map(|v| v.to_string()));
                }
                put("JJ_ACP_WORKERS", self.acp.workers.map(|v| v.to_string()));
                put(
                    "JJ_ACP_QUEUE_CAPACITY",
                    self.acp.queue_capacity.map(|v| v.to_string()),
                );
            }
            Component::Agent => {
                put("BUZZ_AGENT_PROVIDER", self.agent.provider.clone());
                put("BUZZ_AGENT_MODEL", self.agent.model.clone());
                put(
                    "BUZZ_AGENT_THINKING_EFFORT",
                    self.agent.thinking_effort.clone(),
                );
                if let Some(root) = &self.runtime.data_dir {
                    put(
                        "BUZZ_AGENT_CONFIG_DIR",
                        Some(root.join("agent").to_string_lossy().into_owned()),
                    );
                }
            }
            Component::Compute => {
                put(
                    "JJ_STORE_COMPUTE_IDENTITY",
                    self.compute
                        .identity
                        .as_ref()
                        .map(|r| r.resolve(resolver))
                        .transpose()?,
                );
                put(
                    "JJ_STORE_COMPUTE_TOKEN",
                    self.compute
                        .token
                        .as_ref()
                        .map(|r| r.resolve(resolver))
                        .transpose()?,
                );
                put(
                    "JJ_STORE_COMPUTE_PORT",
                    self.compute.port.map(|v| v.to_string()),
                );
                put(
                    "JJ_STORE_COMPUTE_RELAYS",
                    (!self.compute.relay_urls.is_empty())
                        .then(|| self.compute.relay_urls.join(",")),
                );
                put(
                    "MESH_LLM_NATIVE_RUNTIME_BUNDLE_DIR",
                    path(self.compute.native_runtime.as_deref()),
                );
                put(
                    "LAPIS_AI_COMPUTE_CONFIG",
                    Some(self.compute_control().to_string()),
                );
                if let Some(root) = &self.runtime.data_dir {
                    let compute = root.join("compute");
                    for (name, relative) in [
                        ("HOME", ""),
                        ("XDG_CONFIG_HOME", "config"),
                        ("XDG_CACHE_HOME", "cache"),
                        ("XDG_DATA_HOME", "data"),
                        ("HF_HOME", "huggingface"),
                        ("HF_HUB_CACHE", "huggingface/hub"),
                        ("HUGGINGFACE_HUB_CACHE", "huggingface/hub"),
                        ("HF_XET_CACHE", "huggingface/xet"),
                        ("MESH_LLM_DATA_DIR", "data/mesh-llm"),
                    ] {
                        put(
                            name,
                            Some(
                                if relative.is_empty() {
                                    compute.clone()
                                } else {
                                    compute.join(relative)
                                }
                                .to_string_lossy()
                                .into_owned(),
                            ),
                        );
                    }
                    put(
                        "MESH_LLM_NATIVE_RUNTIME_CACHE_DIR",
                        Some(
                            root.join("compute/native-runtime")
                                .to_string_lossy()
                                .into_owned(),
                        ),
                    );
                }
            }
        }
        Ok(Environment(result))
    }
}
