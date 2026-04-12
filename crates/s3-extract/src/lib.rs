#![forbid(unsafe_code)]

//! Minimal Phase 1 extraction bootstrap for SERAPH.

use seraph_types::{Knowledge, Level0Summary, RawRiskSurface};
use std::fs;
use std::path::Path;

pub fn normalize_import_name(crate_name: &str) -> String {
    crate_name.replace('-', "_")
}

pub fn build_minimal_knowledge(crate_name: &str) -> Knowledge {
    Knowledge {
        crate_name: crate_name.to_owned(),
        crate_import_name: normalize_import_name(crate_name),
        crate_doc: format!("Raw extraction placeholder for `{crate_name}`"),
        public_api_count: 0,
        default_features: Vec::new(),
        level_0_summary: Level0Summary {
            one_line: format!("Placeholder extraction summary for `{crate_name}`"),
            key_types: Vec::new(),
        },
        level_1_modules: Vec::new(),
        level_2_types: Vec::new(),
        level_3_apis: Vec::new(),
        trait_registry: Vec::new(),
        examples_index: Vec::new(),
        raw_risk_surface: RawRiskSurface {
            unsafe_functions: Vec::new(),
            ffi_boundaries: Vec::new(),
            panic_points: Vec::new(),
            repr_packed_types: Vec::new(),
            panic_in_drop_types: Vec::new(),
        },
    }
}

pub fn write_knowledge_json(path: impl AsRef<Path>, knowledge: &Knowledge) -> Result<(), std::io::Error> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(knowledge)
        .expect("serializing Knowledge to JSON should not fail");
    fs::write(path, json)?;
    Ok(())
}
