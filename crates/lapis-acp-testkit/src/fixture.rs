//! Locating and configuring the deterministic fixture agent.

use std::path::{Path, PathBuf};

use lapis_acp_agent::CommandSpec;
use lapis_acp_protocol::EnvVar;

/// Fixture path relative to the workspace root.
pub const FIXTURE_PATH: &str = "tests/fixtures/fixture_agent.py";

/// Environment variable naming the fixture's behaviour.
pub const MODE_VARIABLE: &str = "JJ_ACP_FIXTURE_MODE";

/// Environment variable naming the fixture's append-only record log.
pub const LOG_VARIABLE: &str = "JJ_ACP_FIXTURE_LOG";

/// Environment variable overriding the protocol version the fixture reports.
pub const PROTOCOL_VARIABLE: &str = "JJ_ACP_FIXTURE_PROTOCOL";

/// Environment variable overriding the stop reason the fixture returns.
pub const STOP_REASON_VARIABLE: &str = "JJ_ACP_FIXTURE_STOP_REASON";

/// Environment variable naming the Python interpreter, for hosts without one on
/// `PATH` under its default name.
pub const PYTHON_VARIABLE: &str = "JJ_ACP_FIXTURE_PYTHON";

/// Absolute path of the fixture script shipped inside this crate.
pub fn fixture_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH)
}

/// The Python interpreter used to run the fixture.
pub fn python() -> String {
    std::env::var(PYTHON_VARIABLE).unwrap_or_else(|_| "python3".to_owned())
}

/// A behaviour the fixture can be told to exhibit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Answer every method and stop with the configured reason.
    Normal,
    /// Never answer `initialize`.
    StartupHang,
    /// Never answer `session/new`.
    SessionHang,
    /// Never answer `session/prompt`.
    TurnHang,
    /// Answer nothing and ignore `session/cancel`.
    IgnoreCancel,
    /// Ask for approval instead of finishing the turn.
    Permission,
    /// Fail the turn with a message that must never be echoed to a caller.
    Error,
    /// Spawn a long-lived child at startup.
    Descendant,
    /// Spawn a long-lived child, then never answer `initialize`.
    DescendantStartup,
    /// Spawn a long-lived child, then never answer the turn.
    DescendantTurn,
    /// Exit abruptly instead of answering `initialize`.
    CrashStartup,
    /// Exit abruptly instead of answering `session/prompt`.
    CrashTurn,
}

impl Mode {
    /// The spelling the fixture expects.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::StartupHang => "startup-hang",
            Self::SessionHang => "session-hang",
            Self::TurnHang => "turn-hang",
            Self::IgnoreCancel => "ignore-cancel",
            Self::Permission => "permission",
            Self::Error => "error",
            Self::Descendant => "descendant",
            Self::DescendantStartup => "descendant-startup",
            Self::DescendantTurn => "descendant-turn",
            Self::CrashStartup => "crash-startup",
            Self::CrashTurn => "crash-turn",
        }
    }

    /// Report whether this mode spawns a child that outlives the fixture.
    pub fn spawns_descendant(self) -> bool {
        matches!(
            self,
            Self::Descendant | Self::DescendantStartup | Self::DescendantTurn
        )
    }
}

/// One configured fixture launch.
#[derive(Debug, Clone, PartialEq)]
pub struct Fixture {
    /// The behaviour to exhibit.
    pub mode: Mode,
    /// Where the fixture appends its record log.
    pub log: PathBuf,
    /// The protocol version the fixture reports.
    pub protocol: u64,
    /// The stop reason the fixture returns on a normal turn.
    pub stop_reason: String,
}

impl Fixture {
    /// Configure a fixture in its default mode writing to `log`.
    pub fn new(log: impl Into<PathBuf>) -> Self {
        Self {
            mode: Mode::Normal,
            log: log.into(),
            protocol: 2,
            stop_reason: "end_turn".to_owned(),
        }
    }

    /// Select a behaviour.
    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Select the protocol version the fixture reports.
    pub fn with_protocol(mut self, protocol: u64) -> Self {
        self.protocol = protocol;
        self
    }

    /// Select the stop reason the fixture returns.
    pub fn with_stop_reason(mut self, stop_reason: impl Into<String>) -> Self {
        self.stop_reason = stop_reason.into();
        self
    }

    /// The argv that launches this fixture.
    pub fn command(&self) -> CommandSpec {
        CommandSpec::with_args(python(), &[fixture_script().to_string_lossy()])
    }

    /// The environment the fixture needs, excluding harness-reserved names.
    ///
    /// The caller merges these as overrides, which is what proves that reserved
    /// names set by a definition are stripped while the harness still supplies
    /// its own.
    pub fn environment(&self) -> Vec<EnvVar> {
        vec![
            EnvVar::new(MODE_VARIABLE, self.mode.as_str()),
            EnvVar::new(LOG_VARIABLE, self.log.to_string_lossy()),
            EnvVar::new(PROTOCOL_VARIABLE, self.protocol.to_string()),
            EnvVar::new(STOP_REASON_VARIABLE, self.stop_reason.clone()),
        ]
    }
}
