//! The `serve` command: a persistent control loop over stdio.
//!
//! Composition only. The frame contract and the loop live in `lapis-acp-control`;
//! this module builds the runtime, installs signal handlers, and maps the way the
//! loop ended onto a process exit code.

use lapis_acp_control::ServeLimits;
use lapis_acp_control::record::ExitCode;
use lapis_acp_control::serve::{ServeConfig, ServeExit, serve};
use lapis_acp_control::transport::stdio;

use crate::args::LaunchOptions;
use crate::run_task::invocation_config;
use crate::signals;

/// Serve the control protocol, returning the process exit code.
pub async fn execute(launch: &LaunchOptions) -> i32 {
    let config = match invocation_config(launch) {
        Ok(config) => config,
        Err(rejection) => {
            // Launch configuration is unusable, so there is no loop to serve and
            // no controlling application to frame a refusal for. Nothing has been
            // written to stdout, so stderr is the only channel left and the exit
            // code is the only machine-readable signal.
            eprintln!("lapis-acp: {rejection}");
            return ExitCode::Invalid.as_i32();
        }
    };
    let installed = match signals::install() {
        Ok(installed) => installed,
        Err(error) => {
            eprintln!("lapis-acp: cannot install signal handlers: {error}");
            return ExitCode::Failure.as_i32();
        }
    };
    if launch.control_version == 3 {
        if launch.runner.is_none() {
            eprintln!("lapis-acp: warm serve requires --runner");
            return ExitCode::Invalid.as_i32();
        }
        let (mut source, sink) = stdio();
        lapis_acp_control::fleet::serve(
            &mut source,
            sink,
            config,
            launch.queue_capacity,
            installed.receiver.clone(),
        )
        .await;
        return installed.signal_exit().unwrap_or(ExitCode::Ok.as_i32());
    }
    if launch.control_version == 2 {
        if launch.runner.is_none() {
            eprintln!("lapis-acp: interactive serve requires --runner");
            return ExitCode::Invalid.as_i32();
        }
        let (mut source, sink) = stdio();
        lapis_acp_control::interactive::serve(
            &mut source,
            sink,
            config,
            launch.queue_capacity,
            installed.receiver.clone(),
        )
        .await;
        return installed.signal_exit().unwrap_or(ExitCode::Ok.as_i32());
    }
    // A signal ends the loop rather than cancelling one task: a controlling
    // application that is gone has nobody to receive a record, so the honest
    // outcome is a bounded shutdown and the signal's own exit code.
    let mut receiver = installed.receiver.clone();
    let serve_config = ServeConfig {
        limits: ServeLimits {
            workers: launch.workers,
            queue_capacity: launch.queue_capacity,
        },
        invocation: config,
    };
    let (mut source, sink) = stdio();
    let served = tokio::select! {
        exit = serve(&mut source, sink, serve_config) => exit,
        _ = wait_for_signal(&mut receiver) => ServeExit::Shutdown,
    };
    match served {
        ServeExit::Shutdown => installed.signal_exit().unwrap_or(ExitCode::Ok.as_i32()),
        ServeExit::Eof | ServeExit::InputFailed => ExitCode::Ok.as_i32(),
    }
}

/// Resolve once the cancellation watch is set.
async fn wait_for_signal(receiver: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if *receiver.borrow_and_update() {
            return;
        }
        if receiver.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}
