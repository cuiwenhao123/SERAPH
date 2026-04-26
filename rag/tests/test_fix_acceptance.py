import json

from seraph_rag.fix_acceptance import write_fix_acceptance_index


def test_write_fix_acceptance_index_summarizes_successful_fixed_harnesses(tmp_path):
    reports_dir = tmp_path / "reports"
    reports_dir.mkdir()
    fix_loop_report = reports_dir / "fix_loop_004_02.json"
    fix_loop_index = reports_dir / "fix_loop_004_index.json"
    smoke_index = reports_dir / "smoke_004_index.json"
    output_path = reports_dir / "fix_acceptance_004_index.json"
    fixed_ok = tmp_path / "fuzz" / "harness_004_02_fixed_02.rs"
    fixed_review = tmp_path / "fuzz" / "harness_004_03_fixed_01.rs"
    fixed_ok.parent.mkdir(parents=True)
    fixed_ok.write_text("fn main() {}\n", encoding="utf-8")
    fixed_review.write_text("fn main() {}\n", encoding="utf-8")

    fix_loop_report.write_text(
        json.dumps(
            {
                "round": 4,
                "variant": 2,
                "status": "ok",
                "successful_attempt": 2,
                "attempts": [
                    {
                        "attempt": 1,
                        "harness": str(tmp_path / "fuzz" / "harness_004_02_fixed_01.rs"),
                        "report": str(reports_dir / "compile_004_02_fixed_01.json"),
                        "status": "failed",
                    },
                    {
                        "attempt": 2,
                        "harness": str(fixed_ok),
                        "report": str(reports_dir / "compile_004_02_fixed_02.json"),
                        "status": "ok",
                    },
                ],
            }
        ),
        encoding="utf-8",
    )
    (reports_dir / "fix_loop_004_03.json").write_text(
        json.dumps(
            {
                "round": 4,
                "variant": 3,
                "status": "ok",
                "successful_attempt": 1,
                "attempts": [
                    {
                        "attempt": 1,
                        "harness": str(fixed_review),
                        "report": str(reports_dir / "compile_004_03_fixed_01.json"),
                        "status": "ok",
                    }
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
                        "successful_attempt": 2,
                        "report": str(fix_loop_report),
                    },
                    {
                        "status": "ok",
                        "successful_attempt": 1,
                        "report": str(reports_dir / "fix_loop_004_03.json"),
                    },
                ],
            }
        ),
        encoding="utf-8",
    )
    smoke_index.write_text(
        json.dumps(
            {
                "round": 4,
                "status": "review",
                "reports": [
                    {
                        "harness": str(fixed_ok),
                        "report": str(reports_dir / "smoke_004_02_fixed_02.json"),
                        "status": "ok",
                        "classification": "completed",
                    },
                    {
                        "harness": str(fixed_review),
                        "report": str(reports_dir / "smoke_004_03_fixed_01.json"),
                        "status": "review",
                        "classification": "needs_review",
                    },
                ],
            }
        ),
        encoding="utf-8",
    )

    result = write_fix_acceptance_index(fix_loop_index, smoke_index, output_path)

    assert result["status"] == "needs_review"
    assert result["entry_count"] == 2
    assert result["entries"][0]["status"] == "accepted"
    assert result["entries"][0]["reason"] == "smoke_ok"
    assert result["entries"][1]["status"] == "needs_review"
    assert result["entries"][1]["reason"] == "needs_review"
    assert output_path.exists()


def test_write_fix_acceptance_index_marks_missing_smoke_as_review(tmp_path):
    reports_dir = tmp_path / "reports"
    reports_dir.mkdir()
    fix_loop_report = reports_dir / "fix_loop_009_01.json"
    fix_loop_index = reports_dir / "fix_loop_009_index.json"
    output_path = reports_dir / "fix_acceptance_009_index.json"
    fixed_ok = tmp_path / "fuzz" / "harness_009_01_fixed_01.rs"
    fixed_ok.parent.mkdir(parents=True)
    fixed_ok.write_text("fn main() {}\n", encoding="utf-8")

    fix_loop_report.write_text(
        json.dumps(
            {
                "round": 9,
                "variant": 1,
                "status": "ok",
                "successful_attempt": 1,
                "attempts": [
                    {
                        "attempt": 1,
                        "harness": str(fixed_ok),
                        "report": str(reports_dir / "compile_009_01_fixed_01.json"),
                        "status": "ok",
                    }
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

    result = write_fix_acceptance_index(fix_loop_index, None, output_path)

    assert result["status"] == "needs_review"
    assert result["entries"][0]["status"] == "needs_review"
    assert result["entries"][0]["reason"] == "smoke_missing"
