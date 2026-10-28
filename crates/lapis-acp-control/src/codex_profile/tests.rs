use super::*;
use serde_json::json;

fn session() -> Value {
    json!({"configOptions":[{"id":"model","category":"model","type":"select",
        "currentValue":"first[low]","options":[
        {"value":"first[low]"},{"value":"first[medium]"},{"value":"second[high]"}]}]})
}
fn request() -> SessionConfiguration {
    SessionConfiguration {
        model: Some("first[low]".into()),
        effort: Some("medium".into()),
        mode: Some("read-only".into()),
    }
}
#[test]
fn identified_codex_selects_an_exact_advertised_variant_without_generic_id_inference() {
    let raw = session();
    let requested = request();
    let selected = preferences(Some("@agentclientprotocol/codex-acp"), &raw, &requested).unwrap();
    assert_eq!(selected.model.as_deref(), Some("first[medium]"));
    assert_eq!(selected.mode, requested.mode);
    assert_eq!(selected.effort, None);
    assert_eq!(
        resolve_effort(&raw, "medium", Some("first")),
        EffortResolution::Unsupported
    );
    for other in [None, Some("cursor-acp"), Some("custom-codex")] {
        assert!(preferences(other, &raw, &requested).is_none());
    }
}
#[test]
fn compact_profile_never_invents_models_effort_or_overrides_an_explicit_surface() {
    let mut requested = request();
    requested.effort = Some("high".into());
    assert!(
        preferences(
            Some("@agentclientprotocol/codex-acp"),
            &session(),
            &requested
        )
        .is_none()
    );
    requested.effort = Some("medium".into());
    requested.model = Some("first[unknown]".into());
    assert!(
        preferences(
            Some("@agentclientprotocol/codex-acp"),
            &session(),
            &requested
        )
        .is_none()
    );
    let mut raw = session();
    raw["configOptions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"thinking","category":"thought_level","options":[{"value":"medium"}]}));
    assert!(preferences(Some("@agentclientprotocol/codex-acp"), &raw, &request()).is_none());
}

#[test]
fn codex_split_catalog_keeps_base_model_options_and_exact_unstable_variants() {
    let raw = json!({
        "configOptions":[{"id":"model","category":"model","options":[{"value":"first"}]}],
        "models":{"availableModels":[{"modelId":"first[low]"},{"modelId":"first[medium]"}]}
    });
    let selected = preferences(Some("@agentclientprotocol/codex-acp"), &raw, &request()).unwrap();
    assert_eq!(selected.model.as_deref(), Some("first[medium]"));
}
