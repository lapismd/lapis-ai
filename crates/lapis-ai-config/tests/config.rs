use lapis_ai_config::{Component, Config, Resolver};
use std::path::Path;
struct Secrets;
impl Resolver for Secrets {
    fn environment(&self, name: &str) -> Option<String> {
        (name == "API_TOKEN").then(|| "private-value".into())
    }
    fn host_secret(&self, name: &str) -> Option<String> {
        (name == "identity").then(|| "host-value".into())
    }
}
#[test]
fn rejects_unknown_fields_versions_and_literal_secrets_without_disclosure() {
    for text in [
        "version=9",
        "version=1\nunknown='private-value'",
        "version=1\n[agent.environment]\nOPENAI_COMPAT_API_KEY='private-value'",
    ] {
        let error = Config::parse(text, Path::new("/safe")).unwrap_err();
        assert!(!error.to_string().contains("private-value"));
    }
}
#[test]
fn parsing_is_inert_and_credentials_resolve_only_explicitly() {
    let cfg=Config::parse("version=1\n[agent]\nprovider='openai'\n[agent.environment]\nOPENAI_COMPAT_API_KEY={env='API_TOKEN'}",Path::new("/safe")).unwrap();
    assert!(!format!("{cfg:?}").contains("API_TOKEN"));
    let env = cfg.environment(Component::Agent, &Secrets).unwrap();
    assert_eq!(env.0["OPENAI_COMPAT_API_KEY"], "private-value");
    assert!(!format!("{env:?}").contains("private-value"));
    assert!(
        cfg.environment(Component::Agent, &lapis_ai_config::SystemResolver)
            .is_err()
    );
}
#[test]
fn resolves_paths_and_preserves_existing_wire_config() {
    let cfg=Config::parse("version=1\n[runtime]\nacp='bin/lapis-acp'\ndata_dir='state'\n[compute]\nmodel='fixture'\nsharing=false",Path::new("/safe")).unwrap();
    assert_eq!(cfg.runtime.acp.unwrap(), Path::new("/safe/bin/lapis-acp"));
    assert!(!cfg.compute.sharing);
}
#[test]
fn host_secret_and_precedence_are_explicit() {
    let cfg = Config::parse(
        "version=1\n[compute]\nidentity={host='identity'}\ntoken={env='API_TOKEN'}",
        Path::new("/safe"),
    )
    .unwrap();
    let env = cfg.environment(Component::Compute, &Secrets).unwrap();
    assert_eq!(env.0["JJ_STORE_COMPUTE_IDENTITY"], "host-value");
    assert_eq!(env.0["JJ_STORE_COMPUTE_TOKEN"], "private-value");
}
#[test]
fn compute_launch_keeps_model_caches_inside_its_managed_working_directory() {
    let config = Config::parse(
        "version=1\n[runtime]\ndata_dir='state'",
        Path::new("/private-app"),
    )
    .unwrap();
    let env = config.environment(Component::Compute, &Secrets).unwrap();
    assert_eq!(
        env.0.get("HOME").map(String::as_str),
        Some("/private-app/state/compute")
    );
    for name in [
        "HF_HOME",
        "HF_HUB_CACHE",
        "HF_XET_CACHE",
        "XDG_CACHE_HOME",
        "MESH_LLM_DATA_DIR",
    ] {
        assert!(
            Path::new(&env.0[name]).starts_with("/private-app/state/compute"),
            "{name}"
        );
    }
}
