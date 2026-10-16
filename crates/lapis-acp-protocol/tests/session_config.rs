//! Session configuration resolution against two captured real adapters.
//!
//! Effort and mode are the settings adapters disagree about most, so every
//! assertion here is driven by a payload a shipping adapter actually sent rather
//! than by a shape we invented.

use lapis_acp_protocol::config_option::{
    EffortResolution, ModeResolution, available_modes, option_in_category, option_values,
    resolve_effort, resolve_mode,
};
use lapis_acp_protocol::thinking::{
    Thinking, advertised_effort, advertises_fast, badges, effort_matches, model_id_attrs,
    model_id_base,
};
use serde_json::Value;

/// The captured fixtures wrap the handshake in an envelope; configuration
/// resolution reads the `session/new` result, so that is what these return.
fn cursor() -> Value {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/cursor-acp-session.json")).expect("cursor");
    fixture["sessionNew"].clone()
}

fn codex() -> Value {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/codex-acp-session.json")).expect("codex");
    fixture["sessionNew"].clone()
}

#[test]
fn effort_resolves_through_a_discovered_thought_level_option() {
    let session = codex();
    // The identifier is adapter-defined and must be discovered, not assumed.
    assert_eq!(
        resolve_effort(&session, "high", None),
        EffortResolution::ConfigOption {
            config_id: "reasoning_effort".to_owned(),
            value: "high".to_owned(),
        }
    );
    assert_eq!(
        resolve_effort(&session, "xhigh", None),
        EffortResolution::ConfigOption {
            config_id: "reasoning_effort".to_owned(),
            value: "xhigh".to_owned(),
        }
    );
}

#[test]
fn effort_levels_outside_the_portable_vocabulary_stay_selectable() {
    // A real adapter advertises `max` and `ultra`. A closed enum would make them
    // unreachable, which is why matching is exact first and normalized second.
    let session = codex();
    for level in ["max", "ultra"] {
        assert_eq!(
            resolve_effort(&session, level, None),
            EffortResolution::ConfigOption {
                config_id: "reasoning_effort".to_owned(),
                value: level.to_owned(),
            },
            "{level} should be selectable"
        );
    }
    assert_eq!(Thinking::parse("max"), None);
    assert_eq!(Thinking::parse("ultra"), None);
}

#[test]
fn an_effort_the_adapter_does_not_offer_is_unavailable_not_unsupported() {
    let session = codex();
    // This adapter's thought_level starts at low; it has no off or none.
    assert_eq!(
        resolve_effort(&session, "off", None),
        EffortResolution::Unavailable
    );
    assert_eq!(
        resolve_effort(&session, "none", None),
        EffortResolution::Unavailable
    );
    assert_eq!(
        resolve_effort(&session, "nonsense", None),
        EffortResolution::Unavailable
    );
    // The distinction matters: unavailable means the surface exists and could
    // carry another value, unsupported means there is nothing to set.
    assert_eq!(
        resolve_effort(&Value::Null, "high", None),
        EffortResolution::Unsupported
    );
}

#[test]
fn effort_normalizes_across_adapter_spellings() {
    assert!(effort_matches("none", "off"));
    assert!(effort_matches("off", "none"));
    assert!(effort_matches("HIGH", "high"));
    assert!(effort_matches("max", "max"));
    assert!(!effort_matches("max", "high"));
    assert!(!effort_matches("low", "medium"));
    assert_eq!(Thinking::parse("None"), Some(Thinking::Off));
    assert_eq!(Thinking::parse(" XHigh "), Some(Thinking::XHigh));
    assert_eq!(Thinking::Off.label(), "Off");
    assert_eq!(Thinking::XHigh.label(), "Extra High");
}

#[test]
fn effort_resolves_through_parameterized_model_ids_when_no_option_exists() {
    let session = cursor();
    // This adapter exposes no thought_level option at all, so effort lives inside
    // the model identifier and applying it means selecting a different model.
    assert!(option_in_category(&session, "thought_level").is_none());
    assert_eq!(
        resolve_effort(&session, "high", Some("grok-4.7")),
        EffortResolution::ModelId {
            model_id: "grok-4.7[context=256k,reasoning_effort=high,fast=true]".to_owned(),
        }
    );
    // A different key spelling resolves the same way.
    assert_eq!(
        resolve_effort(&session, "medium", Some("claude-opus-5-5")),
        EffortResolution::ModelId {
            model_id: "claude-opus-5-5[context=300k,effort=medium,fast=false]".to_owned(),
        }
    );
    // Without a base model the first match wins, and the value sent is the
    // adapter's own complete identifier.
    match resolve_effort(&session, "high", None) {
        EffortResolution::ModelId { model_id } => {
            assert_eq!(advertised_effort(&model_id).as_deref(), Some("high"));
        }
        other => panic!("expected a model identifier, got {other:?}"),
    }
}

#[test]
fn a_base_model_with_no_matching_effort_is_reported_honestly() {
    let session = cursor();
    // composer-2.5 is only offered with fast=true and no effort attribute.
    assert_eq!(
        resolve_effort(&session, "high", Some("composer-2.5")),
        EffortResolution::Unavailable
    );
    assert_eq!(
        resolve_effort(&session, "high", Some("no-such-model")),
        EffortResolution::Unavailable
    );
}

#[test]
fn parameterized_model_identifiers_are_parsed_not_guessed() {
    let raw = "claude-opus-5[thinking=true,context=300k,effort=high,fast=false]";
    assert_eq!(model_id_base(raw), "claude-opus-5");
    assert_eq!(
        model_id_attrs(raw),
        vec![
            ("thinking".to_owned(), "true".to_owned()),
            ("context".to_owned(), "300k".to_owned()),
            ("effort".to_owned(), "high".to_owned()),
            ("fast".to_owned(), "false".to_owned()),
        ]
    );
    assert_eq!(advertised_effort(raw).as_deref(), Some("high"));
    assert!(!advertises_fast(raw));
    assert_eq!(badges(raw), vec!["High"]);

    let fast = "grok-4.7[context=256k,reasoning_effort=high,fast=true]";
    assert!(advertises_fast(fast));
    assert_eq!(badges(fast), vec!["Fast", "High"]);

    // `thinking=true` asserts that reasoning happens without naming a level.
    let thinking_only = "some-model[thinking=true]";
    assert_eq!(advertised_effort(thinking_only).as_deref(), Some("true"));
    assert_eq!(badges(thinking_only), vec!["Thinking"]);

    // Unparameterized identifiers parse to themselves.
    assert_eq!(model_id_base("gpt-6-astra"), "gpt-6-astra");
    assert!(model_id_attrs("gpt-6-astra").is_empty());
    assert_eq!(badges("gpt-6-astra"), Vec::<&str>::new());
    // A malformed bracket is skipped rather than making the model unselectable.
    assert_eq!(model_id_base("default[]"), "default");
    assert!(model_id_attrs("default[]").is_empty());
    assert!(model_id_attrs("broken[unterminated").is_empty());
}

#[test]
fn mode_resolution_uses_both_advertised_surfaces() {
    let codex_session = codex();
    let cursor_session = cursor();
    assert_eq!(
        available_modes(&codex_session),
        vec!["read-only", "agent", "agent-full-access"]
    );
    assert_eq!(
        available_modes(&cursor_session),
        vec!["agent", "plan", "ask"]
    );

    assert_eq!(
        resolve_mode(&codex_session, "read-only"),
        ModeResolution::Switch {
            config_id: "mode".to_owned(),
            value: "read-only".to_owned(),
        }
    );
    assert_eq!(
        resolve_mode(&cursor_session, "ask"),
        ModeResolution::Switch {
            config_id: "mode".to_owned(),
            value: "ask".to_owned(),
        }
    );
    // Each adapter's modes are its own; neither is a superset of the other.
    assert!(!matches!(
        resolve_mode(&codex_session, "ask"),
        ModeResolution::Switch { .. }
    ));
    assert!(!matches!(
        resolve_mode(&cursor_session, "agent-full-access"),
        ModeResolution::Switch { .. }
    ));
}

#[test]
fn a_mode_offered_under_another_category_is_not_substituted() {
    let session = codex();
    // This adapter separates `mode` from `collaboration_mode`, and `plan` lives
    // in the latter. Selecting it as a session mode would be a guess about what
    // the adapter means, so it is reported instead.
    assert_eq!(
        resolve_mode(&session, "plan"),
        ModeResolution::NotASessionMode {
            offered_under: Some("collaboration_mode".to_owned()),
        }
    );
    // Cursor does offer `plan` as a real session mode.
    assert!(matches!(
        resolve_mode(&cursor(), "plan"),
        ModeResolution::Switch { .. }
    ));
}

#[test]
fn unknown_and_absent_mode_surfaces_are_distinguished() {
    assert_eq!(resolve_mode(&codex(), "turbo"), ModeResolution::Unknown);
    assert_eq!(
        resolve_mode(&Value::Null, "agent"),
        ModeResolution::Unsupported
    );
    // A session that advertises modes only through availableModes, with no
    // configOption to set, cannot be configured and says so.
    let modes_only = serde_json::json!({
        "modes": {"availableModes": [{"id": "agent"}]},
    });
    assert_eq!(
        resolve_mode(&modes_only, "agent"),
        ModeResolution::Unsupported
    );
}

#[test]
fn a_mode_present_in_the_option_but_not_in_available_modes_is_refused() {
    // The two surfaces are independent, and an adapter handed a mode it did not
    // advertise can fail hard rather than refuse politely.
    let session = serde_json::json!({
        "configOptions": [{"category": "mode", "id": "mode", "value": "agent",
                           "options": [{"value": "agent"}, {"value": "yolo"}]}],
        "modes": {"availableModes": [{"id": "agent"}]},
    });
    assert_eq!(
        resolve_mode(&session, "yolo"),
        ModeResolution::NotASessionMode {
            offered_under: None
        }
    );
    assert!(matches!(
        resolve_mode(&session, "agent"),
        ModeResolution::Switch { .. }
    ));
    // With no availableModes at all, the option surface alone is authoritative.
    let option_only = serde_json::json!({
        "configOptions": [{"category": "mode", "id": "mode",
                           "options": [{"value": "agent"}, {"value": "yolo"}]}],
    });
    assert!(matches!(
        resolve_mode(&option_only, "yolo"),
        ModeResolution::Switch { .. }
    ));
}

#[test]
fn other_configuration_categories_are_visible_but_not_conflated() {
    let session = codex();
    // This adapter also exposes a model_config category. It is readable through
    // the same generic surface, and nothing about effort or mode resolution
    // mistakes it for either.
    let fast = option_in_category(&session, "model_config").expect("model_config");
    assert_eq!(option_values(&fast), vec!["off", "on"]);
    assert!(option_in_category(&session, "thought_level").is_some());
    assert!(option_in_category(&session, "collaboration_mode").is_some());
    assert!(option_in_category(&session, "nonsense").is_none());
    assert_eq!(option_values(&Value::Null), Vec::<String>::new());
}

#[test]
fn a_bare_positional_bracket_is_not_read_as_an_effort() {
    // Observed on a live adapter: `gpt-6.1-sol[medium]`. The token is a reasoning
    // level, but it carries no key, so reading it as effort would be an inference
    // about what the adapter means rather than a match against what it advertised.
    // A closed vocabulary would make the inference usually right and occasionally
    // wrong, and the occasionally-wrong case sends a model the operator never
    // asked for. Reporting "no effort control" is the honest outcome, and it is
    // what the workspace's own parser does too.
    assert_eq!(model_id_base("gpt-6.1-sol[medium]"), "gpt-6.1-sol");
    assert!(model_id_attrs("gpt-6.1-sol[medium]").is_empty());
    assert_eq!(advertised_effort("gpt-6.1-sol[medium]"), None);
    assert_eq!(badges("gpt-6.1-sol[medium]"), Vec::<&str>::new());
    assert_eq!(
        resolve_effort(
            &serde_json::json!({
                "configOptions": [{"category": "model", "id": "model",
                                   "options": [{"value": "gpt-6.1-sol[medium]"}]}],
            }),
            "medium",
            None
        ),
        EffortResolution::Unsupported
    );
}

#[test]
fn a_keyed_effort_wins_over_a_thinking_marker_on_the_same_identifier() {
    // Both keys on one identifier: `thinking=true` says reasoning happens,
    // `effort=high` says how much. The level is the more specific claim, and
    // precedence must not depend on the order the adapter happened to emit them.
    let forward = "model[thinking=true,effort=high]";
    let reversed = "model[effort=high,thinking=true]";
    assert_eq!(advertised_effort(forward).as_deref(), Some("high"));
    assert_eq!(advertised_effort(reversed).as_deref(), Some("high"));
    assert_eq!(badges(forward), vec!["High"]);
    assert_eq!(badges(reversed), vec!["High"]);
    // Only the marker present, it is still reported so a caller can tell the
    // model reasons even though no level is named.
    assert_eq!(
        advertised_effort("model[thinking=true]").as_deref(),
        Some("true")
    );
}
