//! TOML is authored configuration; JSON remains a compatibility input.
use serde_json::Value;
use std::path::Path;
pub fn normalize(path: &str, bytes: &[u8]) -> Result<Vec<u8>, ()> {
    if Path::new(path).extension().is_none_or(|v| v != "toml") {
        return Ok(bytes.to_vec());
    }
    let document: toml::Value =
        toml::from_str(std::str::from_utf8(bytes).map_err(|_| ())?).map_err(|_| ())?;
    let mut document = serde_json::to_value(document).map_err(|_| ())?;
    if document
        .as_object()
        .is_none_or(|m| m.len() != 1 || !m.contains_key("tools"))
    {
        return Err(());
    }
    let tools = document["tools"].as_array_mut().ok_or(())?;
    for tool in tools.iter_mut() {
        if tool["type"] == "http" {
            variables(tool["headers"].as_array_mut().ok_or(())?, true)?;
            continue;
        }
        let command = tool["command"].as_str().ok_or(())?;
        if Path::new(command).is_relative() {
            let directory = Path::new(path).canonicalize().map_err(|_| ())?;
            tool["command"] = Value::String(
                directory
                    .parent()
                    .ok_or(())?
                    .join(command)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        variables(tool["env"].as_array_mut().ok_or(())?, false)?;
    }
    serde_json::to_vec(tools).map_err(|_| ())
}

fn variables(variables: &mut [Value], headers: bool) -> Result<(), ()> {
    for variable in variables {
        let map = variable.as_object_mut().ok_or(())?;
        if let Some(reference) = map.remove("env") {
            if map.contains_key("value") {
                return Err(());
            }
            let value = std::env::var(reference.as_str().ok_or(())?).map_err(|_| ())?;
            map.insert("value".into(), Value::String(value));
        } else {
            let name = map
                .get("name")
                .and_then(Value::as_str)
                .ok_or(())?
                .to_ascii_uppercase();
            if ["KEY", "TOKEN", "SECRET", "PASSWORD"]
                .iter()
                .any(|part| name.contains(part))
                || (headers
                    && matches!(
                        name.as_str(),
                        "AUTHORIZATION" | "PROXY-AUTHORIZATION" | "COOKIE"
                    ))
            {
                return Err(());
            }
        }
    }
    Ok(())
}
