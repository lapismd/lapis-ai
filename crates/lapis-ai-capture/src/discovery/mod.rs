//! Read-only location selection and metadata candidates; no session admission.
use crate::{CaptureResult, SourceProfile};
use std::{
    fs::{self, Metadata},
    path::{Path, PathBuf},
};

mod scanner;
pub use scanner::{CodexDirectoryScanner, CodexDiscoveryStep};

pub fn resolve_codex_roots(
    profile: SourceProfile,
    explicit: &[PathBuf],
    home: Option<&Path>,
    codex_home: Option<&Path>,
) -> CaptureResult<Vec<PathBuf>> {
    if !matches!(
        profile,
        SourceProfile::CodexCliJsonlV1 | SourceProfile::CodexDesktopJsonlV1
    ) {
        return Err("profile does not qualify Codex locations".into());
    }
    if explicit.len() > 16 {
        return Err("Codex root limit".into());
    }
    for path in explicit {
        valid_path(path)?;
    }
    let roots = if explicit.is_empty() {
        let data = match codex_home {
            Some(root) => {
                valid_path(root)?;
                root.to_owned()
            }
            None => {
                let home = home.ok_or("Codex user home unavailable")?;
                valid_path(home)?;
                home.join(".codex")
            }
        };
        vec![data.join("sessions")]
    } else {
        explicit.to_vec()
    };
    let mut unique = std::collections::BTreeSet::new();
    for root in &roots {
        valid_path(root)?;
        if !unique.insert(root) {
            return Err("duplicate Codex root".into());
        }
    }
    Ok(roots)
}

fn valid_path(path: &Path) -> CaptureResult<()> {
    if !path.is_absolute()
        || path.parent().is_none()
        || path
            .to_str()
            .is_none_or(|v| v.len() > 4096 || v.chars().any(char::is_control))
    {
        return Err("invalid Codex location".into());
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PhysicalId {
    pub device: u64,
    pub inode: u64,
}
impl PhysicalId {
    pub fn from(metadata: &Metadata) -> CaptureResult<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            Ok(Self {
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        }
        #[cfg(not(unix))]
        {
            let _ = metadata;
            Err("physical Codex discovery identity unavailable".into())
        }
    }
}

/// Device-only path capability. Never persisted or rendered in diagnostics.
#[derive(Clone)]
pub struct ApprovedCodexRoot {
    logical: PathBuf,
    physical: PathBuf,
    identity: PhysicalId,
}
impl ApprovedCodexRoot {
    /// Select a known file under this admitted root without rescanning unrelated directories.
    pub fn candidate(&self, path: &Path) -> CaptureResult<CodexSourceCandidate> {
        if !self.revalidate()? || !self.contains_exact(path) {
            return Err("source selection outside approved root".into());
        }
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("source selection is not a regular file".into());
        }
        let candidate = CodexSourceCandidate {
            root: self.clone(),
            path: path.to_owned(),
            identity: PhysicalId::from(&metadata)?,
        };
        if !candidate.revalidate()? {
            return Err("source selection changed".into());
        }
        Ok(candidate)
    }
    pub fn open(logical: &Path) -> CaptureResult<Option<Self>> {
        valid_path(logical)?;
        let physical = match fs::canonicalize(logical) {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        valid_path(&physical)?;
        let metadata = fs::symlink_metadata(&physical)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("approved Codex root must be a directory".into());
        }
        let root = Self {
            logical: logical.to_owned(),
            physical,
            identity: PhysicalId::from(&metadata)?,
        };
        if !root.revalidate()? {
            return Err("approved Codex root changed".into());
        }
        Ok(Some(root))
    }
    /// Retirement evidence: false proves absence/rebinding; other IO remains an error.
    pub fn current_binding(&self) -> CaptureResult<bool> {
        let Some(resolved) = binding_observation(fs::canonicalize(&self.logical))? else {
            return Ok(false);
        };
        let Some(metadata) = binding_observation(fs::symlink_metadata(&self.physical))? else {
            return Ok(false);
        };
        Ok(resolved == self.physical
            && metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && PhysicalId::from(&metadata)? == self.identity)
    }
    /// Availability fence only; false may include unknown IO and cannot prove retirement.
    pub fn revalidate(&self) -> CaptureResult<bool> {
        let Ok(resolved) = fs::canonicalize(&self.logical) else {
            return Ok(false);
        };
        let Ok(metadata) = fs::symlink_metadata(&self.physical) else {
            return Ok(false);
        };
        Ok(resolved == self.physical
            && metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && PhysicalId::from(&metadata)? == self.identity)
    }
    fn contains_exact(&self, path: &Path) -> bool {
        path.starts_with(&self.physical) && fs::canonicalize(path).is_ok_and(|p| p == path)
    }
}

/// Metadata discovery is not header qualification or permission to ingest this file.
#[derive(Clone)]
pub struct CodexSourceCandidate {
    root: ApprovedCodexRoot,
    path: PathBuf,
    identity: PhysicalId,
}
impl CodexSourceCandidate {
    pub fn identity(&self) -> PhysicalId {
        self.identity
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Retirement evidence: false proves absence/rebinding; other IO remains an error.
    pub fn current_binding(&self) -> CaptureResult<bool> {
        if !self.root.current_binding()? {
            return Ok(false);
        }
        let Some(resolved) = binding_observation(fs::canonicalize(&self.path))? else {
            return Ok(false);
        };
        if !self.path.starts_with(&self.root.physical) || resolved != self.path {
            return Ok(false);
        }
        let Some(metadata) = binding_observation(fs::symlink_metadata(&self.path))? else {
            return Ok(false);
        };
        Ok(metadata.is_file()
            && !metadata.file_type().is_symlink()
            && PhysicalId::from(&metadata)? == self.identity)
    }
    /// Availability fence only; false may include unknown IO and cannot prove retirement.
    pub fn revalidate(&self) -> CaptureResult<bool> {
        if !self.root.revalidate()? || !self.root.contains_exact(&self.path) {
            return Ok(false);
        }
        let Ok(metadata) = fs::symlink_metadata(&self.path) else {
            return Ok(false);
        };
        Ok(metadata.is_file()
            && !metadata.file_type().is_symlink()
            && PhysicalId::from(&metadata)? == self.identity)
    }
}

// Absence is a known negative binding. Other OS outcomes cannot justify retirement.
fn binding_observation<T>(result: std::io::Result<T>) -> CaptureResult<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests;
