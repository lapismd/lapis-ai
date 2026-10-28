//! Bounded, process-free projection of the helper's managed model receipts.
use crate::Error;
type Result<T> = std::result::Result<T, Error>;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct Installed {
    primary: PathBuf,
    files: Vec<PathBuf>,
    #[serde(default)]
    name: Option<String>,
}
/// Device-local model facts. Paths and credentials are deliberately omitted.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeModel {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub bytes: Option<u64>,
}
/// A read never creates a directory, launches a helper or resolves credentials.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeInventory {
    pub models: Vec<ComputeModel>,
    pub runtime_prepared: bool,
}
fn document(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Ok(meta) if meta.is_file() && meta.len() <= 262_144 => fs::read(path)
            .map(Some)
            .map_err(|_| Error("compute_storage_unavailable")),
        _ => Err(Error("compute_inventory_invalid")),
    }
}
fn managed(root: &Path, path: &Path) -> Result<Option<fs::Metadata>> {
    if !path.is_absolute() || !path.starts_with(root) {
        return Err(Error("compute_inventory_invalid"));
    }
    match path.canonicalize() {
        Ok(real) if real.starts_with(root) => {
            let meta = fs::metadata(real).map_err(|_| Error("compute_storage_unavailable"))?;
            if !meta.is_file() {
                return Err(Error("compute_inventory_invalid"));
            }
            Ok(Some(meta))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        _ => Err(Error("compute_inventory_invalid")),
    }
}
impl ComputeInventory {
    /// Read an explicitly selected helper working directory, including while off.
    pub fn read(directory: &Path) -> Result<Self> {
        let root = match directory.canonicalize() {
            Ok(root) => root,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(_) => return Err(Error("compute_storage_unavailable")),
        };
        let mut models = Vec::new();
        if let Some(bytes) = document(&root.join("installed-models.json"))? {
            let index: BTreeMap<String, Installed> =
                serde_json::from_slice(&bytes).map_err(|_| Error("compute_inventory_invalid"))?;
            if index.len() > 64 {
                return Err(Error("compute_inventory_invalid"));
            }
            for (id, item) in index {
                if id.is_empty()
                    || id.len() > 256
                    || id.chars().any(char::is_control)
                    || item.files.len() > 128
                {
                    return Err(Error("compute_inventory_invalid"));
                }
                let mut installed = true;
                let mut size = 0u64;
                let mut seen = BTreeSet::new();
                for path in std::iter::once(&item.primary).chain(item.files.iter()) {
                    if let Some(meta) = managed(&root, path)? {
                        #[cfg(unix)]
                        let key = {
                            use std::os::unix::fs::MetadataExt;
                            (meta.dev(), meta.ino())
                        };
                        #[cfg(not(unix))]
                        let key = path
                            .canonicalize()
                            .map_err(|_| Error("compute_inventory_invalid"))?;
                        if seen.insert(key) {
                            size = size
                                .checked_add(meta.len())
                                .ok_or(Error("compute_inventory_invalid"))?;
                        }
                    } else {
                        installed = false;
                    }
                }
                let name = item
                    .name
                    .filter(|n| !n.is_empty() && n.len() <= 256 && !n.chars().any(char::is_control))
                    .or_else(|| {
                        installed
                            .then(|| item.primary.canonicalize().ok())
                            .flatten()
                            .filter(|path| path.starts_with(&root))
                            .and_then(|path| crate::compute_model_name::read(&path))
                    })
                    .unwrap_or_else(|| {
                        if id.starts_with("imported-") {
                            "Imported GGUF model".into()
                        } else {
                            id.rsplit('/').next().unwrap_or(&id).into()
                        }
                    });
                models.push(ComputeModel {
                    id,
                    name,
                    installed,
                    bytes: installed.then_some(size),
                });
            }
        }
        let runtime_prepared = if let Some(bytes) = document(&root.join("runtime-ready.json"))? {
            let files: Vec<PathBuf> =
                serde_json::from_slice(&bytes).map_err(|_| Error("compute_inventory_invalid"))?;
            !files.is_empty()
                && files.len() <= 64
                && files
                    .iter()
                    .all(|path| managed(&root, path).is_ok_and(|meta| meta.is_some()))
        } else {
            false
        };
        Ok(Self {
            models,
            runtime_prepared,
        })
    }
}
