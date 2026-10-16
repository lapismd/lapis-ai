//! Argument and environment parsing.
//!
//! Every launch option is settable by a flag and by a matching environment
//! variable, with the flag winning, so a controlling application can configure a
//! run either way. Parsing is hand-rolled: the surface is small, and depending on
//! a parser would not reduce it.

use crate::argument_values::{
    count, next_value, optional_count, optional_duration, reject_trailing,
};
use std::path::PathBuf;
use std::time::Duration;

use lapis_acp_control::TaskSource;
use lapis_acp_protocol::session::SystemPromptTransport;

/// Default idle window, matching the upstream harness.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(620);

/// Default absolute turn cap, matching the upstream harness.
pub const DEFAULT_MAX_TURN_DURATION: Duration = Duration::from_secs(7200);

/// Default cooperative cancellation grace.
pub const DEFAULT_CANCEL_GRACE: Duration = Duration::from_secs(5);

/// Default reap bound after a kill.
pub const DEFAULT_REAP_TIMEOUT: Duration = Duration::from_secs(5);

/// Default concurrent submissions for `serve`.
///
/// One worker keeps the serial behaviour a controlling application had before
/// scopes existed, so raising concurrency is an explicit choice.
pub const DEFAULT_WORKERS: usize = 1;

/// Default bound on waiting submissions for `serve`.
pub const DEFAULT_QUEUE_CAPACITY: usize = 16;

/// Highest permitted worker count.
pub const MAX_WORKERS: usize = 32;

/// A parsing refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgError {
    /// The command line was malformed; no work was attempted.
    Usage(String),
    /// A value was present but unusable.
    Invalid(String),
}

impl std::fmt::Display for ArgError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) | Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

/// Trusted launch configuration assembled from flags and environment.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchOptions {
    /// Explicit controlling application protocol; one-shot tasks always use v1.
    pub control_version: u64,
    /// The runner this process acts as, pinning which tasks it will accept.
    pub runner: Option<String>,
    /// Path to a catalog file supplying additional runners.
    pub catalog: Option<String>,
    /// Trusted file containing bounded stdio MCP server declarations.
    pub tools: Option<String>,
    /// Model to request when the agent offers it.
    pub model: Option<String>,
    /// Reasoning effort to request when the agent offers one.
    pub effort: Option<String>,
    /// Execution mode to request when the agent offers one.
    pub mode: Option<String>,
    /// Idle window reset by agent output.
    pub idle_timeout: Duration,
    /// Absolute cap a task document can lower but never raise.
    pub max_turn_duration: Duration,
    /// Cooperative cancellation grace before a kill.
    pub cancel_grace: Duration,
    /// Reap bound after a kill.
    pub reap_timeout: Duration,
    /// File holding standing instructions.
    pub system_prompt_file: Option<String>,
    /// Where the adapter expects standing instructions.
    pub prompt_transport: SystemPromptTransport,
    /// Working directory; defaults to the launch directory.
    pub cwd: Option<PathBuf>,
    /// How many submissions `serve` may execute at once.
    pub workers: usize,
    /// How many submissions `serve` may queue before refusing one.
    pub queue_capacity: usize,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            control_version: 1,
            runner: None,
            catalog: None,
            tools: None,
            model: None,
            effort: None,
            mode: None,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            max_turn_duration: DEFAULT_MAX_TURN_DURATION,
            cancel_grace: DEFAULT_CANCEL_GRACE,
            reap_timeout: DEFAULT_REAP_TIMEOUT,
            system_prompt_file: None,
            prompt_transport: SystemPromptTransport::Field,
            cwd: None,
            workers: DEFAULT_WORKERS,
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
        }
    }
}

/// A parsed `run` invocation.
///
/// Boxed inside [`Command`] so the small variants do not pay for the launch
/// configuration's size on every match.
#[derive(Debug, Clone, PartialEq)]
pub struct RunCommand {
    /// Where the task document comes from.
    pub task: TaskSource,
    /// Trusted launch configuration.
    pub launch: LaunchOptions,
}

/// A parsed command.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Execute one prepared task.
    Run(Box<RunCommand>),
    /// List resolved runners with availability, honouring a catalog file.
    Agents(Box<LaunchOptions>),
    /// Serve the persistent control protocol over stdio.
    Serve(Box<LaunchOptions>),
    /// Print usage.
    Help,
    /// Print the version.
    Version,
}

/// Read one environment variable, injectable so parsing is testable.
pub type Env = dyn Fn(&str) -> Option<String>;

pub(crate) fn duration(value: &str, option: &str) -> Result<Duration, ArgError> {
    let seconds: u64 = value
        .trim()
        .parse()
        .map_err(|_| ArgError::Invalid(format!("{option} must be a whole number of seconds")))?;
    if seconds == 0 {
        return Err(ArgError::Invalid(format!("{option} must be positive")));
    }
    Ok(Duration::from_secs(seconds))
}

fn transport(value: &str) -> Result<SystemPromptTransport, ArgError> {
    match value {
        "field" => Ok(SystemPromptTransport::Field),
        "meta" => Ok(SystemPromptTransport::Meta),
        "meta-append" => Ok(SystemPromptTransport::MetaAppend),
        other => Err(ArgError::Invalid(format!(
            "--prompt-transport must be field, meta or meta-append, not {other:?}"
        ))),
    }
}

/// The usage text, printed to stdout for `--help` and stderr for a refusal.
pub const USAGE: &str = "\
lapis-acp — run an Agent Client Protocol agent for a controlling application

Usage:
  lapis-acp run --task <path|-> [options]
  lapis-acp serve [options]
  lapis-acp agents [options]
  lapis-acp --help | --version

Serve reads newline-delimited JSON control frames on stdin and writes frames on
stdout: submit, cancel, runners and shutdown in; ready, record, runners, ack,
error and diagnostic out. There is no listener, no port and no authentication.

Run options:
  --task <path|->            prepared task document; '-' reads stdin
  --runner <id>              runner this process acts as (JJ_ACP_RUNNER)
  --tools <path>             trusted stdio MCP server declarations (JJ_ACP_TOOLS)
  --catalog <path>           additional runner catalog (JJ_ACP_CATALOG)
  --model <id>               model to request (JJ_ACP_MODEL)
  --effort <level>           reasoning effort to request (JJ_ACP_EFFORT)
  --mode <id>                execution mode to request (JJ_ACP_MODE)
  --cwd <path>               working directory (JJ_ACP_CWD)
  --idle-timeout <secs>      idle bound (JJ_ACP_IDLE_TIMEOUT)
  --max-turn-duration <secs> absolute cap (JJ_ACP_MAX_TURN_DURATION)
  --cancel-grace <secs>      cooperative grace (JJ_ACP_CANCEL_GRACE)
  --reap-timeout <secs>      reap bound (JJ_ACP_REAP_TIMEOUT)
  --system-prompt-file <p>   standing instructions (JJ_ACP_SYSTEM_PROMPT_FILE)
  --prompt-transport <kind>  field | meta | meta-append (JJ_ACP_PROMPT_TRANSPORT)
  --control-version <1|2|3> select interactive or warm worker control explicitly
  --workers <n>              concurrent submissions for serve (JJ_ACP_WORKERS)
  --queue-capacity <n>       waiting submissions for serve (JJ_ACP_QUEUE_CAPACITY)

A requested model, effort or mode that the adapter does not offer is reported on
stderr and the adapter's own default is kept; it never fails the run.

Credentials, the executable and the working directory come only from this trusted
launch configuration. A task document cannot change them.";

/// Parse a command line against an environment lookup.
pub fn parse<I: IntoIterator<Item = String>>(arguments: I, env: &Env) -> Result<Command, ArgError> {
    let mut items = arguments.into_iter().collect::<Vec<String>>().into_iter();
    let Some(first) = items.next() else {
        return Ok(Command::Help);
    };
    match first.as_str() {
        "--help" | "-h" | "help" => {
            reject_trailing(&mut items)?;
            return Ok(Command::Help);
        }
        "--version" | "-V" => {
            reject_trailing(&mut items)?;
            return Ok(Command::Version);
        }
        // `agents` inspects a catalog, so it parses the same launch options as
        // `run` rather than falling back to defaults and silently reporting a
        // smaller runner set than the operator configured.
        "agents" => {
            let (launch, task) = parse_launch(&mut items, env)?;
            if task.is_some() {
                return Err(ArgError::Usage("agents does not take --task".to_owned()));
            }
            return Ok(Command::Agents(Box::new(launch)));
        }
        // `serve` takes a task per control frame, so a launch-time task document
        // would be meaningless and is refused rather than ignored.
        "serve" => {
            let (launch, task) = parse_launch(&mut items, env)?;
            if task.is_some() {
                return Err(ArgError::Usage("serve does not take --task".to_owned()));
            }
            return Ok(Command::Serve(Box::new(launch)));
        }
        "run" => {}
        other => {
            return Err(ArgError::Usage(format!(
                "unknown command {other:?}; expected run, serve, agents, --help or --version"
            )));
        }
    }

    let (launch, task) = parse_launch(&mut items, env)?;
    let Some(task) = task else {
        return Err(ArgError::Usage("run needs --task <path|->".to_owned()));
    };
    Ok(Command::Run(Box::new(RunCommand { task, launch })))
}

/// Launch options whose default is the environment, refined by flags.
fn parse_launch(
    items: &mut std::vec::IntoIter<String>,
    env: &Env,
) -> Result<(LaunchOptions, Option<TaskSource>), ArgError> {
    let mut launch = LaunchOptions {
        control_version: 1,
        runner: env("JJ_ACP_RUNNER"),
        catalog: env("JJ_ACP_CATALOG"),
        tools: env("JJ_ACP_TOOLS"),
        model: env("JJ_ACP_MODEL"),
        effort: env("JJ_ACP_EFFORT"),
        mode: env("JJ_ACP_MODE"),
        idle_timeout: optional_duration(env, "JJ_ACP_IDLE_TIMEOUT", "--idle-timeout")?
            .unwrap_or(DEFAULT_IDLE_TIMEOUT),
        max_turn_duration: optional_duration(
            env,
            "JJ_ACP_MAX_TURN_DURATION",
            "--max-turn-duration",
        )?
        .unwrap_or(DEFAULT_MAX_TURN_DURATION),
        cancel_grace: optional_duration(env, "JJ_ACP_CANCEL_GRACE", "--cancel-grace")?
            .unwrap_or(DEFAULT_CANCEL_GRACE),
        reap_timeout: optional_duration(env, "JJ_ACP_REAP_TIMEOUT", "--reap-timeout")?
            .unwrap_or(DEFAULT_REAP_TIMEOUT),
        system_prompt_file: env("JJ_ACP_SYSTEM_PROMPT_FILE"),
        prompt_transport: match env("JJ_ACP_PROMPT_TRANSPORT") {
            Some(value) => transport(&value)?,
            None => SystemPromptTransport::Field,
        },
        cwd: env("JJ_ACP_CWD").map(PathBuf::from),
        workers: optional_count(env, "JJ_ACP_WORKERS", "--workers")?.unwrap_or(DEFAULT_WORKERS),
        queue_capacity: optional_count(env, "JJ_ACP_QUEUE_CAPACITY", "--queue-capacity")?
            .unwrap_or(DEFAULT_QUEUE_CAPACITY),
    };

    let mut task: Option<TaskSource> = None;
    while let Some(argument) = items.next() {
        match argument.as_str() {
            "--control-version" => {
                launch.control_version =
                    crate::control_version::parse(&next_value(items, "--control-version")?)?;
            }
            "--task" => task = Some(TaskSource::parse(&next_value(items, "--task")?)),
            flag if flag.starts_with("--task=") => {
                task = Some(TaskSource::parse(&flag["--task=".len()..]));
            }
            "--runner" => launch.runner = Some(next_value(items, "--runner")?),
            "--tools" => launch.tools = Some(next_value(items, "--tools")?),
            "--catalog" => launch.catalog = Some(next_value(items, "--catalog")?),
            "--model" => launch.model = Some(next_value(items, "--model")?),
            "--effort" => launch.effort = Some(next_value(items, "--effort")?),
            "--mode" => launch.mode = Some(next_value(items, "--mode")?),
            "--cwd" => launch.cwd = Some(PathBuf::from(next_value(items, "--cwd")?)),
            "--idle-timeout" => {
                launch.idle_timeout =
                    duration(&next_value(items, "--idle-timeout")?, "--idle-timeout")?;
            }
            "--max-turn-duration" => {
                launch.max_turn_duration = duration(
                    &next_value(items, "--max-turn-duration")?,
                    "--max-turn-duration",
                )?;
            }
            "--cancel-grace" => {
                launch.cancel_grace =
                    duration(&next_value(items, "--cancel-grace")?, "--cancel-grace")?;
            }
            "--reap-timeout" => {
                launch.reap_timeout =
                    duration(&next_value(items, "--reap-timeout")?, "--reap-timeout")?;
            }
            "--system-prompt-file" => {
                launch.system_prompt_file = Some(next_value(items, "--system-prompt-file")?);
            }
            "--workers" => {
                launch.workers = count(&next_value(items, "--workers")?, "--workers")?;
            }
            "--queue-capacity" => {
                launch.queue_capacity =
                    count(&next_value(items, "--queue-capacity")?, "--queue-capacity")?;
            }
            "--prompt-transport" => {
                launch.prompt_transport = transport(&next_value(items, "--prompt-transport")?)?;
            }
            other => {
                return Err(ArgError::Usage(format!("unknown option {other:?}")));
            }
        }
    }
    Ok((launch, task))
}

/// The environment lookup used in production.
pub fn system_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}
