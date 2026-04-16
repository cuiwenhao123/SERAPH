use seraph_types::{
    ApiId, ApiKind, ExampleAnchor, Knowledge, ReprKind, ReturnShapeKind, RiskOwner, SymbolKind,
    TraitOrigin, TypeId, VariantKind,
};
use std::collections::BTreeSet;

fn hashbrown_manifest_path() -> &'static str {
    "/tmp/hashbrown-eval/Cargo.toml"
}

fn workspace_nested_hashbrown_manifest_path() -> String {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root should be discoverable");
    repo_root
        .join("examples/target-crates/hashbrown/Cargo.toml")
        .to_string_lossy()
        .into_owned()
}

fn audit_fixture_manifest_path() -> String {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root should be discoverable");
    repo_root
        .join("examples/target-crates/s3-audit-fixture/Cargo.toml")
        .to_string_lossy()
        .into_owned()
}

fn sqlx_core_manifest_path() -> &'static str {
    "/tmp/seraph-real-crates/sqlx/sqlx-core/Cargo.toml"
}

fn camino_manifest_path() -> &'static str {
    "/tmp/seraph-phase2-new-crates/camino/Cargo.toml"
}

fn tar_manifest_path() -> &'static str {
    "/tmp/seraph-phase2-new-crates/tar-rs/Cargo.toml"
}

#[test]
fn build_minimal_knowledge_normalizes_import_name() {
    let knowledge = s3_extract::build_minimal_knowledge("serde-json-wrapper");

    assert_eq!(knowledge.crate_meta.package_name, "serde-json-wrapper");
    assert_eq!(knowledge.crate_meta.lib_target_name, "serde_json_wrapper");
    assert_eq!(knowledge.crate_meta.crate_import_name, "serde_json_wrapper");
    assert!(knowledge.modules.is_empty());
    assert!(knowledge.trait_registry.is_empty());
    assert!(knowledge.trait_impl_registry.is_empty());
}

#[test]
fn extract_knowledge_from_manifest_reads_cargo_metadata_and_rustdoc() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(hashbrown_manifest_path()).unwrap();

    assert_eq!(knowledge.crate_meta.package_name, "hashbrown");
    assert_eq!(knowledge.crate_meta.lib_target_name, "hashbrown");
    assert_eq!(knowledge.crate_meta.crate_import_name, "hashbrown");
    assert_eq!(knowledge.crate_meta.version, "0.17.0");
    assert_eq!(knowledge.crate_meta.edition, "2024");
    assert_eq!(knowledge.crate_meta.rust_version.as_deref(), Some("1.85.0"));
    assert_eq!(
        knowledge.crate_meta.repository.as_deref(),
        Some("https://github.com/rust-lang/hashbrown")
    );
    assert_eq!(
        knowledge.crate_meta.cargo_description.as_deref(),
        Some("A Rust port of Google's SwissTable hash map")
    );
    assert!(knowledge
        .crate_meta
        .default_features
        .contains(&"raw-entry".to_owned()));
    assert_eq!(
        knowledge.crate_meta.manifest_path,
        hashbrown_manifest_path()
    );
    assert!(knowledge.crate_meta.lib_rs_path.ends_with("/src/lib.rs"));
    assert!(knowledge.crate_meta.root_docs.contains("SwissTable"));

    assert!(knowledge
        .modules
        .iter()
        .any(|module| module.name == "hash_map"));

    let hash_map_type = knowledge
        .types
        .iter()
        .find(|ty| ty.name == "HashMap")
        .expect("HashMap type should be extracted");
    assert_eq!(
        hash_map_type.public_anchor_module_id.as_str(),
        "mod::hashbrown::hash_map"
    );
    assert!(hash_map_type
        .public_paths
        .contains(&"hashbrown::HashMap".to_owned()));

    let entry_type = knowledge
        .types
        .iter()
        .find(|ty| ty.canonical_path == "hashbrown::map::Entry")
        .expect("Entry type should be extracted");
    assert_eq!(entry_type.where_clauses, vec!["A: Allocator".to_owned()]);

    let new_api = knowledge
        .apis
        .iter()
        .find(|api| api.name == "new" && api.owner_type_id.as_ref() == Some(&hash_map_type.type_id))
        .expect("HashMap::new should be extracted");
    assert_eq!(new_api.api_kind, ApiKind::Constructor);
    assert_eq!(
        new_api.public_anchor_module_id.as_str(),
        "mod::hashbrown::hash_map"
    );
    assert!(new_api
        .public_paths
        .iter()
        .any(|path| path.ends_with("HashMap::new")));

    let contains_key_api = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "hashbrown::map::HashMap::contains_key")
        .expect("HashMap::contains_key should be extracted");
    assert_eq!(
        contains_key_api.where_clauses,
        vec!["Q: Hash + Equivalent<K> + ?Sized".to_owned()]
    );
    assert!(contains_key_api
        .where_clauses
        .iter()
        .all(|clause| !clause.trim_start().starts_with('{')));

    let equivalent_trait = knowledge
        .trait_registry
        .iter()
        .find(|trait_info| trait_info.name == "Equivalent")
        .expect("Equivalent trait should be extracted");
    assert_eq!(equivalent_trait.origin, TraitOrigin::Reexported);
    assert!(equivalent_trait
        .public_paths
        .contains(&"hashbrown::Equivalent".to_owned()));

    let hash_trait = knowledge
        .trait_registry
        .iter()
        .find(|trait_info| trait_info.canonical_path == "core::hash::Hash")
        .expect("Hash trait should be extracted from public bounds");
    assert!(hash_trait
        .used_by_api_ids
        .contains(&ApiId::from("api::hashbrown::map::HashMap::contains_key")));

    let build_hasher_trait = knowledge
        .trait_registry
        .iter()
        .find(|trait_info| trait_info.canonical_path == "core::hash::BuildHasher")
        .expect("BuildHasher trait should be extracted from public bounds");
    assert!(build_hasher_trait
        .used_by_api_ids
        .contains(&ApiId::from("api::hashbrown::map::Entry::insert")));

    let allocator_trait = knowledge
        .trait_registry
        .iter()
        .find(|trait_info| trait_info.canonical_path == "allocator_api2::stable::alloc::Allocator")
        .expect("Allocator trait should be extracted from public type bounds");
    assert!(allocator_trait
        .used_by_type_ids
        .contains(&TypeId::from("type::hashbrown::map::Entry")));

    let eq_trait = knowledge
        .trait_registry
        .iter()
        .find(|trait_info| trait_info.canonical_path == "core::cmp::Eq")
        .expect("Eq trait should be extracted from public impl bounds");
    assert!(eq_trait
        .used_by_api_ids
        .contains(&ApiId::from("api::hashbrown::map::HashMap::contains_key")));
    assert!(!knowledge
        .trait_registry
        .iter()
        .any(|trait_info| trait_info.canonical_path == "core::ops::bit::BitAnd"));

    let hash_set_bitand_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "core::ops::bit::BitAnd"
                && trait_impl.for_type_text == "&HashSet<T, S, A>"
        })
        .expect("BitAnd impl for &HashSet should be extracted into trait_impl_registry");
    assert_eq!(
        hash_set_bitand_impl.target_type_id.as_str(),
        "type::hashbrown::set::HashSet"
    );
    assert_eq!(
        hash_set_bitand_impl.trait_ref_text,
        "core::ops::bit::BitAnd<&HashSet<T, S, A>>"
    );
    assert!(hash_set_bitand_impl
        .where_clauses
        .contains(&"T: Eq + Hash + Clone".to_owned()));
    assert!(!hash_set_bitand_impl.is_unsafe);

    let hash_set_from_iterator_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "core::iter::traits::collect::FromIterator"
                && trait_impl.for_type_text == "HashSet<T, S, A>"
        })
        .expect("FromIterator impl for HashSet should be extracted into trait_impl_registry");
    assert_eq!(
        hash_set_from_iterator_impl.target_type_id.as_str(),
        "type::hashbrown::set::HashSet"
    );
    assert!(hash_set_from_iterator_impl
        .where_clauses
        .contains(&"T: Eq + Hash".to_owned()));

    let hash_map_default_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "core::default::Default"
                && trait_impl.for_type_text == "HashMap<K, V, S, A>"
        })
        .expect("Default impl for HashMap should be extracted into trait_impl_registry");
    assert_eq!(
        hash_map_default_impl.target_type_id.as_str(),
        "type::hashbrown::map::HashMap"
    );
    assert!(hash_map_default_impl
        .where_clauses
        .contains(&"S: Default".to_owned()));
    assert!(hash_map_default_impl.associated_type_bindings.is_empty());

    let occupied_entry_debug_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "core::fmt::Debug"
                && trait_impl.for_type_text == "OccupiedEntry<'_, K, V, S, A>"
        })
        .expect("Debug impl for OccupiedEntry should be extracted into trait_impl_registry");
    assert_eq!(
        occupied_entry_debug_impl.target_type_id.as_str(),
        "type::hashbrown::map::OccupiedEntry"
    );

    let iter_iterator_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "core::iter::traits::iterator::Iterator"
                && trait_impl.for_type_text == "Iter<'a, K>"
        })
        .expect("Iterator impl for set::Iter should be extracted into trait_impl_registry");
    assert_eq!(iter_iterator_impl.associated_type_bindings.len(), 1);
    assert_eq!(iter_iterator_impl.associated_type_bindings[0].name, "Item");
    assert_eq!(
        iter_iterator_impl.associated_type_bindings[0]
            .assigned_type
            .as_deref(),
        Some("&'a K")
    );

    let hash_map_into_iter_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "core::iter::traits::collect::IntoIterator"
                && trait_impl.for_type_text == "HashMap<K, V, S, A>"
        })
        .expect("IntoIterator impl for owned HashMap should be extracted into trait_impl_registry");
    assert_eq!(hash_map_into_iter_impl.associated_type_bindings.len(), 2);
    assert!(hash_map_into_iter_impl
        .associated_type_bindings
        .iter()
        .any(
            |binding| binding.name == "Item" && binding.assigned_type.as_deref() == Some("(K, V)")
        ));
    assert!(hash_map_into_iter_impl
        .associated_type_bindings
        .iter()
        .any(|binding| {
            binding.name == "IntoIter"
                && binding.assigned_type.as_deref() == Some("IntoIter<K, V, A>")
        }));

    let try_reserve_display_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "core::fmt::Display"
                && trait_impl.for_type_text == "TryReserveError"
        })
        .expect("Display impl for TryReserveError should be extracted into trait_impl_registry");
    assert_eq!(
        try_reserve_display_impl.target_type_id.as_str(),
        "type::hashbrown::TryReserveError"
    );

    let contains_key_example = knowledge
        .examples
        .iter()
        .find(|example| match &example.anchor {
            ExampleAnchor::Api(api_id) => api_id == &contains_key_api.api_id,
            _ => false,
        })
        .expect("contains_key doc example should be extracted");
    assert!(contains_key_example
        .involved_api_ids
        .contains(&contains_key_api.api_id));
    assert_eq!(contains_key_example.code_ref.file, "src/map.rs");
    assert!(contains_key_example
        .snippet
        .as_deref()
        .is_some_and(|snippet| snippet.contains("map.contains_key(&1)")));

    let entry_insert_api = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "hashbrown::map::Entry::insert")
        .expect("Entry::insert should be extracted");
    let entry_insert_example = knowledge
        .examples
        .iter()
        .find(|example| match &example.anchor {
            ExampleAnchor::Api(api_id) => api_id == &entry_insert_api.api_id,
            _ => false,
        })
        .expect("Entry::insert doc example should be extracted");
    assert!(entry_insert_example
        .involved_api_ids
        .contains(&entry_insert_api.api_id));
    assert!(entry_insert_example
        .snippet
        .as_deref()
        .is_some_and(|snippet| snippet.contains("map.entry(\"horseyland\").insert(37)")));

    let hash_map_type_example = knowledge
        .examples
        .iter()
        .find(|example| match &example.anchor {
            ExampleAnchor::Type(type_id) => type_id == &hash_map_type.type_id,
            _ => false,
        })
        .expect("HashMap type doc example should be extracted");
    assert_eq!(hash_map_type_example.code_ref.file, "src/map.rs");
    assert!(hash_map_type_example
        .snippet
        .as_deref()
        .is_some_and(|snippet| snippet.contains("let mut book_reviews = HashMap::new();")));
}

#[test]
fn cli_writes_valid_knowledge_json_from_manifest_path() {
    let output = std::env::temp_dir().join(format!(
        "seraph_s3_extract_test_{}_knowledge.json",
        std::process::id()
    ));

    let bin = std::env::var("CARGO_BIN_EXE_s3-extract").unwrap();
    let status = std::process::Command::new(bin)
        .args([
            "--manifest-path",
            hashbrown_manifest_path(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());

    let raw = std::fs::read_to_string(&output).unwrap();
    let knowledge: Knowledge = serde_json::from_str(&raw).unwrap();

    assert_eq!(knowledge.crate_meta.package_name, "hashbrown");
    assert_eq!(knowledge.crate_meta.version, "0.17.0");
    assert_eq!(knowledge.crate_meta.edition, "2024");
    assert_eq!(
        knowledge.crate_meta.manifest_path,
        hashbrown_manifest_path()
    );
    assert!(knowledge.crate_meta.lib_rs_path.ends_with("/src/lib.rs"));
    assert!(knowledge.crate_meta.root_docs.contains("SwissTable"));
    assert!(knowledge
        .modules
        .iter()
        .any(|module| module.name == "hash_map"));
    assert!(knowledge.types.iter().any(|ty| ty.name == "HashMap"));
    assert!(knowledge.apis.iter().any(|api| api.name == "new"));
    assert!(knowledge
        .trait_registry
        .iter()
        .any(|trait_info| trait_info.name == "Equivalent"));
    assert!(knowledge
        .trait_impl_registry
        .iter()
        .any(|trait_impl| trait_impl.trait_canonical_path == "core::default::Default"));

    let _ = std::fs::remove_file(output);
}

#[test]
fn extract_knowledge_from_nested_workspace_manifest_uses_fallback() {
    let manifest_path = workspace_nested_hashbrown_manifest_path();
    if !std::path::Path::new(&manifest_path).exists() {
        eprintln!("skipping nested-workspace hashbrown fallback test: fixture is unavailable");
        return;
    }

    let knowledge = s3_extract::extract_knowledge_from_manifest(&manifest_path)
        .expect("workspace-nested manifest should extract successfully via fallback");

    assert_eq!(knowledge.crate_meta.package_name, "hashbrown");
    assert_eq!(knowledge.crate_meta.manifest_path, manifest_path);
    assert!(!knowledge.examples.is_empty());
    assert!(knowledge
        .trait_registry
        .iter()
        .any(|trait_info| trait_info.canonical_path == "core::hash::Hash"));
    assert!(knowledge
        .trait_impl_registry
        .iter()
        .any(|trait_impl| trait_impl.trait_canonical_path == "core::ops::bit::BitAnd"));
}

#[test]
fn extract_workspace_member_manifest_reads_rustdoc_from_workspace_target() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(sqlx_core_manifest_path())
        .expect("workspace member manifest should extract successfully");

    assert_eq!(knowledge.crate_meta.package_name, "sqlx-core");
    assert_eq!(knowledge.crate_meta.lib_target_name, "sqlx_core");
    assert_eq!(knowledge.crate_meta.crate_import_name, "sqlx_core");
    assert_eq!(
        knowledge.crate_meta.manifest_path,
        sqlx_core_manifest_path()
    );
    assert!(knowledge
        .crate_meta
        .lib_rs_path
        .ends_with("/sqlx-core/src/lib.rs"));
    assert!(knowledge
        .modules
        .iter()
        .any(|module| module.canonical_path == "sqlx_core::pool"));
    assert!(knowledge
        .types
        .iter()
        .any(|ty| ty.canonical_path == "sqlx_core::pool::Pool"));
}

#[test]
fn extract_real_camino_omits_impl_trait_from_api_generic_params() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(camino_manifest_path())
        .expect("camino manifest should extract successfully");

    let utf8_path_new = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "camino::Utf8Path::new")
        .expect("Utf8Path::new should be extracted");
    assert_eq!(
        utf8_path_new.generic_params,
        Vec::<String>::new(),
        "`impl AsRef<str> + ?Sized` should not be recorded as a generic parameter"
    );

    let utf8_path_join = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "camino::Utf8Path::join")
        .expect("Utf8Path::join should be extracted");
    assert_eq!(
        utf8_path_join.generic_params,
        Vec::<String>::new(),
        "`impl AsRef<Utf8Path>` should stay in the argument type, not generic_params"
    );
}

#[test]
fn extract_real_tar_keeps_lifetimes_but_omits_impl_trait_generic_params() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(tar_manifest_path())
        .expect("tar manifest should extract successfully");

    let append_pax_extensions = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "tar::builder::Builder::append_pax_extensions")
        .expect("append_pax_extensions should be extracted");
    assert_eq!(
        append_pax_extensions.generic_params,
        vec!["'key".to_owned(), "'value".to_owned()],
        "named lifetimes should be preserved while synthetic `impl Trait` params are filtered out"
    );
    assert_eq!(
        append_pax_extensions.arg_types,
        vec!["impl IntoIterator<Item = (&'key str, &'value [u8])>".to_owned()],
        "the argument type should keep the original `impl Trait` surface text"
    );
}

#[test]
fn extract_fixture_promotes_public_trait_methods_into_apis() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");
    let value = serde_json::to_value(&knowledge).expect("knowledge should serialize to JSON value");
    let apis = value["apis"]
        .as_array()
        .expect("apis should serialize as an array");

    let provided = apis
        .iter()
        .find(|api| api["canonical_path"] == "s3_audit_fixture::ExampleTrait::provided")
        .expect("provided trait method should be extracted as an API");
    assert_eq!(provided["api_kind"], "trait_method");
    assert_eq!(
        provided["owner_trait_id"],
        "trait::s3_audit_fixture::ExampleTrait"
    );

    let required = apis
        .iter()
        .find(|api| api["canonical_path"] == "s3_audit_fixture::ExampleTrait::required")
        .expect("required trait method should be extracted as an API");
    assert_eq!(required["api_kind"], "trait_method");
    assert_eq!(
        required["owner_trait_id"],
        "trait::s3_audit_fixture::ExampleTrait"
    );
}

#[test]
fn extract_fixture_partitions_required_and_provided_trait_methods() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");
    let example_trait = knowledge
        .trait_registry
        .iter()
        .find(|trait_info| trait_info.canonical_path == "s3_audit_fixture::ExampleTrait")
        .expect("fixture trait should be extracted into trait_registry");

    assert_eq!(example_trait.required_methods, vec!["required".to_owned()]);
    assert_eq!(example_trait.provided_methods, vec!["provided".to_owned()]);
}

#[test]
fn extract_fixture_classifies_explicit_and_wrapped_constructors() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");
    let example_type = knowledge
        .types
        .iter()
        .find(|ty| ty.canonical_path == "s3_audit_fixture::ExampleType")
        .expect("fixture type should be extracted");

    let named_api = knowledge
        .apis
        .iter()
        .find(|api| {
            api.canonical_path == "s3_audit_fixture::ExampleType::named"
                && api.owner_type_id.as_ref() == Some(&example_type.type_id)
        })
        .expect("ExampleType::named should be extracted");
    assert_eq!(named_api.api_kind, ApiKind::Constructor);

    let wrapped_api = knowledge
        .apis
        .iter()
        .find(|api| {
            api.canonical_path == "s3_audit_fixture::ExampleType::wrapped"
                && api.owner_type_id.as_ref() == Some(&example_type.type_id)
        })
        .expect("ExampleType::wrapped should be extracted");
    assert_eq!(wrapped_api.api_kind, ApiKind::Constructor);
}

#[test]
fn extract_fixture_collects_risk_facts() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");

    let extern_fact = knowledge
        .risk_facts
        .extern_abi_apis
        .iter()
        .find(|fact| fact.api_id.as_str() == "api::s3_audit_fixture::fixture_ffi_add")
        .expect("extern ABI API should be recorded in extern_abi_apis");
    assert_eq!(extern_fact.abi, "C");

    let repr_fact = knowledge
        .risk_facts
        .repr_types
        .iter()
        .find(|fact| fact.type_id.as_str() == "type::s3_audit_fixture::ExampleTransparent")
        .expect("repr-carrying type should be recorded in repr risk facts");
    assert_eq!(repr_fact.repr_kinds, vec![ReprKind::Transparent]);

    assert!(knowledge
        .risk_facts
        .drop_impl_types
        .contains(&TypeId::from("type::s3_audit_fixture::ExampleDrop")));
    assert!(knowledge
        .risk_facts
        .drop_impl_types
        .contains(&TypeId::from("type::s3_audit_fixture::ExampleDropPanic")));
}

#[test]
fn extract_fixture_records_trait_object_api_usage_edges() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");
    let example_trait = knowledge
        .trait_registry
        .iter()
        .find(|trait_info| trait_info.canonical_path == "s3_audit_fixture::ExampleTrait")
        .expect("fixture trait should be extracted into trait_registry");

    assert!(example_trait
        .used_by_api_ids
        .contains(&ApiId::from("api::s3_audit_fixture::use_trait_object")));

    let use_trait_object = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "s3_audit_fixture::use_trait_object")
        .expect("trait-object API should be extracted");
    assert_eq!(
        use_trait_object.arg_types,
        vec!["&dyn ExampleTrait".to_owned()]
    );
    assert_eq!(
        use_trait_object.signature_text,
        "fn use_trait_object(&dyn ExampleTrait) -> usize"
    );
}

#[test]
fn extract_fixture_marks_unsafe_trait_impl_headers() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");
    let send_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "core::marker::Send"
                && trait_impl.for_type_text == "ExampleSend"
        })
        .expect("manual Send impl should be extracted into trait_impl_registry");

    assert!(
        send_impl.is_unsafe,
        "manual `unsafe impl Send` should be marked unsafe"
    );
}

#[test]
fn extract_fixture_marks_internal_unsafe_blocks_without_header_unsafety() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");
    let api = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "s3_audit_fixture::uses_unsafe_block")
        .expect("unsafe-block fixture API should be extracted");

    assert!(!api.is_unsafe, "safe header should remain safe");
    assert!(
        api.contains_unsafe_block,
        "function body should be marked when it contains `unsafe {{}}`"
    );
}

#[test]
fn extract_fixture_keeps_all_root_doc_code_blocks_as_examples() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");
    let root_module = knowledge
        .modules
        .iter()
        .find(|module| module.canonical_path == "s3_audit_fixture")
        .expect("fixture root module should be present");

    let root_examples = knowledge
        .examples
        .iter()
        .filter(|example| matches!(&example.anchor, ExampleAnchor::Module(module_id) if module_id == &root_module.module_id))
        .collect::<Vec<_>>();
    assert_eq!(
        root_examples.len(),
        2,
        "each Rust doc block should become its own example entry"
    );

    let example_ids = root_examples
        .iter()
        .map(|example| example.example_id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        example_ids.len(),
        2,
        "distinct code blocks should not collide on example_id"
    );
}

#[test]
fn extract_fixture_includes_macro_and_constant_surface() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");
    let value = serde_json::to_value(&knowledge).expect("knowledge should serialize to JSON value");
    let symbols = value["symbols"]
        .as_array()
        .expect("symbols should serialize as an array");

    let fixture_macro = symbols
        .iter()
        .find(|symbol| symbol["canonical_path"] == "s3_audit_fixture::fixture_macro")
        .expect("fixture macro should be extracted into symbols");
    assert_eq!(fixture_macro["symbol_kind"], "macro");

    let sample_const = symbols
        .iter()
        .find(|symbol| symbol["canonical_path"] == "s3_audit_fixture::SAMPLE_CONST")
        .expect("fixture constant should be extracted into symbols");
    assert_eq!(sample_const["symbol_kind"], "constant");
    assert_eq!(sample_const["type_text"], "usize");

    let assoc_const = symbols
        .iter()
        .find(|symbol| symbol["canonical_path"] == "s3_audit_fixture::ExampleType::DEFAULT_LABEL")
        .expect("fixture associated const should be extracted into symbols");
    assert_eq!(assoc_const["symbol_kind"], "associated_constant");
    assert_eq!(
        assoc_const["owner_type_id"],
        "type::s3_audit_fixture::ExampleType"
    );
    assert_eq!(assoc_const["type_text"], "&'static str");
}

#[test]
fn extract_outputs_do_not_serialize_removed_compat_fields() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");
    let value = serde_json::to_value(&knowledge).expect("knowledge should serialize to JSON value");

    let trait_impl = value["trait_impl_registry"]
        .as_array()
        .and_then(|items| items.first())
        .expect("fixture should serialize at least one trait impl");
    assert!(
        trait_impl.get("surface_bucket").is_none(),
        "trait impl JSON should not retain removed semantic bucket field"
    );

    let risk_facts = value["risk_facts"]
        .as_object()
        .expect("risk_facts should serialize as an object");
    assert!(
        !risk_facts.contains_key("ffi_apis"),
        "risk_facts JSON should not retain removed ffi_apis alias"
    );
    assert!(
        risk_facts.contains_key("extern_abi_apis"),
        "risk_facts JSON should keep extern_abi_apis as the single ABI fact field"
    );
}

#[test]
fn extract_fixture_populates_doc_sections_and_return_shapes() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");

    assert_eq!(
        knowledge.crate_meta.root_doc_sections.summary,
        "Fixture crate used by `s3-extract` end-to-end tests."
    );

    let example_type = knowledge
        .types
        .iter()
        .find(|ty| ty.canonical_path == "s3_audit_fixture::ExampleType")
        .expect("ExampleType should be extracted");
    assert_eq!(
        example_type.doc_sections.summary,
        "Simple public type for fixture docs."
    );
    assert!(example_type
        .doc_sections
        .examples
        .contains("ExampleType::new"));

    let wrapped = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "s3_audit_fixture::ExampleType::wrapped")
        .expect("wrapped constructor should be extracted");
    assert!(wrapped
        .doc_sections
        .errors
        .contains("Returns a fixture error when construction fails."));
    let wrapped_shape = wrapped
        .return_shape
        .as_ref()
        .expect("wrapped constructor should have a return shape");
    assert_eq!(wrapped_shape.kind, ReturnShapeKind::Result);
    assert_eq!(
        wrapped_shape.inner_types,
        vec!["Self".to_owned(), "ExampleError".to_owned()]
    );

    let value = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "s3_audit_fixture::ExampleType::value")
        .expect("value method should be extracted");
    assert_eq!(
        value
            .return_shape
            .as_ref()
            .expect("value method should have a return shape")
            .kind,
        ReturnShapeKind::Primitive
    );

    let panic_now = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "s3_audit_fixture::panic_now")
        .expect("panic_now should be extracted");
    assert!(panic_now
        .doc_sections
        .panics
        .contains("Always panics for fixture coverage."));

    let assoc_const_trait = knowledge
        .trait_registry
        .iter()
        .find(|trait_info| trait_info.canonical_path == "s3_audit_fixture::ExampleAssocConst")
        .expect("ExampleAssocConst trait should be extracted");
    assert!(assoc_const_trait
        .doc_sections
        .examples
        .contains("ExampleAssocConst"));

    let sample_const = knowledge
        .symbols
        .iter()
        .find(|symbol| symbol.canonical_path == "s3_audit_fixture::SAMPLE_CONST")
        .expect("SAMPLE_CONST should be extracted");
    assert!(sample_const.doc_sections.examples.contains("SAMPLE_CONST"));

    let assoc_const = knowledge
        .symbols
        .iter()
        .find(|symbol| symbol.canonical_path == "s3_audit_fixture::ExampleType::DEFAULT_LABEL")
        .expect("ExampleType::DEFAULT_LABEL should be extracted");
    assert!(assoc_const.doc_sections.examples.contains("DEFAULT_LABEL"));
}

#[test]
fn extract_real_semver_includes_public_inherent_associated_consts() {
    let knowledge =
        s3_extract::extract_knowledge_from_manifest("/tmp/seraph-real-crates/semver/Cargo.toml")
            .expect("semver manifest should extract successfully");

    let version_req_star = knowledge
        .symbols
        .iter()
        .find(|symbol| symbol.canonical_path == "semver::VersionReq::STAR")
        .expect("VersionReq::STAR should be extracted into symbols");
    assert_eq!(version_req_star.symbol_kind, SymbolKind::AssociatedConstant);
    assert_eq!(
        version_req_star
            .owner_type_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("type::semver::VersionReq")
    );
    assert_eq!(version_req_star.type_text.as_deref(), Some("Self"));
    assert!(version_req_star.docs.contains("VersionReq"));

    let build_metadata_empty = knowledge
        .symbols
        .iter()
        .find(|symbol| symbol.canonical_path == "semver::BuildMetadata::EMPTY")
        .expect("BuildMetadata::EMPTY should be extracted into symbols");
    assert_eq!(
        build_metadata_empty
            .owner_type_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("type::semver::BuildMetadata")
    );
    assert_eq!(build_metadata_empty.type_text.as_deref(), Some("Self"));
}

#[test]
fn extract_real_semver_root_doc_sections_skip_badges_and_capture_singular_example() {
    let knowledge =
        s3_extract::extract_knowledge_from_manifest("/tmp/seraph-real-crates/semver/Cargo.toml")
            .expect("semver manifest should extract successfully");

    assert_eq!(
        knowledge.crate_meta.root_doc_sections.summary,
        "A parser and evaluator for Cargo's flavor of Semantic Versioning."
    );
    assert!(knowledge
        .crate_meta
        .root_doc_sections
        .examples
        .contains("VersionReq::parse"));
}

#[test]
fn extract_real_http_api_doc_sections_capture_singular_example_heading() {
    let knowledge =
        s3_extract::extract_knowledge_from_manifest("/tmp/seraph-real-crates/http/Cargo.toml")
            .expect("http manifest should extract successfully");

    let get_mut = knowledge
        .apis
        .iter()
        .find(|api| api.canonical_path == "http::extensions::Extensions::get_mut")
        .expect("Extensions::get_mut should be extracted");
    assert!(get_mut
        .doc_sections
        .examples
        .contains("ext.get_mut::<String>()"));
}

#[test]
fn extract_fixture_populates_type_surface_and_assoc_const_surface() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");

    let record = knowledge
        .types
        .iter()
        .find(|ty| ty.canonical_path == "s3_audit_fixture::ExampleRecord")
        .expect("ExampleRecord should be extracted");
    assert!(record.has_hidden_fields);
    assert_eq!(record.fields.len(), 1);
    assert_eq!(record.fields[0].name.as_deref(), Some("name"));
    assert_eq!(record.fields[0].type_text, "&'static str");

    let example_enum = knowledge
        .types
        .iter()
        .find(|ty| ty.canonical_path == "s3_audit_fixture::ExampleEnum")
        .expect("ExampleEnum should be extracted");
    assert!(example_enum.is_non_exhaustive);
    assert!(!example_enum.has_hidden_variants);
    assert_eq!(example_enum.variants.len(), 3);

    let unit_variant = example_enum
        .variants
        .iter()
        .find(|variant| variant.name == "Unit")
        .expect("unit variant should be extracted");
    assert_eq!(unit_variant.kind, VariantKind::Unit);

    let tuple_variant = example_enum
        .variants
        .iter()
        .find(|variant| variant.name == "Tuple")
        .expect("tuple variant should be extracted");
    assert_eq!(tuple_variant.kind, VariantKind::Tuple);
    assert_eq!(tuple_variant.fields.len(), 1);
    assert_eq!(tuple_variant.fields[0].name, None);
    assert_eq!(tuple_variant.fields[0].type_text, "usize");

    let struct_variant = example_enum
        .variants
        .iter()
        .find(|variant| variant.name == "Struct")
        .expect("struct variant should be extracted");
    assert_eq!(struct_variant.kind, VariantKind::Struct);
    assert!(struct_variant.is_non_exhaustive);
    assert_eq!(struct_variant.fields.len(), 1);
    assert_eq!(struct_variant.fields[0].name.as_deref(), Some("label"));
    assert_eq!(struct_variant.fields[0].type_text, "&'static str");

    let assoc_const_trait = knowledge
        .trait_registry
        .iter()
        .find(|trait_info| trait_info.canonical_path == "s3_audit_fixture::ExampleAssocConst")
        .expect("ExampleAssocConst trait should be extracted");
    assert_eq!(assoc_const_trait.associated_const_defs.len(), 2);
    assert!(assoc_const_trait
        .associated_const_defs
        .iter()
        .any(|assoc_const| {
            assoc_const.name == "LABEL"
                && assoc_const.type_text == "&'static str"
                && assoc_const.default_value_text.is_none()
        }));
    assert!(assoc_const_trait
        .associated_const_defs
        .iter()
        .any(|assoc_const| {
            assoc_const.name == "DEFAULT_LIMIT"
                && assoc_const.type_text == "usize"
                && assoc_const.default_value_text.as_deref() == Some("7")
        }));

    let assoc_const_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "s3_audit_fixture::ExampleAssocConst"
                && trait_impl.for_type_text == "ExampleType"
        })
        .expect("ExampleAssocConst impl for ExampleType should be extracted");
    assert_eq!(assoc_const_impl.associated_const_bindings.len(), 1);
    assert_eq!(assoc_const_impl.associated_const_bindings[0].name, "LABEL");
    assert!(assoc_const_impl.associated_const_bindings[0]
        .value_text
        .as_deref()
        .is_some_and(|value| value.contains("fixture")));
    assert!(assoc_const_impl
        .cfg_attrs
        .iter()
        .any(|attr| attr.contains("cfg") && attr.contains("unix")));
}

#[test]
fn extract_fixture_populates_borrowed_return_and_explicit_panic_facts() {
    let knowledge = s3_extract::extract_knowledge_from_manifest(audit_fixture_manifest_path())
        .expect("fixture manifest should extract successfully");

    let holder_borrow = knowledge
        .risk_facts
        .borrowed_return_apis
        .iter()
        .find(|fact| fact.api_id.as_str() == "api::s3_audit_fixture::ExampleHolder::as_str")
        .expect("ExampleHolder::as_str should be recorded as a borrowed return");
    assert!(holder_borrow.from_self);
    assert!(holder_borrow.from_arg_positions.is_empty());
    assert!(holder_borrow.lifetime_names.is_empty());
    assert_eq!(holder_borrow.return_type_text, "&str");

    let echo_borrow = knowledge
        .risk_facts
        .borrowed_return_apis
        .iter()
        .find(|fact| fact.api_id.as_str() == "api::s3_audit_fixture::echo_str")
        .expect("echo_str should be recorded as a borrowed return");
    assert!(!echo_borrow.from_self);
    assert_eq!(echo_borrow.from_arg_positions, vec![0]);
    assert!(echo_borrow.lifetime_names.is_empty());
    assert_eq!(echo_borrow.return_type_text, "&str");

    let panic_api = knowledge
        .risk_facts
        .explicit_panic_sites
        .iter()
        .find(|fact| matches!(&fact.owner, RiskOwner::Api(api_id) if api_id.as_str() == "api::s3_audit_fixture::panic_now"))
        .expect("panic_now should produce an explicit panic fact");
    assert_eq!(panic_api.panic_kind, "panic!");

    let drop_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "core::ops::drop::Drop"
                && trait_impl.target_type_id.as_str() == "type::s3_audit_fixture::ExampleDropPanic"
        })
        .expect("Drop impl for ExampleDropPanic should be extracted");
    let drop_panic = knowledge
        .risk_facts
        .explicit_panic_sites
        .iter()
        .find(|fact| {
            matches!(
                &fact.owner,
                RiskOwner::TraitImpl(trait_impl_id)
                    if trait_impl_id == &drop_impl.trait_impl_id
            )
        })
        .expect("panic in Drop impl should produce an explicit panic fact");
    assert_eq!(drop_panic.panic_kind, "panic!");
}
