//! Parse trusted launch tool declarations, never untrusted task/control frames.
use lapis_acp_control::task::{TaskRejection, code};
use lapis_acp_protocol::{EnvVar, McpServer, McpServerConfig};
use serde::Deserialize;
use std::{fs::File, io::Read, path::Path};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tool {
    name: String,
    command: String,
    args: Vec<String>,
    env: Vec<Variable>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Declaration {
    Stdio(Tool),
    Http(super::http_tools::HttpTool),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Variable {
    name: String,
    value: String,
}
fn refusal() -> TaskRejection {
    TaskRejection::new(
        code::INVALID_LAUNCH_CONFIGURATION,
        "invalid trusted tool configuration",
    )
}
pub(crate) fn read(path: Option<&str>) -> Result<Vec<McpServerConfig>, TaskRejection> {
    let Some(path) = path else {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    File::open(path)
        .map_err(|_| refusal())?
        .take(65537)
        .read_to_end(&mut data)
        .map_err(|_| refusal())?;
    if data.len() > 65536 {
        return Err(refusal());
    }
    let data = crate::toml_tools::normalize(path, &data).map_err(|_| refusal())?;
    let tools: Vec<Declaration> = serde_json::from_slice(&data).map_err(|_| refusal())?;
    if tools.len() > 8 {
        return Err(refusal());
    }
    let mut names = std::collections::BTreeSet::new();
    tools
        .into_iter()
        .map(|tool| {
            let tool = match tool {
                Declaration::Stdio(tool) => tool,
                Declaration::Http(tool) => {
                    return super::http_tools::validate(tool, &mut names).map_err(|_| refusal());
                }
            };
            if tool.name.is_empty()
                || tool.name.len() > 64
                || !tool
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
                || !names.insert(tool.name.clone())
                || !Path::new(&tool.command).is_absolute()
                || tool.command.contains('\0')
                || tool.args.len() > 64
                || tool.args.iter().any(|v| v.len() > 4096 || v.contains('\0'))
                || tool.env.len() > 64
                || tool.env.iter().any(|v| {
                    v.name.is_empty()
                        || !v
                            .name
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                        || v.name.as_bytes().first().is_some_and(u8::is_ascii_digit)
                        || v.name.len() > 128
                        || v.name.starts_with("JJ_ACP_TASK_")
                        || v.value.len() > 4096
                        || v.value.contains('\0')
                })
            {
                return Err(refusal());
            }
            Ok(McpServerConfig::from(McpServer {
                name: tool.name,
                command: tool.command,
                args: tool.args,
                env: tool
                    .env
                    .into_iter()
                    .map(|v| EnvVar::new(v.name, v.value))
                    .collect(),
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests;
