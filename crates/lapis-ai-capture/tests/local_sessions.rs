use lapis_ai_capture::{
    discovery::{ApprovedCodexRoot, CodexDirectoryScanner},
    sessions::{JsonlSnapshot, OpenCodeDatabase},
};
use std::{fs, io::Write};

fn candidate(path: &std::path::Path) -> lapis_ai_capture::discovery::CodexSourceCandidate {
    let root = ApprovedCodexRoot::open(path).unwrap().unwrap();
    CodexDirectoryScanner::new(root)
        .unwrap()
        .step(32, 16)
        .unwrap()
        .candidates
        .remove(0)
}

#[test]
fn jsonl_pages_are_bounded_and_fenced_without_source_writes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("session.jsonl");
    let original = "{\"role\":\"user\"}\n{\"role\":\"assistant\"}\n";
    fs::write(&path, original).unwrap();
    let mut reader = JsonlSnapshot::open(candidate(root.path()), 1024).unwrap();
    let first = reader.next(1, &|| Ok(())).unwrap();
    assert_eq!(first.records.len(), 1);
    assert!(!first.complete);
    let second = reader.next(1, &|| Ok(())).unwrap();
    assert_eq!(second.records.len(), 1);
    assert!(second.complete);
    assert_ne!(second.sha256, [0; 32]);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    writeln!(
        fs::OpenOptions::new().append(true).open(&path).unwrap(),
        "{{}}"
    )
    .unwrap();
    assert!(reader.next(1, &|| Ok(())).is_err());
}

#[test]
fn partial_rows_replacement_and_cancel_never_complete() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("session.jsonl");
    fs::write(&path, "{\"role\":\"user\"}").unwrap();
    let mut reader = JsonlSnapshot::open(candidate(root.path()), 1024).unwrap();
    assert!(reader.next(8, &|| Ok(())).is_err());
    fs::write(&path, "{}\n").unwrap();
    let mut reader = JsonlSnapshot::open(candidate(root.path()), 1024).unwrap();
    assert!(reader.next(8, &|| Err("cancelled".into())).is_err());
    fs::rename(&path, root.path().join("retained")).unwrap();
    fs::write(&path, "{}\n").unwrap();
    assert!(reader.next(8, &|| Ok(())).is_err());
}

fn database(path: &std::path::Path) {
    let connection = rusqlite::Connection::open(path).unwrap();
    connection.execute_batch("CREATE TABLE session(id TEXT PRIMARY KEY,directory TEXT,title TEXT,version TEXT,time_created INTEGER,time_updated INTEGER); CREATE TABLE message(id TEXT PRIMARY KEY,session_id TEXT,time_created INTEGER,time_updated INTEGER,data TEXT); CREATE TABLE part(id TEXT PRIMARY KEY,message_id TEXT,session_id TEXT,time_created INTEGER,time_updated INTEGER,data TEXT); INSERT INTO session VALUES ('ses_one','/fixture/project','A fixture session','1.18.31',1,2); INSERT INTO message VALUES ('msg_one','ses_one',1,2,'{\"role\":\"user\"}'); INSERT INTO part VALUES ('prt_one','msg_one','ses_one',1,2,'{\"type\":\"text\",\"text\":\"hello\"}');").unwrap();
}

#[test]
fn opencode_metadata_and_selected_messages_use_read_only_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("opencode.db");
    database(&path);
    let before = fs::read(&path).unwrap();
    let reader = OpenCodeDatabase::open(&path).unwrap();
    let rows = reader.sessions("", 8).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "A fixture session");
    assert_eq!(
        rows[0].directory,
        std::path::PathBuf::from("/fixture/project")
    );
    let mut snapshot = reader.snapshot("ses_one").unwrap();
    let page = snapshot.next(1, &|| Ok(())).unwrap();
    assert_eq!(page.messages.len(), 1);
    assert_eq!(page.messages[0]["parts"][0]["text"], "hello");
    assert!(page.complete);
    assert!(reader.snapshot("ses_foreign").is_err());
    drop(snapshot);
    drop(reader);
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn sqlite_symlinks_and_incompatible_schema_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("opencode.db");
    fs::write(&path, "not a database").unwrap();
    assert!(OpenCodeDatabase::open(&path).is_err());
    #[cfg(unix)]
    {
        fs::remove_file(&path).unwrap();
        database(&path);
        let link = root.path().join("link.db");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(OpenCodeDatabase::open(&link).is_err());
    }
}

#[test]
fn sqlite_read_transaction_does_not_mix_later_source_writes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("opencode.db");
    database(&path);
    let writer = rusqlite::Connection::open(&path).unwrap();
    writer.execute_batch("PRAGMA journal_mode=WAL; INSERT INTO message VALUES ('msg_two','ses_one',3,3,'{\"role\":\"assistant\"}');").unwrap();
    let reader = OpenCodeDatabase::open(&path).unwrap();
    let mut snapshot = reader.snapshot("ses_one").unwrap();
    assert!(!snapshot.next(1, &|| Ok(())).unwrap().complete);
    writer
        .execute_batch(
            "INSERT INTO message VALUES ('msg_three','ses_one',4,4,'{\"role\":\"assistant\"}');",
        )
        .unwrap();
    let final_page = snapshot.next(1, &|| Ok(())).unwrap();
    assert!(final_page.complete);
    assert_eq!(final_page.total_messages, 2);
    assert_eq!(final_page.messages[0]["info"]["id"], "msg_two");
    assert!(snapshot.next(1, &|| Err("cancelled".into())).is_err());
}
