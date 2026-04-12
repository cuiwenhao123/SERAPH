use crate::{ApiId, CodeRef, ExampleId, ModuleId, TraitId, TypeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Knowledge {
    pub crate_name: String,
    pub crate_import_name: String,
    pub crate_doc: String,
    pub public_api_count: u32,
    pub default_features: Vec<String>,
    pub level_0_summary: Level0Summary,
    pub level_1_modules: Vec<ModuleInfo>,
    pub level_2_types: Vec<TypeInfo>,
    pub level_3_apis: Vec<ApiInfo>,
    pub trait_registry: Vec<TraitInfo>,
    pub examples_index: Vec<ExampleInfo>,
    pub raw_risk_surface: RawRiskSurface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level0Summary {
    pub one_line: String,
    pub key_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub module_id: ModuleId,
    pub path: String,
    pub doc_summary: String,
    pub types: Vec<ModuleTypeRef>,
    pub api_ids: Vec<ApiId>,
    pub api_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleTypeRef {
    pub type_id: TypeId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeInfo {
    pub type_id: TypeId,
    pub path: String,
    pub kind: String,
    pub constructors: Vec<ApiId>,
    pub method_ids: Vec<ApiId>,
    pub trait_impls: Vec<String>,
    pub lifecycle_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiInfo {
    pub api_id: ApiId,
    pub path: String,
    pub module_id: ModuleId,
    pub signature: String,
    pub receiver: Option<String>,
    pub generic_params: Vec<String>,
    pub where_clauses: Vec<String>,
    pub doc_full: String,
    pub related_example_ids: Vec<ExampleId>,
    pub risk_markers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitInfo {
    pub trait_id: TraitId,
    pub path: String,
    pub is_unsafe: bool,
    pub required_methods: Vec<String>,
    pub provided_methods: Vec<String>,
    pub associated_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExampleInfo {
    pub example_id: ExampleId,
    pub source_api_id: ApiId,
    pub involved_api_ids: Vec<ApiId>,
    pub code_ref: CodeRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawRiskSurface {
    pub unsafe_functions: Vec<ApiId>,
    pub ffi_boundaries: Vec<String>,
    pub panic_points: Vec<String>,
    pub repr_packed_types: Vec<TypeId>,
    pub panic_in_drop_types: Vec<TypeId>,
}
