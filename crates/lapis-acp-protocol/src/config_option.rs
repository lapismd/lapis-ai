//! Model and configuration option selection.
//!
//! Adapters expose model choice through two surfaces: the stable
//! `configOptions` array and an unstable `models` object. Both are read here,
//! with the stable surface taking precedence. No identifier is hardcoded,
//! because adapters define their own option ids and their own model ids.

use serde_json::{Value, json};

/// The configuration category that carries model choices.
pub const MODEL_CATEGORY: &str = "model";

/// The configuration category that carries reasoning effort choices.
pub const THOUGHT_LEVEL_CATEGORY: &str = "thought_level";

/// Build `session/set_model` params.
pub fn set_model_params(session_id: &str, model_id: &str) -> Value {
    json!({ "sessionId": session_id, "modelId": model_id })
}

/// Build `session/set_config_option` params.
///
/// The wire key is always `configId` even though some adapters advertise their
/// option identifier as `id`.
pub fn set_config_option_params(session_id: &str, config_id: &str, value: &str) -> Value {
    json!({ "sessionId": session_id, "configId": config_id, "value": value })
}

/// Select the `configOptions` entries in a category.
pub fn options_in_category(result: &Value, category: &str) -> Vec<Value> {
    result
        .get("configOptions")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter(|option| option.get("category").and_then(Value::as_str) == Some(category))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// The model options an agent advertised at session creation.
pub fn model_options(result: &Value) -> Vec<Value> {
    options_in_category(result, MODEL_CATEGORY)
}

/// The unstable `models` object when the agent supplied one.
pub fn model_state(result: &Value) -> Option<Value> {
    result.get("models").cloned()
}

/// Read an option's own identifier, accepting either spelling.
///
/// The ACP schema uses `configId`; at least one shipping adapter uses `id`. Both
/// are accepted so the identifier is discovered from the session rather than
/// assumed per vendor.
pub fn option_id(option: &Value) -> Option<&str> {
    option
        .get("configId")
        .or_else(|| option.get("id"))
        .and_then(Value::as_str)
}

/// Discover the reasoning-effort option identifier an agent advertised.
pub fn thought_level_config_id(result: &Value) -> Option<String> {
    options_in_category(result, THOUGHT_LEVEL_CATEGORY)
        .iter()
        .find_map(|option| option_id(option))
        .map(str::to_owned)
}

/// How a model should be selected for a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSwitch {
    /// Use the stable `session/set_config_option` path.
    ConfigOption {
        /// The adapter-defined option identifier to set.
        config_id: String,
        /// The value within that option which selects the model.
        option_value: String,
    },
    /// Use the unstable `session/set_model` path.
    SetModel {
        /// The model identifier to request.
        model_id: String,
    },
}

/// Resolve how to select `model_id`, or `None` when the agent does not offer it.
///
/// The stable `configOptions` surface is searched first, matching a candidate
/// inside each option's nested `options` array rather than against the option's
/// own current value: the current value is what the session started with, not
/// the catalogue of what it could become. The unstable `models` object is the
/// fallback. The session result is always authoritative, so a model available in
/// an earlier session is never assumed available in this one.
pub fn resolve_model_switch(result: &Value, model_id: &str) -> Option<ModelSwitch> {
    for option in model_options(result) {
        let Some(config_id) = option_id(&option) else {
            continue;
        };
        let candidates = option.get("options").and_then(Value::as_array);
        let matched = candidates.is_some_and(|candidates| {
            candidates
                .iter()
                .any(|candidate| candidate.get("value").and_then(Value::as_str) == Some(model_id))
        });
        if matched {
            return Some(ModelSwitch::ConfigOption {
                config_id: config_id.to_owned(),
                option_value: model_id.to_owned(),
            });
        }
    }
    let available = model_state(result)?
        .get("availableModels")?
        .as_array()?
        .iter()
        .any(|model| model.get("modelId").and_then(Value::as_str) == Some(model_id));
    available.then_some(ModelSwitch::SetModel {
        model_id: model_id.to_owned(),
    })
}

/// Report whether a session result offers `model_id` at all.
pub fn offers_model(result: &Value, model_id: &str) -> bool {
    resolve_model_switch(result, model_id).is_some()
}

/// Report whether `model_id` appears in already-extracted catalogue halves.
///
/// This mirrors [`resolve_model_switch`] for a caller that cached the two
/// halves instead of retaining the whole session result, so a pre-turn guard can
/// check availability without re-parsing.
pub fn model_in_catalog(
    config_options: &[Value],
    available_models: Option<&Value>,
    model_id: &str,
) -> bool {
    let in_config_options = config_options.iter().any(|option| {
        option
            .get("options")
            .and_then(Value::as_array)
            .is_some_and(|candidates| {
                candidates.iter().any(|candidate| {
                    candidate.get("value").and_then(Value::as_str) == Some(model_id)
                })
            })
    });
    if in_config_options {
        return true;
    }
    available_models
        .and_then(|models| models.get("availableModels"))
        .and_then(Value::as_array)
        .is_some_and(|available| {
            available
                .iter()
                .any(|model| model.get("modelId").and_then(Value::as_str) == Some(model_id))
        })
}

/// The configuration category that carries session execution modes.
pub const MODE_CATEGORY: &str = "mode";

/// The first `configOptions` entry in a category, when one exists.
pub fn option_in_category(result: &Value, category: &str) -> Option<Value> {
    options_in_category(result, category).into_iter().next()
}

/// The selectable values an option advertises, in order.
pub fn option_values(option: &Value) -> Vec<String> {
    option
        .get("options")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| option.get("value").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The session modes the agent advertised under `modes.availableModes`.
///
/// This is a different surface from `configOptions`, and both have to agree
/// before a mode is applied: an adapter that is handed a mode it does not
/// advertise can fail hard rather than politely refuse.
pub fn available_modes(result: &Value) -> Vec<String> {
    result
        .pointer("/modes/availableModes")
        .and_then(Value::as_array)
        .map(|modes| {
            modes
                .iter()
                .filter_map(|mode| mode.get("id").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// How a reasoning effort should be applied, or why it cannot be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffortResolution {
    /// Set the discovered `thought_level` option to the adapter's own spelling.
    ConfigOption {
        /// Adapter-defined option identifier.
        config_id: String,
        /// Adapter-defined value, not the caller's spelling.
        value: String,
    },
    /// Select a different model identifier whose parameters carry the effort.
    ModelId {
        /// The complete parameterized identifier to request.
        model_id: String,
    },
    /// The adapter exposes no effort surface at all.
    Unsupported,
    /// The adapter exposes one, but not this value.
    Unavailable,
}

/// Resolve how to apply a reasoning effort to a session.
///
/// Two strategies exist because adapters differ. When a `thought_level` option is
/// advertised, its identifier is discovered rather than assumed and the value sent
/// is the adapter's own spelling, so a caller asking for `off` reaches an adapter
/// that advertises `none`. When no such option exists, effort is looked for inside
/// the parameterized model identifiers, and applying it means selecting a
/// different model. `model_base` narrows that search to one model when the caller
/// also chose one.
///
/// Matching is exact first and normalized second, which is what keeps
/// adapter-specific levels such as `max` selectable despite being in no portable
/// vocabulary.
pub fn resolve_effort(
    result: &Value,
    requested: &str,
    model_base: Option<&str>,
) -> EffortResolution {
    if let Some(option) = option_in_category(result, THOUGHT_LEVEL_CATEGORY) {
        let Some(config_id) = option_id(&option) else {
            return EffortResolution::Unsupported;
        };
        return match option_values(&option)
            .into_iter()
            .find(|value| crate::thinking::effort_matches(value, requested))
        {
            Some(value) => EffortResolution::ConfigOption {
                config_id: config_id.to_owned(),
                value,
            },
            None => EffortResolution::Unavailable,
        };
    }

    // Whether an effort surface exists at all is a property of the whole
    // catalogue, not of the one model the caller narrowed to. Deciding it after
    // filtering would report "this adapter cannot do effort" when the truth is
    // "not for that model", which sends a caller looking in the wrong place.
    let candidates: Vec<String> = model_options(result)
        .iter()
        .filter_map(|option| option.get("options").and_then(Value::as_array))
        .flatten()
        .filter_map(|candidate| candidate.get("value").and_then(Value::as_str))
        .map(str::to_owned)
        .collect();
    let surface_exists = candidates
        .iter()
        .any(|value| crate::thinking::advertised_effort(value).is_some());
    for value in &candidates {
        if let Some(base) = model_base
            && crate::thinking::model_id_base(value) != base
        {
            continue;
        }
        let Some(advertised) = crate::thinking::advertised_effort(value) else {
            continue;
        };
        if crate::thinking::effort_matches(&advertised, requested) {
            return EffortResolution::ModelId {
                model_id: value.clone(),
            };
        }
    }
    if surface_exists {
        EffortResolution::Unavailable
    } else {
        EffortResolution::Unsupported
    }
}

/// How a session mode should be applied, or why it cannot be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeResolution {
    /// Set the discovered mode option to this value.
    Switch {
        /// Adapter-defined option identifier.
        config_id: String,
        /// Adapter-defined value.
        value: String,
    },
    /// The name exists, but under a different category, so it is not a session
    /// mode. Carries the category it was found under for diagnostics.
    NotASessionMode {
        /// The category that does offer this value.
        offered_under: Option<String>,
    },
    /// Neither the mode option nor `availableModes` offers this value.
    Unknown,
    /// The adapter advertises no mode surface at all.
    Unsupported,
}

/// Resolve how to apply a session execution mode.
///
/// The requested value must be offered by the discovered `mode` option and, when
/// the adapter advertises `modes.availableModes`, must appear there too. Both
/// checks exist because the surfaces are independent and an adapter handed a mode
/// it did not advertise can fail hard rather than refuse politely.
///
/// A value offered under some other category is reported as such rather than
/// silently substituted. One real adapter separates `mode` from
/// `collaboration_mode`, so `plan` there is not a session mode and selecting it
/// as one would be a guess.
pub fn resolve_mode(result: &Value, requested: &str) -> ModeResolution {
    let Some(option) = option_in_category(result, MODE_CATEGORY) else {
        return ModeResolution::Unsupported;
    };
    let Some(config_id) = option_id(&option) else {
        return ModeResolution::Unsupported;
    };
    if !option_values(&option)
        .iter()
        .any(|value| value == requested)
    {
        let offered_under = options_in_category_any(result, requested);
        return if offered_under.is_some() {
            ModeResolution::NotASessionMode { offered_under }
        } else {
            ModeResolution::Unknown
        };
    }
    let modes = available_modes(result);
    if !modes.is_empty() && !modes.iter().any(|mode| mode == requested) {
        return ModeResolution::NotASessionMode {
            offered_under: None,
        };
    }
    ModeResolution::Switch {
        config_id: config_id.to_owned(),
        value: requested.to_owned(),
    }
}

/// Find which category offers a value, ignoring the `mode` category itself.
fn options_in_category_any(result: &Value, value: &str) -> Option<String> {
    result
        .get("configOptions")
        .and_then(Value::as_array)?
        .iter()
        .find(|option| {
            option.get("category").and_then(Value::as_str) != Some(MODE_CATEGORY)
                && option_values(option)
                    .iter()
                    .any(|candidate| candidate == value)
        })
        .and_then(|option| option.get("category"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}
