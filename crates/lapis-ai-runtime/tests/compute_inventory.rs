use lapis_ai_runtime::{ComputeFailure, ComputeInventory};
use serde_json::json;
use std::fs;

#[test]
fn inventory_is_inert_and_recovers_existing_models_without_alias_double_counting() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("off");
    assert!(ComputeInventory::read(&missing).unwrap().models.is_empty());
    assert!(!missing.exists());
    let root = temp.path().canonicalize().unwrap();
    let primary = root.join("weight.gguf");
    let alias = root.join("alias.gguf");
    fs::write(&primary, b"GGUFfixture").unwrap();
    fs::hard_link(&primary, &alias).unwrap();
    fs::write(
        root.join("installed-models.json"),
        json!({"example/model:Q4": {"primary":primary,"files":[primary,alias]}}).to_string(),
    )
    .unwrap();
    let model = &ComputeInventory::read(&root).unwrap().models[0];
    assert!(model.installed);
    assert_eq!(model.bytes, Some(11));
    fs::remove_file(&primary).unwrap();
    let model = &ComputeInventory::read(&root).unwrap().models[0];
    assert!(!model.installed);
    assert_eq!(model.bytes, None);
}

#[test]
fn invalid_and_escaping_indexes_are_failures_instead_of_empty_libraries() {
    let root = tempfile::tempdir().unwrap();
    let index = root.path().join("installed-models.json");
    fs::write(&index, b"invalid").unwrap();
    assert_eq!(
        ComputeInventory::read(root.path()).unwrap_err().0,
        "compute_inventory_invalid"
    );
    fs::write(
        &index,
        json!({"model":{"primary":"/outside/weights.gguf","files":[]}}).to_string(),
    )
    .unwrap();
    assert!(ComputeInventory::read(root.path()).is_err());
    fs::remove_file(index).unwrap();
    let library = root.path().canonicalize().unwrap().join("runtime.dylib");
    fs::write(&library, b"runtime").unwrap();
    fs::write(
        root.path().join("runtime-ready.json"),
        json!([library]).to_string(),
    )
    .unwrap();
    assert!(
        ComputeInventory::read(root.path())
            .unwrap()
            .runtime_prepared
    );
    fs::remove_file(library).unwrap();
    assert!(
        !ComputeInventory::read(root.path())
            .unwrap()
            .runtime_prepared
    );
}

#[test]
fn failure_details_are_allowlisted_and_never_echo_a_dependency_diagnostic() {
    let unknown = ComputeFailure::from_code("secret-token /private/path connection failed");
    assert_eq!(unknown.code, "compute_operation_failed");
    assert!(!unknown.message.contains("secret-token"));
    assert!(
        ComputeFailure::from_code("disk_full")
            .message
            .contains("disk space")
    );
}

#[test]
fn legacy_import_names_come_from_bounded_gguf_metadata_instead_of_content_ids() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let primary = root.join("imported-hash.gguf");
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&0u64.to_le_bytes());
    bytes.extend_from_slice(&1u64.to_le_bytes());
    for text in ["general.name", "Qwen3 0.6B"] {
        bytes.extend_from_slice(&(text.len() as u64).to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
        if text == "general.name" {
            bytes.extend_from_slice(&8u32.to_le_bytes());
        }
    }
    fs::write(&primary, &bytes).unwrap();
    fs::write(
        root.join("installed-models.json"),
        json!({"imported-hash":{"primary":primary,"files":[primary]}}).to_string(),
    )
    .unwrap();
    assert_eq!(
        ComputeInventory::read(&root).unwrap().models[0].name,
        "Qwen3 0.6B"
    );
    fs::write(&primary, b"GGUFbad").unwrap();
    assert_eq!(
        ComputeInventory::read(&root).unwrap().models[0].name,
        "Imported GGUF model"
    );
    let mut oversized = b"GGUF".to_vec();
    oversized.extend_from_slice(&3u32.to_le_bytes());
    oversized.extend_from_slice(&0u64.to_le_bytes());
    oversized.extend_from_slice(&1u64.to_le_bytes());
    oversized.extend_from_slice(&u64::MAX.to_le_bytes());
    fs::write(&primary, oversized).unwrap();
    assert_eq!(
        ComputeInventory::read(&root).unwrap().models[0].name,
        "Imported GGUF model"
    );
}
