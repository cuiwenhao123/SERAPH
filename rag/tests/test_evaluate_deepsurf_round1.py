import json
import importlib.util
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "evaluate_deepsurf_round1.py"
SPEC = importlib.util.spec_from_file_location("evaluate_deepsurf_round1", SCRIPT_PATH)
assert SPEC is not None and SPEC.loader is not None
evaluate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(evaluate)


def test_summarize_phase3_marks_crate_build_failed_when_compile_never_reaches_harness(tmp_path):
    workspace = tmp_path / "workspace"
    reports = workspace / "reports"
    reports.mkdir(parents=True)

    compile_report = reports / "compile_001_01.json"
    compile_report.write_text(
        json.dumps(
            {
                "harness": str(workspace / "fuzz" / "harness_001_01.rs"),
                "status": "failed",
                "exit_code": 101,
                "stderr": (
                    "error[E0554]: #![feature] may not be used on the stable release channel\n"
                    "error: could not compile `leapfrog` (lib) due to 2 previous errors\n"
                ),
                "stdout": "",
            }
        ),
        encoding="utf-8",
    )
    (reports / "compile_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "failed",
                "reports": [
                    {
                        "harness": str(workspace / "fuzz" / "harness_001_01.rs"),
                        "report": str(compile_report),
                        "status": "failed",
                        "exit_code": 101,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    (workspace / "coverage.json").write_text(
        json.dumps(
            {
                "api_status": {"api::fixture::danger": "attempted"},
                "coverage_rate": 0.0,
                "related_coverage_rate": 0.0,
                "found_bugs": [],
                "needs_review": [],
            }
        ),
        encoding="utf-8",
    )

    result = evaluate.summarize_phase3(workspace, "api::fixture::danger")

    assert result["outcome"] == "crate_build_failed"
    assert result["compile_failure_kind"] == "crate_build_failed"
    assert result["compile_failed_crates"] == ["leapfrog"]


def test_summarize_phase3_keeps_attempted_for_harness_compile_failure(tmp_path):
    workspace = tmp_path / "workspace"
    reports = workspace / "reports"
    reports.mkdir(parents=True)

    compile_report = reports / "compile_001_01.json"
    compile_report.write_text(
        json.dumps(
            {
                "harness": str(workspace / "fuzz" / "harness_001_01.rs"),
                "status": "failed",
                "exit_code": 101,
                "stderr": (
                    "error[E0433]: failed to resolve: use of unresolved module or unlinked crate `typenum`\n"
                    "error: could not compile `harness_001_01` (bin \"harness_001_01\") due to 1 previous error\n"
                ),
                "stdout": "",
            }
        ),
        encoding="utf-8",
    )
    (reports / "compile_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "failed",
                "reports": [
                    {
                        "harness": str(workspace / "fuzz" / "harness_001_01.rs"),
                        "report": str(compile_report),
                        "status": "failed",
                        "exit_code": 101,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    (workspace / "coverage.json").write_text(
        json.dumps(
            {
                "api_status": {"api::fixture::danger": "attempted"},
                "coverage_rate": 0.0,
                "related_coverage_rate": 0.0,
                "found_bugs": [],
                "needs_review": [],
            }
        ),
        encoding="utf-8",
    )

    result = evaluate.summarize_phase3(workspace, "api::fixture::danger")

    assert result["outcome"] == "attempted"
    assert result["compile_failure_kind"] == "harness_compile_failed"
    assert result["compile_failed_crates"] == ["harness_001_01"]


def test_summarize_phase3_uses_runtime_diagnosis_to_downgrade_precondition_panics(tmp_path):
    workspace = tmp_path / "workspace"
    reports = workspace / "reports"
    reports.mkdir(parents=True)

    (reports / "smoke_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "bug",
                "bug_count": 1,
                "review_count": 0,
                "reports": [
                    {
                        "harness": str(workspace / "fuzz" / "harness_001_01.rs"),
                        "report": str(reports / "smoke_001_01.json"),
                        "status": "bug",
                        "classification": "panic_detected",
                        "exit_code": 101,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    (reports / "runtime_error_001_index.json").write_text(
        json.dumps(
            {
                "round": 1,
                "status": "runtime_error",
                "bug_count": 0,
                "report_count": 1,
                "reports": [
                    {
                        "harness": str(workspace / "fuzz" / "harness_001_01.rs"),
                        "report": str(reports / "runtime_error_001_01.json"),
                        "status": "runtime_error",
                        "classification": "invalid_input_or_precondition",
                        "summary": "panic triggered by violated documented precondition",
                        "bug": False,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    (workspace / "coverage.json").write_text(
        json.dumps(
            {
                "api_status": {"api::fixture::danger": "validated"},
                "coverage_rate": 1.0,
                "related_coverage_rate": 0.5,
                "found_bugs": [],
                "needs_review": [],
            }
        ),
        encoding="utf-8",
    )

    result = evaluate.summarize_phase3(workspace, "api::fixture::danger")

    assert result["outcome"] == "runtime_error"
    assert result["smoke_bug_count"] == 1
    assert result["runtime_issue_count"] == 1
