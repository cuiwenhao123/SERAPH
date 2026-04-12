use seraph_types::Knowledge;

#[test]
fn build_minimal_knowledge_normalizes_import_name() {
    let knowledge = s3_extract::build_minimal_knowledge("serde-json-wrapper");

    assert_eq!(knowledge.crate_name, "serde-json-wrapper");
    assert_eq!(knowledge.crate_import_name, "serde_json_wrapper");
    assert_eq!(knowledge.public_api_count, 0);
}

#[test]
fn cli_writes_valid_knowledge_json() {
    let output = std::env::temp_dir().join(format!(
        "seraph_s3_extract_test_{}_knowledge.json",
        std::process::id()
    ));

    let bin = std::env::var("CARGO_BIN_EXE_s3-extract").unwrap();
    let status = std::process::Command::new(bin)
        .args(["--crate", "demo-crate", "--output", output.to_str().unwrap()])
        .status()
        .unwrap();

    assert!(status.success());

    let raw = std::fs::read_to_string(&output).unwrap();
    let knowledge: Knowledge = serde_json::from_str(&raw).unwrap();

    assert_eq!(knowledge.crate_name, "demo-crate");
    assert_eq!(knowledge.crate_import_name, "demo_crate");

    let _ = std::fs::remove_file(output);
}
