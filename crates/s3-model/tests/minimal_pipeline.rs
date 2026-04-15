use seraph_types::{CrateMeta, DocSections, Knowledge, Models, RiskFacts};

fn minimal_knowledge() -> Knowledge {
    Knowledge {
        crate_meta: CrateMeta {
            package_name: "demo".into(),
            lib_target_name: "demo".into(),
            crate_import_name: "demo".into(),
            version: "0.1.0".into(),
            edition: "2021".into(),
            rust_version: None,
            repository: None,
            manifest_path: "/tmp/demo/Cargo.toml".into(),
            lib_rs_path: "/tmp/demo/src/lib.rs".into(),
            default_features: vec![],
            cargo_description: Some("demo crate".into()),
            root_docs: String::new(),
            root_doc_sections: DocSections::default(),
        },
        modules: vec![],
        types: vec![],
        apis: vec![],
        symbols: vec![],
        trait_registry: vec![],
        trait_impl_registry: vec![],
        examples: vec![],
        risk_facts: RiskFacts::default(),
    }
}

#[test]
fn build_models_from_knowledge_returns_phase2_sections() {
    let knowledge = minimal_knowledge();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    assert!(models.fcg.capabilities.is_empty());
    assert!(models.slm.is_empty());
    assert!(models.api_contracts.is_empty());
    assert!(models.risk_surface_map.api_risks.is_empty());
}

#[test]
fn models_json_roundtrip_through_s3_model_io() {
    let knowledge = minimal_knowledge();
    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let path = std::env::temp_dir().join("seraph-s3-model-roundtrip.json");

    s3_model::write_models_json(&path, &models).unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    let decoded: Models = serde_json::from_str(&raw).unwrap();

    assert_eq!(decoded, models);

    let _ = std::fs::remove_file(path);
}

#[test]
fn fcg_groups_apis_by_public_anchor_module_and_builds_stage1_summary() {
    let knowledge: Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    assert_eq!(models.fcg.capabilities.len(), 2);
    assert_eq!(
        models
            .fcg
            .capability_api_index
            .get(&seraph_types::CapId::from("cap::demo::io"))
            .unwrap()
            .as_slice(),
        &[seraph_types::ApiId::from("api::demo::io::from_reader")]
    );
    assert_eq!(
        models.fcg.capabilities[0].connects_to,
        vec![seraph_types::CapId::from("cap::demo::query")]
    );
    assert_eq!(
        models.fcg.capability_chains,
        vec![vec![
            seraph_types::CapId::from("cap::demo::io"),
            seraph_types::CapId::from("cap::demo::query")
        ]]
    );
    assert!(models.fcg.stage1_summary.contains("本库提供 2 项核心能力"));
}
