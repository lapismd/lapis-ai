use lapis_ai_app_tools::{Config, Error, Tool};
use serde_json::json;

#[test]
fn hosts_can_validate_snapshots_before_allocating_transport() {
    let config = Config::default();
    let tool = Tool {
        name: "notes.read".into(),
        description: "Read an admitted note".into(),
        input_schema: json!({"type":"object"}),
    };
    assert!(config.validate(std::slice::from_ref(&tool)).is_ok());
    assert!(matches!(
        config.validate(&[tool.clone(), tool]),
        Err(Error::Configuration)
    ));
    let invalid = Config {
        maximum_calls: 9,
        ..config
    };
    assert!(matches!(invalid.validate(&[]), Err(Error::Configuration)));
}
