use lapis_ai_config::ComputeFailure;
/// Map dependency causes to safe categories, without forwarding paths or secrets.
pub fn classify(action: &str, error: &anyhow::Error) -> ComputeFailure {
    let mut code = match action {
        "download" => "download_failed",
        "prepare" => "runtime_prepare_failed",
        "start" => "model_start_failed",
        "import" => "invalid_gguf",
        _ => "compute_operation_failed",
    };
    for cause in error.chain() {
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            if io.raw_os_error() == Some(28) {
                code = "disk_full";
                break;
            }
            if io.kind() == std::io::ErrorKind::OutOfMemory {
                code = "memory_exhausted";
                break;
            }
        }
        let text = cause.to_string().to_lowercase();
        if text.contains("model required") {
            code = "model_required";
            break;
        }
        if text.contains("model not installed") || text.contains("installed file unavailable") {
            code = "model_not_installed";
            break;
        }
        if text.contains("native runtime preparation required") {
            code = "runtime_not_prepared";
            break;
        }
        if text.contains("already running") || text.contains("model is running") {
            code = "compute_busy";
            break;
        }
        if text.contains("out of memory") || text.contains("failed to allocate") {
            code = "memory_exhausted";
            break;
        }
        if text.contains("timed out") || text.contains("inference model unavailable") {
            code = "startup_timeout";
            break;
        }
    }
    ComputeFailure::from_code(code)
}
