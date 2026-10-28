//! Codex ACP 1.13 compact model variants, outside generic opaque-ID parsing.
use lapis_acp_protocol::{
    EffortResolution, Thinking, effort_matches, model_id_base, model_options, offers_model,
    resolve_effort,
};
use lapis_acp_runner::SessionConfiguration;
use serde_json::Value;

pub(crate) fn preferences(
    agent: Option<&str>,
    session: &Value,
    requested: &SessionConfiguration,
) -> Option<SessionConfiguration> {
    if agent != Some("@agentclientprotocol/codex-acp") {
        return None;
    }
    let model = requested.model.as_deref()?;
    let effort = requested.effort.as_deref()?;
    let base = model_id_base(model);
    if !offers_model(session, model)
        || resolve_effort(session, effort, Some(base)) != EffortResolution::Unsupported
    {
        return None;
    }
    let candidates: Vec<String> = model_options(session)
        .iter()
        .filter_map(|option| option["options"].as_array())
        .flatten()
        .filter_map(|option| option["value"].as_str().map(str::to_owned))
        .chain(
            session["models"]["availableModels"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|model| model["modelId"].as_str().map(str::to_owned)),
        )
        .collect();
    let candidate = candidates.into_iter().find(|candidate| {
        let Some((family, token)) = candidate
            .strip_suffix(']')
            .and_then(|id| id.rsplit_once('['))
        else {
            return false;
        };
        family == base
            && (Thinking::parse(token).is_some() || matches!(token, "max" | "ultra"))
            && effort_matches(token, effort)
    })?;
    Some(SessionConfiguration {
        model: Some(candidate),
        effort: None,
        mode: requested.mode.clone(),
    })
}

#[cfg(test)]
mod tests;
