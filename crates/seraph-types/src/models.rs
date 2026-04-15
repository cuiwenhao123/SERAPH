use crate::{ApiId, CapId, RiskLevel, TypeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Models {
    pub fcg: FunctionalCapabilityGraph,
    pub slm: Vec<StateLifecycleModel>,
    pub api_contracts: Vec<ApiContract>,
    pub risk_surface_map: RiskSurfaceMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionalCapabilityGraph {
    pub capabilities: Vec<CapabilityNode>,
    pub capability_chains: Vec<Vec<CapId>>,
    pub capability_api_index: BTreeMap<CapId, Vec<ApiId>>,
    pub stage1_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityNode {
    pub cap_id: CapId,
    pub name: String,
    pub description: String,
    pub api_ids: Vec<ApiId>,
    pub entry_api_ids: Vec<ApiId>,
    pub connects_to: Vec<CapId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateLifecycleModel {
    pub type_id: TypeId,
    pub path: String,
    pub states: Vec<String>,
    pub transitions: Vec<StateTransition>,
    pub fuzzable_states: Vec<String>,
    pub forbidden_transitions: Vec<ForbiddenTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub via_api_id: ApiId,
    pub preconditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForbiddenTransition {
    pub from: String,
    pub via_api_id: ApiId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiContract {
    pub api_id: ApiId,
    pub path: String,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub panic_conditions: Vec<String>,
    pub error_conditions: Vec<String>,
    pub safety: Option<String>,
    pub generic_constraints: Option<GenericConstraints>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericConstraints {
    pub params: Vec<GenericConstraintParam>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericConstraintParam {
    pub name: String,
    pub direct_bounds: Vec<String>,
    pub strategy: String,
    pub bug_hunting_value: String,
    pub synthesis_guidance: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskSurfaceMap {
    pub api_risks: Vec<ApiRisk>,
    pub type_synthesis_overview: TypeSynthesisOverview,
    pub rust_feature_risks: Vec<RustFeatureRisk>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiRisk {
    pub api_id: ApiId,
    pub risk_level: RiskLevel,
    pub reasons: Vec<String>,
    pub recommended_fuzz_strategy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeSynthesisOverview {
    pub generic_api_count: u32,
    pub strategy_distribution: BTreeMap<String, u32>,
    pub one_liner: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustFeatureRisk {
    pub feature: String,
    pub apis_affected: Vec<ApiId>,
    pub risk: String,
}
