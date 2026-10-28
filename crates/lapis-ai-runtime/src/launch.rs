use crate::{Component, Config, Environment, Error, Resolver, Result};
use std::{collections::BTreeMap, path::PathBuf};
/// A prepared launch contains secrets and intentionally has no serialization.
pub struct PreparedLaunch {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub directory: PathBuf,
    pub environment: Environment,
}
impl std::fmt::Debug for PreparedLaunch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedLaunch")
            .field("executable", &self.executable)
            .field("environment", &self.environment)
            .finish_non_exhaustive()
    }
}
impl PreparedLaunch {
    /// Copy only explicit host values; never inherit the parent process environment.
    pub fn from_command(command: &std::process::Command) -> Result<Self> {
        let text = |value: &std::ffi::OsStr| {
            value
                .to_str()
                .map(str::to_owned)
                .ok_or(Error("invalid_launch_text"))
        };
        let mut environment = BTreeMap::new();
        for (name, value) in command.get_envs() {
            if let Some(value) = value {
                environment.insert(text(name)?, text(value)?);
            }
        }
        Ok(Self {
            executable: command.get_program().into(),
            arguments: command.get_args().map(text).collect::<Result<_>>()?,
            directory: command
                .get_current_dir()
                .ok_or(Error("working_directory_required"))?
                .into(),
            environment: Environment(environment),
        })
    }
    pub fn new(config: &Config, component: Component, resolver: &dyn Resolver) -> Result<Self> {
        config.validate()?;
        let executable = config
            .executable(component)
            .ok_or(Error("helper_not_configured"))?
            .to_path_buf();
        let mut environment =
            lapis_ai_config::launch::legacy_environment().collect::<BTreeMap<_, _>>();
        environment.extend(config.environment(component, resolver)?.0);
        let directory = if component == Component::Compute {
            config
                .runtime
                .data_dir
                .as_ref()
                .ok_or(Error("compute_data_directory_required"))?
                .join("compute")
        } else {
            config
                .acp
                .cwd
                .clone()
                .unwrap_or_else(|| config.directory.clone())
        };
        Ok(Self {
            executable,
            arguments: Vec::new(),
            directory,
            environment: Environment(environment),
        })
    }
    /// Explicit host values take precedence over TOML and inherited compatibility values.
    pub fn override_environment(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<()> {
        let name = name.into();
        let value = value.into();
        if name.is_empty()
            || name.contains('=')
            || name.contains('\0')
            || value.contains('\0')
            || name.starts_with("JJ_ACP_TASK_")
        {
            return Err(Error("invalid_launch_environment"));
        }
        self.environment.0.insert(name, value);
        Ok(())
    }
}
