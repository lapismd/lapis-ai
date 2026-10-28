//! Read-only source acquisition. Hosts own activity admission, privacy and persistence.
#![forbid(unsafe_code)]
pub mod discovery;
mod profile;
#[cfg(feature = "session-pages")]
pub mod sessions;
pub mod source_file;
pub use profile::SourceProfile;
pub type CaptureError = Box<dyn std::error::Error + Send + Sync>;
pub type CaptureResult<T> = std::result::Result<T, CaptureError>;
