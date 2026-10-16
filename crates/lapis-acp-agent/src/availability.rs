//! Executable resolution and availability reporting.
//!
//! Availability is determined by resolving a declared executable. It is never
//! determined by installing one: no tier may auto-install, and nothing here
//! executes a shell command or performs a network fetch.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::definition::{CommandSpec, RunnerDefinition};

/// Locates an executable, so tests can inject a synthetic search path.
pub trait CommandProbe {
    /// The resolved path for a command name, or `None` when unavailable.
    fn resolve(&self, command: &str) -> Option<PathBuf>;
}

/// Resolves against the process `PATH`, or directly for an absolute command.
#[derive(Debug, Clone, Copy, Default)]
pub struct PathProbe;

fn executable(path: &Path) -> bool {
    path.is_file()
}

impl CommandProbe for PathProbe {
    fn resolve(&self, command: &str) -> Option<PathBuf> {
        if command.is_empty() {
            return None;
        }
        if command.contains('/') {
            let direct = PathBuf::from(command);
            return executable(&direct).then_some(direct);
        }
        for directory in std::env::split_paths(&std::env::var_os("PATH")?) {
            let candidate = directory.join(command);
            if executable(&candidate) {
                return Some(candidate);
            }
        }
        None
    }
}

/// Resolves only a fixed set of names, for deterministic tests.
#[derive(Debug, Clone, Default)]
pub struct FixedProbe {
    available: BTreeSet<String>,
}

impl FixedProbe {
    /// Build a probe that resolves exactly these command names.
    pub fn new<I, S>(available: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            available: available.into_iter().map(Into::into).collect(),
        }
    }
}

impl CommandProbe for FixedProbe {
    fn resolve(&self, command: &str) -> Option<PathBuf> {
        self.available
            .contains(command)
            .then(|| PathBuf::from("/synthetic").join(command))
    }
}

/// Whether a runner can be launched, and what to tell an operator if not.
#[derive(Debug, Clone, PartialEq)]
pub struct Availability {
    /// The runner this report describes.
    pub runner_id: String,
    /// Its human-readable label.
    pub label: String,
    /// The first candidate argv that resolved, when one did.
    pub resolved: Option<CommandSpec>,
    /// Operator guidance when nothing resolved.
    pub install_hint: Option<String>,
    /// Documentation link when nothing resolved.
    pub install_instructions_url: Option<String>,
}

impl Availability {
    /// Report whether this runner can be launched.
    pub fn available(&self) -> bool {
        self.resolved.is_some()
    }
}

/// Resolve a runner's candidates in preference order.
///
/// The first candidate whose executable resolves wins, so a definition can list a
/// current command name ahead of a legacy one and work with either install.
pub fn probe(definition: &RunnerDefinition, resolver: &dyn CommandProbe) -> Availability {
    let resolved = definition.commands.iter().find_map(|spec| {
        resolver
            .resolve(&spec.command)
            .map(|_| CommandSpec::with_args(&spec.command, &spec.args.iter().collect::<Vec<_>>()))
    });
    Availability {
        runner_id: definition.id.clone(),
        label: definition.label.clone(),
        resolved,
        install_hint: definition.install_hint.clone(),
        install_instructions_url: definition.install_instructions_url.clone(),
    }
}

/// Resolve every enabled runner in a catalog.
pub fn probe_all<'a>(
    definitions: impl IntoIterator<Item = &'a RunnerDefinition>,
    resolver: &dyn CommandProbe,
) -> Vec<Availability> {
    definitions
        .into_iter()
        .map(|definition| probe(definition, resolver))
        .collect()
}
