use std::process::Command;

#[test]
fn run_dry_run_prints_phase2_and_phase3_commands() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "run",
            "--knowledge",
            "tests/fixtures/knowledge.json",
            "--workspace-dir",
            "/tmp/seraph-run-test",
            "--round",
            "7",
            "--target-api-id",
            "fn::fixture_crate::Buffer::get_unchecked",
            "--phase3-prompt",
            "--variants",
            "2",
            "--llm-response",
            "/tmp/seraph-llm-response.md",
            "--compile-check",
            "--compile-command",
            "python3 -c \"import sys; sys.exit(0)\"",
            "--fixer-bundle",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli index"));
    assert!(stdout.contains("--vectordb /tmp/seraph-run-test/vectordb"));
    assert!(stdout.contains("python3 -m seraph_rag.cli graph"));
    assert!(stdout.contains("python3 -m seraph_rag.cli retrieve"));
    assert!(stdout.contains("rag_target_007.md"));
    assert!(stdout.contains("python3 -m seraph_rag.cli harness-prompt"));
    assert!(stdout.contains("harness_prompt_007.json"));
    assert!(stdout.contains("--variants 2"));
    assert!(stdout.contains("python3 -m seraph_rag.cli harness-write"));
    assert!(stdout.contains("--response /tmp/seraph-llm-response.md"));
    assert!(stdout.contains("python3 -m seraph_rag.cli compile-check"));
    assert!(stdout.contains("harness_007_*.rs"));
    assert!(stdout.contains("python3 -m seraph_rag.cli fixer-bundle"));
    assert!(stdout.contains("compile_007_index.json"));
}

#[test]
fn run_dry_run_forwards_phase3_style_to_harness_prompt() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "run",
            "--knowledge",
            "tests/fixtures/knowledge.json",
            "--workspace-dir",
            "/tmp/seraph-run-test",
            "--round",
            "7",
            "--model-command",
            "python3 fake_model.py --input {input}",
            "--phase3-style",
            "aflpp",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli harness-prompt"));
    assert!(stdout.contains("--style aflpp"));
}

#[test]
fn run_dry_run_forwards_target_api_id_to_retrieve() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "run",
            "--knowledge",
            "tests/fixtures/knowledge.json",
            "--workspace-dir",
            "/tmp/seraph-run-test",
            "--round",
            "7",
            "--target-api-id",
            "api::fixture::Buffer::get_unchecked",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli retrieve"));
    assert!(stdout.contains("--target-api-id api::fixture::Buffer::get_unchecked"));
}

#[test]
fn phase3_afl_bootstrap_dry_run_prints_script_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "phase3",
            "afl-bootstrap",
            "--workspace-dir",
            "/tmp/seraph-phase3-real-aflpp-localresp/arrayvec",
            "--merge-report",
            "/tmp/seraph-phase3-real-aflpp-localresp/arrayvec/reports/merge_arrayvec.json",
            "--input-mode",
            "stdin",
            "--build-only",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("bootstrap-fuzz-target.sh"));
    assert!(stdout.contains("--workspace-dir /tmp/seraph-phase3-real-aflpp-localresp/arrayvec"));
    assert!(stdout.contains(
        "--merge-report /tmp/seraph-phase3-real-aflpp-localresp/arrayvec/reports/merge_arrayvec.json"
    ));
    assert!(stdout.contains("--input-mode stdin"));
    assert!(stdout.contains("--build-only"));
}

#[test]
fn run_dry_run_prints_merge_harnesses_before_afl_bootstrap() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "run",
            "--knowledge",
            "tests/fixtures/knowledge.json",
            "--workspace-dir",
            "/tmp/seraph-run-test",
            "--round",
            "7",
            "--target-api-id",
            "fn::fixture_crate::Buffer::get_unchecked",
            "--llm-response",
            "/tmp/seraph-llm-response.md",
            "--compile-check",
            "--smoke-command",
            "python3 -c \"import sys; sys.exit(0)\"",
            "--afl-bootstrap",
            "--afl-build-only",
            "--afl-input-mode",
            "stdin",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli compile-check"));
    assert!(stdout.contains("python3 -m seraph_rag.cli smoke-run"));
    assert!(stdout.contains("python3 -m seraph_rag.cli merge-harnesses"));
    assert!(stdout.contains("bootstrap-fuzz-target.sh"));
    assert!(stdout.contains("--merge-report /tmp/seraph-run-test/reports/merge_fixture_crate.json"));
    assert!(stdout.contains("--input-mode stdin"));
    assert!(stdout.contains("--build-only"));
}

#[test]
fn fixer_write_dry_run_prints_python_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "phase3",
            "fixer-write",
            "--request",
            "workspace/fixes/fix_request_001_01.json",
            "--response",
            "workspace/fixes/fix_response_001_01.md",
            "--output-dir",
            "workspace/fuzz",
            "--attempt",
            "1",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli fixer-write"));
    assert!(stdout.contains("--request workspace/fixes/fix_request_001_01.json"));
    assert!(stdout.contains("--response workspace/fixes/fix_response_001_01.md"));
    assert!(stdout.contains("--attempt 1"));
}

#[test]
fn fix_once_dry_run_prints_python_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "phase3",
            "fix-once",
            "--request",
            "workspace/fixes/fix_request_001_01.json",
            "--response",
            "workspace/fixes/fix_response_001_01.md",
            "--output-dir",
            "workspace/fuzz",
            "--report-dir",
            "workspace/reports",
            "--attempt",
            "1",
            "--command-template",
            "rustc {harness}",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli fix-once"));
    assert!(stdout.contains("--report-dir workspace/reports"));
    assert!(stdout.contains("--command-template 'rustc {harness}'"));
}

#[test]
fn fix_loop_dry_run_prints_python_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "phase3",
            "fix-loop",
            "--request",
            "workspace/fixes/fix_request_001_01.json",
            "--responses-dir",
            "workspace/fixes",
            "--output-dir",
            "workspace/fuzz",
            "--report-dir",
            "workspace/reports",
            "--max-attempts",
            "3",
            "--command-template",
            "rustc {harness}",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli fix-loop"));
    assert!(stdout.contains("--responses-dir workspace/fixes"));
    assert!(stdout.contains("--max-attempts 3"));
    assert!(stdout.contains("--command-template 'rustc {harness}'"));
}

#[test]
fn fix_loop_batch_dry_run_prints_python_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "phase3",
            "fix-loop-batch",
            "--request-glob",
            "workspace/fixes/fix_request_001_*.json",
            "--responses-dir",
            "workspace/fixes",
            "--output-dir",
            "workspace/fuzz",
            "--report-dir",
            "workspace/reports",
            "--index-output",
            "workspace/reports/fix_loop_001_index.json",
            "--max-attempts",
            "3",
            "--command-template",
            "rustc {harness}",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli fix-loop-batch"));
    assert!(stdout.contains("--request-glob 'workspace/fixes/fix_request_001_*.json'"));
    assert!(stdout.contains("--index-output workspace/reports/fix_loop_001_index.json"));
}

#[test]
fn run_dry_run_prints_fix_loop_batch_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "run",
            "--knowledge",
            "tests/fixtures/knowledge.json",
            "--workspace-dir",
            "/tmp/seraph-run-test",
            "--round",
            "7",
            "--target-api-id",
            "fn::fixture_crate::Buffer::get_unchecked",
            "--llm-response",
            "/tmp/seraph-llm-response.md",
            "--fix-loop",
            "--fix-max-attempts",
            "4",
            "--compile-command",
            "rustc {harness}",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli fixer-bundle"));
    assert!(stdout.contains("python3 -m seraph_rag.cli fix-loop-batch"));
    assert!(stdout.contains("--request-glob '/tmp/seraph-run-test/fixes/fix_request_007_*.json'"));
    assert!(stdout.contains("--max-attempts 4"));
}

#[test]
fn model_response_dry_run_prints_python_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "phase3",
            "model-response",
            "--input",
            "workspace/prompts/harness_prompt_001.json",
            "--output",
            "workspace/prompts/llm_response_001.md",
            "--command-template",
            "python3 fake_model.py --input {input}",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli model-response"));
    assert!(stdout.contains("--input workspace/prompts/harness_prompt_001.json"));
    assert!(stdout.contains("--output workspace/prompts/llm_response_001.md"));
}

#[test]
fn run_dry_run_prints_model_generation_commands() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "run",
            "--knowledge",
            "tests/fixtures/knowledge.json",
            "--workspace-dir",
            "/tmp/seraph-run-test",
            "--round",
            "7",
            "--model-command",
            "python3 fake_model.py --input {input}",
            "--fix-loop",
            "--fix-model-command",
            "python3 fake_fixer.py --input {input} --attempt {attempt}",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli harness-prompt"));
    assert!(stdout.contains("python3 -m seraph_rag.cli model-response"));
    assert!(stdout.contains("/tmp/seraph-run-test/prompts/llm_response_007.md"));
    assert!(stdout.contains("python3 -m seraph_rag.cli harness-write"));
    assert!(stdout.contains("python3 -m seraph_rag.cli fix-loop-batch"));
    assert!(stdout.contains(
        "--response-command-template 'python3 fake_fixer.py --input {input} --attempt {attempt}'"
    ));
}

#[test]
fn run_dry_run_prints_smoke_run_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "run",
            "--knowledge",
            "tests/fixtures/knowledge.json",
            "--workspace-dir",
            "/tmp/seraph-run-test",
            "--round",
            "7",
            "--target-api-id",
            "fn::fixture_crate::Buffer::get_unchecked",
            "--llm-response",
            "/tmp/seraph-llm-response.md",
            "--compile-check",
            "--smoke-command",
            "python3 -c \"import sys; sys.exit(0)\"",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli smoke-run"));
    assert!(stdout.contains("--compile-index /tmp/seraph-run-test/reports/compile_007_index.json"));
    assert!(stdout.contains("--report-dir /tmp/seraph-run-test/reports"));
}

#[test]
fn run_dry_run_prints_runtime_diagnose_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "run",
            "--knowledge",
            "tests/fixtures/knowledge.json",
            "--workspace-dir",
            "/tmp/seraph-run-test",
            "--round",
            "7",
            "--target-api-id",
            "fn::fixture_crate::Buffer::get_unchecked",
            "--llm-response",
            "/tmp/seraph-llm-response.md",
            "--compile-check",
            "--smoke-command",
            "python3 -c \"import sys; sys.exit(0)\"",
            "--runtime-model-command",
            "python3 fake_runtime.py --input {input} --output {output}",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli runtime-diagnose"));
    assert!(stdout.contains("--context /tmp/seraph-run-test/contexts/rag_target_007.md"));
    assert!(stdout.contains("--smoke-index /tmp/seraph-run-test/reports/smoke_007_index.json"));
    assert!(stdout.contains(
        "--command-template 'python3 fake_runtime.py --input {input} --output {output}'"
    ));
}

#[test]
fn run_dry_run_notes_batch_mode_when_no_target_is_pinned() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "run",
            "--knowledge",
            "tests/fixtures/knowledge.json",
            "--workspace-dir",
            "/tmp/seraph-run-test",
            "--round",
            "3",
            "--model-command",
            "python3 fake_model.py --input {input}",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("# batch mode -> commands below show the first per-target template"));
    assert!(stdout.contains("starting from round 3"));
    assert!(stdout.contains("rag_target_003.md"));
}

#[test]
fn fix_acceptance_write_dry_run_prints_python_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "phase3",
            "fix-acceptance-write",
            "--index",
            "workspace/reports/fix_acceptance_001_index.json",
            "--harness",
            "workspace/fuzz/harness_001_01_fixed_01.rs",
            "--status",
            "accepted",
            "--reason",
            "manual_triage_ok",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli fix-acceptance-write"));
    assert!(stdout.contains("--status accepted"));
    assert!(stdout.contains("--reason manual_triage_ok"));
}

#[test]
fn fix_acceptance_write_batch_dry_run_prints_python_command() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "phase3",
            "fix-acceptance-write-batch",
            "--index",
            "workspace/reports/fix_acceptance_001_index.json",
            "--decisions",
            "workspace/reports/fix_acceptance_001_decisions.json",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli fix-acceptance-write-batch"));
    assert!(stdout.contains("--decisions workspace/reports/fix_acceptance_001_decisions.json"));
}

#[test]
fn fix_acceptance_write_dry_run_prints_optional_coverage_sync_note() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "phase3",
            "fix-acceptance-write",
            "--index",
            "workspace/reports/fix_acceptance_001_index.json",
            "--harness",
            "workspace/fuzz/harness_001_01_fixed_01.rs",
            "--status",
            "accepted",
            "--reason",
            "manual_triage_ok",
            "--coverage",
            "workspace/coverage.json",
            "--context",
            "workspace/contexts/rag_target_001.md",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli fix-acceptance-write"));
    assert!(stdout.contains("# coverage sync -> workspace/coverage.json"));
    assert!(stdout.contains("context workspace/contexts/rag_target_001.md"));
}

#[test]
fn fix_acceptance_write_batch_dry_run_prints_optional_coverage_sync_note() {
    let binary = env!("CARGO_BIN_EXE_seraph-cli");
    let output = Command::new(binary)
        .args([
            "phase3",
            "fix-acceptance-write-batch",
            "--index",
            "workspace/reports/fix_acceptance_001_index.json",
            "--decisions",
            "workspace/reports/fix_acceptance_001_decisions.json",
            "--coverage",
            "workspace/coverage.json",
            "--context",
            "workspace/contexts/rag_target_001.md",
            "--dry-run",
        ])
        .output()
        .expect("run seraph-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("python3 -m seraph_rag.cli fix-acceptance-write-batch"));
    assert!(stdout.contains("# coverage sync -> workspace/coverage.json"));
}
