//! Deterministic ACP peer fixtures for integration tests.
//!
//! This crate locates the scripted fixture agent, builds the argv and environment
//! that launch it in a chosen failure mode, and reads back the record log it
//! writes. It exists so that framing, deadlines, cancellation, cleanup,
//! sanitization and exit codes can all be proven without a model provider, a
//! network endpoint or a credential.
//!
//! This is test infrastructure. No production crate may depend on it, and
//! `scripts/check-boundaries.mjs` fails closed when one does.
//!
//! ```
//! use lapis_acp_testkit::{Fixture, FixtureLog, Mode};
//!
//! let log = FixtureLog::new("/tmp/lapis-acp-example.log");
//! let fixture = Fixture::new(log.path().to_path_buf()).with_mode(Mode::StartupHang);
//! assert_eq!(fixture.command().args.len(), 1);
//! assert!(fixture.mode.spawns_descendant() == false);
//! assert!(fixture
//!     .environment()
//!     .iter()
//!     .any(|variable| variable.value == "startup-hang"));
//! ```

#![forbid(unsafe_code)]

pub mod fixture;
pub mod log;

pub use fixture::{
    FIXTURE_PATH, Fixture, LOG_VARIABLE, MODE_VARIABLE, Mode, PROTOCOL_VARIABLE, PYTHON_VARIABLE,
    STOP_REASON_VARIABLE, fixture_script, python,
};
pub use log::FixtureLog;
