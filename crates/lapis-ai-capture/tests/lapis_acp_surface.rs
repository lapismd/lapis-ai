use lapis_ai_capture::{SourceProfile, source_file::inspect_codex};
use serde_json::json;
use std::fs;
#[test]
fn installed_codex_lapis_acp_surface_is_exact_and_version_pinned() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("session.jsonl");
    for (source, originator, version, accepted) in [
        ("vscode", "lapis-acp", "0.156.1", true),
        ("vscode", "other-adapter", "0.156.1", false),
        ("cli", "lapis-acp", "0.156.1", false),
        ("vscode", "lapis-acp", "0.159.2", false),
        ("vscode", "Codex Desktop", "0.163.0-alpha.10.2", false),
    ] {
        let header = json!({"type":"session_meta","payload":{"id":"12345678-1234-1234-1234-123456789abc","cwd":"/workspace/vault","cli_version":version,"source":source,"originator":originator}});
        let bytes = format!("{header}\n");
        fs::write(&path, &bytes).unwrap();
        let result = inspect_codex(SourceProfile::CodexCliJsonlV1, &path, None);
        assert_eq!(result.is_ok(), accepted);
        if let Ok((_, parsed, _)) = result {
            assert_eq!(parsed.bytes, bytes.as_bytes());
        }
    }
}
