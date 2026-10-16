use crate::{Config, EnvValue, Error, SecretRef};
use std::path::PathBuf;
fn reference(value: &SecretRef) -> Result<(), Error> {
    let names = [value.env.as_deref(), value.host.as_deref()];
    if names.iter().flatten().count() != 1
        || names
            .iter()
            .flatten()
            .any(|v| v.is_empty() || v.len() > 256 || v.chars().any(char::is_control))
    {
        return Err(Error("invalid_credential_reference"));
    }
    Ok(())
}
pub fn sensitive(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    [
        "KEY",
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "CAPABILITY",
        "IDENTITY",
    ]
    .iter()
    .any(|part| upper.contains(part))
}
impl Config {
    pub fn validate(&self) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error("unsupported_configuration_version"));
        }
        if self
            .acp
            .control_version
            .is_some_and(|v| !(1..=3).contains(&v))
        {
            return Err(Error("unsupported_control_version"));
        }
        if self.compute.memory_gb == Some(0) || self.compute.peers.len() > 16 {
            return Err(Error("invalid_compute_limits"));
        }
        if self.acp.workers.is_some_and(|v| v == 0 || v > 32) || self.acp.queue_capacity == Some(0)
        {
            return Err(Error("invalid_acp_limits"));
        }
        for value in [&self.compute.identity, &self.compute.token]
            .into_iter()
            .flatten()
        {
            reference(value)?;
        }
        for map in [
            &self.acp.environment,
            &self.agent.environment,
            &self.compute.environment,
        ] {
            for (key, value) in map {
                if key.is_empty()
                    || key.contains('=')
                    || key.chars().any(char::is_control)
                    || key.starts_with("JJ_ACP_TASK_")
                {
                    return Err(Error("invalid_environment_name"));
                }
                match value {
                    EnvValue::Value(_) if sensitive(key) => {
                        return Err(Error("literal_credential_forbidden"));
                    }
                    EnvValue::Reference(secret) => reference(secret)?,
                    _ => {}
                }
            }
        }
        Ok(())
    }
    pub(crate) fn resolve_paths(&mut self) {
        let base = self.directory.clone();
        let resolve = |path: &mut Option<PathBuf>| {
            if let Some(p) = path
                && p.is_relative()
            {
                *p = base.join(&*p);
            }
        };
        for path in [
            &mut self.runtime.data_dir,
            &mut self.runtime.acp,
            &mut self.runtime.agent,
            &mut self.runtime.compute,
            &mut self.runtime.tools,
            &mut self.acp.cwd,
            &mut self.acp.catalog,
            &mut self.acp.tools,
            &mut self.acp.system_prompt_file,
            &mut self.compute.native_runtime,
        ] {
            resolve(path);
        }
    }
}
