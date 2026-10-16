use super::*;
use serde_json::json;
struct File(std::path::PathBuf);
impl File {
    fn new(bytes: &[u8]) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "lapis-acp-tools-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::write(&path, bytes).unwrap();
        Self(path)
    }
    fn read(&self) -> Result<Vec<McpServer>, TaskRejection> {
        read(self.0.to_str())
    }
}
impl Drop for File {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
#[test]
fn trusted_tools_refuse_duplicate_names_relative_commands_and_reserved_identity() {
    let valid = json!({"name":"workspace","command":"/bin/false","args":[],"env":[]});
    assert_eq!(
        File::new(json!([valid]).to_string().as_bytes())
            .read()
            .unwrap()
            .len(),
        1
    );
    assert!(
        File::new(json!([valid, valid]).to_string().as_bytes())
            .read()
            .is_err()
    );
    for (field, value) in [
        ("command", json!("relative")),
        ("env", json!([{"name":"JJ_ACP_TASK_ID","value":"spoof"}])),
        ("name", json!("bad\nname")),
    ] {
        let mut tool = valid.clone();
        tool[field] = value;
        assert!(
            File::new(json!([tool]).to_string().as_bytes())
                .read()
                .is_err()
        );
    }
}
#[test]
fn trusted_tools_are_bounded_and_diagnostics_do_not_echo_values() {
    let file = File::new(&vec![b' '; 65537]);
    assert!(file.read().is_err());
    let file=File::new(br#"[{"name":"workspace","command":"/bin/false","args":[],"env":[{"name":"JJ_ACP_TASK_ID","value":"secret-test-value"}]}]"#);
    let error = file.read().err().unwrap();
    assert!(!format!("{error:?}").contains("secret-test-value"));
    assert!(read(None).unwrap().is_empty());
}
