//! Explicit CLI launch adapter. Parsing configuration alone never executes this.
use crate::{Component, Config, Error, SystemResolver};
use std::{path::PathBuf, process::Command};
pub fn selected(args: &[String]) -> Result<Option<(Config, Vec<String>)>, Error> {
    let mut path = None;
    let mut remaining = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--config" {
            if path.is_some() {
                return Err(Error("duplicate_configuration_argument"));
            }
            path = Some(PathBuf::from(
                iter.next().ok_or(Error("configuration_path_required"))?,
            ));
        } else {
            remaining.push(arg.clone());
        }
    }
    path.map(|p| Config::load(&p).map(|c| (c, remaining)))
        .transpose()
}
pub fn legacy_environment() -> impl Iterator<Item = (String, String)> {
    std::env::vars().filter(|(key, _)| {
        [
            "PATH",
            "HOME",
            "LANG",
            "LC_ALL",
            "LC_CTYPE",
            "TMPDIR",
            "TEMP",
            "TMP",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "XDG_DATA_HOME",
            "SSH_AUTH_SOCK",
            "SSL_CERT_FILE",
            "SSL_CERT_DIR",
        ]
        .contains(&key.as_str())
            || [
                "JJ_ACP_",
                "BUZZ_",
                "ANTHROPIC_",
                "OPENAI_",
                "OPENROUTER_",
                "DATABRICKS_",
                "JJ_STORE_COMPUTE_",
                "JJ_STORE_CHATGPT_",
                "LAPIS_AGENT_TOKEN_",
                "MESH_LLM_",
            ]
            .iter()
            .any(|prefix| key.starts_with(prefix))
    })
}
/// Replace this helper with an independently configured process, preserving PID
/// and signal ownership on the initially supported Unix platforms.
pub fn cli(component: Component) -> Option<i32> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = (|| {
        let Some((config, mut args)) = selected(&args)? else {
            return Ok(None);
        };
        if args.iter().any(|s| s == "--check-config") {
            println!("configuration valid");
            return Ok(Some(0));
        }
        if component == Component::Compute && config.runtime.data_dir.is_none() {
            return Err(Error("compute_data_directory_required"));
        }
        let env = config.environment(component, &SystemResolver)?;
        if component == Component::Acp
            && let Some(version) = config.acp.control_version
            && args.first().map(String::as_str) == Some("serve")
            && !args.iter().any(|s| s == "--control-version")
        {
            args.extend(["--control-version".into(), version.to_string()]);
        }
        let mut command =
            Command::new(std::env::current_exe().map_err(|_| Error("executable_unavailable"))?);
        command
            .args(args)
            .env_clear()
            .envs(legacy_environment())
            .envs(&env.0);
        if component == Component::Compute
            && let Some(root) = &config.runtime.data_dir
        {
            let cwd = root.join("compute");
            private_directory(&cwd)?;
            command.current_dir(cwd);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let _error = command.exec();
            Err(Error("helper_exec_failed"))
        }
        #[cfg(not(unix))]
        {
            Ok(Some(
                command
                    .status()
                    .map_err(|_| Error("helper_spawn_failed"))?
                    .code()
                    .unwrap_or(1),
            ))
        }
    })();
    match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            Some(2)
        }
    }
}
pub fn private_directory(path: &std::path::Path) -> Result<(), Error> {
    std::fs::create_dir_all(path).map_err(|_| Error("private_directory_unavailable"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| Error("private_directory_unavailable"))?;
    }
    Ok(())
}
