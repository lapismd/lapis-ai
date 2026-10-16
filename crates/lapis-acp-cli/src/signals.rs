//! Signal handling and the cancellation watch.
//!
//! SIGINT and SIGTERM set a watch that the turn race observes, and record which
//! signal arrived so the exit code can distinguish them. Cleanup stays bounded:
//! the runner requests cooperative cancellation, then kills the process group and
//! reaps within its own bounds.

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use tokio::sync::watch;

/// Exit code for SIGINT after bounded cleanup.
pub const EXIT_SIGINT: i32 = 130;

/// Exit code for SIGTERM after bounded cleanup.
pub const EXIT_SIGTERM: i32 = 143;

/// An installed cancellation watch and the signal that triggered it.
#[derive(Debug, Clone)]
pub struct Signals {
    /// Notifies the turn that cancellation was requested.
    pub receiver: watch::Receiver<bool>,
    /// The signal-specific exit code, zero until a signal arrives.
    pub exit: Arc<AtomicI32>,
}

impl Signals {
    /// The exit code a signal requested, or `None` when no signal arrived.
    pub fn signal_exit(&self) -> Option<i32> {
        match self.exit.load(Ordering::SeqCst) {
            0 => None,
            code => Some(code),
        }
    }
}

/// Install handlers for the interruption signals this platform provides.
///
/// Installing a handler replaces the default terminate-immediately behaviour, so
/// a signal now produces bounded cleanup instead of an orphaned agent. A failure
/// to install is reported rather than ignored, because running without cancellation
/// handling would silently weaken the cleanup guarantee.
pub fn install() -> std::io::Result<Signals> {
    let (sender, receiver) = watch::channel(false);
    let exit = Arc::new(AtomicI32::new(0));
    let task_exit = exit.clone();

    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut interrupt = signal(SignalKind::interrupt())?;
        let mut terminate = signal(SignalKind::terminate())?;
        tokio::spawn(async move {
            let code = tokio::select! {
                _ = interrupt.recv() => EXIT_SIGINT,
                _ = terminate.recv() => EXIT_SIGTERM,
            };
            task_exit.store(code, Ordering::SeqCst);
            let _ = sender.send(true);
        });
    }

    #[cfg(not(unix))]
    {
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                task_exit.store(EXIT_SIGINT, Ordering::SeqCst);
                let _ = sender.send(true);
            }
        });
    }

    Ok(Signals { receiver, exit })
}
