use crate::{ApiId, CapId, MappingId, PlanId, ScenarioId, ScenarioType, TraitId, TypeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioArtifact {
    pub scenario_id: ScenarioId,
    pub round: u32,
    pub name: String,
    pub description: String,
    pub scenario_type: ScenarioType,
    pub selected_capability_ids: Vec<CapId>,
    pub fuzz_variation_points: Vec<String>,
    pub target_api_name_hints: Vec<String>,
    pub semantic_constraints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiMappingArtifact {
    pub mapping_id: MappingId,
    pub scenario_id: ScenarioId,
    pub scenario_name: String,
    pub scenario_description: String,
    pub selected_capability_ids: Vec<CapId>,
    pub api_mapping: Vec<ApiMappingEntry>,
    pub types_needed: Vec<TypeNeed>,
    pub targeted_api_ids: Vec<ApiId>,
    pub targeted_api_names: Vec<String>,
    pub capability_trace: BTreeMap<CapId, Vec<ApiId>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiMappingEntry {
    pub api_id: ApiId,
    pub api_path: String,
    pub capability_id: CapId,
    pub role: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeNeed {
    pub type_id: TypeId,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiPlanArtifact {
    pub plan_id: PlanId,
    pub scenario_id: ScenarioId,
    pub mapping_id: MappingId,
    pub api_ids: Vec<ApiId>,
    pub types_needed: Vec<TypeId>,
    pub required_trait_ids: Vec<TraitId>,
    pub stateful_type_ids: Vec<TypeId>,
    pub ordered_steps: Vec<OrderedStep>,
    pub type_synthesis: BTreeMap<String, TypeSynthesisSpec>,
    pub invariants: Vec<String>,
    pub codegen_constraints: CodegenConstraints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderedStep {
    pub step_no: u32,
    pub api_id: ApiId,
    pub purpose: String,
    pub state_before: Option<String>,
    pub state_after: Option<String>,
    pub arg_sources: BTreeMap<String, String>,
    pub preconditions: Vec<String>,
    pub result_handling: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeSynthesisSpec {
    pub strategy: String,
    pub assumption_violated: String,
    pub construction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodegenConstraints {
    pub prefer_raw_bytes: bool,
    pub inject_simplified_slm: bool,
    pub disallow_unwrap_on_option: bool,
}
