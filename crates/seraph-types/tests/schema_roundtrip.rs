use seraph_types::{
    ApiCoverageStatus, ApiId, ApiInfo, ApiKind,
    BorrowedReturnFact,
    CodeRef, CoverageState, CrateMeta,
    DocSections, EnumVariantInfo, ExampleAnchor, ExampleId, ExampleInfo,
    ExplicitPanicSiteFact, ExternAbiApiFact, FailedAttempt,
    Knowledge,
    ModuleId, ModuleInfo, NextPriorityItem, ReprKind, ReturnShape, ReturnShapeKind, RiskFacts,
    RiskOwner, ScenarioArtifact, ScenarioType,
    SymbolId, SymbolInfo, SymbolKind, TraitAssociatedConstBinding, TraitAssociatedConstDef,
    TraitAssociatedTypeBinding, TraitAssociatedTypeDef, TraitExposureKind, TraitId, TraitImplId,
    TraitImplInfo, TraitInfo, TraitOrigin,
    TypeId, TypeInfo, TypeKind,
    TypeLayoutFact, VariantKind,
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
fn knowledge_roundtrip_preserves_trait_graph_and_examples() {
    let knowledge = Knowledge {
        crate_meta: CrateMeta {
            package_name: "serde_json".into(),
            lib_target_name: "serde_json".into(),
            crate_import_name: "serde_json".into(),
            version: "1.0.0".into(),
            edition: "2021".into(),
            rust_version: Some("1.70.0".into()),
            repository: Some("https://github.com/serde-rs/json".into()),
            manifest_path: "/tmp/serde_json/Cargo.toml".into(),
            lib_rs_path: "/tmp/serde_json/src/lib.rs".into(),
            default_features: vec!["std".into()],
            cargo_description: Some("JSON serialization file format".into()),
            root_docs: "Top-level crate docs".into(),
            root_doc_sections: DocSections {
                summary: "Top-level crate docs".into(),
                panics: String::new(),
                errors: String::new(),
                safety: String::new(),
                examples: "```rust\nserde_json::json!({\"ok\": true});\n```".into(),
            },
        },
        modules: vec![ModuleInfo {
            module_id: ModuleId::from("mod::serde_json"),
            name: "serde_json".into(),
            canonical_path: "serde_json".into(),
            public_paths: vec!["serde_json".into()],
            parent_module_id: None,
            code_ref: CodeRef {
                file: "src/lib.rs".into(),
                start_line: 1,
                end_line: 10,
            },
            docs: "Top-level entry".into(),
            doc_sections: DocSections {
                summary: "Top-level entry".into(),
                panics: String::new(),
                errors: String::new(),
                safety: String::new(),
                examples: String::new(),
            },
        }],
        types: vec![TypeInfo {
            type_id: TypeId::from("type::serde_json::Value"),
            name: "Value".into(),
            canonical_path: "serde_json::Value".into(),
            public_paths: vec!["serde_json::Value".into()],
            public_anchor_module_id: ModuleId::from("mod::serde_json"),
            code_ref: CodeRef {
                file: "src/value.rs".into(),
                start_line: 10,
                end_line: 60,
            },
            docs: "JSON value enum".into(),
            doc_sections: DocSections {
                summary: "JSON value enum".into(),
                panics: String::new(),
                errors: String::new(),
                safety: String::new(),
                examples: "```rust\nlet value = serde_json::Value::Null;\n```".into(),
            },
            kind: TypeKind::Enum,
            generic_params: vec![],
            where_clauses: vec![],
            is_non_exhaustive: false,
            fields: vec![],
            variants: vec![EnumVariantInfo {
                name: "Null".into(),
                kind: VariantKind::Unit,
                is_non_exhaustive: false,
                discriminant_text: None,
                fields: vec![],
                source: Some(CodeRef {
                    file: "src/value.rs".into(),
                    start_line: 12,
                    end_line: 12,
                }),
            }],
            has_hidden_fields: false,
            has_hidden_variants: false,
        }],
        apis: vec![ApiInfo {
            api_id: ApiId::from("api::serde_json::from_str"),
            name: "from_str".into(),
            canonical_path: "serde_json::from_str".into(),
            public_paths: vec!["serde_json::from_str".into()],
            public_anchor_module_id: ModuleId::from("mod::serde_json"),
            owner_type_id: None,
            owner_trait_id: None,
            code_ref: CodeRef {
                file: "src/de.rs".into(),
                start_line: 100,
                end_line: 120,
            },
            docs: "Parse JSON from a string.".into(),
            doc_sections: DocSections {
                summary: "Parse JSON from a string.".into(),
                panics: String::new(),
                errors: "Returns an error if the string is not valid JSON.".into(),
                safety: String::new(),
                examples: "```rust\nlet value = serde_json::from_str::<serde_json::Value>(\"null\")?;\n```".into(),
            },
            api_kind: ApiKind::FreeFunction,
            signature_text: "pub fn from_str<T>(s: &str) -> Result<T>".into(),
            receiver: None,
            generic_params: vec!["T".into()],
            where_clauses: vec!["T: DeserializeOwned".into()],
            arg_types: vec!["&str".into()],
            return_type: Some("Result<T>".into()),
            return_shape: Some(ReturnShape {
                kind: ReturnShapeKind::Result,
                inner_types: vec!["T".into()],
            }),
            is_unsafe: false,
            is_async: false,
            is_const: false,
            has_body: true,
            contains_unsafe_block: false,
        }],
        symbols: vec![
            SymbolInfo {
                symbol_id: SymbolId::from("symbol::serde_json::json"),
                name: "json".into(),
                canonical_path: "serde_json::json".into(),
                public_paths: vec!["serde_json::json".into()],
                public_anchor_module_id: ModuleId::from("mod::serde_json"),
                owner_type_id: None,
                code_ref: CodeRef {
                    file: "src/macros.rs".into(),
                    start_line: 10,
                    end_line: 30,
                },
                docs: "Construct a JSON value.".into(),
                doc_sections: DocSections {
                    summary: "Construct a JSON value.".into(),
                    panics: String::new(),
                    errors: String::new(),
                    safety: String::new(),
                    examples: "```rust\nlet value = serde_json::json!({\"ok\": true});\n```".into(),
                },
                symbol_kind: SymbolKind::Macro,
                signature_text: Some("macro_rules! json { ($($json:tt)+) => { ... }; }".into()),
                type_text: None,
                value_text: None,
            },
            SymbolInfo {
                symbol_id: SymbolId::from("symbol::serde_json::Value::NULL"),
                name: "NULL".into(),
                canonical_path: "serde_json::Value::NULL".into(),
                public_paths: vec!["serde_json::Value::NULL".into()],
                public_anchor_module_id: ModuleId::from("mod::serde_json"),
                owner_type_id: Some(TypeId::from("type::serde_json::Value")),
                code_ref: CodeRef {
                    file: "src/value.rs".into(),
                    start_line: 32,
                    end_line: 32,
                },
                docs: "Associated JSON null marker.".into(),
                doc_sections: DocSections {
                    summary: "Associated JSON null marker.".into(),
                    panics: String::new(),
                    errors: String::new(),
                    safety: String::new(),
                    examples: String::new(),
                },
                symbol_kind: SymbolKind::AssociatedConstant,
                signature_text: None,
                type_text: Some("Self".into()),
                value_text: None,
            },
        ],
        trait_registry: vec![
            TraitInfo {
                trait_id: TraitId::from("trait::serde::de::DeserializeOwned"),
                name: "DeserializeOwned".into(),
                canonical_path: "serde::de::DeserializeOwned".into(),
                public_paths: vec![],
                public_anchor_module_id: None,
                code_ref: None,
                docs: String::new(),
                doc_sections: DocSections {
                    summary: String::new(),
                    panics: String::new(),
                    errors: String::new(),
                    safety: String::new(),
                    examples: String::new(),
                },
                origin: TraitOrigin::External,
                exposure_kinds: vec![TraitExposureKind::Bound],
                is_unsafe: false,
                direct_supertrait_ids: vec![TraitId::from("trait::serde::de::Deserialize")],
                required_methods: vec![],
                provided_methods: vec![],
                associated_type_defs: vec![TraitAssociatedTypeDef {
                    name: "Owned".into(),
                    generic_params: vec![],
                    where_clauses: vec![],
                    bounds: vec!["Clone".into()],
                    default_type: None,
                    source: None,
                }],
                associated_const_defs: vec![TraitAssociatedConstDef {
                    name: "BUFFER_HINT".into(),
                    type_text: "usize".into(),
                    default_value_text: Some("0".into()),
                    source: None,
                }],
                used_by_api_ids: vec![ApiId::from("api::serde_json::from_str")],
                used_by_trait_ids: vec![TraitId::from("trait::serde_json::TraitSurface")],
                used_by_type_ids: vec![],
            },
            TraitInfo {
                trait_id: TraitId::from("trait::serde::de::Deserialize"),
                name: "Deserialize".into(),
                canonical_path: "serde::de::Deserialize".into(),
                public_paths: vec![],
                public_anchor_module_id: None,
                code_ref: None,
                docs: String::new(),
                doc_sections: DocSections {
                    summary: String::new(),
                    panics: String::new(),
                    errors: String::new(),
                    safety: String::new(),
                    examples: String::new(),
                },
                origin: TraitOrigin::External,
                exposure_kinds: vec![TraitExposureKind::SupertraitDependency],
                is_unsafe: false,
                direct_supertrait_ids: vec![],
                required_methods: vec!["deserialize".into()],
                provided_methods: vec![],
                associated_type_defs: vec![],
                associated_const_defs: vec![],
                used_by_api_ids: vec![],
                used_by_trait_ids: vec![],
                used_by_type_ids: vec![],
            },
        ],
        trait_impl_registry: vec![TraitImplInfo {
            trait_impl_id: TraitImplId::from(
                "trait_impl::core::default::Default::for::serde_json::Value",
            ),
            target_type_id: TypeId::from("type::serde_json::Value"),
            trait_ref_text: "core::default::Default".into(),
            for_type_text: "Value".into(),
            trait_id: TraitId::from("trait::core::default::Default"),
            trait_name: "Default".into(),
            trait_canonical_path: "core::default::Default".into(),
            trait_origin: TraitOrigin::External,
            source: CodeRef {
                file: "src/value.rs".into(),
                start_line: 70,
                end_line: 74,
            },
            associated_type_bindings: vec![TraitAssociatedTypeBinding {
                name: "Output".into(),
                generic_params: vec![],
                where_clauses: vec![],
                bounds: vec![],
                assigned_type: Some("Value".into()),
                source: Some(CodeRef {
                    file: "src/value.rs".into(),
                    start_line: 71,
                    end_line: 71,
                }),
            }],
            associated_const_bindings: vec![TraitAssociatedConstBinding {
                name: "BUFFER_HINT".into(),
                value_text: Some("0".into()),
                source: Some(CodeRef {
                    file: "src/value.rs".into(),
                    start_line: 72,
                    end_line: 72,
                }),
            }],
            where_clauses: vec![],
            cfg_attrs: vec!["#[cfg(feature = \"std\")]".into()],
            is_unsafe: false,
        }],
        examples: vec![ExampleInfo {
            example_id: ExampleId::from("ex::src/de.rs::100::1"),
            anchor: ExampleAnchor::Api(ApiId::from("api::serde_json::from_str")),
            involved_api_ids: vec![ApiId::from("api::serde_json::from_str")],
            code_ref: CodeRef {
                file: "src/de.rs".into(),
                start_line: 100,
                end_line: 110,
            },
            snippet: Some("let value = serde_json::from_str::<Value>(raw)?;".into()),
        }],
        risk_facts: RiskFacts {
            extern_abi_apis: vec![ExternAbiApiFact {
                api_id: ApiId::from("api::serde_json::ffi_bridge"),
                abi: "C".into(),
                source: Some(CodeRef {
                    file: "src/ffi.rs".into(),
                    start_line: 1,
                    end_line: 8,
                }),
            }],
            repr_types: vec![TypeLayoutFact {
                type_id: TypeId::from("type::serde_json::RawValue"),
                repr_kinds: vec![ReprKind::Transparent],
                source: Some(CodeRef {
                    file: "src/raw.rs".into(),
                    start_line: 5,
                    end_line: 5,
                }),
            }],
            drop_impl_types: vec![TypeId::from("type::serde_json::Value")],
            explicit_panic_sites: vec![ExplicitPanicSiteFact {
                owner: RiskOwner::Api(ApiId::from("api::serde_json::from_str")),
                panic_kind: "assert!".into(),
                source: CodeRef {
                    file: "src/de.rs".into(),
                    start_line: 109,
                    end_line: 109,
                },
            }],
            borrowed_return_apis: vec![BorrowedReturnFact {
                api_id: ApiId::from("api::serde_json::Value::as_str"),
                return_type_text: "&str".into(),
                from_self: true,
                from_arg_positions: vec![],
                lifetime_names: vec![],
                source: Some(CodeRef {
                    file: "src/value.rs".into(),
                    start_line: 88,
                    end_line: 90,
                }),
            }],
        },
    };

    let json = serde_json::to_string(&knowledge).unwrap();
    let decoded: Knowledge = serde_json::from_str(&json).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.crate_meta.package_name, "serde_json");
    assert_eq!(decoded.modules[0].module_id.as_str(), "mod::serde_json");
    assert_eq!(
        decoded.symbols[0].symbol_id.as_str(),
        "symbol::serde_json::json"
    );
    assert_eq!(
        decoded.trait_registry[0].direct_supertrait_ids[0].as_str(),
        "trait::serde::de::Deserialize"
    );
    assert_eq!(
        decoded.trait_registry[0].associated_type_defs[0].name,
        "Owned"
    );
    assert_eq!(
        decoded.trait_registry[0].associated_const_defs[0].name,
        "BUFFER_HINT"
    );
    assert_eq!(
        decoded.trait_registry[0].used_by_trait_ids[0].as_str(),
        "trait::serde_json::TraitSurface"
    );
    assert_eq!(
        decoded.trait_impl_registry[0].trait_impl_id.as_str(),
        "trait_impl::core::default::Default::for::serde_json::Value"
    );
    assert_eq!(
        decoded.trait_impl_registry[0].associated_type_bindings[0]
            .assigned_type
            .as_deref(),
        Some("Value")
    );
    assert_eq!(
        decoded.trait_impl_registry[0].associated_const_bindings[0]
            .value_text
            .as_deref(),
        Some("0")
    );
    assert_eq!(
        decoded.trait_impl_registry[0].cfg_attrs[0].as_str(),
        "#[cfg(feature = \"std\")]"
    );
    assert_eq!(
        decoded.crate_meta.root_doc_sections.summary,
        "Top-level crate docs"
    );
    assert_eq!(decoded.modules[0].doc_sections.summary, "Top-level entry");
    assert_eq!(decoded.types[0].variants[0].name, "Null");
    assert_eq!(
        decoded.apis[0].return_shape.as_ref().unwrap().kind,
        ReturnShapeKind::Result
    );
    assert_eq!(
        decoded.symbols[0].doc_sections.summary,
        "Construct a JSON value."
    );
    assert_eq!(
        decoded.symbols[1].owner_type_id.as_ref().unwrap().as_str(),
        "type::serde_json::Value"
    );
    assert_eq!(
        serde_json::to_string(&decoded.symbols[1].symbol_kind).unwrap(),
        "\"associated_constant\""
    );
    assert_eq!(
        decoded.risk_facts.extern_abi_apis[0]
            .source
            .as_ref()
            .unwrap()
            .file,
        "src/ffi.rs"
    );
    assert!(value["risk_facts"]["ffi_apis"].is_null());
    assert!(
        value["trait_impl_registry"][0]["surface_bucket"].is_null(),
        "serialized trait impl JSON should not retain removed compatibility bucket"
    );
    assert_eq!(
        decoded.risk_facts.explicit_panic_sites[0].panic_kind,
        "assert!"
    );
    assert!(decoded.risk_facts.borrowed_return_apis[0].from_self);
    match &decoded.examples[0].anchor {
        ExampleAnchor::Api(api_id) => assert_eq!(api_id.as_str(), "api::serde_json::from_str"),
        other => panic!("unexpected example anchor: {other:?}"),
    }
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
