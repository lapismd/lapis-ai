//! Explicit versioned product surfaces; registration is not runtime qualification.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceProfile {
    CodexCliJsonlV1,
    CodexDesktopJsonlV1,
    CursorDesktopTranscriptV1,
    CursorCliTranscriptV1,
    OpencodeReadOnlyApiV1,
    OpencodeExportJsonV1,
}

impl SourceProfile {
    pub fn product_version(self) -> &'static str {
        match self {
            Self::CodexCliJsonlV1 => "0.156.1",
            Self::CodexDesktopJsonlV1 => "0.159.2",
            Self::CursorDesktopTranscriptV1 => "3.22.7",
            Self::CursorCliTranscriptV1 => "2026.09.26-dd393fe",
            Self::OpencodeReadOnlyApiV1 | Self::OpencodeExportJsonV1 => "1.18.30",
        }
    }
}
