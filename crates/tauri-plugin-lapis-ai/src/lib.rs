//! Optional Tauri transport; applications own admission and conversation policy.
#![forbid(unsafe_code)]
mod commands;
mod host;
pub use host::AcpHost;
pub use lapis_ai_runtime as runtime;
pub use lapis_ai_service::{
    Config, Error, HostContext, HostPolicy, Launch, Result, SessionBinding, SessionInfo, Sink,
    StreamMessage, Target,
};
use std::sync::Arc;
use tauri::{Manager, RunEvent, Runtime, WindowEvent, plugin::TauriPlugin};

pub struct Builder {
    policy: Arc<dyn HostPolicy>,
    config: Config,
}
impl Builder {
    pub fn new(policy: impl HostPolicy) -> Self {
        Self {
            policy: Arc::new(policy),
            config: Config::default(),
        }
    }
    pub fn config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }
    pub fn build<R: Runtime>(self) -> TauriPlugin<R> {
        tauri::plugin::Builder::new("lapis-ai")
            .invoke_handler(tauri::generate_handler![
                commands::targets,
                commands::list,
                commands::open,
                commands::submit,
                commands::snapshot,
                commands::attach,
                commands::detach,
                commands::ack,
                commands::close
            ])
            .setup(move |app, _| {
                let host = AcpHost::new(self.policy, self.config)?;
                if !app.manage(Arc::new(host)) {
                    return Err("AI plugin already registered".into());
                }
                Ok(())
            })
            .on_event(|app, event| {
                if let Some(host) = app.try_state::<Arc<AcpHost>>() {
                    match event {
                        RunEvent::Exit => {
                            report_cleanup(tauri::async_runtime::block_on(host.shutdown()));
                        }
                        RunEvent::WindowEvent {
                            label,
                            event: WindowEvent::Destroyed,
                            ..
                        } => {
                            let host = host.inner().clone();
                            let label = label.clone();
                            tauri::async_runtime::spawn(async move {
                                report_cleanup(host.revoke(&label, None).await);
                            });
                        }
                        // Requested close/exit can still be cancelled by the host.
                        _ => {}
                    }
                }
            })
            .on_drop(|app| {
                if let Some(host) = app.try_state::<Arc<AcpHost>>() {
                    report_cleanup(tauri::async_runtime::block_on(host.shutdown()));
                }
            })
            .build()
    }
}
/// Lifecycle controls are Rust-only; WebViews cannot grant or disable service authority.
pub trait AiExt<R: Runtime>: Manager<R> {
    fn ai(&self) -> Arc<AcpHost> {
        self.state::<Arc<AcpHost>>().inner().clone()
    }
}
impl<R: Runtime, M: Manager<R>> AiExt<R> for M {}
fn report_cleanup(result: Result<()>) {
    if let Err(error) = result {
        eprintln!("lapis-ai cleanup failed: {error}");
    }
}
