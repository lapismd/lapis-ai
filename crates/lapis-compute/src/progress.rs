//! Forward only bounded model download facts, never dependency diagnostics.
use mesh_llm_events::{ConsoleSessionMode, ModelProgressStatus, OutputEvent, OutputSink};
use std::{io, sync::Arc};
static OPERATION: std::sync::Mutex<Option<(String, String)>> = std::sync::Mutex::new(None);
pub fn begin(id: &str, action: &str) {
    if let Ok(mut active) = OPERATION.lock() {
        *active = Some((id.into(), action.into()));
    }
}
pub fn end() {
    if let Ok(mut active) = OPERATION.lock() {
        *active = None;
    }
}
pub fn report(
    label: &str,
    file: Option<&str>,
    downloaded: Option<u64>,
    total: Option<u64>,
    stage: &str,
) {
    if let Ok(active) = OPERATION.lock()
        && let Some((id, action)) = &*active
    {
        crate::emit(
            serde_json::json!({"type":"progress","requestId":id,"action":action,"label":label,"file":file,"downloaded_bytes":downloaded,"total_bytes":total,"stage":stage}),
        );
    }
}
struct Sink;
impl OutputSink for Sink {
    fn emit_event(&self, event: OutputEvent) -> io::Result<()> {
        if let OutputEvent::ModelDownloadProgress {
            label,
            file,
            downloaded_bytes,
            total_bytes,
            status,
        } = event
        {
            report(
                &label,
                file.as_deref(),
                downloaded_bytes,
                total_bytes,
                match status {
                    ModelProgressStatus::Ensuring => "preparing",
                    ModelProgressStatus::Downloading => "downloading",
                    ModelProgressStatus::Ready => "downloaded",
                },
            );
        }
        Ok(())
    }
    fn console_session_mode(&self) -> Option<ConsoleSessionMode> {
        Some(ConsoleSessionMode::InteractiveDashboard)
    }
}
pub fn install() {
    mesh_llm_events::set_output_sink(Arc::new(Sink));
}
