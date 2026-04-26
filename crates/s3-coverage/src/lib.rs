#![forbid(unsafe_code)]

use seraph_types::{
    ApiCoverageStatus, ApiId, CoverageState, FailedAttempt, HarnessRecord, NextPriorityItem,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
struct ContextApiSet {
    target_api_id: ApiId,
    related_candidates: Vec<StaticApiCandidate>,
}

#[derive(Debug, Clone)]
struct StaticApiCandidate {
    api_id: ApiId,
    canonical_path: String,
    owner_and_name: Option<String>,
    method_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageUpdateResult {
    pub target_api_id: ApiId,
    pub status: ApiCoverageStatus,
    pub state: CoverageState,
}

pub fn write_coverage_from_phase3(
    coverage_path: &Path,
    context_path: &Path,
    compile_index_path: Option<&Path>,
    fix_loop_index_path: Option<&Path>,
    smoke_index_path: Option<&Path>,
    post_smoke_index_path: Option<&Path>,
) -> Result<CoverageUpdateResult, String> {
    let context_api_set =
        extract_context_api_set(&fs::read_to_string(context_path).map_err(|error| {
            format!("failed to read context {}: {error}", context_path.display())
        })?)?;
    let target_api_id = context_api_set.target_api_id.clone();
    let mut state = load_or_default(coverage_path)?;
    clear_target_state(&mut state, &target_api_id);
    state.related_api_ids_by_target.insert(
        target_api_id.clone(),
        context_api_set
            .related_candidates
            .iter()
            .map(|candidate| candidate.api_id.clone())
            .collect(),
    );
    let outcome = determine_outcome(compile_index_path, fix_loop_index_path)?;
    let runtime_outcome = determine_runtime_outcome(smoke_index_path, post_smoke_index_path)?;

    if !state.total_api_ids.contains(&target_api_id) {
        state.total_api_ids.push(target_api_id.clone());
    }

    match outcome {
        RoundOutcome::Validated => {
            state
                .api_status
                .insert(target_api_id.clone(), ApiCoverageStatus::Validated);
            state.failed_attempts.remove(&target_api_id);
            state
                .exhausted_api_ids
                .retain(|api_id| api_id != &target_api_id);
        }
        RoundOutcome::Attempted { reason, exhausted } => {
            let current = state.api_status.get(&target_api_id).cloned();
            if !matches!(current, Some(ApiCoverageStatus::Validated)) {
                state
                    .api_status
                    .insert(target_api_id.clone(), ApiCoverageStatus::Attempted);
                let failed = state
                    .failed_attempts
                    .entry(target_api_id.clone())
                    .or_insert(FailedAttempt {
                        compile_fails: 0,
                        misuse_fails: 0,
                        last_reason: String::new(),
                    });
                failed.compile_fails += 1;
                failed.last_reason = reason;
                if exhausted && !state.exhausted_api_ids.contains(&target_api_id) {
                    state.exhausted_api_ids.push(target_api_id.clone());
                }
            }
        }
        RoundOutcome::Targeted => {
            state
                .api_status
                .entry(target_api_id.clone())
                .or_insert(ApiCoverageStatus::Targeted);
        }
    }

    match runtime_outcome {
        RuntimeOutcome::None => {}
        RuntimeOutcome::Accepted => {
            let target = target_api_id.as_str().to_string();
            state.found_bugs.retain(|item| item != &target);
            state.needs_review.retain(|item| item != &target);
        }
        RuntimeOutcome::FoundBug => {
            let target = target_api_id.as_str().to_string();
            ensure_string_present(&mut state.found_bugs, &target);
            state.needs_review.retain(|item| item != &target);
        }
        RuntimeOutcome::NeedsReview => {
            let target = target_api_id.as_str().to_string();
            if !state.found_bugs.iter().any(|item| item == &target) {
                ensure_string_present(&mut state.needs_review, &target);
            }
        }
        RuntimeOutcome::ReviewOverride => {
            let target = target_api_id.as_str().to_string();
            state.found_bugs.retain(|item| item != &target);
            ensure_string_present(&mut state.needs_review, &target);
        }
    }

    update_harness_records(
        &mut state,
        &target_api_id,
        &context_api_set.related_candidates,
        compile_index_path,
        fix_loop_index_path,
        smoke_index_path,
        post_smoke_index_path,
    )?;

    recompute_derived_fields(&mut state);
    fs::write(
        coverage_path,
        serde_json::to_string_pretty(&state)
            .map_err(|error| format!("failed to serialize coverage: {error}"))?
            + "\n",
    )
    .map_err(|error| {
        format!(
            "failed to write coverage {}: {error}",
            coverage_path.display()
        )
    })?;

    let status = state
        .api_status
        .get(&target_api_id)
        .cloned()
        .unwrap_or(ApiCoverageStatus::Targeted);
    Ok(CoverageUpdateResult {
        target_api_id,
        status,
        state,
    })
}

fn load_or_default(path: &Path) -> Result<CoverageState, String> {
    if !path.exists() {
        return Ok(CoverageState {
            total_api_ids: Vec::new(),
            api_status: Default::default(),
            failed_attempts: Default::default(),
            covered_api_ids: Vec::new(),
            exhausted_api_ids: Vec::new(),
            uncovered_api_ids: Vec::new(),
            related_total_api_ids: Vec::new(),
            related_api_ids_by_target: Default::default(),
            related_covered_api_ids: Vec::new(),
            related_uncovered_api_ids: Vec::new(),
            harnesses: Default::default(),
            found_bugs: Vec::new(),
            needs_review: Vec::new(),
            next_priority: Vec::new(),
            coverage_rate: 0.0,
            related_coverage_rate: 0.0,
        });
    }
    serde_json::from_str(
        &fs::read_to_string(path)
            .map_err(|error| format!("failed to read coverage {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("failed to parse coverage {}: {error}", path.display()))
}

fn extract_context_api_set(context: &str) -> Result<ContextApiSet, String> {
    let mut target_api_id = None;
    let mut related_candidates = Vec::new();
    let mut section = "";

    for line in context.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("## ") {
            section = value.trim();
            continue;
        }
        if section == "Target API" {
            if let Some(value) = trimmed.strip_prefix("- api_id:") {
                let value = value.trim();
                if !value.is_empty() {
                    target_api_id = Some(ApiId::from(value));
                }
            }
            continue;
        }
        if section != "Required Setup APIs"
            && section != "Known Reachable Paths"
            && section != "Related APIs"
        {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("- ") else {
            continue;
        };
        let Some((api_id, detail)) = rest.split_once(": ") else {
            continue;
        };
        if !api_id.starts_with("api::") {
            continue;
        }
        if let Some(candidate) = build_static_api_candidate(ApiId::from(api_id), detail) {
            related_candidates.push(candidate);
        }
    }

    let Some(target_api_id) = target_api_id else {
        return Err("context missing target api_id".to_string());
    };
    related_candidates.retain(|candidate| candidate.api_id != target_api_id);
    Ok(ContextApiSet {
        target_api_id,
        related_candidates,
    })
}

enum RoundOutcome {
    Targeted,
    Attempted { reason: String, exhausted: bool },
    Validated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeOutcome {
    None,
    Accepted,
    FoundBug,
    NeedsReview,
    ReviewOverride,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HarnessIdentity {
    round: u32,
    sub_index: u32,
    attempt: Option<u32>,
}

fn determine_outcome(
    compile_index_path: Option<&Path>,
    fix_loop_index_path: Option<&Path>,
) -> Result<RoundOutcome, String> {
    if let Some(path) = fix_loop_index_path.filter(|path| path.exists()) {
        let value = read_json(path)?;
        if value.get("status").and_then(Value::as_str) == Some("ok") {
            return Ok(RoundOutcome::Validated);
        }
        let reason = value
            .get("loops")
            .and_then(Value::as_array)
            .and_then(|loops| loops.first())
            .and_then(|loop_item| loop_item.get("stop_reason"))
            .and_then(Value::as_str)
            .unwrap_or("fix_loop_failed")
            .to_string();
        return Ok(RoundOutcome::Attempted {
            reason,
            exhausted: true,
        });
    }
    if let Some(path) = compile_index_path.filter(|path| path.exists()) {
        let value = read_json(path)?;
        if value.get("status").and_then(Value::as_str) == Some("ok") {
            return Ok(RoundOutcome::Validated);
        }
        return Ok(RoundOutcome::Attempted {
            reason: "compile_failed".to_string(),
            exhausted: false,
        });
    }
    Ok(RoundOutcome::Targeted)
}

fn determine_runtime_outcome(
    smoke_index_path: Option<&Path>,
    post_smoke_index_path: Option<&Path>,
) -> Result<RuntimeOutcome, String> {
    if let Some(path) = post_smoke_index_path.filter(|path| path.exists()) {
        let value = read_json(path)?;
        if looks_like_runtime_error_index(&value) {
            let has_bug = value
                .get("reports")
                .and_then(Value::as_array)
                .map(|reports| {
                    reports.iter().any(|entry| {
                        entry.get("bug").and_then(Value::as_bool) == Some(true)
                            || entry.get("status").and_then(Value::as_str) == Some("bug")
                            || entry.get("classification").and_then(Value::as_str)
                                == Some("asan_bug")
                    })
                })
                .unwrap_or(false);
            if has_bug || value.get("status").and_then(Value::as_str) == Some("bug") {
                return Ok(RuntimeOutcome::FoundBug);
            }
            return Ok(RuntimeOutcome::None);
        }
        let mut has_review = false;
        if let Some(entries) = value.get("entries").and_then(Value::as_array) {
            for entry in entries {
                match entry.get("status").and_then(Value::as_str) {
                    Some("bug") => return Ok(RuntimeOutcome::FoundBug),
                    Some("needs_review") => has_review = true,
                    Some("accepted") => {}
                    _ => {}
                }
            }
        }
        return match value.get("status").and_then(Value::as_str) {
            Some("bug") => Ok(RuntimeOutcome::FoundBug),
            Some("needs_review") => Ok(RuntimeOutcome::ReviewOverride),
            Some("accepted") => Ok(RuntimeOutcome::Accepted),
            _ if has_review => Ok(RuntimeOutcome::ReviewOverride),
            _ => Ok(RuntimeOutcome::None),
        };
    }
    let Some(path) = smoke_index_path.filter(|path| path.exists()) else {
        return Ok(RuntimeOutcome::None);
    };
    let value = read_json(path)?;
    let mut has_review = false;
    if let Some(reports) = value.get("reports").and_then(Value::as_array) {
        for report in reports {
            match report.get("status").and_then(Value::as_str) {
                Some("bug") => return Ok(RuntimeOutcome::FoundBug),
                Some("review") => has_review = true,
                _ => {}
            }
        }
    }
    match value.get("status").and_then(Value::as_str) {
        Some("bug") => Ok(RuntimeOutcome::FoundBug),
        Some("review") => Ok(RuntimeOutcome::NeedsReview),
        _ if has_review => Ok(RuntimeOutcome::NeedsReview),
        _ => Ok(RuntimeOutcome::None),
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_str(
        &fs::read_to_string(path)
            .map_err(|error| format!("failed to read json {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("failed to parse json {}: {error}", path.display()))
}

fn recompute_derived_fields(state: &mut CoverageState) {
    let total: BTreeSet<ApiId> = state.total_api_ids.iter().cloned().collect();
    let covered: BTreeSet<ApiId> = state
        .api_status
        .iter()
        .filter_map(|(api_id, status)| {
            if matches!(status, ApiCoverageStatus::Validated) {
                Some(api_id.clone())
            } else {
                None
            }
        })
        .collect();
    let exhausted: BTreeSet<ApiId> = state
        .exhausted_api_ids
        .iter()
        .filter(|api_id| total.contains(*api_id) && !covered.contains(*api_id))
        .cloned()
        .collect();
    let uncovered: BTreeSet<ApiId> = total.difference(&covered).cloned().collect();

    state
        .failed_attempts
        .retain(|api_id, _| !covered.contains(api_id));
    state.total_api_ids = total.iter().cloned().collect();
    state.covered_api_ids = covered.iter().cloned().collect();
    state.exhausted_api_ids = exhausted.iter().cloned().collect();
    state.uncovered_api_ids = uncovered.iter().cloned().collect();
    state.next_priority = uncovered
        .iter()
        .filter(|api_id| !exhausted.contains(*api_id))
        .map(|api_id| NextPriorityItem {
            api_id: api_id.clone(),
            reason: state
                .failed_attempts
                .get(api_id)
                .map(|attempt| attempt.last_reason.clone())
                .unwrap_or_else(|| "not validated".to_string()),
        })
        .collect();
    state.coverage_rate = if total.is_empty() {
        0.0
    } else {
        covered.len() as f64 / total.len() as f64
    };

    let related_total: BTreeSet<ApiId> = if state.related_api_ids_by_target.is_empty() {
        state.related_total_api_ids.iter().cloned().collect()
    } else {
        state
            .related_api_ids_by_target
            .values()
            .flat_map(|api_ids| api_ids.iter().cloned())
            .collect()
    };
    let related_covered: BTreeSet<ApiId> = state
        .harnesses
        .values()
        .filter(|record| matches!(record.status, ApiCoverageStatus::Validated))
        .flat_map(|record| record.api_ids.iter().cloned())
        .filter(|api_id| related_total.contains(api_id))
        .collect();
    let related_uncovered: BTreeSet<ApiId> = related_total
        .difference(&related_covered)
        .cloned()
        .collect();
    state.related_total_api_ids = related_total.iter().cloned().collect();
    state.related_covered_api_ids = related_covered.iter().cloned().collect();
    state.related_uncovered_api_ids = related_uncovered.iter().cloned().collect();
    state.related_coverage_rate = if related_total.is_empty() {
        0.0
    } else {
        related_covered.len() as f64 / related_total.len() as f64
    };
}

fn ensure_string_present(items: &mut Vec<String>, value: &str) {
    if !items.iter().any(|item| item == value) {
        items.push(value.to_string());
    }
}

fn update_harness_records(
    state: &mut CoverageState,
    target_api_id: &ApiId,
    related_candidates: &[StaticApiCandidate],
    compile_index_path: Option<&Path>,
    fix_loop_index_path: Option<&Path>,
    smoke_index_path: Option<&Path>,
    post_smoke_index_path: Option<&Path>,
) -> Result<(), String> {
    if let Some(path) = compile_index_path.filter(|path| path.exists()) {
        let value = read_json(path)?;
        if let Some(reports) = value.get("reports").and_then(Value::as_array) {
            for report in reports {
                upsert_compile_record(state, target_api_id, related_candidates, report)?;
            }
        }
    }

    if let Some(path) = fix_loop_index_path.filter(|path| path.exists()) {
        let value = read_json(path)?;
        if let Some(loops) = value.get("loops").and_then(Value::as_array) {
            for loop_item in loops {
                let Some(detail_path) = loop_item.get("report").and_then(Value::as_str) else {
                    continue;
                };
                let detail = read_json(Path::new(detail_path))?;
                if let Some(attempts) = detail.get("attempts").and_then(Value::as_array) {
                    for attempt in attempts {
                        upsert_compile_record(state, target_api_id, related_candidates, attempt)?;
                    }
                }
            }
        }
    }

    if let Some(path) = smoke_index_path.filter(|path| path.exists()) {
        let value = read_json(path)?;
        if let Some(reports) = value.get("reports").and_then(Value::as_array) {
            for report in reports {
                upsert_smoke_record(state, target_api_id, report)?;
            }
        }
    }

    if let Some(path) = post_smoke_index_path.filter(|path| path.exists()) {
        let value = read_json(path)?;
        if looks_like_runtime_error_index(&value) {
            if let Some(reports) = value.get("reports").and_then(Value::as_array) {
                for report in reports {
                    upsert_runtime_error_record(state, target_api_id, report)?;
                }
            }
        } else if let Some(entries) = value.get("entries").and_then(Value::as_array) {
            for entry in entries {
                upsert_acceptance_record(state, target_api_id, path, entry)?;
            }
        }
    }

    Ok(())
}

fn upsert_compile_record(
    state: &mut CoverageState,
    target_api_id: &ApiId,
    related_candidates: &[StaticApiCandidate],
    report: &Value,
) -> Result<(), String> {
    let Some(harness_path) = report.get("harness").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(identity) = parse_harness_identity(harness_path) else {
        return Ok(());
    };
    let key = harness_key(&identity);
    let compile_status = match report.get("status").and_then(Value::as_str) {
        Some("ok") => ApiCoverageStatus::Validated,
        _ => ApiCoverageStatus::Attempted,
    };
    let compile_validated = matches!(compile_status, ApiCoverageStatus::Validated);
    let entry = state
        .harnesses
        .entry(key)
        .or_insert_with(|| default_harness_record(&identity, target_api_id));
    entry.status = compile_status;
    ensure_api_id_present(&mut entry.api_ids, target_api_id);
    if compile_validated {
        for api_id in detect_static_related_api_ids(harness_path, related_candidates)? {
            ensure_api_id_present(&mut entry.api_ids, &api_id);
        }
    }
    entry.harness_path = Some(harness_path.to_string());
    if let Some(report_path) = report.get("report").and_then(Value::as_str) {
        entry.compile_report_path = Some(report_path.to_string());
    }
    Ok(())
}

fn upsert_smoke_record(
    state: &mut CoverageState,
    target_api_id: &ApiId,
    report: &Value,
) -> Result<(), String> {
    let Some(harness_path) = report.get("harness").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(identity) = parse_harness_identity(harness_path) else {
        return Ok(());
    };
    let key = harness_key(&identity);
    let entry = state.harnesses.entry(key).or_insert_with(|| {
        let mut record = default_harness_record(&identity, target_api_id);
        record.status = ApiCoverageStatus::Validated;
        record
    });
    ensure_api_id_present(&mut entry.api_ids, target_api_id);
    entry.harness_path = Some(harness_path.to_string());
    if let Some(report_path) = report.get("report").and_then(Value::as_str) {
        entry.smoke_report_path = Some(report_path.to_string());
    }
    entry.runtime_status = report
        .get("status")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    entry.runtime_classification = report
        .get("classification")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    Ok(())
}

fn upsert_acceptance_record(
    state: &mut CoverageState,
    target_api_id: &ApiId,
    acceptance_index_path: &Path,
    entry: &Value,
) -> Result<(), String> {
    let Some(harness_path) = entry.get("harness").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(identity) = parse_harness_identity(harness_path) else {
        return Ok(());
    };
    let key = harness_key(&identity);
    let record = state
        .harnesses
        .entry(key)
        .or_insert_with(|| default_harness_record(&identity, target_api_id));
    ensure_api_id_present(&mut record.api_ids, target_api_id);
    record.harness_path = Some(harness_path.to_string());
    if let Some(report_path) = entry.get("compile_report").and_then(Value::as_str) {
        record.compile_report_path = Some(report_path.to_string());
    }
    if let Some(report_path) = entry.get("smoke_report").and_then(Value::as_str) {
        record.smoke_report_path = Some(report_path.to_string());
    }
    record.review_status = entry
        .get("status")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    record.review_reason = entry
        .get("reason")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    record.review_report_path = Some(acceptance_index_path.display().to_string());
    Ok(())
}

fn upsert_runtime_error_record(
    state: &mut CoverageState,
    target_api_id: &ApiId,
    entry: &Value,
) -> Result<(), String> {
    let Some(harness_path) = entry.get("harness").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(identity) = parse_harness_identity(harness_path) else {
        return Ok(());
    };
    let key = harness_key(&identity);
    let record = state
        .harnesses
        .entry(key)
        .or_insert_with(|| default_harness_record(&identity, target_api_id));
    ensure_api_id_present(&mut record.api_ids, target_api_id);
    record.harness_path = Some(harness_path.to_string());
    record.runtime_status = entry
        .get("status")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    record.runtime_classification = entry
        .get("classification")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    record.runtime_error_report_path = entry
        .get("report")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    record.runtime_error_summary = entry
        .get("summary")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    Ok(())
}

fn looks_like_runtime_error_index(value: &Value) -> bool {
    value.get("reports").is_some() && value.get("entries").is_none()
}

fn default_harness_record(identity: &HarnessIdentity, target_api_id: &ApiId) -> HarnessRecord {
    HarnessRecord {
        round: identity.round,
        sub_index: identity.sub_index,
        attempt: identity.attempt,
        target_api_id: Some(target_api_id.clone()),
        api_ids: vec![target_api_id.clone()],
        status: ApiCoverageStatus::Attempted,
        harness_path: None,
        compile_report_path: None,
        smoke_report_path: None,
        runtime_status: None,
        runtime_classification: None,
        runtime_error_report_path: None,
        runtime_error_summary: None,
        review_status: None,
        review_reason: None,
        review_report_path: None,
        scenario_id: None,
        mapping_id: None,
        plan_id: None,
    }
}

fn clear_target_state(state: &mut CoverageState, target_api_id: &ApiId) {
    let target = target_api_id.as_str().to_string();
    state.api_status.remove(target_api_id);
    state.failed_attempts.remove(target_api_id);
    state
        .exhausted_api_ids
        .retain(|api_id| api_id != target_api_id);
    state
        .covered_api_ids
        .retain(|api_id| api_id != target_api_id);
    state
        .uncovered_api_ids
        .retain(|api_id| api_id != target_api_id);
    state.found_bugs.retain(|item| item != &target);
    state.needs_review.retain(|item| item != &target);
    state
        .next_priority
        .retain(|item| &item.api_id != target_api_id);
    state
        .harnesses
        .retain(|_, record| !harness_record_belongs_to_target(record, target_api_id));
}

fn harness_record_belongs_to_target(record: &HarnessRecord, target_api_id: &ApiId) -> bool {
    if record.target_api_id.as_ref() == Some(target_api_id) {
        return true;
    }
    record.api_ids.first() == Some(target_api_id)
}

fn ensure_api_id_present(api_ids: &mut Vec<ApiId>, api_id: &ApiId) {
    if !api_ids.iter().any(|current| current == api_id) {
        api_ids.push(api_id.clone());
    }
}

fn build_static_api_candidate(api_id: ApiId, detail: &str) -> Option<StaticApiCandidate> {
    let canonical_path = detail.split(" — ").next()?.trim().to_string();
    let method_name = canonical_path.rsplit("::").next()?.trim().to_string();
    if method_name.is_empty() {
        return None;
    }
    let owner_and_name = canonical_path
        .rsplit_once("::")
        .and_then(|(owner_path, name)| owner_path.rsplit("::").next().map(|owner| (owner, name)))
        .map(|(owner, name)| format!("{owner}::{name}"));
    Some(StaticApiCandidate {
        api_id,
        canonical_path,
        owner_and_name,
        method_name,
    })
}

fn detect_static_related_api_ids(
    harness_path: &str,
    related_candidates: &[StaticApiCandidate],
) -> Result<Vec<ApiId>, String> {
    if related_candidates.is_empty() {
        return Ok(Vec::new());
    }
    let source = fs::read_to_string(harness_path)
        .map_err(|error| format!("failed to read harness {}: {error}", harness_path))?;
    let mut matches = Vec::new();
    for candidate in related_candidates {
        if static_candidate_called(candidate, &source, related_candidates) {
            matches.push(candidate.api_id.clone());
        }
    }
    Ok(matches)
}

fn static_candidate_called(
    candidate: &StaticApiCandidate,
    source: &str,
    related_candidates: &[StaticApiCandidate],
) -> bool {
    if source.contains(&format!("{}(", candidate.canonical_path)) {
        return true;
    }
    if let Some(owner_and_name) = &candidate.owner_and_name {
        if source.contains(&format!("{}(", owner_and_name)) {
            return true;
        }
    }
    let same_name_count = related_candidates
        .iter()
        .filter(|other| other.method_name == candidate.method_name)
        .count();
    same_name_count == 1 && source.contains(&format!(".{}(", candidate.method_name))
}

fn parse_harness_identity(harness_path: &str) -> Option<HarnessIdentity> {
    let stem = Path::new(harness_path).file_stem()?.to_str()?;
    let parts: Vec<&str> = stem.split('_').collect();
    match parts.as_slice() {
        ["harness", round, sub_index] => Some(HarnessIdentity {
            round: round.parse().ok()?,
            sub_index: sub_index.parse().ok()?,
            attempt: None,
        }),
        ["harness", round, sub_index, "fixed", attempt] => Some(HarnessIdentity {
            round: round.parse().ok()?,
            sub_index: sub_index.parse().ok()?,
            attempt: Some(attempt.parse().ok()?),
        }),
        _ => None,
    }
}

fn harness_key(identity: &HarnessIdentity) -> String {
    match identity.attempt {
        Some(attempt) => format!(
            "{}:{}:fixed:{}",
            identity.round, identity.sub_index, attempt
        ),
        None => format!("{}:{}", identity.round, identity.sub_index),
    }
}
