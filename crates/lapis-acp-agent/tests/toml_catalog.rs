use lapis_acp_agent::config::read_catalog;
fn check(text: &str) -> bool {
    let path = std::env::temp_dir().join(format!(
        "lapis-catalog-{}-{}.toml",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::write(&path, text).unwrap();
    let result = read_catalog(&path).is_ok();
    std::fs::remove_file(path).unwrap();
    result
}
static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[test]
fn authored_catalog_is_strict_and_secrets_require_references() {
    let valid = "[[runners]]\nid='local'\nlabel='Local'\ncommand='./agent'\n";
    assert!(check(valid));
    assert!(!check(&(valid.to_owned() + "mdoe='ask'\n")));
    assert!(!check(&(valid.to_owned() + "env={API_TOKEN='literal'}\n")));
    assert!(!check(
        &(valid.to_owned() + "mcp_transport='stdio'\nmcpTransport='stdio'\n")
    ));
    assert!(!check(
        "typo=1\n[[runners]]\nid='a'\nlabel='A'\ncommand='a'"
    ));
}
