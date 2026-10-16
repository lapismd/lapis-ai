use crate::{Error, PreparedLaunch, Result};
use serde_json::Value;
use std::{process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin},
    sync::{Mutex, mpsc, watch},
};
const MAX_FRAME: usize = 4 * 1024 * 1024;
#[derive(Clone)]
pub struct Sender(Arc<Mutex<Option<ChildStdin>>>);
impl Sender {
    pub async fn send(&self, value: &Value) -> Result<()> {
        self.send_bounded(value, MAX_FRAME).await
    }
    pub(crate) async fn send_bounded(&self, value: &Value, maximum: usize) -> Result<()> {
        let mut bytes = serde_json::to_vec(value).map_err(|_| Error("invalid_request"))?;
        if bytes.len() > maximum {
            return Err(Error("request_limit"));
        }
        bytes.push(b'\n');
        let mut input = self.0.lock().await;
        let input = input.as_mut().ok_or(Error("helper_input_closed"))?;
        tokio::time::timeout(Duration::from_secs(10), async {
            input.write_all(&bytes).await?;
            input.flush().await
        })
        .await
        .map_err(|_| Error("submission_unknown"))?
        .map_err(|_| Error("submission_unknown"))
    }
    pub async fn close(&self) {
        self.0.lock().await.take();
    }
}
pub struct NativeProcess {
    child: Option<Child>,
    sender: Sender,
    frames: mpsc::Receiver<Result<Value>>,
    failure: watch::Receiver<Option<&'static str>>,
    reader: tokio::task::JoinHandle<()>,
}
impl NativeProcess {
    pub async fn spawn(plan: PreparedLaunch) -> Result<Self> {
        Self::spawn_mode(plan, false).await
    }
    pub(crate) async fn spawn_single(plan: PreparedLaunch) -> Result<Self> {
        Self::spawn_mode(plan, true).await
    }
    async fn spawn_mode(plan: PreparedLaunch, single: bool) -> Result<Self> {
        if !plan.directory.is_dir() {
            return Err(Error("working_directory_unavailable"));
        }
        let mut command = tokio::process::Command::new(&plan.executable);
        command
            .args(&plan.arguments)
            .current_dir(&plan.directory)
            .env_clear()
            .envs(&plan.environment.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command.spawn().map_err(|_| Error("helper_spawn_failed"))?;
        let input = child
            .stdin
            .take()
            .ok_or(Error("helper_pipes_unavailable"))?;
        let output = child
            .stdout
            .take()
            .ok_or(Error("helper_pipes_unavailable"))?;
        let (tx, frames) = mpsc::channel(256);
        let (failed, failure) = watch::channel(None);
        let reader = tokio::spawn(async move {
            let mut input = BufReader::new(output);
            if single {
                use tokio::io::AsyncReadExt;
                let mut bytes = Vec::new();
                let read = input
                    .take((MAX_FRAME + 1) as u64)
                    .read_to_end(&mut bytes)
                    .await;
                let result = if read.is_err() || bytes.len() > MAX_FRAME {
                    Err(Error("helper_protocol_failed"))
                } else {
                    serde_json::from_slice(&bytes).map_err(|_| Error("helper_protocol_failed"))
                };
                let _ = tx.try_send(result);
                return;
            }
            loop {
                let mut bytes = Vec::new();
                let read = tokio::io::AsyncReadExt::take(&mut input, (MAX_FRAME + 1) as u64)
                    .read_until(b'\n', &mut bytes)
                    .await;
                if matches!(read, Ok(0)) {
                    break;
                }
                let value =
                    if read.is_err() || bytes.len() > MAX_FRAME || bytes.last() != Some(&b'\n') {
                        Err(Error("helper_protocol_failed"))
                    } else {
                        serde_json::from_slice(&bytes).map_err(|_| Error("helper_protocol_failed"))
                    };
                let fatal = value.is_err();
                if tx.try_send(value).is_err() {
                    failed.send_replace(Some("output_coverage_lost"));
                    break;
                }
                if fatal {
                    break;
                }
            }
        });
        Ok(Self {
            child: Some(child),
            sender: Sender(Arc::new(Mutex::new(Some(input)))),
            frames,
            failure,
            reader,
        })
    }
    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }
    pub fn id(&self) -> Option<u32> {
        self.child.as_ref().and_then(Child::id)
    }
    pub async fn next(&mut self, timeout: Duration) -> Result<Value> {
        if let Some(reason) = *self.failure.borrow() {
            return Err(Error(reason));
        }
        let frame = tokio::time::timeout(timeout, self.frames.recv())
            .await
            .map_err(|_| Error("outcome_unknown"))?
            .ok_or(Error("helper_output_closed"))?;
        if let Some(reason) = *self.failure.borrow() {
            return Err(Error(reason));
        }
        frame
    }
    pub async fn shutdown(&mut self, grace: Duration) -> Result<()> {
        self.sender.close().await;
        if let Some(child) = self.child.as_mut() {
            let pid = child.id();
            if let Ok(status) = tokio::time::timeout(grace, child.wait()).await {
                // The leader has been reaped: its PID can now be recycled.
                // Never signal its former process group after this point.
                let status = status.map_err(|_| Error("helper_reap_failed"))?;
                self.child = None;
                self.reader.abort();
                return if status.success() {
                    Ok(())
                } else {
                    Err(Error("helper_failed"))
                };
            }
            signal_group(pid, false);
            if let Ok(status) = tokio::time::timeout(Duration::from_secs(2), child.wait()).await {
                status.map_err(|_| Error("helper_reap_failed"))?;
                self.child = None;
                self.reader.abort();
                return Ok(());
            }
            signal_group(pid, true);
            let _ = child.start_kill();
            let result = tokio::time::timeout(Duration::from_secs(5), child.wait())
                .await
                .map_err(|_| Error("helper_reap_failed"))?;
            self.reader.abort();
            result.map_err(|_| Error("helper_reap_failed"))?;
            self.child = None;
        }
        Ok(())
    }
}
fn signal_group(pid: Option<u32>, force: bool) {
    #[cfg(unix)]
    if let Some(pid) = pid {
        use nix::{
            sys::signal::{Signal, killpg},
            unistd::Pid,
        };
        let _ = killpg(
            Pid::from_raw(pid as i32),
            if force {
                Signal::SIGKILL
            } else {
                Signal::SIGTERM
            },
        );
    }
    #[cfg(not(unix))]
    let _ = (pid, force);
}
impl Drop for NativeProcess {
    fn drop(&mut self) {
        self.reader.abort();
        if let Some(mut child) = self.child.take() {
            signal_group(child.id(), true);
            let _ = child.start_kill();
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn(async move {
                    let _ = child.wait().await;
                });
            }
        }
    }
}
