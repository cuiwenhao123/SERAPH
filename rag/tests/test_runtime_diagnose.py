import json

from seraph_rag.runtime_diagnose import run_runtime_diagnosis


def test_run_runtime_diagnosis_writes_detail_and_index(tmp_path):
    context_path = tmp_path / "rag_target_001.md"
    reports_dir = tmp_path / "reports"
    smoke_index_path = reports_dir / "smoke_001_index.json"
    harness_path = tmp_path / "fuzz" / "harness_001_02_fixed_01.rs"
    harness_path.parent.mkdir(parents=True)
    reports_dir.mkdir(parents=True)
    harness_path.write_text("fn main() {}\n", encoding="utf-8")
    context_path.write_text("## Target API\n- api_id: api::fixture::danger\n", encoding="utf-8")
    (reports_dir / "smoke_001_02_fixed_01.json").write_text(
        json.dumps(
            {
                "harness": str(harness_path),
                "status": "bug",
                "classification": "panic_detected",
                "exit_code": 101,
                "stdout": "",
                "stderr": "thread panicked at bad input",
            }
        ),
        encoding="utf-8",
    )
    smoke_index_path.write_text(
        json.dumps(
            {
                "round": 1,
                "status": "bug",
                "reports": [
                    {
                        "harness": str(harness_path),
                        "report": str(reports_dir / "smoke_001_02_fixed_01.json"),
                        "status": "bug",
                        "classification": "panic_detected",
                        "exit_code": 101,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    result = run_runtime_diagnosis(
        context_path,
        smoke_index_path,
        reports_dir,
        round_no=1,
        command_template=(
            "python3 -c \"import json; print(json.dumps({"
            "'classification':'panic_or_crash',"
            "'summary':'panic after reaching target',"
            "'is_bug':False}))\""
        ),
    )

    assert result["status"] == "runtime_error"
    assert result["report_count"] == 1
    assert result["bug_count"] == 0
    assert result["reports"][0]["classification"] == "panic_or_crash"
    assert (reports_dir / "runtime_error_001_02_fixed_01.json").exists()
    assert (reports_dir / "runtime_error_001_index.json").exists()


def test_run_runtime_diagnosis_marks_asan_as_bug(tmp_path):
    context_path = tmp_path / "rag_target_001.md"
    reports_dir = tmp_path / "reports"
    smoke_index_path = reports_dir / "smoke_001_index.json"
    harness_path = tmp_path / "fuzz" / "harness_001_01.rs"
    harness_path.parent.mkdir(parents=True)
    reports_dir.mkdir(parents=True)
    harness_path.write_text("fn main() {}\n", encoding="utf-8")
    context_path.write_text("## Target API\n- api_id: api::fixture::danger\n", encoding="utf-8")
    (reports_dir / "smoke_001_01.json").write_text(
        json.dumps(
            {
                "harness": str(harness_path),
                "status": "bug",
                "classification": "nonzero_exit",
                "exit_code": 1,
                "stdout": "",
                "stderr": "==1==ERROR: AddressSanitizer: heap-buffer-overflow",
            }
        ),
        encoding="utf-8",
    )
    smoke_index_path.write_text(
        json.dumps(
            {
                "round": 1,
                "status": "bug",
                "reports": [
                    {
                        "harness": str(harness_path),
                        "report": str(reports_dir / "smoke_001_01.json"),
                        "status": "bug",
                        "classification": "nonzero_exit",
                        "exit_code": 1,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    result = run_runtime_diagnosis(
        context_path,
        smoke_index_path,
        reports_dir,
        round_no=1,
        command_template=(
            "python3 -c \"import json; print(json.dumps({"
            "'classification':'asan_bug',"
            "'summary':'asan hit inside target',"
            "'is_bug':True}))\""
        ),
    )

    assert result["status"] == "bug"
    assert result["bug_count"] == 1
    assert result["reports"][0]["bug"] is True
