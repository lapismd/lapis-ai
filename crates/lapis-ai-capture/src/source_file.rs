//! Read-only source inspection. Returned bytes require host admission and privacy.
use crate::{CaptureResult, SourceProfile, discovery::PhysicalId};
use std::{
    fs::{self, File, Metadata},
    io::{BufRead as _, BufReader, Read as _},
    path::{Path, PathBuf},
    time::SystemTime,
};

pub const MAX_HEADER_BYTES: usize = 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
pub struct FileStamp {
    pub len: u64,
    modified: SystemTime,
    pub device: u64,
    pub inode: u64,
}

/// Unsanitized native metadata; no serialization or logging representation.
pub struct CodexHeader {
    pub session: String,
    pub directory: PathBuf,
    pub bytes: Vec<u8>,
}

fn stamp(metadata: &Metadata) -> CaptureResult<FileStamp> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        Ok(FileStamp {
            len: metadata.len(),
            modified: metadata.modified()?,
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Err("physical transcript identity is unavailable on this platform".into())
    }
}

pub fn open_regular(path: &Path, identity: Option<PhysicalId>) -> CaptureResult<(File, FileStamp)> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("transcript must be a regular file".into());
    }
    let file = File::open(path)?;
    let observed = stamp(&file.metadata()?)?;
    if observed != stamp(&metadata)? {
        return Err("transcript changed during open".into());
    }
    if identity.is_some_and(|id| id.device != observed.device || id.inode != observed.inode) {
        return Err("transcript source identity changed".into());
    }
    Ok((file, observed))
}

pub fn inspect_codex(
    profile: SourceProfile,
    path: &Path,
    identity: Option<PhysicalId>,
) -> CaptureResult<(File, CodexHeader, FileStamp)> {
    let (file, observed) = open_regular(path, identity)?;
    let mut bytes = Vec::new();
    BufReader::new((&file).take(MAX_HEADER_BYTES as u64 + 1)).read_until(b'\n', &mut bytes)?;
    if bytes.len() > MAX_HEADER_BYTES || bytes.last() != Some(&b'\n') {
        return Err("complete bounded transcript header required".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "invalid transcript header")?;
    let payload = &value["payload"];
    let surface_matches = match profile {
        SourceProfile::CodexCliJsonlV1 => {
            (matches!(payload["source"].as_str(), Some("cli" | "exec"))
                && matches!(
                    payload["originator"].as_str(),
                    Some("codex_cli_rs" | "codex_exec" | "Codex Desktop")
                ))
                || (payload["source"].as_str() == Some("vscode")
                    && payload["originator"].as_str() == Some("lapis-acp"))
        }
        SourceProfile::CodexDesktopJsonlV1 => {
            payload["source"].as_str() == Some("vscode")
                && payload["originator"].as_str() == Some("Codex Desktop")
        }
        _ => return Err("profile does not select Codex JSONL".into()),
    };
    if value["type"].as_str() != Some("session_meta")
        || payload["cli_version"].as_str() != Some(profile.product_version())
        || !surface_matches
    {
        return Err("unsupported transcript product surface or version".into());
    }
    let session = payload["id"]
        .as_str()
        .ok_or("transcript session identity missing")?;
    if !uuid_label(session) {
        return Err("invalid transcript session identity".into());
    }
    let directory = PathBuf::from(
        payload["cwd"]
            .as_str()
            .ok_or("transcript directory missing")?,
    );
    if !directory.is_absolute() {
        return Err("transcript directory must be absolute".into());
    }
    let header = CodexHeader {
        session: session.into(),
        directory,
        bytes,
    };
    if !unchanged(&file, path, &observed)? {
        return Err("transcript changed during inspection".into());
    }
    Ok((file, header, observed))
}

pub fn unchanged(file: &File, path: &Path, observed: &FileStamp) -> CaptureResult<bool> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(!metadata.file_type().is_symlink()
        && metadata.is_file()
        && stamp(&file.metadata()?)? == *observed
        && stamp(&metadata)? == *observed)
}

pub fn uuid_label(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
