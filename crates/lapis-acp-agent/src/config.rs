//! Parsing a catalog document.
//!
//! A catalog document is data. Parsing it cannot execute anything, and an invalid
//! entry is reported with the identifier it failed on rather than being skipped
//! silently, because a quietly smaller runner set is worse than a loud failure.

use lapis_acp_protocol::EnvVar;
use serde_json::Value;

use crate::definition::{CommandSpec, McpTransport, RunnerDefinition};
use crate::error::{Error, Result};

/// The array member holding runner declarations.
pub const RUNNERS_KEY: &str = "runners";

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Parse one argv, which may be a bare command or a complete argument vector.
fn command_spec(value: &Value, id: &str) -> Result<CommandSpec> {
    match value {
        Value::String(command) => Ok(CommandSpec::new(command.clone())),
        Value::Array(parts) => {
            let mut parts = parts.iter().map(|part| {
                part.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| Error::Parse(format!("{id}: command argv must be strings")))
            });
            let command = parts
                .next()
                .ok_or_else(|| Error::Parse(format!("{id}: command argv is empty")))??;
            let args = parts.collect::<Result<Vec<String>>>()?;
            Ok(CommandSpec { command, args })
        }
        _ => Err(Error::Parse(format!(
            "{id}: command must be a string or an array of strings"
        ))),
    }
}

fn arguments(value: &Value, id: &str) -> Result<Vec<String>> {
    let Some(args) = value.get("args") else {
        return Ok(Vec::new());
    };
    let Value::Array(args) = args else {
        return Err(Error::Parse(format!("{id}: args must be an array")));
    };
    args.iter()
        .map(|arg| {
            arg.as_str()
                .map(str::to_owned)
                .ok_or_else(|| Error::Parse(format!("{id}: args must contain only strings")))
        })
        .collect()
}

fn environment(value: &Value, id: &str) -> Result<Vec<EnvVar>> {
    let Some(env) = value.get("env") else {
        return Ok(Vec::new());
    };
    let Value::Object(env) = env else {
        return Err(Error::Parse(format!("{id}: env must be an object")));
    };
    let mut variables = Vec::new();
    for (name, value) in env {
        let value = value
            .as_str()
            .ok_or_else(|| Error::Parse(format!("{id}: env entry {name} must be a string")))?;
        variables.push(EnvVar::new(name.clone(), value.to_owned()));
    }
    variables.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(variables)
}

/// Parse one runner declaration.
pub fn parse_definition(value: &Value) -> Result<RunnerDefinition> {
    let id = string_field(value, "id")
        .ok_or_else(|| Error::Parse("a runner declaration needs a string id".to_owned()))?;
    let label = string_field(value, "label")
        .ok_or_else(|| Error::Parse(format!("{id}: label must be a string")))?;
    let command_value = value
        .get("command")
        .ok_or_else(|| Error::Parse(format!("{id}: command is required")))?;
    let mut commands = vec![command_spec(command_value, &id)?];
    if let Some(alternates) = value.get("alternateCommands") {
        let Value::Array(alternates) = alternates else {
            return Err(Error::Parse(format!(
                "{id}: alternateCommands must be an array"
            )));
        };
        for alternate in alternates {
            commands.push(command_spec(alternate, &id)?);
        }
    }
    if commands.len() == 1 {
        let args = arguments(value, &id)?;
        if !args.is_empty() {
            let command = commands[0].command.clone();
            commands[0] = CommandSpec { command, args };
        }
    } else if value.get("args").is_some() {
        return Err(Error::Parse(format!(
            "{id}: args cannot be combined with alternateCommands"
        )));
    }
    let transport = match string_field(value, "mcpTransport") {
        None => McpTransport::Stdio,
        Some(value) => {
            McpTransport::parse(&value).ok_or_else(|| Error::InvalidTransport(id.clone()))?
        }
    };
    let mut definition = RunnerDefinition::try_from_parts(
        id.clone(),
        label,
        commands,
        environment(value, &id)?,
        transport,
    )?;
    if value.get("enabled") == Some(&Value::Bool(false)) {
        definition = definition.disabled();
    }
    // Each is honoured on its own: a definition with a hint and no link, or a
    // link and no hint, is still useful guidance for an operator.
    definition.install_hint = string_field(value, "installHint").or(definition.install_hint);
    definition.install_instructions_url =
        string_field(value, "installInstructionsUrl").or(definition.install_instructions_url);
    Ok(definition)
}

/// Parse a complete catalog document.
pub fn parse_catalog(value: &Value) -> Result<Vec<RunnerDefinition>> {
    let runners = value
        .get(RUNNERS_KEY)
        .ok_or_else(|| Error::Parse(format!("{RUNNERS_KEY} array is required")))?;
    let Value::Array(runners) = runners else {
        return Err(Error::Parse(format!("{RUNNERS_KEY} must be an array")));
    };
    if runners.is_empty() {
        return Err(Error::Parse(format!(
            "{RUNNERS_KEY} must declare at least one runner"
        )));
    }
    runners.iter().map(parse_definition).collect()
}

/// Parse a catalog document from its JSON text.
pub fn parse_catalog_text(text: &str) -> Result<Vec<RunnerDefinition>> {
    let value: Value =
        serde_json::from_str(text).map_err(|err| Error::Parse(format!("invalid JSON: {err}")))?;
    parse_catalog(&value)
}

/// Read a trusted catalog. TOML relative executable paths are file-relative.
pub fn read_catalog(path: &std::path::Path) -> Result<Vec<RunnerDefinition>> {
    let metadata =
        std::fs::metadata(path).map_err(|_| Error::Parse("catalog unavailable".into()))?;
    if metadata.len() > 1024 * 1024 {
        return Err(Error::Parse("catalog limit".into()));
    }
    let text =
        std::fs::read_to_string(path).map_err(|_| Error::Parse("catalog unavailable".into()))?;
    if path.extension().is_none_or(|extension| extension != "toml") {
        return parse_catalog_text(&text);
    }
    let mut value = serde_json::to_value(
        toml::from_str::<toml::Value>(&text)
            .map_err(|_| Error::Parse("invalid TOML catalog".into()))?,
    )
    .map_err(|_| Error::Parse("invalid TOML catalog".into()))?;
    if value
        .as_object()
        .is_none_or(|map| map.len() != 1 || !map.contains_key("runners"))
    {
        return Err(Error::Parse("unknown TOML catalog field".into()));
    }
    let directory = path
        .canonicalize()
        .map_err(|_| Error::Parse("catalog unavailable".into()))?;
    let directory = directory
        .parent()
        .ok_or_else(|| Error::Parse("catalog path".into()))?;
    for runner in value["runners"]
        .as_array_mut()
        .ok_or_else(|| Error::Parse("runners required".into()))?
    {
        let map = runner
            .as_object()
            .ok_or_else(|| Error::Parse("invalid TOML runner".into()))?;
        let allowed = [
            "id",
            "label",
            "command",
            "args",
            "env",
            "enabled",
            "mcpTransport",
            "alternateCommands",
            "installHint",
            "installInstructionsUrl",
            "mcp_transport",
            "alternate_commands",
            "install_hint",
            "install_instructions_url",
        ];
        if map.keys().any(|key| !allowed.contains(&key.as_str()))
            || map.get("enabled").is_some_and(|v| !v.is_boolean())
        {
            return Err(Error::Parse("unknown or invalid TOML runner field".into()));
        }
        for (old, new) in [
            ("mcp_transport", "mcpTransport"),
            ("alternate_commands", "alternateCommands"),
            ("install_hint", "installHint"),
            ("install_instructions_url", "installInstructionsUrl"),
        ] {
            if let Some(v) = runner.as_object_mut().and_then(|r| r.remove(old)) {
                if runner.get(new).is_some() {
                    return Err(Error::Parse("duplicate TOML runner field".into()));
                }
                runner[new] = v;
            }
        }
        resolve_command(
            runner
                .get_mut("command")
                .ok_or_else(|| Error::Parse("command required".into()))?,
            directory,
        )?;
        if let Some(alternates) = runner
            .get_mut("alternateCommands")
            .and_then(Value::as_array_mut)
        {
            for command in alternates {
                resolve_command(command, directory)?;
            }
        }
        if let Some(env) = runner.get_mut("env").and_then(Value::as_object_mut) {
            for (name, value) in env {
                if let Some(reference) = value.get("env").and_then(Value::as_str) {
                    if value.as_object().is_none_or(|map| map.len() != 1) {
                        return Err(Error::Parse("invalid environment reference".into()));
                    }
                    *value =
                        Value::String(std::env::var(reference).map_err(|_| {
                            Error::Parse("environment reference unavailable".into())
                        })?);
                } else if [
                    "KEY",
                    "TOKEN",
                    "SECRET",
                    "PASSWORD",
                    "CAPABILITY",
                    "IDENTITY",
                ]
                .iter()
                .any(|p| name.to_ascii_uppercase().contains(p))
                {
                    return Err(Error::Parse("literal credential forbidden".into()));
                }
            }
        }
    }
    parse_catalog(&value)
}

fn resolve_command(command: &mut Value, directory: &std::path::Path) -> Result<()> {
    let command = if command.is_array() {
        command
            .get_mut(0)
            .ok_or_else(|| Error::Parse("empty command".into()))?
    } else {
        command
    };
    if let Some(path) = command.as_str()
        && std::path::Path::new(path).is_relative()
        && (path.contains('/') || path.contains('\\'))
    {
        *command = Value::String(directory.join(path).to_string_lossy().into_owned());
    }
    Ok(())
}
