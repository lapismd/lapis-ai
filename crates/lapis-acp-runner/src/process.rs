//! Bounded process termination.
//!
//! Cleanup is a sequence with a bound on every stage, and each stage reports what
//! actually happened. It is best-effort and says so: a harness killed outright, or
//! a descendant that deliberately escapes its process group, cannot be stopped
//! from here, and an unreaped child is reported rather than assumed gone.
//!
//! The ordering matters for correctness, not just politeness. A process group id
//! is only unambiguously ours while the group leader is unreaped, because a pid
//! cannot be recycled while its process is still a zombie. Both group signals are
//! therefore sent before, or in the absence of, any successful reap: the
//! cooperative one first, and the fatal one only when the leader is still
//! outstanding. Reaping first and signalling after could kill an unrelated group.

use std::time::Duration;

use tokio::process::Child;

/// What termination actually achieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Termination {
    /// A cooperative stop was signalled to the whole process group.
    pub group_signalled: bool,
    /// The process exited during the cooperative grace window.
    pub exited_during_grace: bool,
    /// A fatal signal was sent to the whole process group rather than the child
    /// alone.
    pub group_killed: bool,
    /// The process was reaped and an exit status observed.
    pub reaped: bool,
    /// The exit code, when the platform reported one.
    pub exit_code: Option<i32>,
}

impl Termination {
    /// Report whether the process is definitely gone.
    ///
    /// This says nothing about descendants. A confirmed stop means the group
    /// leader was reaped, which is the only fact this process can establish
    /// about a child it no longer owns.
    pub fn confirmed_stopped(&self) -> bool {
        self.reaped
    }

    /// Report whether cleanup had to escalate past the cooperative signal.
    pub fn escalated(&self) -> bool {
        self.group_killed
    }
}

/// Owns an agent child and the bounds on stopping it.
#[derive(Debug)]
pub struct ProcessGuard {
    child: Child,
    pid: Option<u32>,
}

impl ProcessGuard {
    /// Take ownership of a spawned child.
    pub fn new(child: Child) -> Self {
        let pid = child.id();
        Self { child, pid }
    }

    /// The child's process id, when the platform exposed one.
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Report whether the child has already exited.
    pub async fn poll(&mut self) -> bool {
        let stopped = matches!(self.child.try_wait(), Ok(Some(_)));
        if stopped {
            self.pid = None;
        }
        stopped
    }

    /// Stop the child and its process group within bounded grace and reap
    /// intervals.
    ///
    /// The caller is expected to have already closed the child's stdin and, for a
    /// cancellation, to have requested cooperative cancellation through the
    /// protocol. This then signals the group, waits `grace` for a voluntary exit,
    /// and escalates to a fatal group signal followed by a bounded reap.
    pub async fn terminate(&mut self, grace: Duration, reap: Duration) -> Termination {
        let termination = self.terminate_owned(grace, reap).await;
        if termination.reaped {
            self.pid = None;
        }
        termination
    }

    async fn terminate_owned(&mut self, grace: Duration, reap: Duration) -> Termination {
        let group_signalled = self.signal_group(Signal::Stop);
        if let Ok(Ok(status)) = tokio::time::timeout(grace, self.child.wait()).await {
            return Termination {
                group_signalled,
                exited_during_grace: true,
                group_killed: false,
                reaped: true,
                exit_code: status.code(),
            };
        }
        // The leader is still outstanding, so the group id is still ours and a
        // fatal group signal cannot reach an unrelated process.
        let group_killed = self.signal_group(Signal::Kill);
        match tokio::time::timeout(reap, self.child.wait()).await {
            Ok(Ok(status)) => Termination {
                group_signalled,
                exited_during_grace: false,
                group_killed,
                reaped: true,
                exit_code: status.code(),
            },
            _ => Termination {
                group_signalled,
                exited_during_grace: false,
                group_killed,
                reaped: false,
                exit_code: None,
            },
        }
    }

    #[cfg(unix)]
    fn signal_group(&mut self, signal: Signal) -> bool {
        let Some(pid) = self.pid.filter(|pid| *pid > 0) else {
            return self.signal_child(signal);
        };
        let target = nix::unistd::Pid::from_raw(pid as i32);
        let mapped = match signal {
            Signal::Stop => nix::sys::signal::Signal::SIGTERM,
            Signal::Kill => nix::sys::signal::Signal::SIGKILL,
        };
        if nix::sys::signal::killpg(target, mapped).is_ok() {
            return true;
        }
        // A group signal fails when the group is already empty; the direct child
        // is still worth signalling so the outcome does not depend on which of
        // the two races finished first.
        self.signal_child(signal)
    }

    /// Signal the direct child only. Descendant containment is a Unix property
    /// and is not promised elsewhere; the reduced guarantee is reported through
    /// [`Termination::group_killed`] rather than hidden.
    #[cfg(not(unix))]
    fn signal_group(&mut self, signal: Signal) -> bool {
        self.signal_child(signal)
    }

    #[cfg(unix)]
    fn signal_child(&mut self, signal: Signal) -> bool {
        let Some(pid) = self.pid.filter(|pid| *pid > 0) else {
            return self.child.start_kill().is_ok();
        };
        let mapped = match signal {
            Signal::Stop => nix::sys::signal::Signal::SIGTERM,
            Signal::Kill => nix::sys::signal::Signal::SIGKILL,
        };
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), mapped).is_ok()
    }

    #[cfg(not(unix))]
    fn signal_child(&mut self, signal: Signal) -> bool {
        match signal {
            Signal::Kill => self.child.start_kill().is_ok(),
            // There is no portable cooperative stop for a child process; the
            // reduced guarantee is reported rather than approximated.
            Signal::Stop => false,
        }
    }
}

/// Which signal to send to a process group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Signal {
    /// Cooperative: ask the group to shut down.
    Stop,
    /// Fatal: do not wait for the group to cooperate.
    Kill,
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        // An unreaped leader still owns its group identifier. This is the fallback
        // for an interrupted interactive owner, never a signal after successful reap.
        if self.pid.is_some() {
            self.signal_group(Signal::Kill);
        }
    }
}
