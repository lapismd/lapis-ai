//! Filesystem operations bounded to the canonical session workspace.
use serde_json::Value;
use std::{
    fs,
    path::{Component, Path},
};
const LIMIT: usize = 262144;
#[cfg(unix)]
fn open(root: &Path, input: &str, write: bool) -> Result<fs::File, &'static str> {
    use nix::fcntl::{OFlag, open, openat};
    use nix::sys::stat::Mode;
    let components = Path::new(input)
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect::<Vec<_>>();
    if components.is_empty()
        || components
            .iter()
            .any(|c| !matches!(c,Component::Normal(n) if *n!=".jj" && *n!=".git"))
    {
        return Err("path outside session workspace");
    }
    let mut directory = open(
        root,
        OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW,
        Mode::empty(),
    )
    .map_err(|_| "workspace unavailable")?;
    for component in &components[..components.len() - 1] {
        directory = openat(
            &directory,
            component.as_os_str(),
            OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|_| "path unavailable")?;
    }
    let flags = if write {
        OFlag::O_WRONLY | OFlag::O_CREAT
    } else {
        OFlag::O_RDONLY
    };
    let descriptor = openat(
        &directory,
        components.last().unwrap().as_os_str(),
        flags | OFlag::O_NOFOLLOW | OFlag::O_NONBLOCK,
        Mode::S_IRUSR | Mode::S_IWUSR,
    )
    .map_err(|_| "file unavailable")?;
    let file = fs::File::from(descriptor);
    let metadata = file.metadata().map_err(|_| "file unavailable")?;
    if !metadata.is_file() {
        return Err("regular file required");
    }
    if write {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err("linked file cannot be overwritten");
        }
        file.set_len(0).map_err(|_| "write failed")?;
    }
    Ok(file)
}
#[cfg(not(unix))]
fn open(_root: &Path, _input: &str, _write: bool) -> Result<fs::File, &'static str> {
    Err("unsupported tool platform")
}

pub fn call(root: &Path, name: &str, args: &Value) -> Result<String, &'static str> {
    match name {
        "read_file" => {
            use std::io::Read;
            let file = open(root, args["path"].as_str().ok_or("path required")?, false)?;
            let mut bytes = Vec::new();
            file.take(LIMIT as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "file unavailable")?;
            if bytes.len() > LIMIT {
                return Err("file exceeds read budget");
            }
            String::from_utf8(bytes).map_err(|_| "file is not UTF-8")
        }
        "write_file" => {
            let content = args["content"].as_str().ok_or("content required")?;
            if content.len() > LIMIT {
                return Err("content exceeds write budget");
            }
            use std::io::Write;
            let mut file = open(root, args["path"].as_str().ok_or("path required")?, true)?;
            file.write_all(content.as_bytes())
                .map_err(|_| "write failed")?;
            Ok("File saved in session workspace".into())
        }
        "search" => {
            let query = args["query"]
                .as_str()
                .filter(|q| !q.is_empty() && q.len() <= 4096)
                .ok_or("query required")?;
            let mut directories = vec![root.to_path_buf()];
            let mut result = String::new();
            let mut visited = 0;
            while let Some(directory) = directories.pop() {
                for entry in fs::read_dir(directory)
                    .map_err(|_| "directory unavailable")?
                    .flatten()
                {
                    visited += 1;
                    if visited > 10000 || result.len() > 32768 {
                        return Ok(result + "\nSearch budget reached");
                    }
                    let Ok(kind) = entry.file_type() else {
                        continue;
                    };
                    let name = entry.file_name();
                    if kind.is_symlink()
                        || matches!(
                            name.to_str(),
                            Some(".jj" | ".git" | "node_modules" | "target")
                        )
                    {
                        continue;
                    }
                    if kind.is_dir() {
                        directories.push(entry.path());
                        continue;
                    }
                    if !entry.metadata().is_ok_and(|m| m.len() <= LIMIT as u64) {
                        continue;
                    }
                    if let Ok(text) = call(
                        root,
                        "read_file",
                        &serde_json::json!({"path":entry.path().strip_prefix(root).map_err(|_|"path outside session workspace")?.to_string_lossy()}),
                    ) {
                        for (index, line) in text
                            .lines()
                            .enumerate()
                            .filter(|(_, l)| l.contains(query))
                            .take(128)
                        {
                            result.push_str(&format!(
                                "{}:{}: {}\n",
                                entry.path().strip_prefix(root).unwrap().display(),
                                index + 1,
                                line.chars().take(512).collect::<String>()
                            ));
                        }
                    }
                }
            }
            Ok(result)
        }
        _ => Err("unknown tool"),
    }
}
