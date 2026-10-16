//! Forward only bounded model download facts, never dependency diagnostics.
use mesh_llm_events::{ConsoleSessionMode, ModelProgressStatus, OutputEvent, OutputSink};
use std::{io, sync::Arc};
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
            crate::emit(
                serde_json::json!({"type":"progress","label":label,"file":file,"downloaded_bytes":downloaded_bytes,"total_bytes":total_bytes,"stage":match status{ModelProgressStatus::Ensuring=>"preparing",ModelProgressStatus::Downloading=>"downloading",ModelProgressStatus::Ready=>"downloaded"}}),
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
