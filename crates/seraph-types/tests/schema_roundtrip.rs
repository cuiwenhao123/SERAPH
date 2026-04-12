use seraph_types::{
    ApiCoverageStatus, ApiId, CodeRef, CoverageState, ExampleId, FailedAttempt, Knowledge,
    Level0Summary, ModuleId, ModuleInfo, ModuleTypeRef, NextPriorityItem, RawRiskSurface,
    ScenarioArtifact, ScenarioType,
};
use std::collections::BTreeMap;

#[test]
fn api_status_uses_snake_case_json() {
    let json = serde_json::to_string(&ApiCoverageStatus::Validated).unwrap();
    assert_eq!(json, "\"validated\"");
}

#[test]
fn scenario_roundtrip_preserves_selected_capabilities() {
    let artifact = ScenarioArtifact {
        scenario_id: "scn_001".into(),
        round: 1,
        name: "parse and inspect".into(),
        description: "parse input and inspect fields".into(),
        scenario_type: ScenarioType::Functional,
        selected_capability_ids: vec!["cap_parse".into(), "cap_query".into()],
        fuzz_variation_points: vec!["input".into()],
        target_api_name_hints: vec!["from_reader".into()],
        semantic_constraints: vec!["must be valid usage".into()],
    };

    let json = serde_json::to_string(&artifact).unwrap();
    let decoded: ScenarioArtifact = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.selected_capability_ids.len(), 2);
    assert_eq!(decoded.selected_capability_ids[0].as_str(), "cap_parse");
}

#[test]
fn knowledge_roundtrip_preserves_ids_and_examples() {
    let knowledge = Knowledge {
        crate_name: "serde_json".into(),
        crate_import_name: "serde_json".into(),
        crate_doc: "JSON serialization file format".into(),
        public_api_count: 1,
        default_features: vec!["std".into()],
        level_0_summary: Level0Summary {
            one_line: "JSON parser".into(),
            key_types: vec!["Value".into()],
        },
        level_1_modules: vec![ModuleInfo {
            module_id: ModuleId::from("mod_001"),
            path: "serde_json".into(),
            doc_summary: "top-level entry".into(),
            types: vec![ModuleTypeRef {
                type_id: "type_001".into(),
                name: "Value".into(),
            }],
            api_ids: vec![ApiId::from("fn_001")],
            api_names: vec!["from_str".into()],
        }],
        level_2_types: vec![],
        level_3_apis: vec![],
        trait_registry: vec![],
        examples_index: vec![seraph_types::ExampleInfo {
            example_id: ExampleId::from("ex_001"),
            source_api_id: ApiId::from("fn_001"),
            involved_api_ids: vec![ApiId::from("fn_001")],
            code_ref: CodeRef {
                file: "src/lib.rs".into(),
                start_line: 1,
                end_line: 3,
            },
        }],
        raw_risk_surface: RawRiskSurface {
            unsafe_functions: vec![],
            ffi_boundaries: vec![],
            panic_points: vec![],
            repr_packed_types: vec![],
            panic_in_drop_types: vec![],
        },
    };

    let json = serde_json::to_string(&knowledge).unwrap();
    let decoded: Knowledge = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.level_1_modules[0].module_id.as_str(), "mod_001");
    assert_eq!(decoded.examples_index[0].example_id.as_str(), "ex_001");
}

#[test]
fn coverage_state_roundtrip_keeps_api_id_map_keys() {
    let mut api_status = BTreeMap::new();
    api_status.insert(ApiId::from("fn_003"), ApiCoverageStatus::Validated);

    let mut failed_attempts = BTreeMap::new();
    failed_attempts.insert(
        ApiId::from("fn_015"),
        FailedAttempt {
            compile_fails: 0,
            misuse_fails: 2,
            last_reason: "documented panic".into(),
        },
    );

    let coverage = CoverageState {
        total_api_ids: vec![ApiId::from("fn_003"), ApiId::from("fn_015")],
        api_status,
        failed_attempts,
        covered_api_ids: vec![ApiId::from("fn_003")],
        exhausted_api_ids: vec![],
        uncovered_api_ids: vec![ApiId::from("fn_015")],
        harnesses: BTreeMap::new(),
        found_bugs: vec![],
        needs_review: vec![],
        next_priority: vec![NextPriorityItem {
            api_id: ApiId::from("fn_015"),
            reason: "not validated".into(),
        }],
        coverage_rate: 0.5,
    };

    let json = serde_json::to_string(&coverage).unwrap();
    let decoded: CoverageState = serde_json::from_str(&json).unwrap();

    assert_eq!(
        decoded.api_status.get(&ApiId::from("fn_003")),
        Some(&ApiCoverageStatus::Validated)
    );
    assert_eq!(decoded.next_priority[0].api_id.as_str(), "fn_015");
}
