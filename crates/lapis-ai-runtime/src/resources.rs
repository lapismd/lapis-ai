use crate::{Config, Error, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Deserialize)]
struct Manifest {
    version: u32,
    package_version: String,
    target: String,
    native_abi: Option<String>,
    binary: Vec<Artifact>,
    #[serde(default)]
    native_file: Vec<Artifact>,
}
#[derive(Deserialize)]
struct Artifact {
    #[serde(default)]
    name: String,
    path: PathBuf,
    sha256: String,
}
pub struct ResourceBundle {
    directory: PathBuf,
    manifest: Manifest,
}
impl ResourceBundle {
    pub fn load(directory: &Path) -> Result<Self> {
        let directory = directory
            .canonicalize()
            .map_err(|_| Error("bundle_unavailable"))?;
        let file = directory.join("runtime-manifest.toml");
        regular(&directory, Path::new("runtime-manifest.toml"))?;
        if std::fs::metadata(&file)
            .map_err(|_| Error("bundle_unavailable"))?
            .len()
            > 1024 * 1024
        {
            return Err(Error("bundle_manifest_limit"));
        }
        let text = std::fs::read_to_string(file).map_err(|_| Error("bundle_unavailable"))?;
        let manifest: Manifest =
            toml::from_str(&text).map_err(|_| Error("invalid_bundle_manifest"))?;
        if manifest.version != 1
            || manifest.package_version != env!("CARGO_PKG_VERSION")
            || manifest.target != env!("LAPIS_AI_TARGET")
            || (!manifest.native_file.is_empty()
                && manifest.native_abi.as_deref() != Some("0.1.44"))
        {
            return Err(Error("unsupported_bundle"));
        }
        let mut names = std::collections::BTreeSet::new();
        for binary in &manifest.binary {
            if !names.insert(binary.name.as_str()) {
                return Err(Error("duplicate_bundle_binary"));
            }
        }
        if names
            != [
                "lapis-acp",
                "lapis-agent",
                "lapis-agent-tools",
                "lapis-compute",
            ]
            .into_iter()
            .collect()
        {
            return Err(Error("incomplete_bundle"));
        }
        if !manifest.native_file.is_empty()
            && !manifest
                .native_file
                .iter()
                .any(|item| item.path == Path::new("native-runtime/manifest.json"))
        {
            return Err(Error("incomplete_native_bundle"));
        }
        let mut paths = std::collections::BTreeSet::new();
        for item in manifest.binary.iter().chain(&manifest.native_file) {
            regular(&directory, &item.path)?;
            if !paths.insert(&item.path) {
                return Err(Error("duplicate_bundle_path"));
            }
            if item.path.is_absolute()
                || item
                    .path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                return Err(Error("invalid_bundle_path"));
            }
            let path = directory
                .join(&item.path)
                .canonicalize()
                .map_err(|_| Error("bundle_file_unavailable"))?;
            if !path.starts_with(&directory) {
                return Err(Error("invalid_bundle_path"));
            }
            let mut file =
                std::fs::File::open(path).map_err(|_| Error("bundle_file_unavailable"))?;
            let mut hash = Sha256::new();
            let mut bytes = [0u8; 65536];
            loop {
                let size = file
                    .read(&mut bytes)
                    .map_err(|_| Error("bundle_file_unavailable"))?;
                if size == 0 {
                    break;
                }
                hash.update(&bytes[..size]);
            }
            if format!("{:x}", hash.finalize()) != item.sha256 {
                return Err(Error("bundle_checksum_mismatch"));
            }
        }
        Ok(Self {
            directory,
            manifest,
        })
    }
    pub fn configure(&self, config: &mut Config) {
        for binary in &self.manifest.binary {
            let path = Some(self.directory.join(&binary.path));
            match binary.name.as_str() {
                "lapis-acp" => config.runtime.acp = path,
                "lapis-agent" => config.runtime.agent = path,
                "lapis-agent-tools" => config.runtime.tools = path,
                "lapis-compute" => config.runtime.compute = path,
                _ => {}
            }
        }
        if !self.manifest.native_file.is_empty() {
            config.compute.native_runtime = Some(self.directory.join("native-runtime"));
        }
    }
}

fn regular(directory: &Path, relative: &Path) -> Result<()> {
    let mut path = directory.to_path_buf();
    let components = relative.components().collect::<Vec<_>>();
    if components.is_empty() {
        return Err(Error("invalid_bundle_path"));
    }
    for (index, component) in components.iter().enumerate() {
        let std::path::Component::Normal(name) = component else {
            return Err(Error("invalid_bundle_path"));
        };
        path.push(name);
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| Error("bundle_file_unavailable"))?;
        if metadata.file_type().is_symlink()
            || (index + 1 == components.len() && !metadata.is_file())
            || (index + 1 < components.len() && !metadata.is_dir())
        {
            return Err(Error("invalid_bundle_path"));
        }
    }
    Ok(())
}
