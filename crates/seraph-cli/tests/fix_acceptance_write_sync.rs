use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates")
        .parent()
        .expect("repo")
        .to_path_buf()
}

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
fn fix_acceptance_write_sync_updates_coverage_immediately() {
    let repo = repo_root();
    let workspace = temp_dir("fix-acceptance-write-sync");
    let reports_dir = workspace.join("reports");
    let fuzz_dir = workspace.join("fuzz");
    let context_dir = workspace.join("contexts");
    let coverage_path = workspace.join("coverage.json");
    let acceptance_index_path = reports_dir.join("fix_acceptance_001_index.json");
    let context_path = context_dir.join("rag_target_001.md");
    let harness_path = fuzz_dir.join("harness_001_02_fixed_02.rs");

    fs::create_dir_all(&reports_dir).expect("reports dir");
    fs::create_dir_all(&fuzz_dir).expect("fuzz dir");
    fs::create_dir_all(&context_dir).expect("context dir");
    fs::write(&harness_path, "fn main() {}\n").expect("write harness");
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
            "{{\"version\":\"seraph.phase3.fix_acceptance_index.v1\",\"round\":1,\"status\":\"needs_review\",\"entry_count\":1,\"entries\":[{{\"harness\":\"{}\",\"compile_report\":\"{}\",\"smoke_report\":\"{}\",\"status\":\"needs_review\",\"reason\":\"smoke_missing\",\"source\":\"auto\"}}]}}\n",
            harness_path.display(),
            reports_dir.join("compile_001_02_fixed_02.json").display(),
            reports_dir.join("smoke_001_02_fixed_02.json").display(),
        ),
    )
    .expect("write acceptance index");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .args([
            "phase3",
            "fix-acceptance-write",
            "--index",
            acceptance_index_path.to_str().expect("index str"),
            "--harness",
            harness_path.to_str().expect("harness str"),
            "--status",
            "accepted",
            "--reason",
            "manual_triage_ok",
            "--coverage",
            coverage_path.to_str().expect("coverage str"),
            "--context",
            context_path.to_str().expect("context str"),
        ])
        .output()
        .expect("run seraph-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let acceptance = fs::read_to_string(&acceptance_index_path).expect("read acceptance");
    assert!(acceptance.contains("\"status\": \"accepted\""));
    assert!(acceptance.contains("\"reason\": \"manual_triage_ok\""));

    let coverage = fs::read_to_string(&coverage_path).expect("read coverage");
    assert!(coverage.contains("\"needs_review\": []"));
    assert!(coverage.contains("\"review_status\": \"accepted\""));
    assert!(coverage.contains("\"review_reason\": \"manual_triage_ok\""));
}

#[test]
fn fix_acceptance_write_batch_sync_updates_coverage_immediately() {
    let repo = repo_root();
    let workspace = temp_dir("fix-acceptance-write-batch-sync");
    let reports_dir = workspace.join("reports");
    let fuzz_dir = workspace.join("fuzz");
    let context_dir = workspace.join("contexts");
    let coverage_path = workspace.join("coverage.json");
    let acceptance_index_path = reports_dir.join("fix_acceptance_001_index.json");
    let decisions_path = reports_dir.join("fix_acceptance_001_decisions.json");
    let context_path = context_dir.join("rag_target_001.md");
    let harness_a = fuzz_dir.join("harness_001_01_fixed_01.rs");
    let harness_b = fuzz_dir.join("harness_001_02_fixed_01.rs");

    fs::create_dir_all(&reports_dir).expect("reports dir");
    fs::create_dir_all(&fuzz_dir).expect("fuzz dir");
    fs::create_dir_all(&context_dir).expect("context dir");
    fs::write(&harness_a, "fn main() {}\n").expect("write harness a");
    fs::write(&harness_b, "fn main() {}\n").expect("write harness b");
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
            "{{\"version\":\"seraph.phase3.fix_acceptance_index.v1\",\"round\":1,\"status\":\"needs_review\",\"entry_count\":2,\"entries\":[{{\"harness\":\"{}\",\"compile_report\":\"{}\",\"smoke_report\":\"{}\",\"status\":\"needs_review\",\"reason\":\"smoke_missing\",\"source\":\"auto\"}},{{\"harness\":\"{}\",\"compile_report\":\"{}\",\"smoke_report\":\"{}\",\"status\":\"needs_review\",\"reason\":\"needs_review\",\"source\":\"auto\"}}]}}\n",
            harness_a.display(),
            reports_dir.join("compile_001_01_fixed_01.json").display(),
            reports_dir.join("smoke_001_01_fixed_01.json").display(),
            harness_b.display(),
            reports_dir.join("compile_001_02_fixed_01.json").display(),
            reports_dir.join("smoke_001_02_fixed_01.json").display(),
        ),
    )
    .expect("write acceptance index");
    fs::write(
        &decisions_path,
        format!(
            "{{\"decisions\":[{{\"harness\":\"{}\",\"status\":\"accepted\",\"reason\":\"manual_triage_ok\"}},{{\"harness\":\"{}\",\"status\":\"accepted\",\"reason\":\"manual_triage_ok_too\"}}]}}\n",
            harness_a.display(),
            harness_b.display(),
        ),
    )
    .expect("write decisions");

    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .current_dir(&repo)
        .env("PYTHONPATH", repo.join("rag"))
        .args([
            "phase3",
            "fix-acceptance-write-batch",
            "--index",
            acceptance_index_path.to_str().expect("index str"),
            "--decisions",
            decisions_path.to_str().expect("decisions str"),
            "--coverage",
            coverage_path.to_str().expect("coverage str"),
            "--context",
            context_path.to_str().expect("context str"),
        ])
        .output()
        .expect("run seraph-cli");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let acceptance = fs::read_to_string(&acceptance_index_path).expect("read acceptance");
    assert!(acceptance.contains("\"status\": \"accepted\""));

    let coverage = fs::read_to_string(&coverage_path).expect("read coverage");
    assert!(coverage.contains("\"needs_review\": []"));
    assert!(coverage.contains("\"review_status\": \"accepted\""));
    assert!(coverage.contains("\"review_reason\": \"manual_triage_ok_too\""));
}
