//! Listing resolved runners.
//!
//! Emits one JSON object per line so a controlling application can read the
//! catalog without scraping prose. Availability is a filesystem question answered
//! by probing, never by installing: nothing here executes a candidate.

use lapis_acp_agent::availability::{self, PathProbe};
use lapis_acp_agent::config as catalog_config;
use lapis_acp_agent::{Catalog, Tier};
use serde_json::json;

use crate::args::LaunchOptions;

/// Print the resolved catalog, returning the process exit code.
pub fn execute(options: &LaunchOptions) -> i32 {
    let catalog = match build(options) {
        Ok(catalog) => catalog,
        Err(message) => {
            eprintln!("lapis-acp: {message}");
            return 2;
        }
    };
    for report in availability::probe_all(catalog.list(), &PathProbe) {
        let line = json!({
            "id": report.runner_id,
            "label": report.label,
            "available": report.available(),
            "command": report.resolved.as_ref().map(|spec| spec.command.clone()),
            "args": report.resolved.as_ref().map(|spec| spec.args.clone()),
            "installHint": report.install_hint,
            "installInstructionsUrl": report.install_instructions_url,
        });
        println!("{line}");
    }
    0
}

fn build(options: &LaunchOptions) -> std::result::Result<Catalog, String> {
    let mut catalog = Catalog::built_in().map_err(|error| error.to_string())?;
    let Some(path) = &options.catalog else {
        return Ok(catalog);
    };
    let definitions = catalog_config::read_catalog(std::path::Path::new(path))
        .map_err(|error| error.to_string())?;
    catalog
        .extend(Tier::ConfigFile, definitions)
        .map_err(|error| error.to_string())?;
    Ok(catalog)
}
