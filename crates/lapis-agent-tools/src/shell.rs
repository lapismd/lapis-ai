//! Bounded command output and Unix descendant cleanup.
use serde_json::Value;
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
};
struct OwnedShell {
    child: Child,
    pid: Option<u32>,
}
impl Drop for OwnedShell {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(pid) = self.pid {
            let _ = nix::sys::signal::killpg(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
        let _ = self.child.start_kill();
    }
}
pub async fn run(root: &Path, args: &Value) -> Result<String, &'static str> {
    let command = args["command"]
        .as_str()
        .filter(|v| !v.is_empty() && v.len() <= 16384)
        .ok_or("command required")?;
    let deadline = args["timeout_seconds"].as_u64().unwrap_or(30);
    if !(1..=120).contains(&deadline) {
        return Err("invalid command deadline");
    }
    let mut builder = Command::new("/bin/sh");
    builder
        .args(["-c", command])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    builder.process_group(0);
    let child = builder.spawn().map_err(|_| "command unavailable")?;
    let pid = child.id();
    let mut owned = OwnedShell { child, pid };
    let stdout = owned.child.stdout.take().ok_or("output unavailable")?;
    let stderr = owned.child.stderr.take().ok_or("output unavailable")?;
    let task = async {
        let out = async {
            let mut bytes = Vec::new();
            stdout.take(65536).read_to_end(&mut bytes).await?;
            Ok::<_, std::io::Error>(bytes)
        };
        let err = async {
            let mut bytes = Vec::new();
            stderr.take(65536).read_to_end(&mut bytes).await?;
            Ok::<_, std::io::Error>(bytes)
        };
        let (out, err) = tokio::try_join!(out, err)?;
        let status = owned.child.wait().await?;
        Ok::<_, std::io::Error>((status, out, err))
    };
    let result = tokio::time::timeout(Duration::from_secs(deadline), task).await;
    #[cfg(unix)]
    if let Some(pid) = pid {
        let _ = nix::sys::signal::killpg(
            nix::unistd::Pid::from_raw(pid as i32),
            nix::sys::signal::Signal::SIGKILL,
        );
    }
    match result {
        Ok(Ok((status, out, err))) => Ok(format!(
            "Exit: {}\n{}\n{}",
            status.code().map_or("signal".into(), |c| c.to_string()),
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(&err)
        )),
        Ok(Err(_)) => Err("command output unavailable"),
        Err(_) => {
            let _ = owned.child.kill().await;
            let _ = owned.child.wait().await;
            Err("command timed out; outcome may be unknown")
        }
    }
}
