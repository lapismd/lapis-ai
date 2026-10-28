use lapis_ai_capture::{SourceProfile as CaptureProfile, discovery::*};
use std::fs;

#[test]
fn codex_locations_honor_explicit_roots_then_home_override_without_creating_paths() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let override_home = root.path().join("codex-data");
    let explicit = root.path().join("approved");
    let profile = CaptureProfile::CodexCliJsonlV1;
    assert_eq!(
        resolve_codex_roots(profile, &[], Some(&home), None).unwrap(),
        vec![home.join(".codex/sessions")]
    );
    assert_eq!(
        resolve_codex_roots(profile, &[], Some(&home), Some(&override_home)).unwrap(),
        vec![override_home.join("sessions")]
    );
    assert_eq!(
        resolve_codex_roots(
            CaptureProfile::CodexDesktopJsonlV1,
            std::slice::from_ref(&explicit),
            None,
            Some(std::path::Path::new("relative-invalid"))
        )
        .unwrap(),
        vec![explicit]
    );
    assert!(!home.exists());
    assert!(!override_home.exists());
    assert!(resolve_codex_roots(profile, &[], None, None).is_err());
    let oversized: Vec<_> = (0..17).map(|i| root.path().join(i.to_string())).collect();
    assert!(resolve_codex_roots(profile, &oversized, None, None).is_err());
    assert!(
        resolve_codex_roots(
            CaptureProfile::CursorDesktopTranscriptV1,
            &[],
            Some(&home),
            None
        )
        .is_err()
    );
    assert!(
        resolve_codex_roots(
            profile,
            &[root.path().to_owned(), root.path().to_owned()],
            None,
            None
        )
        .is_err()
    );
    assert!(
        resolve_codex_roots(
            profile,
            &[],
            Some(&home),
            Some(std::path::Path::new("relative"))
        )
        .is_err()
    );
}

#[test]
fn incremental_discovery_finishes_large_directories_without_reading_transcript_bodies() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("sessions/2026/10/02");
    fs::create_dir_all(&folder).unwrap();
    for i in 0..19 {
        // Not a valid transcript: discovery is metadata only, qualification is separate.
        fs::write(folder.join(format!("candidate-{i}.jsonl")), b"not json\n").unwrap();
    }
    fs::write(folder.join("unrelated.txt"), b"private unrelated body").unwrap();
    let approved = ApprovedCodexRoot::open(&root.path().join("sessions"))
        .unwrap()
        .unwrap();
    let mut scanner = CodexDirectoryScanner::new(approved).unwrap();
    let mut found = std::collections::BTreeSet::new();
    let mut complete = false;
    for _ in 0..100 {
        let step = scanner.step(3, 2).unwrap();
        assert!(step.entries_examined <= 3);
        assert!(step.candidates.len() <= 2);
        for candidate in step.candidates {
            assert!(candidate.revalidate().unwrap());
            assert!(found.insert(candidate.path().to_owned()));
        }
        if step.complete {
            complete = true;
            break;
        }
    }
    assert!(complete);
    assert_eq!(found.len(), 19);
    assert!(scanner.step(3, 2).unwrap().candidates.is_empty());
    fs::write(folder.join("late.jsonl"), b"later body\n").unwrap();
    scanner.restart().unwrap();
    found.clear();
    complete = false;
    for _ in 0..100 {
        let step = scanner.step(3, 2).unwrap();
        for candidate in step.candidates {
            assert!(found.insert(candidate.path().to_owned()));
        }
        if step.complete {
            complete = true;
            break;
        }
    }
    assert!(complete);
    assert_eq!(found.len(), 20);
}

#[cfg(unix)]
#[test]
fn discovery_withholds_rebound_roots_symlink_children_and_changed_candidates() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let approved = root.path().join("approved");
    let outside = root.path().join("outside");
    fs::create_dir_all(&approved).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("foreign.jsonl"), b"foreign").unwrap();
    symlink(&outside, approved.join("escape")).unwrap();
    symlink(outside.join("foreign.jsonl"), approved.join("escape.jsonl")).unwrap();
    let file = approved.join("local.jsonl");
    fs::write(&file, b"local").unwrap();
    let mut scanner =
        CodexDirectoryScanner::new(ApprovedCodexRoot::open(&approved).unwrap().unwrap()).unwrap();
    let step = scanner.step(20, 20).unwrap();
    assert!(step.complete);
    assert_eq!(step.candidates.len(), 1);
    let candidate = &step.candidates[0];
    fs::rename(&file, approved.join("retained.jsonl")).unwrap();
    fs::write(&file, b"replacement").unwrap();
    assert!(!candidate.revalidate().unwrap());
    fs::rename(&approved, root.path().join("old-approved")).unwrap();
    symlink(&outside, &approved).unwrap();
    assert!(scanner.restart().is_err());
    assert!(!candidate.revalidate().unwrap());
    assert!(
        ApprovedCodexRoot::open(&root.path().join("missing"))
            .unwrap()
            .is_none()
    );
}

#[cfg(unix)]
#[test]
fn interrupted_scan_withholds_rebound_child_and_reports_depth_and_budget_limits() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let approved = root.path().join("approved");
    let child = approved.join("child");
    let outside = root.path().join("outside");
    fs::create_dir_all(&child).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("foreign.jsonl"), b"foreign").unwrap();
    let mut scanner =
        CodexDirectoryScanner::new(ApprovedCodexRoot::open(&approved).unwrap().unwrap()).unwrap();
    assert!(scanner.step(0, 1).is_err());
    assert!(scanner.step(1, 257).is_err());
    let first = scanner.step(1, 1).unwrap();
    assert!(first.candidates.is_empty());
    assert!(!first.complete);
    fs::rename(&child, root.path().join("old-child")).unwrap();
    symlink(&outside, &child).unwrap();
    let next = scanner.step(20, 20).unwrap();
    assert_eq!(next.unavailable_entries, 1);
    assert!(next.candidates.is_empty());
    assert!(next.complete);
    fs::remove_file(&child).unwrap();
    let mut deep = approved.clone();
    for _ in 0..9 {
        deep.push("nested");
    }
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("deep.jsonl"), b"deep").unwrap();
    scanner.restart().unwrap();
    let next = scanner.step(100, 100).unwrap();
    assert_eq!(next.depth_limited, 1);
    assert!(next.candidates.is_empty());
    assert!(next.complete);
}
