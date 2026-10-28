//! Inert, allowlisted compute failures; dependency diagnostics never cross IPC.
use serde::{Deserialize, Serialize};
/// Safe operation failure, separate from a transport's unknown outcome.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeFailure {
    pub code: String,
    pub message: String,
}
impl ComputeFailure {
    /// Unknown codes deliberately collapse to a safe generic failure.
    pub fn from_code(code: &str) -> Self {
        let (code, message) = match code {
            "helper_unavailable" => (
                code,
                "The bundled compute helper could not start. Check the AI runtime installation, then retry.",
            ),
            "identity_unavailable" => (
                code,
                "Compute cannot access its device identity. Unlock the device credential store, then retry.",
            ),
            "model_required" => (code, "Choose a model before continuing."),
            "model_not_installed" => (
                code,
                "Model weights are missing. Download or import the model, then start it.",
            ),
            "runtime_not_prepared" => (
                code,
                "Prepare the native runtime before starting the model.",
            ),
            "invalid_gguf" => (code, "Choose a valid, readable GGUF file to import."),
            "disk_full" => (
                code,
                "There is not enough free disk space. Free some space, then retry.",
            ),
            "memory_exhausted" => (
                code,
                "The model needs more available memory. Stop other work or choose a smaller model.",
            ),
            "compute_busy" => (
                code,
                "Compute is busy. Stop the active operation before changing models.",
            ),
            "download_failed" => (
                code,
                "The download failed. Check the model reference and network connection, then retry.",
            ),
            "runtime_prepare_failed" => (
                code,
                "Runtime preparation failed. Check network access and available disk space, then retry.",
            ),
            "model_start_failed" => (
                code,
                "The model could not start. Check its GGUF compatibility and memory requirements.",
            ),
            "startup_timeout" => (
                code,
                "The model did not become ready in time. Stop compute, then try a smaller model.",
            ),
            "compute_storage_unavailable" => (
                code,
                "Model storage cannot be read. Check this device's storage permissions.",
            ),
            "compute_inventory_invalid" => (
                code,
                "The local model index is invalid. Existing weights have been retained; refresh after checking model storage.",
            ),
            "context_capacity" => (
                code,
                "This conversation exceeds the local model's context capacity. Start a shorter conversation or choose a larger-context model.",
            ),
            "inference_rejected" => (
                code,
                "The local inference runtime rejected the request. Check model compatibility and context size.",
            ),
            _ => (
                "compute_operation_failed",
                "The compute operation failed. Stop compute and check the model configuration before retrying.",
            ),
        };
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
