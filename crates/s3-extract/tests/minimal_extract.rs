use seraph_types::{ApiId, ApiKind, ExampleAnchor, Knowledge, ReprKind, TraitOrigin, TypeId};
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
        serde_json::to_string(&hash_set_bitand_impl.surface_bucket).unwrap(),
        "\"primary_container\""
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
    assert_eq!(
        serde_json::to_string(&hash_map_default_impl.surface_bucket).unwrap(),
        "\"primary_container\""
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
        serde_json::to_string(&occupied_entry_debug_impl.surface_bucket).unwrap(),
        "\"entry_or_raw_entry\""
    );

    let iter_iterator_impl = knowledge
        .trait_impl_registry
        .iter()
        .find(|trait_impl| {
            trait_impl.trait_canonical_path == "core::iter::traits::iterator::Iterator"
                && trait_impl.for_type_text == "Iter<'a, K>"
        })
        .expect("Iterator impl for set::Iter should be extracted into trait_impl_registry");
    assert_eq!(
        serde_json::to_string(&iter_iterator_impl.surface_bucket).unwrap(),
        "\"iterator_or_view\""
    );
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
        serde_json::to_string(&try_reserve_display_impl.surface_bucket).unwrap(),
        "\"error_or_hasher\""
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

    let ffi_fact = knowledge
        .risk_facts
        .ffi_apis
        .iter()
        .find(|fact| fact.api_id.as_str() == "api::s3_audit_fixture::fixture_ffi_add")
        .expect("extern API should be recorded in ffi risk facts");
    assert_eq!(ffi_fact.abi, "C");

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
}
