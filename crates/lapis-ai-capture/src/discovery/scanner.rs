use super::{ApprovedCodexRoot, CaptureResult, CodexSourceCandidate, PhysicalId, valid_path};
use std::{
    fs::{self, ReadDir},
    path::PathBuf,
};

const MAX_DEPTH: usize = 8;
struct Frame {
    path: PathBuf,
    identity: PhysicalId,
    entries: ReadDir,
    depth: usize,
}
impl Frame {
    fn open(root: &ApprovedCodexRoot, path: PathBuf, depth: usize) -> CaptureResult<Self> {
        valid_path(&path)?;
        if !root.contains_exact(&path) {
            return Err("Codex directory unavailable".into());
        }
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("Codex directory unavailable".into());
        }
        let frame = Self {
            identity: PhysicalId::from(&metadata)?,
            entries: fs::read_dir(&path)?,
            path,
            depth,
        };
        if !frame.revalidate(root)? {
            return Err("Codex directory changed".into());
        }
        Ok(frame)
    }
    fn revalidate(&self, root: &ApprovedCodexRoot) -> CaptureResult<bool> {
        if !root.contains_exact(&self.path) {
            return Ok(false);
        }
        let Ok(metadata) = fs::symlink_metadata(&self.path) else {
            return Ok(false);
        };
        Ok(metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && PhysicalId::from(&metadata)? == self.identity)
    }
}

pub struct CodexDiscoveryStep {
    pub candidates: Vec<CodexSourceCandidate>,
    pub entries_examined: usize,
    pub skipped_entries: usize,
    pub unavailable_entries: usize,
    pub depth_limited: usize,
    pub complete: bool,
}
/// Resumable metadata walk: stack memory/handles are bounded by directory depth.
pub struct CodexDirectoryScanner {
    root: ApprovedCodexRoot,
    stack: Vec<Frame>,
    cursor_layout: bool,
    projects_root: bool,
    hint_extension: Option<String>,
}
impl CodexDirectoryScanner {
    pub fn new(root: ApprovedCodexRoot) -> CaptureResult<Self> {
        Self::with_layout(root, false, None)
    }
    /// Cursor's default project tree contains generated payloads unrelated to capture.
    pub fn cursor(root: ApprovedCodexRoot) -> CaptureResult<Self> {
        Self::with_layout(root, true, None)
    }
    /// An optional consumer hint narrows discovery without granting activity admission.
    pub fn cursor_with_hint_extension(
        root: ApprovedCodexRoot,
        extension: &str,
    ) -> CaptureResult<Self> {
        if extension.is_empty()
            || extension.len() > 64
            || !extension
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err("invalid source hint extension".into());
        }
        Self::with_layout(root, true, Some(extension.to_owned()))
    }
    fn with_layout(
        root: ApprovedCodexRoot,
        cursor_layout: bool,
        hint_extension: Option<String>,
    ) -> CaptureResult<Self> {
        let projects_root = root
            .physical
            .file_name()
            .is_some_and(|name| name == "projects");
        let mut scanner = Self {
            root,
            stack: Vec::new(),
            cursor_layout,
            projects_root,
            hint_extension,
        };
        scanner.restart()?;
        Ok(scanner)
    }
    pub fn restart(&mut self) -> CaptureResult<()> {
        self.stack.clear();
        if !self.root.revalidate()? {
            return Err("approved Codex root unavailable".into());
        }
        self.stack
            .push(Frame::open(&self.root, self.root.physical.clone(), 0)?);
        Ok(())
    }
    pub fn step(
        &mut self,
        max_entries: usize,
        max_candidates: usize,
    ) -> CaptureResult<CodexDiscoveryStep> {
        if !(1..=4096).contains(&max_entries) || !(1..=256).contains(&max_candidates) {
            return Err("Codex discovery step bounds".into());
        }
        if !self.root.revalidate()? {
            self.stack.clear();
            return Err("approved Codex root unavailable".into());
        }
        let mut step = CodexDiscoveryStep {
            candidates: Vec::new(),
            entries_examined: 0,
            skipped_entries: 0,
            unavailable_entries: 0,
            depth_limited: 0,
            complete: false,
        };
        while step.entries_examined < max_entries && step.candidates.len() < max_candidates {
            let Some(frame) = self.stack.last_mut() else {
                break;
            };
            if !frame.revalidate(&self.root)? {
                self.stack.pop();
                step.unavailable_entries += 1;
                continue;
            }
            let Some(entry) = frame.entries.next() else {
                self.stack.pop();
                continue;
            };
            step.entries_examined += 1;
            let Ok(entry) = entry else {
                self.stack.pop();
                step.unavailable_entries += 1;
                continue;
            };
            let path = entry.path();
            let depth = frame.depth;
            let metadata = match fs::symlink_metadata(&path) {
                Ok(value) => value,
                Err(_) => {
                    step.unavailable_entries += 1;
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                step.skipped_entries += 1;
                continue;
            }
            if metadata.is_dir() {
                if self.cursor_layout
                    && (path.file_name().is_some_and(|name| {
                        name == "mcps" || name == "canvases" || name == "node_modules"
                    }) || (self.projects_root
                        && depth == 1
                        && path
                            .file_name()
                            .is_none_or(|name| name != "agent-transcripts")))
                {
                    step.skipped_entries += 1;
                    continue;
                }
                if depth >= MAX_DEPTH {
                    step.depth_limited += 1;
                    continue;
                }
                match Frame::open(&self.root, path, depth + 1) {
                    Ok(next) => self.stack.push(next),
                    Err(_) => step.unavailable_entries += 1,
                }
            } else if metadata.is_file() && path.extension().is_some_and(|e| e == "jsonl") {
                if self.cursor_layout && self.projects_root && depth < 2 {
                    step.skipped_entries += 1;
                    continue;
                }
                if self.hint_extension.as_ref().is_some_and(|extension| {
                    !fs::symlink_metadata(path.with_extension(extension))
                        .is_ok_and(|binding| binding.is_file() && !binding.file_type().is_symlink())
                }) {
                    step.skipped_entries += 1;
                    continue;
                }
                if valid_path(&path).is_err() || !self.root.contains_exact(&path) {
                    step.unavailable_entries += 1;
                    continue;
                }
                let candidate = CodexSourceCandidate {
                    root: self.root.clone(),
                    path,
                    identity: PhysicalId::from(&metadata)?,
                };
                if candidate.revalidate()? {
                    step.candidates.push(candidate);
                } else {
                    step.unavailable_entries += 1;
                }
            } else {
                step.skipped_entries += 1;
            }
        }
        if !self.root.revalidate()? {
            self.stack.clear();
            return Err("approved Codex root unavailable".into());
        }
        step.complete = self.stack.is_empty();
        Ok(step)
    }
}
