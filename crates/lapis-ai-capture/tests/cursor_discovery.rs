use lapis_ai_capture::discovery::{ApprovedCodexRoot, CodexDirectoryScanner};
use std::fs;

#[test]
fn cursor_layout_and_consumer_hints_are_independent_of_publication() {
    let base = tempfile::tempdir().unwrap();
    let root = base.path().join("projects");
    let transcripts = root.join("example/agent-transcripts");
    fs::create_dir_all(&transcripts).unwrap();
    let selected = transcripts.join("session.jsonl");
    fs::write(&selected, b"not yet format-qualified\n").unwrap();
    for name in ["mcps", "canvases", "node_modules"] {
        fs::create_dir_all(root.join("example").join(name)).unwrap();
        fs::write(
            root.join("example").join(name).join("unrelated.jsonl"),
            b"private unrelated body",
        )
        .unwrap();
    }
    let approved = ApprovedCodexRoot::open(&root).unwrap().unwrap();
    let mut unrestricted = CodexDirectoryScanner::cursor(approved.clone()).unwrap();
    let step = unrestricted.step(100, 100).unwrap();
    assert!(step.complete);
    assert_eq!(step.candidates.len(), 1);
    assert_eq!(step.candidates[0].path(), selected.canonicalize().unwrap());
    let mut hinted =
        CodexDirectoryScanner::cursor_with_hint_extension(approved.clone(), "consumer-hint.json")
            .unwrap();
    assert!(hinted.step(100, 100).unwrap().candidates.is_empty());
    fs::write(selected.with_extension("consumer-hint.json"), b"{}\n").unwrap();
    hinted.restart().unwrap();
    let step = hinted.step(100, 100).unwrap();
    assert_eq!(step.candidates.len(), 1);
    assert_eq!(step.candidates[0].identity().inode, fs_identity(&selected));
    assert!(step.candidates[0].current_binding().unwrap());
    assert!(
        CodexDirectoryScanner::cursor_with_hint_extension(approved.clone(), "../escape").is_err()
    );
    assert!(CodexDirectoryScanner::cursor_with_hint_extension(approved, "").is_err());
}

#[cfg(unix)]
fn fs_identity(path: &std::path::Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(path).unwrap().ino()
}
#[cfg(not(unix))]
fn fs_identity(_: &std::path::Path) -> u64 {
    0
}
