//! Bundled upstream ACP model/tool loop, with application-owned launch policy.
#[path = "cli/chatgpt_token.rs"]
mod chatgpt_token;
#[path = "cli/metadata.rs"]
mod metadata;
#[path = "cli/provider_catalog.rs"]
mod provider_catalog;
fn main() {
    if let Some(code) = lapis_ai_config::launch::cli(lapis_ai_config::Component::Agent) {
        std::process::exit(code);
    }
    if std::env::args().nth(1).as_deref() == Some("--metadata") {
        metadata::run();
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--models") {
        provider_catalog::run();
        return;
    }
    let result = match chatgpt_token::source() {
        Ok(Some(source)) => lapis_agent::run_with_local_catalog_and_token_source(source),
        Ok(None) => lapis_agent::run_with_local_catalog(),
        Err(e) => Err(e.into()),
    };
    if result.is_err() {
        eprintln!("local agent unavailable");
        std::process::exit(1);
    }
}
