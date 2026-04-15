use crate::{ApiId, MappingId, PlanId, ScenarioId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiCoverageStatus {
    Targeted,
    Attempted,
    Validated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedAttempt {
    pub compile_fails: u32,
    pub misuse_fails: u32,
    pub last_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessRecord {
    pub round: u32,
    pub sub_index: u32,
    pub scenario_id: ScenarioId,
    pub mapping_id: MappingId,
    pub plan_id: PlanId,
    pub api_ids: Vec<ApiId>,
    pub status: ApiCoverageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextPriorityItem {
    pub api_id: ApiId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageState {
    pub total_api_ids: Vec<ApiId>,
    pub api_status: BTreeMap<ApiId, ApiCoverageStatus>,
    pub failed_attempts: BTreeMap<ApiId, FailedAttempt>,
    pub covered_api_ids: Vec<ApiId>,
    pub exhausted_api_ids: Vec<ApiId>,
    pub uncovered_api_ids: Vec<ApiId>,
    pub harnesses: BTreeMap<String, HarnessRecord>,
    pub found_bugs: Vec<String>,
    pub needs_review: Vec<String>,
    pub next_priority: Vec<NextPriorityItem>,
    pub coverage_rate: f64,
}
