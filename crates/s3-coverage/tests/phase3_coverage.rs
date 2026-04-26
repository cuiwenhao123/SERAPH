use s3_coverage::write_coverage_from_phase3;
use seraph_types::{ApiCoverageStatus, ApiId, CoverageState, FailedAttempt, HarnessRecord};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("seraph-{name}-{unique}"));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[test]
fn write_coverage_marks_target_validated_from_compile_index() {
    let root = temp_dir("coverage-validated");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("compile_001_index.json");

    fs::write(
        &context_path,
        "## Target API\n- api_id: api::fixture::danger\n",
    )
    .expect("write context");
    fs::write(&compile_index_path, "{\"status\":\"ok\",\"reports\":[]}\n")
        .expect("write compile index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        None,
        None,
    )
    .expect("update coverage");

    assert_eq!(update.target_api_id.as_str(), "api::fixture::danger");
    assert_eq!(update.status, ApiCoverageStatus::Validated);
    assert_eq!(update.state.coverage_rate, 1.0);
    assert_eq!(update.state.covered_api_ids.len(), 1);
    assert!(update.state.failed_attempts.is_empty());
    assert!(coverage_path.exists());
}

#[test]
fn write_coverage_marks_target_attempted_and_exhausted_from_fix_loop_failure() {
    let root = temp_dir("coverage-exhausted");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("compile_001_index.json");
    let fix_loop_index_path = root.join("fix_loop_001_index.json");

    fs::write(
        &context_path,
        "## Target API\n- api_id: api::fixture::danger\n",
    )
    .expect("write context");
    fs::write(
        &compile_index_path,
        "{\"status\":\"failed\",\"reports\":[]}\n",
    )
    .expect("write compile index");
    fs::write(
        &fix_loop_index_path,
        "{\"status\":\"failed\",\"loops\":[{\"stop_reason\":\"max_attempts_exhausted\"}]}\n",
    )
    .expect("write fix loop index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        Some(&fix_loop_index_path),
        None,
        None,
    )
    .expect("update coverage");

    assert_eq!(update.status, ApiCoverageStatus::Attempted);
    assert_eq!(update.state.coverage_rate, 0.0);
    assert_eq!(update.state.exhausted_api_ids.len(), 1);
    assert_eq!(
        update.state.exhausted_api_ids[0].as_str(),
        "api::fixture::danger"
    );
    assert_eq!(
        update
            .state
            .failed_attempts
            .get(&update.target_api_id)
            .expect("failed attempt")
            .last_reason,
        "max_attempts_exhausted"
    );
    assert!(update.state.next_priority.is_empty());
}

#[test]
fn write_coverage_records_found_bug_from_smoke_index() {
    let root = temp_dir("coverage-found-bug");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("compile_001_index.json");
    let smoke_index_path = root.join("smoke_001_index.json");

    fs::write(
        &context_path,
        "## Target API\n- api_id: api::fixture::danger\n",
    )
    .expect("write context");
    fs::write(&compile_index_path, "{\"status\":\"ok\",\"reports\":[]}\n")
        .expect("write compile index");
    fs::write(
        &smoke_index_path,
        "{\"status\":\"bug\",\"reports\":[{\"status\":\"bug\",\"classification\":\"panic_detected\",\"harness\":\"/tmp/harness_001_01.rs\",\"report\":\"/tmp/smoke_001_01.json\"}]}\n",
    )
    .expect("write smoke index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        Some(&smoke_index_path),
        None,
    )
    .expect("update coverage");

    assert_eq!(update.status, ApiCoverageStatus::Validated);
    assert_eq!(update.state.found_bugs, vec!["api::fixture::danger"]);
    assert!(update.state.needs_review.is_empty());
}

#[test]
fn write_coverage_records_needs_review_from_smoke_index() {
    let root = temp_dir("coverage-needs-review");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("compile_001_index.json");
    let smoke_index_path = root.join("smoke_001_index.json");

    fs::write(
        &context_path,
        "## Target API\n- api_id: api::fixture::danger\n",
    )
    .expect("write context");
    fs::write(&compile_index_path, "{\"status\":\"ok\",\"reports\":[]}\n")
        .expect("write compile index");
    fs::write(
        &smoke_index_path,
        "{\"status\":\"review\",\"reports\":[{\"status\":\"review\",\"classification\":\"needs_review\",\"harness\":\"/tmp/harness_001_01.rs\",\"report\":\"/tmp/smoke_001_01.json\"}]}\n",
    )
    .expect("write smoke index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        Some(&smoke_index_path),
        None,
    )
    .expect("update coverage");

    assert_eq!(update.status, ApiCoverageStatus::Validated);
    assert!(update.state.found_bugs.is_empty());
    assert_eq!(update.state.needs_review, vec!["api::fixture::danger"]);
}

#[test]
fn write_coverage_populates_structured_harness_records() {
    let root = temp_dir("coverage-harness-records");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("reports/compile_001_index.json");
    let fix_loop_report_path = root.join("reports/fix_loop_001_02.json");
    let fix_loop_index_path = root.join("reports/fix_loop_001_index.json");
    let smoke_index_path = root.join("reports/smoke_001_index.json");
    let fuzz_dir = root.join("fuzz");
    let reports_dir = root.join("reports");

    fs::create_dir_all(&fuzz_dir).expect("create fuzz dir");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    let harness_01 = fuzz_dir.join("harness_001_01.rs");
    let harness_02 = fuzz_dir.join("harness_001_02.rs");
    let harness_02_fixed_01 = fuzz_dir.join("harness_001_02_fixed_01.rs");
    let harness_02_fixed_02 = fuzz_dir.join("harness_001_02_fixed_02.rs");

    fs::write(
        &context_path,
        "## Target API\n- api_id: api::fixture::danger\n",
    )
    .expect("write context");
    fs::write(
        &compile_index_path,
        format!(
            "{{\"round\":1,\"status\":\"failed\",\"reports\":[{{\"harness\":\"{}\",\"report\":\"{}\",\"status\":\"ok\",\"exit_code\":0}},{{\"harness\":\"{}\",\"report\":\"{}\",\"status\":\"failed\",\"exit_code\":1}}]}}\n",
            harness_01.display(),
            reports_dir.join("compile_001_01.json").display(),
            harness_02.display(),
            reports_dir.join("compile_001_02.json").display(),
        ),
    )
    .expect("write compile index");
    fs::write(
        &fix_loop_report_path,
        format!(
            "{{\"round\":1,\"variant\":2,\"status\":\"ok\",\"stop_reason\":\"compiled\",\"successful_attempt\":2,\"attempts\":[{{\"attempt\":1,\"harness\":\"{}\",\"report\":\"{}\",\"status\":\"failed\",\"exit_code\":1}},{{\"attempt\":2,\"harness\":\"{}\",\"report\":\"{}\",\"status\":\"ok\",\"exit_code\":0}}]}}\n",
            harness_02_fixed_01.display(),
            reports_dir.join("compile_001_02_fixed_01.json").display(),
            harness_02_fixed_02.display(),
            reports_dir.join("compile_001_02_fixed_02.json").display(),
        ),
    )
    .expect("write fix loop report");
    fs::write(
        &fix_loop_index_path,
        format!(
            "{{\"status\":\"ok\",\"loops\":[{{\"request\":\"{}\",\"report\":\"{}\",\"status\":\"ok\",\"stop_reason\":\"compiled\",\"successful_attempt\":2,\"attempt_count\":2}}]}}\n",
            root.join("fixes/fix_request_001_02.json").display(),
            fix_loop_report_path.display(),
        ),
    )
    .expect("write fix loop index");
    fs::write(
        &smoke_index_path,
        format!(
            "{{\"round\":1,\"status\":\"bug\",\"reports\":[{{\"harness\":\"{}\",\"report\":\"{}\",\"status\":\"review\",\"classification\":\"needs_review\",\"exit_code\":0}},{{\"harness\":\"{}\",\"report\":\"{}\",\"status\":\"bug\",\"classification\":\"panic_detected\",\"exit_code\":101}}]}}\n",
            harness_01.display(),
            reports_dir.join("smoke_001_01.json").display(),
            harness_02_fixed_02.display(),
            reports_dir.join("smoke_001_02_fixed_02.json").display(),
        ),
    )
    .expect("write smoke index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        Some(&fix_loop_index_path),
        Some(&smoke_index_path),
        None,
    )
    .expect("update coverage");

    assert_eq!(update.state.harnesses.len(), 4);

    let original_ok = update.state.harnesses.get("1:1").expect("original ok");
    assert_eq!(original_ok.round, 1);
    assert_eq!(original_ok.sub_index, 1);
    assert_eq!(original_ok.attempt, None);
    assert_eq!(original_ok.status, ApiCoverageStatus::Validated);
    assert_eq!(original_ok.api_ids[0].as_str(), "api::fixture::danger");
    assert_eq!(original_ok.runtime_status.as_deref(), Some("review"));
    assert_eq!(
        original_ok.runtime_classification.as_deref(),
        Some("needs_review")
    );

    let original_failed = update.state.harnesses.get("1:2").expect("original failed");
    assert_eq!(original_failed.status, ApiCoverageStatus::Attempted);
    assert_eq!(original_failed.attempt, None);

    let fixed_failed = update
        .state
        .harnesses
        .get("1:2:fixed:1")
        .expect("fixed failed");
    assert_eq!(fixed_failed.status, ApiCoverageStatus::Attempted);
    assert_eq!(fixed_failed.attempt, Some(1));

    let fixed_ok = update.state.harnesses.get("1:2:fixed:2").expect("fixed ok");
    assert_eq!(fixed_ok.status, ApiCoverageStatus::Validated);
    assert_eq!(fixed_ok.attempt, Some(2));
    assert_eq!(fixed_ok.runtime_status.as_deref(), Some("bug"));
    assert_eq!(
        fixed_ok.runtime_classification.as_deref(),
        Some("panic_detected")
    );
}

#[test]
fn write_coverage_ingests_fix_acceptance_review_status() {
    let root = temp_dir("coverage-acceptance");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("reports/compile_001_index.json");
    let acceptance_index_path = root.join("reports/fix_acceptance_001_index.json");
    let harness_path = root.join("fuzz/harness_001_02_fixed_02.rs");

    fs::create_dir_all(root.join("reports")).expect("reports dir");
    fs::create_dir_all(root.join("fuzz")).expect("fuzz dir");
    fs::write(
        &context_path,
        "## Target API\n- api_id: api::fixture::danger\n",
    )
    .expect("write context");
    fs::write(
        &compile_index_path,
        "{\"round\":1,\"status\":\"ok\",\"reports\":[]}\n",
    )
    .expect("write compile index");
    fs::write(
        &acceptance_index_path,
        format!(
            "{{\"round\":1,\"status\":\"needs_review\",\"entries\":[{{\"harness\":\"{}\",\"compile_report\":\"{}\",\"smoke_report\":\"{}\",\"status\":\"needs_review\",\"reason\":\"semantic_diff_needs_review\"}}]}}\n",
            harness_path.display(),
            root.join("reports/compile_001_02_fixed_02.json").display(),
            root.join("reports/smoke_001_02_fixed_02.json").display(),
        ),
    )
    .expect("write acceptance index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        None,
        Some(&acceptance_index_path),
    )
    .expect("update coverage");

    let record = update
        .state
        .harnesses
        .get("1:2:fixed:2")
        .expect("accepted harness record");
    assert_eq!(record.review_status.as_deref(), Some("needs_review"));
    assert_eq!(
        record.review_reason.as_deref(),
        Some("semantic_diff_needs_review")
    );
    assert_eq!(
        record.review_report_path.as_deref(),
        Some(acceptance_index_path.to_str().unwrap())
    );
    assert_eq!(update.state.needs_review, vec!["api::fixture::danger"]);
}

#[test]
fn write_coverage_acceptance_accepted_clears_target_review_state() {
    let root = temp_dir("coverage-acceptance-clears-review");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let acceptance_index_path = root.join("reports/fix_acceptance_001_index.json");
    let harness_path = root.join("fuzz/harness_001_02_fixed_02.rs");

    fs::create_dir_all(root.join("reports")).expect("reports dir");
    fs::create_dir_all(root.join("fuzz")).expect("fuzz dir");
    fs::write(
        &coverage_path,
        "{\n  \"total_api_ids\": [\"api::fixture::danger\"],\n  \"api_status\": {\"api::fixture::danger\": \"validated\"},\n  \"failed_attempts\": {},\n  \"covered_api_ids\": [\"api::fixture::danger\"],\n  \"exhausted_api_ids\": [],\n  \"uncovered_api_ids\": [],\n  \"harnesses\": {},\n  \"found_bugs\": [],\n  \"needs_review\": [\"api::fixture::danger\"],\n  \"next_priority\": [],\n  \"coverage_rate\": 1.0\n}\n",
    )
    .expect("seed coverage");
    fs::write(
        &context_path,
        "## Target API\n- api_id: api::fixture::danger\n",
    )
    .expect("write context");
    fs::write(
        &acceptance_index_path,
        format!(
            "{{\"round\":1,\"status\":\"accepted\",\"entries\":[{{\"harness\":\"{}\",\"compile_report\":\"{}\",\"smoke_report\":\"{}\",\"status\":\"accepted\",\"reason\":\"manual_triage_ok\",\"source\":\"manual\"}}]}}\n",
            harness_path.display(),
            root.join("reports/compile_001_02_fixed_02.json").display(),
            root.join("reports/smoke_001_02_fixed_02.json").display(),
        ),
    )
    .expect("write acceptance index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        None,
        None,
        None,
        Some(&acceptance_index_path),
    )
    .expect("update coverage");

    assert!(update.state.needs_review.is_empty());
    let record = update
        .state
        .harnesses
        .get("1:2:fixed:2")
        .expect("accepted harness record");
    assert_eq!(record.review_status.as_deref(), Some("accepted"));
    assert_eq!(record.review_reason.as_deref(), Some("manual_triage_ok"));
}

#[test]
fn write_coverage_runtime_error_keeps_target_validated() {
    let root = temp_dir("coverage-runtime-error");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("reports/compile_001_index.json");
    let smoke_index_path = root.join("reports/smoke_001_index.json");
    let runtime_error_index_path = root.join("reports/runtime_error_001_index.json");

    fs::create_dir_all(root.join("reports")).expect("reports dir");
    fs::write(
        &context_path,
        "## Target API\n- api_id: api::fixture::danger\n",
    )
    .expect("write context");
    fs::write(
        &compile_index_path,
        "{\"round\":1,\"status\":\"ok\",\"reports\":[{\"harness\":\"/tmp/harness_001_01.rs\",\"report\":\"/tmp/compile_001_01.json\",\"status\":\"ok\",\"exit_code\":0}]}\n",
    )
    .expect("write compile index");
    fs::write(&smoke_index_path, "{\"round\":1,\"status\":\"bug\",\"reports\":[{\"harness\":\"/tmp/harness_001_01.rs\",\"report\":\"/tmp/smoke_001_01.json\",\"status\":\"bug\",\"classification\":\"panic_detected\",\"exit_code\":101}]}\n").expect("write smoke index");
    fs::write(
        &runtime_error_index_path,
        "{\"round\":1,\"status\":\"runtime_error\",\"reports\":[{\"harness\":\"/tmp/harness_001_01.rs\",\"report\":\"/tmp/runtime_error_001_01.json\",\"status\":\"runtime_error\",\"classification\":\"panic_or_crash\",\"summary\":\"panic after target\",\"bug\":false}]}\n",
    )
    .expect("write runtime error index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        Some(&smoke_index_path),
        Some(&runtime_error_index_path),
    )
    .expect("update coverage");

    assert_eq!(update.status, ApiCoverageStatus::Validated);
    assert!(update.state.found_bugs.is_empty());
    let record = update.state.harnesses.get("1:1").expect("runtime record");
    assert_eq!(record.runtime_status.as_deref(), Some("runtime_error"));
    assert_eq!(
        record.runtime_classification.as_deref(),
        Some("panic_or_crash")
    );
    assert_eq!(
        record.runtime_error_summary.as_deref(),
        Some("panic after target")
    );
}

#[test]
fn write_coverage_runtime_error_asan_marks_found_bug() {
    let root = temp_dir("coverage-runtime-asan");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("reports/compile_001_index.json");
    let runtime_error_index_path = root.join("reports/runtime_error_001_index.json");

    fs::create_dir_all(root.join("reports")).expect("reports dir");
    fs::write(
        &context_path,
        "## Target API\n- api_id: api::fixture::danger\n",
    )
    .expect("write context");
    fs::write(
        &compile_index_path,
        "{\"round\":1,\"status\":\"ok\",\"reports\":[{\"harness\":\"/tmp/harness_001_01.rs\",\"report\":\"/tmp/compile_001_01.json\",\"status\":\"ok\",\"exit_code\":0}]}\n",
    )
    .expect("write compile index");
    fs::write(
        &runtime_error_index_path,
        "{\"round\":1,\"status\":\"bug\",\"reports\":[{\"harness\":\"/tmp/harness_001_01.rs\",\"report\":\"/tmp/runtime_error_001_01.json\",\"status\":\"bug\",\"classification\":\"asan_bug\",\"summary\":\"asan hit inside target\",\"bug\":true}]}\n",
    )
    .expect("write runtime error index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        None,
        Some(&runtime_error_index_path),
    )
    .expect("update coverage");

    assert_eq!(update.status, ApiCoverageStatus::Validated);
    assert_eq!(update.state.found_bugs, vec!["api::fixture::danger"]);
}

#[test]
fn write_coverage_records_related_api_static_coverage_from_validated_harness() {
    let root = temp_dir("coverage-related-static");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_006.md");
    let compile_index_path = root.join("reports/compile_006_index.json");
    let fuzz_dir = root.join("fuzz");
    let reports_dir = root.join("reports");

    fs::create_dir_all(&fuzz_dir).expect("create fuzz dir");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    let harness_path = fuzz_dir.join("harness_006_01.rs");
    fs::write(
        &context_path,
        "\
# SERAPH RAG Harness Context

## Target API
- api_id: api::fixture::Buffer::get_unchecked

## Required Setup APIs
- api::fixture::Buffer::new: fixture::Buffer::new — fn new() -> Buffer
- api::fixture::Builder::with_capacity: fixture::Builder::with_capacity — fn with_capacity(usize) -> Builder

## Related APIs
- api::fixture::Buffer::len: fixture::Buffer::len — fn len(&Self) -> usize
- api::fixture::Buffer::is_empty: fixture::Buffer::is_empty — fn is_empty(&Self) -> bool
",
    )
    .expect("write context");
    fs::write(
        &harness_path,
        "\
use fixture::Buffer;
use fixture::Builder;

fn main() {
    let mut buffer = Buffer::new();
    let builder = Builder::with_capacity(16);
    let _ = buffer.len();
    println!(\"SERAPH_STEP_ENTER:1:api::fixture::Buffer::get_unchecked\");
    let _ = buffer.get_unchecked(0);
    println!(\"SERAPH_STEP_OK:1:api::fixture::Buffer::get_unchecked\");
    let _ = builder;
}
",
    )
    .expect("write harness");
    fs::write(
        &compile_index_path,
        format!(
            "{{\"round\":6,\"status\":\"ok\",\"reports\":[{{\"harness\":\"{}\",\"report\":\"{}\",\"status\":\"ok\",\"exit_code\":0}}]}}\n",
            harness_path.display(),
            reports_dir.join("compile_006_01.json").display(),
        ),
    )
    .expect("write compile index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        None,
        None,
    )
    .expect("update coverage");

    assert_eq!(update.status, ApiCoverageStatus::Validated);
    assert_eq!(
        update.state.related_total_api_ids,
        vec![
            ApiId::from("api::fixture::Buffer::is_empty"),
            ApiId::from("api::fixture::Buffer::len"),
            ApiId::from("api::fixture::Buffer::new"),
            ApiId::from("api::fixture::Builder::with_capacity"),
        ]
    );
    assert_eq!(
        update.state.related_covered_api_ids,
        vec![
            ApiId::from("api::fixture::Buffer::len"),
            ApiId::from("api::fixture::Buffer::new"),
            ApiId::from("api::fixture::Builder::with_capacity"),
        ]
    );
    assert_eq!(
        update.state.related_uncovered_api_ids,
        vec![ApiId::from("api::fixture::Buffer::is_empty")]
    );
    assert_eq!(update.state.related_coverage_rate, 0.75);

    let record = update.state.harnesses.get("6:1").expect("harness record");
    assert_eq!(
        record.api_ids,
        vec![
            ApiId::from("api::fixture::Buffer::get_unchecked"),
            ApiId::from("api::fixture::Buffer::new"),
            ApiId::from("api::fixture::Builder::with_capacity"),
            ApiId::from("api::fixture::Buffer::len"),
        ]
    );
}

#[test]
fn write_coverage_does_not_count_related_imports_without_static_calls() {
    let root = temp_dir("coverage-related-import-only");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_007.md");
    let compile_index_path = root.join("reports/compile_007_index.json");
    let fuzz_dir = root.join("fuzz");
    let reports_dir = root.join("reports");

    fs::create_dir_all(&fuzz_dir).expect("create fuzz dir");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    let harness_path = fuzz_dir.join("harness_007_01.rs");
    fs::write(
        &context_path,
        "\
## Target API
- api_id: api::fixture::Buffer::get_unchecked

## Required Setup APIs
- api::fixture::Buffer::new: fixture::Buffer::new — fn new() -> Buffer

## Related APIs
- api::fixture::Buffer::len: fixture::Buffer::len — fn len(&Self) -> usize
",
    )
    .expect("write context");
    fs::write(
        &harness_path,
        "\
use fixture::Buffer;

fn main() {
    let _unused: Option<Buffer> = None;
    println!(\"SERAPH_STEP_ENTER:1:api::fixture::Buffer::get_unchecked\");
    println!(\"SERAPH_STEP_OK:1:api::fixture::Buffer::get_unchecked\");
}
",
    )
    .expect("write harness");
    fs::write(
        &compile_index_path,
        format!(
            "{{\"round\":7,\"status\":\"ok\",\"reports\":[{{\"harness\":\"{}\",\"report\":\"{}\",\"status\":\"ok\",\"exit_code\":0}}]}}\n",
            harness_path.display(),
            reports_dir.join("compile_007_01.json").display(),
        ),
    )
    .expect("write compile index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        None,
        None,
    )
    .expect("update coverage");

    assert_eq!(
        update.state.related_total_api_ids,
        vec![
            ApiId::from("api::fixture::Buffer::len"),
            ApiId::from("api::fixture::Buffer::new"),
        ]
    );
    assert!(update.state.related_covered_api_ids.is_empty());
    assert_eq!(
        update.state.related_uncovered_api_ids,
        vec![
            ApiId::from("api::fixture::Buffer::len"),
            ApiId::from("api::fixture::Buffer::new"),
        ]
    );
    assert_eq!(update.state.related_coverage_rate, 0.0);

    let record = update.state.harnesses.get("7:1").expect("harness record");
    assert_eq!(
        record.api_ids,
        vec![ApiId::from("api::fixture::Buffer::get_unchecked")]
    );
}

#[test]
fn write_coverage_rerun_replaces_stale_target_state_with_latest_attempted_result() {
    let root = temp_dir("coverage-rerun-overwrite");
    let coverage_path = root.join("coverage.json");
    let context_path = root.join("rag_target_001.md");
    let compile_index_path = root.join("reports/compile_001_index.json");
    let reports_dir = root.join("reports");
    let fuzz_dir = root.join("fuzz");

    fs::create_dir_all(&reports_dir).expect("reports dir");
    fs::create_dir_all(&fuzz_dir).expect("fuzz dir");

    let target_api_id = ApiId::from("api::fixture::danger");
    let old_related = ApiId::from("api::fixture::Old::helper");
    let new_related = ApiId::from("api::fixture::New::helper");
    let old_harness_path = fuzz_dir.join("harness_001_01_fixed_01.rs");
    let current_harness_path = fuzz_dir.join("harness_001_01.rs");

    let seed = CoverageState {
        total_api_ids: vec![target_api_id.clone()],
        api_status: BTreeMap::from([(target_api_id.clone(), ApiCoverageStatus::Validated)]),
        failed_attempts: BTreeMap::from([(
            target_api_id.clone(),
            FailedAttempt {
                compile_fails: 7,
                misuse_fails: 1,
                last_reason: "stale_compile_failed".into(),
            },
        )]),
        covered_api_ids: vec![target_api_id.clone()],
        exhausted_api_ids: vec![target_api_id.clone()],
        uncovered_api_ids: vec![],
        related_total_api_ids: vec![old_related.clone()],
        related_api_ids_by_target: BTreeMap::from([(
            target_api_id.clone(),
            vec![old_related.clone()],
        )]),
        related_covered_api_ids: vec![old_related.clone()],
        related_uncovered_api_ids: vec![],
        harnesses: BTreeMap::from([
            (
                "1:1".into(),
                HarnessRecord {
                    round: 1,
                    sub_index: 1,
                    attempt: None,
                    target_api_id: Some(target_api_id.clone()),
                    api_ids: vec![target_api_id.clone(), old_related.clone()],
                    status: ApiCoverageStatus::Validated,
                    harness_path: Some(current_harness_path.display().to_string()),
                    compile_report_path: Some(
                        reports_dir
                            .join("compile_001_01.json")
                            .display()
                            .to_string(),
                    ),
                    smoke_report_path: Some(
                        reports_dir.join("smoke_001_01.json").display().to_string(),
                    ),
                    runtime_status: Some("bug".into()),
                    runtime_classification: Some("panic_detected".into()),
                    runtime_error_report_path: Some(
                        reports_dir
                            .join("runtime_error_001_01.json")
                            .display()
                            .to_string(),
                    ),
                    runtime_error_summary: Some("stale runtime".into()),
                    review_status: Some("needs_review".into()),
                    review_reason: Some("stale review".into()),
                    review_report_path: Some(
                        reports_dir
                            .join("fix_acceptance_001_index.json")
                            .display()
                            .to_string(),
                    ),
                    scenario_id: None,
                    mapping_id: None,
                    plan_id: None,
                },
            ),
            (
                "1:1:fixed:1".into(),
                HarnessRecord {
                    round: 1,
                    sub_index: 1,
                    attempt: Some(1),
                    target_api_id: Some(target_api_id.clone()),
                    api_ids: vec![target_api_id.clone(), old_related.clone()],
                    status: ApiCoverageStatus::Validated,
                    harness_path: Some(old_harness_path.display().to_string()),
                    compile_report_path: Some(
                        reports_dir
                            .join("compile_001_01_fixed_01.json")
                            .display()
                            .to_string(),
                    ),
                    smoke_report_path: Some(
                        reports_dir
                            .join("smoke_001_01_fixed_01.json")
                            .display()
                            .to_string(),
                    ),
                    runtime_status: Some("runtime_error".into()),
                    runtime_classification: Some("panic_or_crash".into()),
                    runtime_error_report_path: Some(
                        reports_dir
                            .join("runtime_error_001_01_fixed_01.json")
                            .display()
                            .to_string(),
                    ),
                    runtime_error_summary: Some("stale fixed runtime".into()),
                    review_status: None,
                    review_reason: None,
                    review_report_path: None,
                    scenario_id: None,
                    mapping_id: None,
                    plan_id: None,
                },
            ),
        ]),
        found_bugs: vec![target_api_id.as_str().to_string()],
        needs_review: vec![target_api_id.as_str().to_string()],
        next_priority: vec![],
        coverage_rate: 1.0,
        related_coverage_rate: 1.0,
    };
    fs::write(
        &coverage_path,
        serde_json::to_string_pretty(&seed).expect("serialize seed") + "\n",
    )
    .expect("write seed coverage");

    fs::write(
        &context_path,
        "\
## Target API
- api_id: api::fixture::danger

## Related APIs
- api::fixture::New::helper: fixture::New::helper — fn helper(&Self) -> bool
",
    )
    .expect("write context");
    fs::write(
        &compile_index_path,
        format!(
            "{{\"round\":1,\"status\":\"failed\",\"reports\":[{{\"harness\":\"{}\",\"report\":\"{}\",\"status\":\"failed\",\"exit_code\":2}}]}}\n",
            current_harness_path.display(),
            reports_dir.join("compile_001_01.json").display(),
        ),
    )
    .expect("write compile index");

    let update = write_coverage_from_phase3(
        &coverage_path,
        &context_path,
        Some(&compile_index_path),
        None,
        None,
        None,
    )
    .expect("update coverage");

    assert_eq!(update.status, ApiCoverageStatus::Attempted);
    assert_eq!(
        update.state.api_status.get(&target_api_id),
        Some(&ApiCoverageStatus::Attempted)
    );
    assert!(update.state.covered_api_ids.is_empty());
    assert!(update.state.found_bugs.is_empty());
    assert!(update.state.needs_review.is_empty());
    assert!(update.state.exhausted_api_ids.is_empty());
    assert_eq!(
        update
            .state
            .failed_attempts
            .get(&target_api_id)
            .expect("failed attempt")
            .compile_fails,
        1
    );
    assert_eq!(
        update
            .state
            .failed_attempts
            .get(&target_api_id)
            .expect("failed attempt")
            .last_reason,
        "compile_failed"
    );
    assert_eq!(update.state.related_total_api_ids, vec![new_related]);
    assert!(update.state.related_covered_api_ids.is_empty());
    assert_eq!(update.state.harnesses.len(), 1);

    let record = update.state.harnesses.get("1:1").expect("current harness");
    assert_eq!(record.status, ApiCoverageStatus::Attempted);
    assert_eq!(
        record.target_api_id.as_ref().map(|value| value.as_str()),
        Some("api::fixture::danger")
    );
    assert_eq!(record.api_ids, vec![ApiId::from("api::fixture::danger")]);
    assert_eq!(
        record.compile_report_path.as_deref(),
        Some(reports_dir.join("compile_001_01.json").to_str().unwrap())
    );
    assert!(record.smoke_report_path.is_none());
    assert!(record.runtime_status.is_none());
    assert!(record.review_status.is_none());
}
