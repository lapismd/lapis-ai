//! The compiled-in runner tier.
//!
//! These identifiers are reserved: a catalog file or a caller-supplied definition
//! cannot shadow one, so an operator always knows what `codex` means.
//!
//! An entry here is a convenience, not a claim. Each names an entrypoint taken
//! from the published adapter set, and only those recorded as qualified in
//! `docs/upstream-lock.md` and the ledger have actually been launched. Declaring an
//! argv is not evidence that it speaks ACP, which is why availability is probed
//! rather than assumed and why qualification is its own gate.

use crate::definition::{CommandSpec, McpTransport, RunnerDefinition};
use crate::error::Result;

const ACP_DOCUMENTATION: &str = "https://agentclientprotocol.com/";

/// Identifiers reserved for the compiled-in tier.
pub const RESERVED_IDS: [&str; 5] = ["codex", "claude", "opencode", "goose", "cursor"];

/// Report whether an identifier is reserved for the compiled-in tier.
pub fn is_reserved(id: &str) -> bool {
    RESERVED_IDS.contains(&id)
}

/// The Codex adapter.
pub fn codex() -> Result<RunnerDefinition> {
    Ok(RunnerDefinition::try_from_parts(
        "codex".to_owned(),
        "Codex".to_owned(),
        vec![CommandSpec::new("codex-acp")],
        Vec::new(),
        McpTransport::Stdio,
    )?
    .with_install(
        "npm install -g @agentclientprotocol/codex-acp",
        ACP_DOCUMENTATION,
    ))
}

/// The Claude adapter, accepting both current and legacy command names.
///
/// Both names describe the same zero-argument runtime, so they are candidates on
/// one definition rather than two runners. An operator with an older install
/// still resolves `claude`.
pub fn claude() -> Result<RunnerDefinition> {
    Ok(RunnerDefinition::try_from_parts(
        "claude".to_owned(),
        "Claude Code".to_owned(),
        vec![
            CommandSpec::new("claude-agent-acp"),
            CommandSpec::new("claude-code-acp"),
        ],
        Vec::new(),
        McpTransport::Stdio,
    )?
    .with_install(
        "npm install -g @agentclientprotocol/claude-agent-acp",
        ACP_DOCUMENTATION,
    ))
}

/// The OpenCode adapter.
pub fn opencode() -> Result<RunnerDefinition> {
    Ok(RunnerDefinition::try_from_parts(
        "opencode".to_owned(),
        "OpenCode".to_owned(),
        vec![
            CommandSpec::with_args("opencode", &["acp"]),
            CommandSpec::new("opencode"),
        ],
        Vec::new(),
        McpTransport::Stdio,
    )?
    .with_install("see the OpenCode documentation", ACP_DOCUMENTATION))
}

/// The Goose adapter.
pub fn goose() -> Result<RunnerDefinition> {
    Ok(RunnerDefinition::try_from_parts(
        "goose".to_owned(),
        "Goose".to_owned(),
        vec![CommandSpec::with_args("goose", &["acp"])],
        Vec::new(),
        McpTransport::Stdio,
    )?
    .with_install("see the Goose documentation", ACP_DOCUMENTATION))
}

/// The Cursor adapter.
///
/// Qualified against `cursor-agent` 2026.09.26-dd393fe: the `acp` subcommand is
/// real but hidden from the adapter's own top-level help, and the adapter
/// advertises MCP over http and sse only, so its transport is http rather than the
/// stdio most adapters use. It authenticates through the operator's existing
/// `cursor-agent login` session; this harness performs no authentication and never
/// calls the `authenticate` method the adapter offers.
pub fn cursor() -> Result<RunnerDefinition> {
    Ok(RunnerDefinition::try_from_parts(
        "cursor".to_owned(),
        "Cursor".to_owned(),
        vec![CommandSpec::with_args("cursor-agent", &["acp"])],
        Vec::new(),
        McpTransport::Http,
    )?
    .with_install(
        "install Cursor, then run: cursor-agent login",
        "https://cursor.com/docs",
    ))
}

/// Every compiled-in runner, in display order.
pub fn built_in() -> Result<Vec<RunnerDefinition>> {
    Ok(vec![codex()?, claude()?, opencode()?, goose()?, cursor()?])
}
