import json
from pathlib import Path

from seraph_rag.smoke_run import collect_successful_harnesses, run_smoke_check, run_smoke_checks


def test_run_smoke_check_records_bug_for_nonzero_exit(tmp_path):
    harness = tmp_path / "harness_001_01.rs"
    report = tmp_path / "smoke_001_01.json"
    harness.write_text("fn main() {}\n", encoding="utf-8")

    result = run_smoke_check(
        harness,
        report,
        command_template="python3 -c 'import sys; print(\"panicked at smoke\", file=sys.stderr); sys.exit(101)'",
    )

    data = json.loads(report.read_text(encoding="utf-8"))
    assert result["status"] == "bug"
    assert result["classification"] == "panic_detected"
    assert data["exit_code"] == 101
    assert data["panic_detected"] is True


def test_run_smoke_check_records_review_marker(tmp_path):
    harness = tmp_path / "harness_001_01.rs"
    report = tmp_path / "smoke_001_01.json"
    harness.write_text("fn main() {}\n", encoding="utf-8")

    result = run_smoke_check(
        harness,
        report,
        command_template="python3 -c 'print(\"SERAPH_NEEDS_REVIEW: nondeterministic output\")'",
    )

    data = json.loads(report.read_text(encoding="utf-8"))
    assert result["status"] == "review"
    assert result["classification"] == "needs_review"
    assert "nondeterministic output" in data["review_reason"]


def test_run_smoke_checks_writes_index_for_successful_harnesses(tmp_path):
    output_dir = tmp_path / "reports"
    harness_a = tmp_path / "harness_003_01.rs"
    harness_b = tmp_path / "harness_003_02_fixed_01.rs"
    harness_a.write_text("fn main() {}\n", encoding="utf-8")
    harness_b.write_text("fn main() {}\n", encoding="utf-8")

    result = run_smoke_checks(
        [harness_a, harness_b],
        output_dir,
        round_no=3,
        command_template="python3 -c 'import sys; sys.exit(0)'",
    )

    index_path = output_dir / "smoke_003_index.json"
    index = json.loads(index_path.read_text(encoding="utf-8"))
    assert result["status"] == "ok"
    assert index["round"] == 3
    assert len(index["reports"]) == 2
    assert (output_dir / "smoke_003_01.json").exists()
    assert (output_dir / "smoke_003_02_fixed_01.json").exists()


def test_collect_successful_harnesses_reads_compile_and_fix_loop_indexes(tmp_path):
    compile_index = tmp_path / "compile_004_index.json"
    fix_loop_index = tmp_path / "fix_loop_004_index.json"
    compile_ok = tmp_path / "harness_004_01.rs"
    compile_failed = tmp_path / "harness_004_02.rs"
    fixed_ok = tmp_path / "harness_004_02_fixed_02.rs"
    compile_ok.write_text("fn main() {}\n", encoding="utf-8")
    compile_failed.write_text("fn main() {}\n", encoding="utf-8")
    fixed_ok.write_text("fn main() {}\n", encoding="utf-8")
    compile_index.write_text(
        json.dumps(
            {
                "round": 4,
                "status": "failed",
                "reports": [
                    {"harness": str(compile_ok), "status": "ok", "exit_code": 0},
                    {"harness": str(compile_failed), "status": "failed", "exit_code": 1},
                ],
            }
        ),
        encoding="utf-8",
    )
    fix_loop_index.write_text(
        json.dumps(
            {
                "status": "failed",
                "loops": [
                    {
                        "status": "ok",
                        "successful_attempt": 2,
                        "attempts": [
                            {"attempt": 1, "harness": str(tmp_path / "harness_004_02_fixed_01.rs")},
                            {"attempt": 2, "harness": str(fixed_ok)},
                        ],
                    },
                    {"status": "failed", "successful_attempt": None, "attempts": []},
                ],
            }
        ),
        encoding="utf-8",
    )

    harnesses = collect_successful_harnesses(compile_index, fix_loop_index)

    assert harnesses == [Path(compile_ok), Path(fixed_ok)]


def test_collect_successful_harnesses_reads_fix_loop_batch_reports(tmp_path):
    compile_index = tmp_path / "compile_005_index.json"
    fix_loop_index = tmp_path / "fix_loop_005_index.json"
    fix_loop_report = tmp_path / "fix_loop_005_01.json"
    compile_ok = tmp_path / "harness_005_01.rs"
    fixed_ok = tmp_path / "harness_005_02_fixed_01.rs"
    compile_ok.write_text("fn main() {}\n", encoding="utf-8")
    fixed_ok.write_text("fn main() {}\n", encoding="utf-8")
    compile_index.write_text(
        json.dumps(
            {
                "round": 5,
                "status": "failed",
                "reports": [
                    {"harness": str(compile_ok), "status": "ok", "exit_code": 0},
                ],
            }
        ),
        encoding="utf-8",
    )
    fix_loop_report.write_text(
        json.dumps(
            {
                "status": "ok",
                "successful_attempt": 1,
                "attempts": [
                    {"attempt": 1, "harness": str(fixed_ok)},
                ],
            }
        ),
        encoding="utf-8",
    )
    fix_loop_index.write_text(
        json.dumps(
            {
                "status": "ok",
                "loops": [
                    {
                        "status": "ok",
                        "successful_attempt": 1,
                        "report": str(fix_loop_report),
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    harnesses = collect_successful_harnesses(compile_index, fix_loop_index)

    assert harnesses == [Path(compile_ok), Path(fixed_ok)]
