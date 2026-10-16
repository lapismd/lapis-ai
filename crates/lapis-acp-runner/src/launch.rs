//! Spawning an agent subprocess.
//!
//! The child gets its own process group on Unix so that cleanup can reach
//! descendants the agent spawned. Without that, killing the agent leaves its
//! children running, which the fixture peer's descendant mode demonstrates
//! concretely rather than hypothetically.

use std::path::PathBuf;
use std::process::Stdio;

use lapis_acp_agent::{CommandSpec, EnvironmentPlan};
use lapis_acp_protocol::EnvVar;
use tokio::process::{Child, Command};

/// Everything needed to launch one agent.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchSpec {
    /// The resolved argv.
    pub command: CommandSpec,
    /// The assembled launch environment.
    pub environment: EnvironmentPlan,
    /// Harness-reserved variables, applied last and produced only by the harness.
    pub harness_env: Vec<EnvVar>,
    /// Working directory. This comes from trusted launch configuration and is
    /// never taken from a task document.
    pub cwd: Option<PathBuf>,
}

impl LaunchSpec {
    /// A launch specification with no reserved variables and the current
    /// directory.
    pub fn new(command: CommandSpec, environment: EnvironmentPlan) -> Self {
        Self {
            command,
            environment,
            harness_env: Vec::new(),
            cwd: None,
        }
    }
}

/// Spawn the agent with piped stdio and its own process group.
///
/// The child's stderr is inherited rather than piped, so agent diagnostics reach
/// the operator directly and can never be mistaken for protocol frames on stdout.
/// Nothing here logs the environment: it can carry credentials the controlling
/// application supplied, and the spawn is the one place they are used.
pub fn spawn(spec: &LaunchSpec) -> std::io::Result<Child> {
    spawn_with_stderr(spec, Stdio::inherit())
}

/// Interactive parents receive classified diagnostics; raw child stderr is discarded.
pub fn spawn_quiet(spec: &LaunchSpec) -> std::io::Result<Child> {
    spawn_with_stderr(spec, Stdio::null())
}

fn spawn_with_stderr(spec: &LaunchSpec, stderr: Stdio) -> std::io::Result<Child> {
    let mut command = Command::new(&spec.command.command);
    command
        .args(&spec.command.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(stderr)
        .kill_on_drop(true);
    if let Some(cwd) = &spec.cwd {
        command.current_dir(cwd);
    }
    for variable in spec
        .environment
        .variables
        .iter()
        .chain(spec.harness_env.iter())
    {
        command.env(&variable.name, &variable.value);
    }
    #[cfg(unix)]
    command.process_group(0);
    command.spawn()
}

/// The command name, for diagnostics.
///
/// Only the executable is reported. Arguments can carry configuration and the
/// environment can carry credentials, so neither belongs in a message.
pub fn describe(command: &CommandSpec) -> String {
    command.command.clone()
}
