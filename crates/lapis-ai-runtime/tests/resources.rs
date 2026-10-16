use lapis_ai_runtime::{Config, Error, ResourceBundle};
use sha2::{Digest, Sha256};
use std::path::Path;
fn bundle(root: &Path, target: &str, first: &str) -> String {
    let mut manifest = format!("version=1\npackage_version='0.1.0'\ntarget='{target}'\n");
    for (index, name) in [
        "lapis-acp",
        "lapis-agent",
        "lapis-agent-tools",
        "lapis-compute",
    ]
    .iter()
    .enumerate()
    {
        std::fs::write(root.join(name), b"helper").unwrap();
        let path = if index == 0 { first } else { name };
        manifest += &format!(
            "[[binary]]\nname='{name}'\npath='{path}'\nsha256='{:x}'\n",
            Sha256::digest(b"helper")
        );
    }
    manifest
}
fn save(root: &Path, text: &str) {
    std::fs::write(root.join("runtime-manifest.toml"), text).unwrap();
}
#[test]
fn verified_bundle_configures_helpers_and_rejects_tampering() {
    let root = tempfile::tempdir().unwrap();
    let manifest = bundle(root.path(), env!("LAPIS_AI_TARGET"), "lapis-acp");
    save(root.path(), &manifest);
    let bundle = ResourceBundle::load(root.path()).unwrap();
    let mut config = Config::parse("version=1", root.path()).unwrap();
    bundle.configure(&mut config);
    assert_eq!(
        config.runtime.acp.unwrap(),
        root.path().canonicalize().unwrap().join("lapis-acp")
    );
    std::fs::write(root.path().join("lapis-agent"), b"changed").unwrap();
    assert!(matches!(
        ResourceBundle::load(root.path()),
        Err(Error("bundle_checksum_mismatch"))
    ));
}
#[test]
fn mismatched_platform_and_escaping_paths_fail_before_launch() {
    let root = tempfile::tempdir().unwrap();
    let manifest = bundle(root.path(), "foreign-platform", "lapis-acp");
    save(root.path(), &manifest);
    assert!(matches!(
        ResourceBundle::load(root.path()),
        Err(Error("unsupported_bundle"))
    ));
    let manifest = bundle(root.path(), env!("LAPIS_AI_TARGET"), "../lapis-acp");
    save(root.path(), &manifest);
    assert!(matches!(
        ResourceBundle::load(root.path()),
        Err(Error("invalid_bundle_path"))
    ));
}
#[test]
fn native_files_require_the_pinned_abi_and_manifest() {
    let root = tempfile::tempdir().unwrap();
    let manifest = bundle(root.path(), env!("LAPIS_AI_TARGET"), "lapis-acp");
    save(
        root.path(),
        &(manifest.clone() + "[[native_file]]\npath='native-runtime/lib'\nsha256='ignored'\n"),
    );
    assert!(matches!(
        ResourceBundle::load(root.path()),
        Err(Error("unsupported_bundle"))
    ));
    save(
        root.path(),
        &("native_abi='0.1.44'\n".to_owned()
            + &manifest
            + "[[native_file]]\npath='native-runtime/lib'\nsha256='ignored'\n"),
    );
    assert!(matches!(
        ResourceBundle::load(root.path()),
        Err(Error("incomplete_native_bundle"))
    ));
}
#[cfg(unix)]
#[test]
fn symlink_outside_bundle_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let manifest = bundle(root.path(), env!("LAPIS_AI_TARGET"), "lapis-acp");
    std::fs::write(outside.path().join("helper"), b"helper").unwrap();
    std::fs::remove_file(root.path().join("lapis-acp")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("helper"), root.path().join("lapis-acp"))
        .unwrap();
    save(root.path(), &manifest);
    assert!(matches!(
        ResourceBundle::load(root.path()),
        Err(Error("invalid_bundle_path"))
    ));
}
