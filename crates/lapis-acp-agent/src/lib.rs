//! Pluggable runner catalog and launch environment.
//!
//! A runner is an external ACP-speaking agent. This crate owns runner
//! declarations, catalog tiering, executable resolution and the environment a
//! spawned agent receives. Adding a runner is a data change: nothing here can
//! execute a shell command, fetch anything or install anything.
//!
//! The crate never spawns a process, schedules a turn or opens a socket. It
//! answers two questions — which runners exist, and what environment should one
//! receive — and leaves acting on those answers to `lapis-acp-runner`.
//!
//! ```
//! use lapis_acp_agent::{Catalog, FixedProbe, Tier, availability};
//!
//! let catalog = Catalog::built_in().expect("compiled-in catalog");
//! let probe = FixedProbe::new(["codex-acp"]);
//! let report = availability::probe(catalog.resolve("codex").expect("codex"), &probe);
//! assert!(report.available());
//! assert!(catalog.resolve("claude").is_ok());
//! assert!(!availability::probe(catalog.resolve("claude").unwrap(), &probe).available());
//! // A reserved identifier cannot be claimed by a lower tier.
//! let mut extended = Catalog::built_in().unwrap();
//! assert!(extended.extend(Tier::Caller, vec![]).is_ok());
//! ```

#![forbid(unsafe_code)]

pub mod availability;
pub mod catalog;
pub mod config;
pub mod defaults;
pub mod definition;
pub mod environment;
pub mod error;

pub use availability::{Availability, CommandProbe, FixedProbe, PathProbe};
pub use catalog::{Catalog, Entry, Tier};
pub use defaults::{RESERVED_IDS, built_in, is_reserved};
pub use definition::{
    CommandSpec, ID_MAX_LENGTH, LABEL_MAX_LENGTH, McpTransport, RunnerDefinition, valid_id,
};
pub use environment::{EnvironmentPlan, RESERVED_PREFIX, harness_variables, is_reserved_env};
pub use error::{Error, Result};
