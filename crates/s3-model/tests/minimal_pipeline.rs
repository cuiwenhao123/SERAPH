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
        &[
            seraph_types::ApiId::from("api::demo::io::from_reader"),
            seraph_types::ApiId::from("api::demo::io::ffi_probe")
        ]
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

#[test]
fn slm_scores_stateful_types_and_emits_forbidden_transitions() {
    let knowledge: Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let parser_model = models
        .slm
        .iter()
        .find(|model| model.path == "demo::query::Parser")
        .unwrap();

    assert_eq!(parser_model.model_kind, seraph_types::SlmModelKind::Full);
    assert!(parser_model.states.contains(&"Closed".to_string()));
    assert_eq!(parser_model.forbidden_transitions.len(), 1);
    assert_eq!(
        parser_model.forbidden_transitions[0].via_api_id,
        seraph_types::ApiId::from("api::demo::query::Parser::start")
    );
}

#[test]
fn contract_builder_extracts_side_effects_and_generic_constraints() {
    let knowledge: Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let from_reader = models
        .api_contracts
        .iter()
        .find(|contract| contract.path == "demo::io::from_reader")
        .unwrap();

    assert_eq!(
        from_reader.error_conditions,
        vec!["Returns an error when input is malformed."]
    );
    assert_eq!(from_reader.side_effects, vec!["consumes input bytes"]);
    assert_eq!(
        from_reader
            .generic_constraints
            .as_ref()
            .unwrap()
            .params[0]
            .full_bound_chain,
        vec!["demo::io::Reader", "core::fmt::Debug"]
    );
    assert_eq!(
        from_reader
            .generic_constraints
            .as_ref()
            .unwrap()
            .params[0]
            .associated_type_constraints[0]
            .associated_type_name,
        "Item"
    );
}

#[test]
fn risk_builder_maps_phase1_facts_to_api_and_rust_feature_risks() {
    let knowledge: Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let from_reader_risk = models
        .risk_surface_map
        .api_risks
        .iter()
        .find(|risk| risk.api_id == seraph_types::ApiId::from("api::demo::io::from_reader"))
        .unwrap();

    assert_eq!(from_reader_risk.risk_level, seraph_types::RiskLevel::High);
    assert!(models
        .risk_surface_map
        .rust_feature_risks
        .iter()
        .any(|risk| risk.feature == "extern_abi"));
    assert_eq!(
        models.risk_surface_map.type_synthesis_overview.strategy_distribution["C"],
        1
    );
}

#[test]
fn models_output_matches_golden_fixture() {
    let knowledge: Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();
    let expected: Models =
        serde_json::from_str(include_str!("fixtures/minimal_models.json")).unwrap();

    let actual = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    assert_eq!(actual, expected);
}
