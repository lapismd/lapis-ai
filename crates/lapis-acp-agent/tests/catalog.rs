//! Runner catalog behaviour: declaration, tiering, resolution and environment.

use lapis_acp_agent::availability::{self, FixedProbe, PathProbe};
use lapis_acp_agent::config;
use lapis_acp_agent::definition::{CommandSpec, McpTransport, RunnerDefinition};
use lapis_acp_agent::environment::{RESERVED_PREFIX, harness_variables, plan};
use lapis_acp_agent::{Catalog, Error, Tier, valid_id};
use lapis_acp_protocol::EnvVar;

fn runner(id: &str) -> RunnerDefinition {
    RunnerDefinition::new(id, id.to_uppercase(), CommandSpec::new(format!("{id}-bin")))
        .expect("valid declaration")
}

#[test]
fn a_runner_is_declared_entirely_as_data() {
    let definition = RunnerDefinition::try_from_parts(
        "my-agent".to_owned(),
        "My Agent".to_owned(),
        vec![CommandSpec::with_args("my-agent-bin", &["acp"])],
        vec![EnvVar::new("MODE", "acp")],
        McpTransport::Http,
    )
    .expect("valid")
    .with_install("download it", "https://example.invalid/docs");
    assert_eq!(definition.id, "my-agent");
    assert_eq!(definition.label, "My Agent");
    assert!(definition.enabled);
    assert_eq!(definition.commands[0].args, vec!["acp".to_owned()]);
    assert_eq!(definition.env, vec![EnvVar::new("MODE", "acp")]);
    assert_eq!(definition.mcp_transport, McpTransport::Http);
    assert_eq!(definition.install_hint.as_deref(), Some("download it"));
    assert_eq!(
        definition.install_instructions_url.as_deref(),
        Some("https://example.invalid/docs")
    );
}

#[test]
fn identifiers_are_pattern_checked_and_length_bounded() {
    for id in ["codex", "a", "my-agent", "my_agent", "v1.2", "x9"] {
        assert!(valid_id(id), "{id} should be accepted");
    }
    for id in [
        "",
        "-lead",
        ".lead",
        "Upper",
        "spa ce",
        "slash/id",
        &"a".repeat(65),
    ] {
        assert!(!valid_id(id), "{id:?} should be refused");
    }
    assert!(matches!(
        RunnerDefinition::new("Bad Id", "label", CommandSpec::new("cmd")),
        Err(Error::InvalidId(_))
    ));
}

#[test]
fn labels_and_commands_are_validated() {
    assert!(matches!(
        RunnerDefinition::new("ok", "   ", CommandSpec::new("cmd")),
        Err(Error::InvalidLabel(_))
    ));
    assert!(matches!(
        RunnerDefinition::new("ok", "x".repeat(101), CommandSpec::new("cmd")),
        Err(Error::InvalidLabel(_))
    ));
    assert!(matches!(
        RunnerDefinition::new("ok", "label", CommandSpec::new("  ")),
        Err(Error::EmptyCommand(_))
    ));
    assert!(matches!(
        RunnerDefinition::try_from_parts(
            "ok".to_owned(),
            "label".to_owned(),
            vec![],
            vec![],
            McpTransport::Stdio
        ),
        Err(Error::EmptyCommand(_))
    ));
    // A label is trimmed rather than rejected for surrounding whitespace.
    assert_eq!(
        RunnerDefinition::new("ok", "  Padded  ", CommandSpec::new("cmd"))
            .expect("valid")
            .label,
        "Padded"
    );
}

#[test]
fn the_compiled_in_tier_reserves_its_identifiers() {
    let catalog = Catalog::built_in().expect("built-in catalog");
    for id in ["codex", "claude", "opencode", "goose", "cursor"] {
        assert!(catalog.resolve(id).is_ok(), "{id} must resolve");
        assert_eq!(catalog.tier_of(id), Some(Tier::Compiled));
    }
    assert_eq!(catalog.list().len(), 5);

    let mut extended = Catalog::built_in().unwrap();
    assert!(matches!(
        extended.extend(Tier::ConfigFile, vec![runner("codex")]),
        Err(Error::Reserved(id)) if id == "codex"
    ));
    assert!(matches!(
        extended.extend(Tier::Caller, vec![runner("goose")]),
        Err(Error::Reserved(_))
    ));
}

#[test]
fn no_tier_may_claim_an_identifier_twice() {
    let mut catalog = Catalog::empty();
    catalog
        .extend(Tier::ConfigFile, vec![runner("first")])
        .expect("first");
    assert!(matches!(
        catalog.extend(Tier::Caller, vec![runner("first")]),
        Err(Error::Duplicate(id)) if id == "first"
    ));
    // A collision inside one batch also fails, and fails the whole batch rather
    // than quietly producing a smaller runner set.
    assert!(matches!(
        catalog.extend(Tier::Caller, vec![runner("second"), runner("second")]),
        Err(Error::Duplicate(_))
    ));
    assert!(catalog.resolve("second").is_err());
    assert_eq!(catalog.list().len(), 1);
}

#[test]
fn unknown_and_disabled_runners_fail_without_a_default_fallback() {
    let mut catalog = Catalog::built_in().unwrap();
    catalog
        .extend(Tier::ConfigFile, vec![runner("present").disabled()])
        .expect("extend");
    assert!(matches!(
        catalog.resolve("absent"),
        Err(Error::Unknown(id)) if id == "absent"
    ));
    assert!(matches!(
        catalog.resolve("present"),
        Err(Error::Disabled(id)) if id == "present"
    ));
    // A disabled runner is still listed as an entry, so an operator can see why
    // it was refused rather than wondering whether it exists.
    assert_eq!(catalog.entries().len(), 6);
    assert_eq!(catalog.list().len(), 5);
}

#[test]
fn reserved_environment_keys_are_stripped_from_every_layer() {
    let reserved = format!("{RESERVED_PREFIX}RUNNER");
    let inherited = vec![
        ("PATH".to_owned(), "/bin".to_owned()),
        (reserved.clone(), "spoofed".to_owned()),
    ];
    let floor = vec![
        EnvVar::new("MODE", "acp"),
        EnvVar::new(&reserved, "also-spoofed"),
    ];
    let overrides = vec![
        ("MODE".to_owned(), "override".to_owned()),
        ("JJ_ACP_MODEL".to_owned(), "chosen".to_owned()),
        (format!("{RESERVED_PREFIX}ID"), "spoofed".to_owned()),
    ];
    let plan = plan(&inherited, &floor, &overrides);
    assert_eq!(plan.get("PATH"), Some("/bin"));
    assert_eq!(plan.get("MODE"), Some("override"));
    assert_eq!(plan.get("JJ_ACP_MODEL"), Some("chosen"));
    assert_eq!(plan.get(&reserved), None);
    assert_eq!(
        plan.stripped,
        vec![reserved.clone(), format!("{RESERVED_PREFIX}ID")]
    );
}

#[test]
fn environment_layers_apply_inherited_then_floor_then_overrides() {
    let plan = plan(
        &[("A".to_owned(), "inherited".to_owned())],
        &[EnvVar::new("A", "floor"), EnvVar::new("B", "floor")],
        &[("B".to_owned(), "override".to_owned())],
    );
    assert_eq!(plan.get("A"), Some("floor"));
    assert_eq!(plan.get("B"), Some("override"));
    // First appearance fixes position, so the order is deterministic.
    let names: Vec<&str> = plan
        .variables
        .iter()
        .map(|variable| variable.name.as_str())
        .collect();
    assert_eq!(names, vec!["A", "B"]);
    assert!(plan.stripped.is_empty());
}

#[test]
fn harness_variables_are_produced_only_by_the_harness() {
    let with_task = harness_variables(Some("task-1"), "codex");
    assert_eq!(with_task.len(), 2);
    assert_eq!(with_task[0].name, format!("{RESERVED_PREFIX}RUNNER"));
    assert_eq!(with_task[0].value, "codex");
    assert_eq!(with_task[1].name, format!("{RESERVED_PREFIX}ID"));
    assert_eq!(with_task[1].value, "task-1");
    assert_eq!(harness_variables(None, "codex").len(), 1);
    // Whatever the harness sets cannot be contradicted by a declaration, because
    // every layer strips reserved names before the harness adds its own.
    let plan = plan(&[], &with_task, &[]);
    assert!(plan.variables.is_empty());
    assert_eq!(plan.stripped.len(), 2);
}

#[test]
fn availability_resolves_the_first_candidate_that_exists() {
    let definition = RunnerDefinition::try_from_parts(
        "legacy".to_owned(),
        "Legacy".to_owned(),
        vec![
            CommandSpec::new("current-bin"),
            CommandSpec::new("legacy-bin"),
        ],
        vec![],
        McpTransport::Stdio,
    )
    .expect("valid");
    let only_legacy = FixedProbe::new(["legacy-bin"]);
    let report = availability::probe(&definition, &only_legacy);
    assert!(report.available());
    assert_eq!(
        report.resolved.expect("resolved").command,
        "legacy-bin".to_owned()
    );

    let both = FixedProbe::new(["current-bin", "legacy-bin"]);
    assert_eq!(
        availability::probe(&definition, &both)
            .resolved
            .expect("resolved")
            .command,
        "current-bin".to_owned()
    );
}

#[test]
fn an_unavailable_runner_reports_guidance_rather_than_installing() {
    let definition = RunnerDefinition::new("absent", "Absent", CommandSpec::new("absent-bin"))
        .expect("valid")
        .with_install("install it yourself", "https://example.invalid");
    let report = availability::probe(&definition, &FixedProbe::default());
    assert!(!report.available());
    assert!(report.resolved.is_none());
    assert_eq!(report.install_hint.as_deref(), Some("install it yourself"));
    assert_eq!(report.runner_id, "absent");
    assert_eq!(report.label, "Absent");
}

#[test]
fn probing_a_whole_catalog_reports_every_enabled_runner() {
    let catalog = Catalog::built_in().unwrap();
    let reports = availability::probe_all(catalog.list(), &FixedProbe::new(["codex-acp", "goose"]));
    assert_eq!(reports.len(), 5);
    let available: Vec<&str> = reports
        .iter()
        .filter(|report| report.available())
        .map(|report| report.runner_id.as_str())
        .collect();
    assert_eq!(available, vec!["codex", "goose"]);
}

#[test]
fn the_real_probe_needs_a_filesystem_hit_and_never_executes() {
    // `sh` exists on every platform this repository qualifies on; a name that
    // cannot exist resolves to nothing rather than being run.
    let probe = PathProbe;
    assert!(lapis_acp_agent::CommandProbe::resolve(&probe, "sh").is_some());
    assert!(lapis_acp_agent::CommandProbe::resolve(&probe, "").is_none());
    assert!(
        lapis_acp_agent::CommandProbe::resolve(&probe, "definitely-not-installed-9f3a").is_none()
    );
}

#[test]
fn a_catalog_document_declares_runners_without_executing_anything() {
    let document = r#"{
      "runners": [
        {"id": "my-agent", "label": "My Agent", "command": "my-agent-bin",
         "args": ["acp"], "env": {"MODE": "acp"}, "mcpTransport": "stdio",
         "installHint": "download it", "installInstructionsUrl": "https://example.invalid"},
        {"id": "argv-style", "label": "Argv", "command": ["runner", "--acp"]},
        {"id": "with-alternate", "label": "Alt", "command": "primary",
         "alternateCommands": ["fallback"]},
        {"id": "off", "label": "Off", "command": "off-bin", "enabled": false}
      ]
    }"#;
    let definitions = config::parse_catalog_text(document).expect("valid catalog");
    assert_eq!(definitions.len(), 4);

    let first = &definitions[0];
    assert_eq!(
        first.commands,
        vec![CommandSpec::with_args("my-agent-bin", &["acp"])]
    );
    assert_eq!(first.env, vec![EnvVar::new("MODE", "acp")]);
    assert_eq!(first.install_hint.as_deref(), Some("download it"));

    assert_eq!(
        definitions[1].commands,
        vec![CommandSpec::with_args("runner", &["--acp"])]
    );
    assert_eq!(
        definitions[2].commands,
        vec![CommandSpec::new("primary"), CommandSpec::new("fallback")]
    );
    assert!(!definitions[3].enabled);

    let mut catalog = Catalog::built_in().unwrap();
    catalog
        .extend(Tier::ConfigFile, definitions)
        .expect("extends");
    assert_eq!(catalog.list().len(), 8);
}

#[test]
fn catalog_parsing_rejects_malformed_declarations_explicitly() {
    let cases: Vec<(&str, &str)> = vec![
        ("{}", "runners array is required"),
        (r#"{"runners": []}"#, "at least one runner"),
        (r#"{"runners": {}}"#, "must be an array"),
        (
            r#"{"runners": [{"label": "x", "command": "c"}]}"#,
            "string id",
        ),
        (r#"{"runners": [{"id": "a", "command": "c"}]}"#, "label"),
        (
            r#"{"runners": [{"id": "a", "label": "A"}]}"#,
            "command is required",
        ),
        (
            r#"{"runners": [{"id": "a", "label": "A", "command": 7}]}"#,
            "string or an array",
        ),
        (
            r#"{"runners": [{"id": "a", "label": "A", "command": []}]}"#,
            "argv is empty",
        ),
        (
            r#"{"runners": [{"id": "a", "label": "A", "command": "c", "args": "x"}]}"#,
            "args must be an array",
        ),
        (
            r#"{"runners": [{"id": "a", "label": "A", "command": "c", "env": 7}]}"#,
            "env must be an object",
        ),
        (
            r#"{"runners": [{"id": "a", "label": "A", "command": "c", "mcpTransport": "socket"}]}"#,
            "transport",
        ),
        (
            r#"{"runners": [{"id": "a", "label": "A", "command": "c", "alternateCommands": ["x"], "args": ["y"]}]}"#,
            "cannot be combined",
        ),
        ("not json", "invalid JSON"),
    ];
    for (document, expected) in cases {
        let error = config::parse_catalog_text(document).expect_err(document);
        assert!(
            error.to_string().contains(expected),
            "{document} produced {error}, expected {expected}"
        );
    }
}

#[test]
fn cursor_is_declared_with_the_transport_it_actually_advertises() {
    let catalog = Catalog::built_in().unwrap();
    let cursor = catalog.resolve("cursor").expect("cursor is compiled in");
    assert_eq!(
        cursor.commands,
        vec![CommandSpec::with_args("cursor-agent", &["acp"])]
    );
    // This adapter offers MCP over http and sse and not over stdio, so declaring
    // stdio would hand it a server configuration it cannot use.
    assert_eq!(cursor.mcp_transport, McpTransport::Http);
    assert_eq!(catalog.tier_of("cursor"), Some(Tier::Compiled));
    // Reserved like the others: a catalog file cannot redefine it.
    let mut extended = Catalog::built_in().unwrap();
    assert!(matches!(
        extended.extend(Tier::ConfigFile, vec![runner("cursor")]),
        Err(Error::Reserved(id)) if id == "cursor"
    ));
}

#[test]
fn mcp_transport_defaults_to_stdio_and_rejects_unknown_spellings() {
    assert_eq!(McpTransport::parse("stdio"), Some(McpTransport::Stdio));
    assert_eq!(McpTransport::parse("http"), Some(McpTransport::Http));
    assert_eq!(McpTransport::parse("socket"), None);
    assert_eq!(McpTransport::Stdio.as_str(), "stdio");
    let parsed =
        config::parse_catalog_text(r#"{"runners": [{"id": "a", "label": "A", "command": "c"}]}"#)
            .expect("valid");
    assert_eq!(parsed[0].mcp_transport, McpTransport::Stdio);
}
