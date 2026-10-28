use lapis_ai_capture::{SourceProfile, discovery::PhysicalId, source_file::*};
use std::fs;

const SESSION: &str = "12345678-1234-1234-1234-123456789abc";
fn header() -> Vec<u8> {
    let mut bytes=serde_json::to_vec(&serde_json::json!({"type":"session_meta","payload":{"id":SESSION,"cwd":"/workspace/vault","cli_version":"0.156.1","source":"cli","originator":"codex_cli_rs"}})).unwrap();
    bytes.push(b'\n');
    bytes
}
#[test]
fn inspection_retains_exact_source_bytes_and_rejects_changed_bindings() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("session.jsonl");
    let bytes = header();
    fs::write(&path, &bytes).unwrap();
    let identity = PhysicalId::from(&fs::metadata(&path).unwrap()).unwrap();
    let (file, parsed, observed) =
        inspect_codex(SourceProfile::CodexCliJsonlV1, &path, Some(identity)).unwrap();
    assert_eq!(parsed.session, SESSION);
    assert_eq!(parsed.directory, std::path::Path::new("/workspace/vault"));
    assert_eq!(parsed.bytes, bytes);
    assert!(unchanged(&file, &path, &observed).unwrap());
    assert!(inspect_codex(SourceProfile::CodexDesktopJsonlV1, &path, Some(identity)).is_err());
    fs::rename(&path, root.path().join("retained-original")).unwrap();
    fs::write(&path, b"private replacement\n").unwrap();
    assert!(!unchanged(&file, &path, &observed).unwrap());
    let error = inspect_codex(SourceProfile::CodexCliJsonlV1, &path, Some(identity))
        .err()
        .unwrap();
    assert_eq!(error.to_string(), "transcript source identity changed");
}
#[test]
fn complete_bounded_versioned_headers_are_required() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("s.jsonl");
    for bytes in [
        b"not json\n".to_vec(),
        header().strip_suffix(b"\n").unwrap().to_vec(),
        vec![b'x'; MAX_HEADER_BYTES + 1],
    ] {
        fs::write(&path, bytes).unwrap();
        assert!(inspect_codex(SourceProfile::CodexCliJsonlV1, &path, None).is_err());
    }
    let mut value: serde_json::Value = serde_json::from_slice(&header()).unwrap();
    value["payload"]["cli_version"] = serde_json::json!("unqualified-version");
    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');
    fs::write(&path, bytes).unwrap();
    assert!(inspect_codex(SourceProfile::CodexCliJsonlV1, &path, None).is_err());
    #[cfg(unix)]
    {
        let link = root.path().join("link.jsonl");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(open_regular(&link, None).is_err());
    }
}
