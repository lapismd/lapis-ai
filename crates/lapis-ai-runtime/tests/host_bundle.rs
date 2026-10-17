#![cfg(unix)]
use lapis_ai_runtime::ResourceBundle;
use sha2::{Digest, Sha256};

#[test]
fn internal_symlinks_and_manifest_symlinks_are_not_executable_resources() {
    let root = tempfile::tempdir().unwrap();
    let bytes = b"fixture executable";
    let digest = format!("{:x}", Sha256::digest(bytes));
    std::fs::write(root.path().join("real-helper"), bytes).unwrap();
    let mut manifest = format!(
        "version=1\npackage_version='{}'\ntarget='{}'\n",
        env!("CARGO_PKG_VERSION"),
        env!("LAPIS_AI_TARGET")
    );
    for name in [
        "lapis-acp",
        "lapis-agent",
        "lapis-agent-tools",
        "lapis-compute",
    ] {
        std::os::unix::fs::symlink("real-helper", root.path().join(name)).unwrap();
        manifest.push_str(&format!(
            "[[binary]]\nname='{name}'\npath='{name}'\nsha256='{digest}'\n"
        ));
    }
    let file = root.path().join("runtime-manifest.toml");
    std::fs::write(&file, &manifest).unwrap();
    assert!(ResourceBundle::load(root.path()).is_err());
    for name in [
        "lapis-acp",
        "lapis-agent",
        "lapis-agent-tools",
        "lapis-compute",
    ] {
        std::fs::remove_file(root.path().join(name)).unwrap();
        std::fs::write(root.path().join(name), bytes).unwrap();
    }
    assert!(ResourceBundle::load(root.path()).is_ok());
    std::fs::rename(&file, root.path().join("real-manifest")).unwrap();
    std::os::unix::fs::symlink("real-manifest", &file).unwrap();
    assert!(ResourceBundle::load(root.path()).is_err());
}
