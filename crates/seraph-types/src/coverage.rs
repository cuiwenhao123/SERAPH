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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_api_id: Option<ApiId>,
    pub api_ids: Vec<ApiId>,
    pub status: ApiCoverageStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compile_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smoke_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_classification: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_error_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_error_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_id: Option<ScenarioId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping_id: Option<MappingId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<PlanId>,
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
    #[serde(default)]
    pub related_total_api_ids: Vec<ApiId>,
    #[serde(default)]
    pub related_api_ids_by_target: BTreeMap<ApiId, Vec<ApiId>>,
    #[serde(default)]
    pub related_covered_api_ids: Vec<ApiId>,
    #[serde(default)]
    pub related_uncovered_api_ids: Vec<ApiId>,
    pub harnesses: BTreeMap<String, HarnessRecord>,
    pub found_bugs: Vec<String>,
    pub needs_review: Vec<String>,
    pub next_priority: Vec<NextPriorityItem>,
    pub coverage_rate: f64,
    #[serde(default)]
    pub related_coverage_rate: f64,
}
