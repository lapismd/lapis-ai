//! Private managed weights and installed-path receipts. Start never downloads.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(Clone, Serialize, Deserialize)]
struct Installed {
    primary: PathBuf,
    files: Vec<PathBuf>,
}
fn read() -> Result<BTreeMap<String, Installed>> {
    let path = Path::new("installed-models.json");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    ensure!(
        std::fs::metadata(path)?.len() <= 262144,
        "model index too large"
    );
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn save(value: &BTreeMap<String, Installed>) -> Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let path = Path::new("installed-models.pending");
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let mut file = options.open(path)?;
    file.write_all(&serde_json::to_vec(value)?)?;
    file.sync_all()?;
    std::fs::rename(path, "installed-models.json")?;
    Ok(())
}
pub fn record(id: &str, primary: PathBuf, files: Vec<PathBuf>) -> Result<()> {
    let root = std::env::current_dir()?.canonicalize()?;
    let mut primary = primary.canonicalize()?;
    let mut files = files
        .into_iter()
        .map(|p| p.canonicalize())
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(
        primary.starts_with(&root) && files.iter().all(|p| p.starts_with(&root)),
        "weights outside managed storage"
    );
    // The resolver identifies local GGUFs by extension; HF's blob cache has none.
    if primary
        .extension()
        .is_none_or(|extension| extension != "gguf")
    {
        use sha2::{Digest, Sha256};
        let directory = root.join("managed-models");
        std::fs::create_dir_all(&directory)?;
        let alias = directory.join(format!("{:x}.gguf", Sha256::digest(id.as_bytes())));
        if alias.exists() {
            ensure!(
                std::fs::symlink_metadata(&alias)?.is_file(),
                "invalid managed model alias"
            );
            std::fs::remove_file(&alias)?;
        }
        std::fs::hard_link(&primary, &alias)?;
        files.push(alias.clone());
        primary = alias;
    }
    let mut index = read()?;
    ensure!(index.len() < 64 || index.contains_key(id), "model limit");
    index.insert(id.into(), Installed { primary, files });
    save(&index)
}
pub fn installed(id: &str) -> Result<PathBuf> {
    let index = read()?;
    let item = index.get(id).context("model not installed")?;
    let root = std::env::current_dir()?.canonicalize()?;
    ensure!(
        item.primary.is_file() && item.primary.canonicalize()?.starts_with(root),
        "installed file unavailable"
    );
    Ok(item.primary.clone())
}
pub async fn import(path: &str) -> Result<String> {
    use sha2::{Digest, Sha256};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let source = Path::new(path);
    ensure!(source.is_absolute(), "absolute import path required");
    let metadata = tokio::fs::symlink_metadata(source).await?;
    ensure!(
        metadata.is_file()
            && !metadata.file_type().is_symlink()
            && metadata.len() >= 24
            && metadata.len() <= 256 * 1024 * 1024 * 1024,
        "invalid GGUF source"
    );
    tokio::fs::create_dir_all("imports").await?;
    let temporary = PathBuf::from("imports/pending.gguf");
    let mut input = tokio::fs::File::open(source).await?;
    let mut magic = [0u8; 4];
    input.read_exact(&mut magic).await?;
    ensure!(&magic == b"GGUF", "GGUF required");
    let mut output = tokio::fs::File::create(&temporary).await?;
    output.write_all(&magic).await?;
    let mut hash = Sha256::new();
    hash.update(magic);
    let mut bytes = vec![0u8; 1024 * 1024];
    let mut length = 4u64;
    loop {
        let n = input.read(&mut bytes).await?;
        if n == 0 {
            break;
        }
        length += n as u64;
        ensure!(length <= metadata.len(), "source changed");
        hash.update(&bytes[..n]);
        output.write_all(&bytes[..n]).await?;
    }
    ensure!(length == metadata.len(), "source changed");
    output.sync_all().await?;
    drop(output);
    let id = format!("imported-{}", hex::encode(hash.finalize()));
    let target = PathBuf::from(format!("imports/{id}.gguf"));
    tokio::fs::rename(temporary, &target).await?;
    record(&id, target.clone(), vec![target])?;
    Ok(id)
}
pub async fn remove(id: &str) -> Result<()> {
    let mut index = read()?;
    let removed = index.remove(id).context("model not installed")?;
    let root = std::env::current_dir()?.canonicalize()?;
    for path in &removed.files {
        if index.values().any(|m| m.files.contains(path)) {
            continue;
        }
        ensure!(path.starts_with(&root), "unmanaged weight");
        if path.exists() {
            ensure!(path.canonicalize()?.starts_with(&root), "unmanaged weight");
            tokio::fs::remove_file(path).await?;
        }
    }
    save(&index)
}
