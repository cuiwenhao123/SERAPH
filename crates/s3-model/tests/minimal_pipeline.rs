use seraph_types::{
    CodeRef, CrateMeta, DocSections, ExplicitPanicSiteFact, Knowledge, Models, ReprKind, RiskFacts,
    RiskOwner, TypeLayoutFact,
};

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

fn fixture_knowledge() -> Knowledge {
    serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap()
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
fn fcg_emits_type_centered_capabilities_and_structured_stage1_summary() {
    let knowledge: Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    let io_entry = models
        .fcg
        .capabilities
        .iter()
        .find(|cap| {
            cap.cap_id
                == seraph_types::CapId::from("cap::demo::io::module_entry::construction")
        })
        .unwrap();
    assert_eq!(
        models
            .fcg
            .capability_api_index
            .get(&seraph_types::CapId::from(
                "cap::demo::io::module_entry::construction",
            ))
            .unwrap()
            .as_slice(),
        &[seraph_types::ApiId::from("api::demo::io::from_reader")]
    );
    assert_eq!(
        io_entry.anchor_kind,
        seraph_types::CapabilityAnchorKind::ModuleEntry
    );
    assert_eq!(io_entry.role, seraph_types::CapabilityRole::Construction);

    let document_query = models
        .fcg
        .capabilities
        .iter()
        .find(|cap| {
            cap.cap_id == seraph_types::CapId::from("cap::demo::query::Document::query")
        })
        .unwrap();
    assert_eq!(
        document_query.anchor_kind,
        seraph_types::CapabilityAnchorKind::Type
    );
    assert_eq!(
        document_query.anchor_type_id,
        Some(seraph_types::TypeId::from("type::demo::query::Document"))
    );
    assert_eq!(document_query.role, seraph_types::CapabilityRole::Query);
    assert_eq!(
        io_entry.connects_to,
        vec![seraph_types::CapId::from(
            "cap::demo::query::Document::query"
        )]
    );
    assert!(
        models
            .fcg
            .stage1_summary
            .capability_cards
            .iter()
            .any(|card| {
                card.cap_id
                    == seraph_types::CapId::from(
                        "cap::demo::io::module_entry::construction"
                    )
                    && card.anchor_path == "demo::io"
            })
    );
    assert!(models.fcg.stage1_summary.one_liner.contains("项能力"));
    assert!(
        models
            .fcg
            .capability_chains
            .iter()
            .any(|chain| chain
                == &vec![
                    seraph_types::CapId::from("cap::demo::io::module_entry::construction"),
                    seraph_types::CapId::from("cap::demo::query::Document::query")
                ])
    );
}

#[test]
fn fcg_connects_modules_from_unique_short_type_names_in_return_shapes() {
    let mut knowledge = fixture_knowledge();
    let from_reader = knowledge
        .apis
        .iter_mut()
        .find(|api| api.canonical_path == "demo::io::from_reader")
        .unwrap();
    from_reader.return_type = Some("Result<Document<'_>, Error>".into());
    from_reader.return_shape = Some(seraph_types::ReturnShape {
        kind: seraph_types::ReturnShapeKind::Result,
        inner_types: vec!["Document<'_>".into(), "Error".into()],
    });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let io_cap = models
        .fcg
        .capabilities
        .iter()
        .find(|cap| {
            cap.cap_id
                == seraph_types::CapId::from("cap::demo::io::module_entry::construction")
        })
        .unwrap();

    assert_eq!(
        io_cap.connects_to,
        vec![seraph_types::CapId::from(
            "cap::demo::query::Document::query"
        )]
    );
    assert!(
        models
            .fcg
            .capability_chains
            .iter()
            .any(|chain| chain
                == &vec![
                    seraph_types::CapId::from("cap::demo::io::module_entry::construction"),
                    seraph_types::CapId::from("cap::demo::query::Document::query")
                ])
    );
}

#[test]
fn fcg_does_not_connect_ambiguous_short_type_names() {
    let mut knowledge = fixture_knowledge();
    let query_document = knowledge
        .types
        .iter()
        .find(|ty| ty.canonical_path == "demo::query::Document")
        .unwrap()
        .clone();
    let mut io_document = query_document.clone();
    io_document.type_id = seraph_types::TypeId::from("type::demo::io::Document");
    io_document.name = "Document".into();
    io_document.canonical_path = "demo::io::Document".into();
    io_document.public_paths = vec!["demo::io::Document".into()];
    io_document.public_anchor_module_id = seraph_types::ModuleId::from("mod::demo::io");
    knowledge.types.push(io_document);

    let from_reader = knowledge
        .apis
        .iter_mut()
        .find(|api| api.canonical_path == "demo::io::from_reader")
        .unwrap();
    from_reader.return_type = Some("Result<Document<'_>, Error>".into());
    from_reader.return_shape = Some(seraph_types::ReturnShape {
        kind: seraph_types::ReturnShapeKind::Result,
        inner_types: vec!["Document<'_>".into(), "Error".into()],
    });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let io_cap = models
        .fcg
        .capabilities
        .iter()
        .find(|cap| {
            cap.cap_id
                == seraph_types::CapId::from("cap::demo::io::module_entry::construction")
        })
        .unwrap();

    assert!(io_cap.connects_to.is_empty());
    assert!(
        !models
            .fcg
            .capability_chains
            .iter()
            .any(|chain| chain
                == &vec![
                    seraph_types::CapId::from("cap::demo::io::module_entry::construction"),
                    seraph_types::CapId::from("cap::demo::query::Document::query")
                ])
    );
}

#[test]
fn fcg_avoids_query_to_query_cross_anchor_edges_and_compresses_one_liner() {
    let mut knowledge = fixture_knowledge();

    knowledge.types.push(seraph_types::TypeInfo {
        type_id: seraph_types::TypeId::from("type::demo::status::Status"),
        name: "Status".into(),
        canonical_path: "demo::status::Status".into(),
        public_paths: vec!["demo::status::Status".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::status"),
        code_ref: CodeRef {
            file: "src/status.rs".into(),
            start_line: 1,
            end_line: 8,
        },
        docs: "Simple status value.".into(),
        doc_sections: DocSections::default(),
        kind: seraph_types::TypeKind::Struct,
        generic_params: vec![],
        where_clauses: vec![],
        is_non_exhaustive: false,
        fields: vec![],
        variants: vec![],
        has_hidden_fields: false,
        has_hidden_variants: false,
    });

    knowledge.apis.push(seraph_types::ApiInfo {
        api_id: seraph_types::ApiId::from("api::demo::status::Status::code"),
        name: "code".into(),
        canonical_path: "demo::status::Status::code".into(),
        public_paths: vec!["demo::status::Status::code".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::status"),
        owner_type_id: Some(seraph_types::TypeId::from("type::demo::status::Status")),
        owner_trait_id: None,
        code_ref: CodeRef {
            file: "src/status.rs".into(),
            start_line: 10,
            end_line: 10,
        },
        docs: "Returns the numeric code.".into(),
        doc_sections: DocSections::default(),
        api_kind: seraph_types::ApiKind::InherentMethod,
        signature_text: "pub fn code(&self) -> u32".into(),
        receiver: Some("&self".into()),
        generic_params: vec![],
        where_clauses: vec![],
        arg_types: vec![],
        return_type: Some("u32".into()),
        return_shape: Some(seraph_types::ReturnShape {
            kind: seraph_types::ReturnShapeKind::Primitive,
            inner_types: vec![],
        }),
        is_unsafe: false,
        is_async: false,
        is_const: false,
        has_body: true,
        contains_unsafe_block: false,
    });

    let pointer = knowledge
        .apis
        .iter_mut()
        .find(|api| api.canonical_path == "demo::query::Document::pointer")
        .unwrap();
    pointer.return_type = Some("Option<demo::status::Status>".into());
    pointer.return_shape = Some(seraph_types::ReturnShape {
        kind: seraph_types::ReturnShapeKind::Option,
        inner_types: vec!["demo::status::Status".into()],
    });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let document_query = models
        .fcg
        .capabilities
        .iter()
        .find(|cap| cap.cap_id == seraph_types::CapId::from("cap::demo::query::Document::query"))
        .unwrap();

    assert!(document_query.connects_to.is_empty());
    assert!(models.fcg.stage1_summary.one_liner.len() <= 80);
}

#[test]
fn fcg_uses_into_iterator_trait_impl_surface_for_construction_handoffs() {
    let mut knowledge = fixture_knowledge();

    knowledge.types.push(seraph_types::TypeInfo {
        type_id: seraph_types::TypeId::from("type::demo::query::ParserIntoIter"),
        name: "ParserIntoIter".into(),
        canonical_path: "demo::query::ParserIntoIter".into(),
        public_paths: vec!["demo::query::ParserIntoIter".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::query"),
        code_ref: CodeRef {
            file: "src/query.rs".into(),
            start_line: 40,
            end_line: 44,
        },
        docs: "Iterator returned when a parser is consumed.".into(),
        doc_sections: DocSections::default(),
        kind: seraph_types::TypeKind::Struct,
        generic_params: vec![],
        where_clauses: vec![],
        is_non_exhaustive: false,
        fields: vec![],
        variants: vec![],
        has_hidden_fields: false,
        has_hidden_variants: false,
    });

    knowledge.apis.push(seraph_types::ApiInfo {
        api_id: seraph_types::ApiId::from("api::demo::query::ParserIntoIter::peek"),
        name: "peek".into(),
        canonical_path: "demo::query::ParserIntoIter::peek".into(),
        public_paths: vec!["demo::query::ParserIntoIter::peek".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::query"),
        owner_type_id: Some(seraph_types::TypeId::from("type::demo::query::ParserIntoIter")),
        owner_trait_id: None,
        code_ref: CodeRef {
            file: "src/query.rs".into(),
            start_line: 46,
            end_line: 46,
        },
        docs: "Peeks the next parser item.".into(),
        doc_sections: DocSections::default(),
        api_kind: seraph_types::ApiKind::InherentMethod,
        signature_text: "pub fn peek(&self) -> Option<&str>".into(),
        receiver: Some("&self".into()),
        generic_params: vec![],
        where_clauses: vec![],
        arg_types: vec![],
        return_type: Some("Option<&str>".into()),
        return_shape: Some(seraph_types::ReturnShape {
            kind: seraph_types::ReturnShapeKind::Option,
            inner_types: vec!["&str".into()],
        }),
        is_unsafe: false,
        is_async: false,
        is_const: false,
        has_body: true,
        contains_unsafe_block: false,
    });

    knowledge
        .trait_impl_registry
        .push(seraph_types::TraitImplInfo {
            trait_impl_id: "trait_impl::core::iter::traits::collect::IntoIterator::for::demo::query::Parser".into(),
            target_type_id: seraph_types::TypeId::from("type::demo::query::Parser"),
            trait_ref_text: "core::iter::traits::collect::IntoIterator".into(),
            for_type_text: "demo::query::Parser".into(),
            trait_id: seraph_types::TraitId::from("trait::core::iter::traits::collect::IntoIterator"),
            trait_name: "IntoIterator".into(),
            trait_canonical_path: "core::iter::traits::collect::IntoIterator".into(),
            trait_origin: seraph_types::TraitOrigin::External,
            source: CodeRef {
                file: "src/query.rs".into(),
                start_line: 48,
                end_line: 52,
            },
            associated_type_bindings: vec![
                seraph_types::TraitAssociatedTypeBinding {
                    name: "IntoIter".into(),
                    generic_params: vec![],
                    where_clauses: vec![],
                    bounds: vec![],
                    assigned_type: Some("demo::query::ParserIntoIter".into()),
                    source: None,
                },
            ],
            associated_const_bindings: vec![],
            where_clauses: vec![],
            cfg_attrs: vec![],
            is_unsafe: false,
        });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let parser_construction = models
        .fcg
        .capabilities
        .iter()
        .find(|cap| cap.cap_id == seraph_types::CapId::from("cap::demo::query::Parser::construction"))
        .unwrap();

    assert!(
        parser_construction
            .connects_to
            .contains(&seraph_types::CapId::from("cap::demo::query::ParserIntoIter::query"))
    );
    assert!(
        models
            .fcg
            .capability_chains
            .iter()
            .any(|chain| chain == &vec![
                seraph_types::CapId::from("cap::demo::query::Parser::construction"),
                seraph_types::CapId::from("cap::demo::query::ParserIntoIter::query")
            ])
    );
}

#[test]
fn fcg_does_not_promote_helper_free_functions_to_module_entry_construction() {
    let mut knowledge = fixture_knowledge();

    knowledge.types.push(seraph_types::TypeInfo {
        type_id: seraph_types::TypeId::from("type::demo::io::Error"),
        name: "Error".into(),
        canonical_path: "demo::io::Error".into(),
        public_paths: vec!["demo::io::Error".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::io"),
        code_ref: CodeRef {
            file: "src/io.rs".into(),
            start_line: 50,
            end_line: 55,
        },
        docs: "Public IO error type.".into(),
        doc_sections: DocSections::default(),
        kind: seraph_types::TypeKind::Struct,
        generic_params: vec![],
        where_clauses: vec![],
        is_non_exhaustive: false,
        fields: vec![],
        variants: vec![],
        has_hidden_fields: false,
        has_hidden_variants: false,
    });

    knowledge.apis.push(seraph_types::ApiInfo {
        api_id: seraph_types::ApiId::from("api::demo::io::invalid_option"),
        name: "invalid_option".into(),
        canonical_path: "demo::io::invalid_option".into(),
        public_paths: vec!["demo::io::invalid_option".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::io"),
        owner_type_id: None,
        owner_trait_id: None,
        code_ref: CodeRef {
            file: "src/io.rs".into(),
            start_line: 50,
            end_line: 54,
        },
        docs: "Serde helper that normalizes empty values.".into(),
        doc_sections: DocSections::default(),
        api_kind: seraph_types::ApiKind::FreeFunction,
        signature_text: "pub fn invalid_option<'de, D, T>(D) -> Result<Option<T>, D::Error>".into(),
        receiver: None,
        generic_params: vec!["'de".into(), "D".into(), "T".into()],
        where_clauses: vec![],
        arg_types: vec!["D".into()],
        return_type: Some("Result<Option<T>, D::Error>".into()),
        return_shape: Some(seraph_types::ReturnShape {
            kind: seraph_types::ReturnShapeKind::Nominal,
            inner_types: vec!["Option<T>".into(), "D::Error".into()],
        }),
        is_unsafe: false,
        is_async: false,
        is_const: false,
        has_body: true,
        contains_unsafe_block: false,
    });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let io_construction = models
        .fcg
        .capability_api_index
        .get(&seraph_types::CapId::from(
            "cap::demo::io::module_entry::construction",
        ))
        .unwrap();
    assert_eq!(
        io_construction,
        &vec![seraph_types::ApiId::from("api::demo::io::from_reader")]
    );

    let io_query = models
        .fcg
        .capability_api_index
        .get(&seraph_types::CapId::from("cap::demo::io::module_entry::query"))
        .unwrap();
    assert!(io_query.contains(&seraph_types::ApiId::from(
        "api::demo::io::invalid_option"
    )));
}

#[test]
fn slm_scores_stateful_types_and_emits_forbidden_transitions() {
    let knowledge = fixture_knowledge();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let parser_model = models
        .slm
        .iter()
        .find(|model| model.path == "demo::query::Parser")
        .unwrap();

    assert_eq!(parser_model.model_kind, seraph_types::SlmModelKind::Full);
    assert!(parser_model.states.contains(&"Closed".to_string()));
    let start_transition = parser_model
        .transitions
        .iter()
        .find(|transition| {
            transition.via_api_id == seraph_types::ApiId::from("api::demo::query::Parser::start")
        })
        .unwrap();
    assert_eq!(start_transition.from, "Constructed");
    assert_eq!(start_transition.to, "Active");
    let close_transition = parser_model
        .transitions
        .iter()
        .find(|transition| {
            transition.via_api_id == seraph_types::ApiId::from("api::demo::query::Parser::close")
        })
        .unwrap();
    assert_eq!(close_transition.from, "Active");
    assert_eq!(close_transition.to, "Closed");
    assert_eq!(parser_model.forbidden_transitions.len(), 1);
    assert_eq!(
        parser_model.forbidden_transitions[0].via_api_id,
        seraph_types::ApiId::from("api::demo::query::Parser::start")
    );
}

#[test]
fn slm_full_models_do_not_invent_active_from_constructor_names() {
    let mut knowledge = fixture_knowledge();
    let constructor = knowledge
        .apis
        .iter_mut()
        .find(|api| api.canonical_path == "demo::query::Parser::new")
        .unwrap();
    constructor.name = "open".into();
    let start = knowledge
        .apis
        .iter_mut()
        .find(|api| api.canonical_path == "demo::query::Parser::start")
        .unwrap();
    start.name = "read".into();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let parser_model = models
        .slm
        .iter()
        .find(|model| model.path == "demo::query::Parser")
        .unwrap();

    assert_eq!(
        parser_model.states,
        vec!["Constructed".to_string(), "Closed".to_string()]
    );
    assert!(!parser_model.states.contains(&"Active".to_string()));
    let read_transition = parser_model
        .transitions
        .iter()
        .find(|transition| {
            transition.via_api_id == seraph_types::ApiId::from("api::demo::query::Parser::start")
        })
        .unwrap();
    assert_eq!(read_transition.from, "Constructed");
    assert_eq!(read_transition.to, "Constructed");
    let close_transition = parser_model
        .transitions
        .iter()
        .find(|transition| {
            transition.via_api_id == seraph_types::ApiId::from("api::demo::query::Parser::close")
        })
        .unwrap();
    assert_eq!(close_transition.from, "Constructed");
    assert_eq!(close_transition.to, "Closed");
}

#[test]
fn slm_counts_mut_self_receivers_when_phase1_uses_uppercase_self() {
    let mut knowledge = fixture_knowledge();
    knowledge.risk_facts.drop_impl_types.clear();
    let parser_type = knowledge
        .types
        .iter_mut()
        .find(|ty| ty.canonical_path == "demo::query::Parser")
        .unwrap();
    parser_type.docs.clear();
    parser_type.doc_sections.summary.clear();
    for api in &mut knowledge.apis {
        if api.owner_type_id.as_ref()
            == Some(&seraph_types::TypeId::from("type::demo::query::Parser"))
            && api.receiver.as_deref() == Some("&mut self")
        {
            api.receiver = Some("&mut Self".into());
        }
    }
    let mut reset_api = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "demo::query::Parser::close")
        .unwrap()
        .clone();
    reset_api.api_id = seraph_types::ApiId::from("api::demo::query::Parser::reset");
    reset_api.name = "reset".into();
    reset_api.canonical_path = "demo::query::Parser::reset".into();
    reset_api.public_paths = vec!["demo::query::Parser::reset".into()];
    reset_api.doc_sections.panics.clear();
    knowledge.apis.push(reset_api);

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let parser_model = models
        .slm
        .iter()
        .find(|model| model.path == "demo::query::Parser")
        .unwrap();

    assert_eq!(parser_model.model_kind, seraph_types::SlmModelKind::Full);
}

#[test]
fn slm_only_emits_error_when_recovery_api_exists() {
    let mut knowledge = fixture_knowledge();

    let base_models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let base_parser_model = base_models
        .slm
        .iter()
        .find(|model| model.path == "demo::query::Parser")
        .unwrap();
    assert!(!base_parser_model.states.contains(&"Error".to_string()));

    let mut reset_api = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "demo::query::Parser::close")
        .unwrap()
        .clone();
    reset_api.api_id = seraph_types::ApiId::from("api::demo::query::Parser::reset");
    reset_api.name = "reset".into();
    reset_api.canonical_path = "demo::query::Parser::reset".into();
    reset_api.public_paths = vec!["demo::query::Parser::reset".into()];
    reset_api.docs = "Reset parser after an error.".into();
    reset_api.doc_sections.summary = "Reset parser after an error.".into();
    reset_api.doc_sections.panics.clear();
    knowledge.apis.push(reset_api);

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let parser_model = models
        .slm
        .iter()
        .find(|model| model.path == "demo::query::Parser")
        .unwrap();

    assert!(parser_model.states.contains(&"Error".to_string()));
}

#[test]
fn slm_downgrades_drop_only_types_without_runtime_states() {
    let mut knowledge = fixture_knowledge();

    knowledge.types.push(seraph_types::TypeInfo {
        type_id: seraph_types::TypeId::from("type::demo::drop::DropBomb"),
        name: "DropBomb".into(),
        canonical_path: "demo::drop::DropBomb".into(),
        public_paths: vec!["demo::drop::DropBomb".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::drop"),
        code_ref: CodeRef {
            file: "src/drop.rs".into(),
            start_line: 1,
            end_line: 8,
        },
        docs: "Stateful drop bomb.".into(),
        doc_sections: DocSections::default(),
        kind: seraph_types::TypeKind::Struct,
        generic_params: vec![],
        where_clauses: vec![],
        is_non_exhaustive: false,
        fields: vec![],
        variants: vec![],
        has_hidden_fields: false,
        has_hidden_variants: false,
    });
    knowledge.apis.push(seraph_types::ApiInfo {
        api_id: seraph_types::ApiId::from("api::demo::drop::DropBomb::new"),
        name: "new".into(),
        canonical_path: "demo::drop::DropBomb::new".into(),
        public_paths: vec!["demo::drop::DropBomb::new".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::drop"),
        owner_type_id: Some(seraph_types::TypeId::from("type::demo::drop::DropBomb")),
        owner_trait_id: None,
        code_ref: CodeRef {
            file: "src/drop.rs".into(),
            start_line: 10,
            end_line: 10,
        },
        docs: "Creates a drop bomb.".into(),
        doc_sections: DocSections::default(),
        api_kind: seraph_types::ApiKind::Constructor,
        signature_text: "pub fn new() -> DropBomb".into(),
        receiver: None,
        generic_params: vec![],
        where_clauses: vec![],
        arg_types: vec![],
        return_type: Some("DropBomb".into()),
        return_shape: Some(seraph_types::ReturnShape {
            kind: seraph_types::ReturnShapeKind::Nominal,
            inner_types: vec!["DropBomb".into()],
        }),
        is_unsafe: false,
        is_async: false,
        is_const: false,
        has_body: true,
        contains_unsafe_block: false,
    });
    knowledge
        .risk_facts
        .drop_impl_types
        .push(seraph_types::TypeId::from("type::demo::drop::DropBomb"));

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let model = models
        .slm
        .iter()
        .find(|model| model.path == "demo::drop::DropBomb")
        .unwrap();

    assert_eq!(model.model_kind, seraph_types::SlmModelKind::Simplified);
    assert_eq!(model.states, vec!["Constructed".to_string()]);
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
        from_reader.generic_constraints.as_ref().unwrap().params[0].full_bound_chain,
        vec!["demo::io::Reader", "core::fmt::Debug"]
    );
    assert_eq!(
        from_reader.generic_constraints.as_ref().unwrap().params[0].associated_type_constraints[0]
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
        models
            .risk_surface_map
            .type_synthesis_overview
            .strategy_distribution["C"],
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

#[test]
fn contract_builder_normalizes_receiver_side_effects_and_keeps_panics_separate() {
    let mut knowledge = fixture_knowledge();
    let parser_start = knowledge
        .apis
        .iter_mut()
        .find(|api| api.canonical_path == "demo::query::Parser::start")
        .unwrap();
    parser_start.receiver = Some("&mut Self".into());

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let contract = models
        .api_contracts
        .iter()
        .find(|contract| contract.path == "demo::query::Parser::start")
        .unwrap();

    assert_eq!(
        contract.preconditions,
        vec!["Panics if the parser is already closed."]
    );
    assert_eq!(
        contract.panic_conditions,
        vec!["Panics if the parser is already closed."]
    );
    assert_eq!(contract.side_effects, vec!["mutates receiver"]);
}

#[test]
fn contract_builder_includes_owner_type_generic_constraints_for_assoc_functions() {
    let mut knowledge = fixture_knowledge();
    let parser_type = knowledge
        .types
        .iter_mut()
        .find(|ty| ty.canonical_path == "demo::query::Parser")
        .unwrap();
    parser_type.generic_params = vec!["A".into()];
    parser_type.where_clauses = vec!["A: demo::io::Reader".into()];

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let contract = models
        .api_contracts
        .iter()
        .find(|contract| contract.path == "demo::query::Parser::new")
        .unwrap();
    let generic_constraints = contract.generic_constraints.as_ref().unwrap();

    assert_eq!(generic_constraints.params.len(), 1);
    assert_eq!(generic_constraints.params[0].name, "A");
    assert_eq!(
        generic_constraints.params[0].direct_bounds,
        vec!["demo::io::Reader"]
    );
    assert_eq!(generic_constraints.params[0].strategy, "C");
}

#[test]
fn contract_builder_separates_lifetime_params_from_synthesis_params() {
    let mut knowledge = fixture_knowledge();
    let from_reader = knowledge
        .apis
        .iter_mut()
        .find(|api| api.canonical_path == "demo::io::from_reader")
        .unwrap();
    from_reader.generic_params = vec!["'a".into(), "R".into()];
    from_reader.where_clauses = vec!["R: demo::io::Reader".into()];

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let constraints = models
        .api_contracts
        .iter()
        .find(|contract| contract.path == "demo::io::from_reader")
        .unwrap()
        .generic_constraints
        .as_ref()
        .unwrap();

    assert_eq!(constraints.lifetime_params, vec!["'a".to_string()]);
    assert_eq!(
        constraints
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>(),
        vec!["R".to_string()]
    );
}

#[test]
fn risk_builder_scores_unsafe_and_explicit_panic_evidence() {
    let mut knowledge = fixture_knowledge();
    let parser_start_api_id = {
        let parser_start = knowledge
            .apis
            .iter_mut()
            .find(|api| api.canonical_path == "demo::query::Parser::start")
            .unwrap();
        parser_start.contains_unsafe_block = true;
        parser_start.api_id.clone()
    };
    let parser_start = knowledge
        .apis
        .iter()
        .find(|api| api.api_id == parser_start_api_id)
        .unwrap();
    knowledge
        .risk_facts
        .explicit_panic_sites
        .push(ExplicitPanicSiteFact {
            owner: RiskOwner::Api(parser_start.api_id.clone()),
            panic_kind: "panic!".into(),
            source: CodeRef {
                file: "src/query.rs".into(),
                start_line: 40,
                end_line: 40,
            },
        });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let risk = models
        .risk_surface_map
        .api_risks
        .iter()
        .find(|risk| risk.api_id == parser_start_api_id)
        .unwrap();

    assert!(risk.reasons.contains(&"contains unsafe block".to_owned()));
    assert!(risk.reasons.contains(&"explicit panic site".to_owned()));
    assert_ne!(risk.risk_level, seraph_types::RiskLevel::Low);
}

#[test]
fn risk_builder_emits_repr_packed_risk_for_affected_owner_type_apis() {
    let mut knowledge = fixture_knowledge();
    knowledge.risk_facts.repr_types.push(TypeLayoutFact {
        type_id: seraph_types::TypeId::from("type::demo::query::Parser"),
        repr_kinds: vec![ReprKind::Packed],
        source: None,
    });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let repr_risk = models
        .risk_surface_map
        .rust_feature_risks
        .iter()
        .find(|risk| risk.feature == "repr_packed")
        .unwrap();

    assert!(repr_risk.apis_affected.is_empty());
    assert_eq!(
        repr_risk.types_affected,
        vec![seraph_types::TypeId::from("type::demo::query::Parser")]
    );
}

#[test]
fn risk_surface_reports_conditional_impl_and_panic_in_drop() {
    let mut knowledge = fixture_knowledge();

    knowledge
        .trait_impl_registry
        .push(seraph_types::TraitImplInfo {
            trait_impl_id: "trait_impl::demo::query::FeatureExt::for::Parser".into(),
            target_type_id: seraph_types::TypeId::from("type::demo::query::Parser"),
            trait_ref_text: "demo::query::FeatureExt".into(),
            for_type_text: "Parser".into(),
            trait_id: "trait::demo::query::FeatureExt".into(),
            trait_name: "FeatureExt".into(),
            trait_canonical_path: "demo::query::FeatureExt".into(),
            trait_origin: seraph_types::TraitOrigin::External,
            source: CodeRef {
                file: "src/query.rs".into(),
                start_line: 60,
                end_line: 62,
            },
            associated_type_bindings: vec![],
            associated_const_bindings: vec![],
            where_clauses: vec![],
            cfg_attrs: vec!["#[cfg(feature = \"experimental\")]".into()],
            is_unsafe: false,
        });

    let drop_impl_id: seraph_types::TraitImplId =
        "trait_impl::core::ops::drop::Drop::for::Parser".into();
    knowledge
        .trait_impl_registry
        .push(seraph_types::TraitImplInfo {
            trait_impl_id: drop_impl_id.clone(),
            target_type_id: seraph_types::TypeId::from("type::demo::query::Parser"),
            trait_ref_text: "core::ops::drop::Drop".into(),
            for_type_text: "Parser".into(),
            trait_id: "trait::core::ops::drop::Drop".into(),
            trait_name: "Drop".into(),
            trait_canonical_path: "core::ops::drop::Drop".into(),
            trait_origin: seraph_types::TraitOrigin::External,
            source: CodeRef {
                file: "src/query.rs".into(),
                start_line: 64,
                end_line: 66,
            },
            associated_type_bindings: vec![],
            associated_const_bindings: vec![],
            where_clauses: vec![],
            cfg_attrs: vec![],
            is_unsafe: false,
        });
    knowledge
        .risk_facts
        .explicit_panic_sites
        .push(ExplicitPanicSiteFact {
            owner: RiskOwner::TraitImpl(drop_impl_id),
            panic_kind: "panic!".into(),
            source: CodeRef {
                file: "src/query.rs".into(),
                start_line: 65,
                end_line: 65,
            },
        });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    let conditional_impl = models
        .risk_surface_map
        .rust_feature_risks
        .iter()
        .find(|risk| risk.feature == "conditional_impl")
        .unwrap();
    assert!(conditional_impl.apis_affected.is_empty());
    assert_eq!(
        conditional_impl.types_affected,
        vec![seraph_types::TypeId::from("type::demo::query::Parser")]
    );

    let panic_in_drop = models
        .risk_surface_map
        .rust_feature_risks
        .iter()
        .find(|risk| risk.feature == "panic_in_drop")
        .unwrap();
    assert!(panic_in_drop.apis_affected.is_empty());
    assert_eq!(
        panic_in_drop.types_affected,
        vec![seraph_types::TypeId::from("type::demo::query::Parser")]
    );
}

#[test]
fn risk_surface_keeps_type_level_risks_without_api_expansion() {
    let mut knowledge = fixture_knowledge();

    knowledge.types.push(seraph_types::TypeInfo {
        type_id: seraph_types::TypeId::from("type::demo::drop::Bomb"),
        name: "Bomb".into(),
        canonical_path: "demo::drop::Bomb".into(),
        public_paths: vec!["demo::drop::Bomb".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::drop"),
        code_ref: CodeRef {
            file: "src/drop.rs".into(),
            start_line: 1,
            end_line: 8,
        },
        docs: "Panicking drop bomb.".into(),
        doc_sections: DocSections::default(),
        kind: seraph_types::TypeKind::Struct,
        generic_params: vec![],
        where_clauses: vec![],
        is_non_exhaustive: false,
        fields: vec![],
        variants: vec![],
        has_hidden_fields: false,
        has_hidden_variants: false,
    });

    let bomb_drop_impl_id: seraph_types::TraitImplId =
        "trait_impl::core::ops::drop::Drop::for::demo::drop::Bomb".into();
    knowledge
        .trait_impl_registry
        .push(seraph_types::TraitImplInfo {
            trait_impl_id: bomb_drop_impl_id.clone(),
            target_type_id: seraph_types::TypeId::from("type::demo::drop::Bomb"),
            trait_ref_text: "core::ops::drop::Drop".into(),
            for_type_text: "demo::drop::Bomb".into(),
            trait_id: seraph_types::TraitId::from("trait::core::ops::drop::Drop"),
            trait_name: "Drop".into(),
            trait_canonical_path: "core::ops::drop::Drop".into(),
            trait_origin: seraph_types::TraitOrigin::External,
            source: CodeRef {
                file: "src/drop.rs".into(),
                start_line: 10,
                end_line: 14,
            },
            associated_type_bindings: vec![],
            associated_const_bindings: vec![],
            where_clauses: vec![],
            cfg_attrs: vec![],
            is_unsafe: false,
        });

    knowledge
        .trait_impl_registry
        .push(seraph_types::TraitImplInfo {
            trait_impl_id: "trait_impl::demo::Feature::for::demo::query::Parser".into(),
            target_type_id: seraph_types::TypeId::from("type::demo::query::Parser"),
            trait_ref_text: "demo::Feature".into(),
            for_type_text: "demo::query::Parser".into(),
            trait_id: seraph_types::TraitId::from("trait::demo::Feature"),
            trait_name: "Feature".into(),
            trait_canonical_path: "demo::Feature".into(),
            trait_origin: seraph_types::TraitOrigin::Local,
            source: CodeRef {
                file: "src/query.rs".into(),
                start_line: 40,
                end_line: 40,
            },
            associated_type_bindings: vec![],
            associated_const_bindings: vec![],
            where_clauses: vec![],
            cfg_attrs: vec!["#[cfg(feature = \"nightly\")]".into()],
            is_unsafe: false,
        });

    knowledge
        .risk_facts
        .drop_impl_types
        .push(seraph_types::TypeId::from("type::demo::drop::Bomb"));
    knowledge
        .risk_facts
        .explicit_panic_sites
        .push(ExplicitPanicSiteFact {
            owner: RiskOwner::TraitImpl(bomb_drop_impl_id),
            panic_kind: "panic!".into(),
            source: CodeRef {
                file: "src/drop.rs".into(),
                start_line: 12,
                end_line: 12,
            },
        });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    let panic_in_drop = models
        .risk_surface_map
        .rust_feature_risks
        .iter()
        .find(|risk| risk.feature == "panic_in_drop")
        .unwrap();
    assert!(panic_in_drop.apis_affected.is_empty());
    assert_eq!(
        panic_in_drop.types_affected,
        vec![seraph_types::TypeId::from("type::demo::drop::Bomb")]
    );

    let conditional_impl = models
        .risk_surface_map
        .rust_feature_risks
        .iter()
        .find(|risk| risk.feature == "conditional_impl")
        .unwrap();
    assert!(conditional_impl.apis_affected.is_empty());
    assert_eq!(
        conditional_impl.types_affected,
        vec![seraph_types::TypeId::from("type::demo::query::Parser")]
    );
}
