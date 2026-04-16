use crate::{ApiId, CapId, ModuleId, RiskLevel, TraitId, TypeId};
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
    pub stage1_summary: Stage1Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityNode {
    pub cap_id: CapId,
    pub anchor_kind: CapabilityAnchorKind,
    pub anchor_module_id: ModuleId,
    pub anchor_type_id: Option<TypeId>,
    pub role: CapabilityRole,
    pub name: String,
    pub description: String,
    pub api_ids: Vec<ApiId>,
    pub entry_api_ids: Vec<ApiId>,
    pub connects_to: Vec<CapId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityAnchorKind {
    ModuleEntry,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityRole {
    Construction,
    Query,
    Mutation,
    Iteration,
    Conversion,
    Finalization,
    Ffi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage1Summary {
    pub capability_cards: Vec<Stage1CapabilityCard>,
    pub recommended_chains: Vec<Vec<CapId>>,
    pub one_liner: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage1CapabilityCard {
    pub cap_id: CapId,
    pub name: String,
    pub anchor_path: String,
    pub role: CapabilityRole,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateLifecycleModel {
    // Stable local type node that this lifecycle model describes.
    pub type_id: TypeId,
    // Canonical path copied through for easier human inspection in JSON artifacts.
    pub path: String,
    // Phase 2 only emits explicit lifecycle entries for simplified/full models.
    pub model_kind: SlmModelKind,
    // Ordered state labels used by downstream planners and summaries.
    pub states: Vec<String>,
    // Deterministic transitions derived from constructor / mutator / closer APIs.
    pub transitions: Vec<StateTransition>,
    // States where Stage 3 should prefer to exercise core behavior.
    pub fuzzable_states: Vec<String>,
    // Explicitly illegal transitions backed by docs or other extracted evidence.
    pub forbidden_transitions: Vec<ForbiddenTransition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlmModelKind {
    Full,
    Simplified,
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
    // Stable API node that this contract row describes.
    pub api_id: ApiId,
    // Canonical path copied through for easier debugging and fixture readability.
    pub path: String,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub panic_conditions: Vec<String>,
    pub error_conditions: Vec<String>,
    pub safety: Option<String>,
    // Deterministic side-effect summary derived from receiver semantics and docs.
    pub side_effects: Vec<String>,
    pub generic_constraints: Option<GenericConstraints>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericConstraints {
    pub params: Vec<GenericConstraintParam>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericConstraintParam {
    // Generic parameter name as written on the API, e.g. `R` or `T`.
    pub name: String,
    // Direct trait bounds from the API signature / where-clause.
    pub direct_bounds: Vec<String>,
    // Direct bounds plus supertrait expansion from the Phase 1 trait graph.
    pub full_bound_chain: Vec<String>,
    // Associated type requirements that are explicitly recoverable from source facts.
    pub associated_type_constraints: Vec<AssociatedTypeConstraint>,
    // True when any bound in the relevant chain is an unsafe trait.
    pub is_unsafe_trait: bool,
    // CTS strategy bucket reused by risk aggregation and later planning.
    pub strategy: String,
    // Stable severity-like label for how valuable custom instantiation is.
    pub bug_hunting_value: BugHuntingValue,
    // Human-readable but deterministic guidance assembled from the known bounds.
    pub synthesis_guidance: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssociatedTypeConstraint {
    // Best-effort stable trait node when the associated type owner is known.
    pub trait_id: Option<TraitId>,
    // Canonical path text of the trait owning the associated type.
    pub trait_path: String,
    // Associated type name such as `Item` or `Error`.
    pub associated_type_name: String,
    // Rendered bounds declared on the associated type requirement.
    pub bounds: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BugHuntingValue {
    Low,
    Medium,
    High,
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
