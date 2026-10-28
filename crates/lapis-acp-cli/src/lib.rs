//! `lapis-acp` command line entry points.
//!
//! Composition only: argument parsing, launch configuration assembly, signal
//! installation, one terminal record on stdout, diagnostics on stderr, and the
//! process exit code. Behaviour lives in the owning crate.
//!
//! Exit codes are a published contract and never change meaning: `0` normal
//! completion, `1` startup, execution or output failure, `2` invalid arguments,
//! launch configuration or task input, `124` deadline exceeded, `130` SIGINT and
//! `143` SIGTERM after bounded cleanup.
//!
//! Stdout carries terminal records, and for `--help`, `--version` and `agents`
//! the output those commands exist to produce. Nothing else is ever written
//! there, so a controlling application can parse it unconditionally.

#![forbid(unsafe_code)]

pub mod agents;
pub mod args;
mod control_version;
pub mod run_task;
pub mod serve;
pub mod signals;
mod toml_tools;

use std::io::Write;

use lapis_acp_control::TaskSource;
use lapis_acp_control::record::ExitCode;

use crate::args::{Command, LaunchOptions};
use crate::run_task::RunResult;

/// Run the binary against the process arguments and environment.
pub fn run() -> i32 {
    if let Some(code) = lapis_ai_config::launch::cli(lapis_ai_config::Component::Acp) {
        return code;
    }
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match args::parse(argv, &args::system_env) {
        Ok(Command::Help) => {
            println!("{}", args::USAGE);
            ExitCode::Ok.as_i32()
        }
        Ok(Command::Version) => {
            println!("lapis-acp {}", env!("CARGO_PKG_VERSION"));
            ExitCode::Ok.as_i32()
        }
        Ok(Command::Agents(launch)) => agents::execute(&launch),
        Ok(Command::Serve(launch)) => serve_blocking(&launch),
        Ok(Command::Run(command)) => run_task_command(&command.task, &command.launch),
        Err(error) => {
            // Usage goes to stderr: stdout is reserved for records.
            eprintln!("lapis-acp: {error}");
            eprintln!("{}", args::USAGE);
            ExitCode::Invalid.as_i32()
        }
    }
}

/// Build a runtime for the persistent control loop.
fn serve_blocking(launch: &LaunchOptions) -> i32 {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("lapis-acp: cannot start the async runtime: {error}");
            return ExitCode::Failure.as_i32();
        }
    };
    runtime.block_on(serve::execute(launch))
}

/// Emit exactly one terminal record, then resolve the exit code.
fn emit(result: &RunResult, signal_exit: Option<i32>) -> i32 {
    for diagnostic in &result.diagnostics {
        eprintln!("lapis-acp: {diagnostic}");
    }
    let mut stdout = std::io::stdout();
    if writeln!(stdout, "{}", result.record.to_line()).is_err() || stdout.flush().is_err() {
        // A broken stdout is a terminal-output failure, not a task outcome.
        eprintln!("lapis-acp: cannot write the terminal record");
        return ExitCode::Failure.as_i32();
    }
    if result.cancelled {
        return signal_exit.unwrap_or(ExitCode::Failure.as_i32());
    }
    result.exit.as_i32()
}

fn run_task_command(task: &TaskSource, launch: &LaunchOptions) -> i32 {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("lapis-acp: cannot start the async runtime: {error}");
            return ExitCode::Failure.as_i32();
        }
    };
    runtime.block_on(async move {
        let installed = match signals::install() {
            Ok(installed) => installed,
            Err(error) => {
                eprintln!("lapis-acp: cannot install signal handlers: {error}");
                return ExitCode::Failure.as_i32();
            }
        };
        let mut receiver = installed.receiver.clone();
        let result = run_task::execute(launch, task, &mut receiver).await;
        let signal_exit = installed.signal_exit();
        // A cancelled run reports the signal even when the record says completed,
        // because the caller asked us to stop and that is what it needs to know.
        emit(&result, signal_exit)
    })
}

mod http_tools;
mod tools;

mod argument_values;
