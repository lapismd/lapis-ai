use lapis_ai_capture::SourceProfile;

#[test]
fn product_inventory_preserves_existing_source_versions() {
    for (profile, encoded, version) in [
        (
            SourceProfile::CodexCliJsonlV1,
            "codex-cli-jsonl-v1",
            "0.156.1",
        ),
        (
            SourceProfile::CodexDesktopJsonlV1,
            "codex-desktop-jsonl-v1",
            "0.159.2",
        ),
        (
            SourceProfile::CursorDesktopTranscriptV1,
            "cursor-desktop-transcript-v1",
            "3.22.7",
        ),
        (
            SourceProfile::CursorCliTranscriptV1,
            "cursor-cli-transcript-v1",
            "2026.09.26-dd393fe",
        ),
        (
            SourceProfile::OpencodeReadOnlyApiV1,
            "opencode-read-only-api-v1",
            "1.18.30",
        ),
        (
            SourceProfile::OpencodeExportJsonV1,
            "opencode-export-json-v1",
            "1.18.30",
        ),
    ] {
        assert_eq!(profile.product_version(), version);
        let text = serde_json::to_string(&profile).unwrap();
        assert_eq!(text, format!("\"{encoded}\""));
        assert_eq!(
            serde_json::from_str::<SourceProfile>(&text).unwrap(),
            profile
        );
    }
    assert!(serde_json::from_str::<SourceProfile>("\"unknown-adapter\"").is_err());
}
