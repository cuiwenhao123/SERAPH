import json

import pytest

from seraph_rag.fix_acceptance_write import (
    write_fix_acceptance_decision,
    write_fix_acceptance_decisions,
)


def test_write_fix_acceptance_decision_updates_entry_and_rollup(tmp_path):
    index_path = tmp_path / "fix_acceptance_012_index.json"
    harness = tmp_path / "fuzz" / "harness_012_02_fixed_01.rs"
    harness.parent.mkdir(parents=True)
    harness.write_text("fn main() {}\n", encoding="utf-8")
    index_path.write_text(
        json.dumps(
            {
                "version": "seraph.phase3.fix_acceptance_index.v1",
                "round": 12,
                "status": "needs_review",
                "entry_count": 1,
                "entries": [
                    {
                        "harness": str(harness),
                        "compile_report": "compile.json",
                        "smoke_report": "smoke.json",
                        "status": "needs_review",
                        "reason": "needs_review",
                        "source": "auto",
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    result = write_fix_acceptance_decision(
        index_path,
        harness,
        "accepted",
        "manual_triage_ok",
    )

    assert result["status"] == "accepted"
    assert result["entry_count"] == 1
    assert result["entries"][0]["status"] == "accepted"
    assert result["entries"][0]["reason"] == "manual_triage_ok"
    assert result["entries"][0]["source"] == "manual"


def test_write_fix_acceptance_decision_errors_when_harness_missing(tmp_path):
    index_path = tmp_path / "fix_acceptance_013_index.json"
    index_path.write_text(
        json.dumps(
            {
                "version": "seraph.phase3.fix_acceptance_index.v1",
                "round": 13,
                "status": "accepted",
                "entry_count": 0,
                "entries": [],
            }
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="harness not found"):
        write_fix_acceptance_decision(
            index_path,
            tmp_path / "fuzz" / "harness_013_01_fixed_01.rs",
            "bug",
            "manual_bug_confirmed",
        )


def test_write_fix_acceptance_decisions_updates_multiple_entries(tmp_path):
    index_path = tmp_path / "fix_acceptance_014_index.json"
    decisions_path = tmp_path / "decisions.json"
    harness_a = tmp_path / "fuzz" / "harness_014_01_fixed_01.rs"
    harness_b = tmp_path / "fuzz" / "harness_014_02_fixed_01.rs"
    harness_a.parent.mkdir(parents=True)
    harness_a.write_text("fn main() {}\n", encoding="utf-8")
    harness_b.write_text("fn main() {}\n", encoding="utf-8")
    index_path.write_text(
        json.dumps(
            {
                "version": "seraph.phase3.fix_acceptance_index.v1",
                "round": 14,
                "status": "needs_review",
                "entry_count": 2,
                "entries": [
                    {
                        "harness": str(harness_a),
                        "compile_report": "compile_a.json",
                        "smoke_report": "smoke_a.json",
                        "status": "needs_review",
                        "reason": "smoke_missing",
                        "source": "auto",
                    },
                    {
                        "harness": str(harness_b),
                        "compile_report": "compile_b.json",
                        "smoke_report": "smoke_b.json",
                        "status": "needs_review",
                        "reason": "needs_review",
                        "source": "auto",
                    },
                ],
            }
        ),
        encoding="utf-8",
    )
    decisions_path.write_text(
        json.dumps(
            {
                "decisions": [
                    {
                        "harness": str(harness_a),
                        "status": "accepted",
                        "reason": "manual_triage_ok",
                    },
                    {
                        "harness": str(harness_b),
                        "status": "bug",
                        "reason": "manual_bug_confirmed",
                        "source": "reviewer:alice",
                    },
                ]
            }
        ),
        encoding="utf-8",
    )

    result = write_fix_acceptance_decisions(index_path, decisions_path)

    assert result["status"] == "bug"
    assert result["entry_count"] == 2
    assert result["entries"][0]["status"] == "accepted"
    assert result["entries"][0]["source"] == "manual"
    assert result["entries"][1]["status"] == "bug"
    assert result["entries"][1]["reason"] == "manual_bug_confirmed"
    assert result["entries"][1]["source"] == "reviewer:alice"


def test_write_fix_acceptance_decisions_rejects_duplicate_harness_entries(tmp_path):
    index_path = tmp_path / "fix_acceptance_015_index.json"
    decisions_path = tmp_path / "decisions.json"
    harness = tmp_path / "fuzz" / "harness_015_01_fixed_01.rs"
    harness.parent.mkdir(parents=True)
    harness.write_text("fn main() {}\n", encoding="utf-8")
    index_path.write_text(
        json.dumps(
            {
                "version": "seraph.phase3.fix_acceptance_index.v1",
                "round": 15,
                "status": "needs_review",
                "entry_count": 1,
                "entries": [
                    {
                        "harness": str(harness),
                        "compile_report": "compile.json",
                        "smoke_report": "smoke.json",
                        "status": "needs_review",
                        "reason": "smoke_missing",
                        "source": "auto",
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    decisions_path.write_text(
        json.dumps(
            [
                {
                    "harness": str(harness),
                    "status": "accepted",
                    "reason": "manual_triage_ok",
                },
                {
                    "harness": str(harness),
                    "status": "bug",
                    "reason": "manual_bug_confirmed",
                },
            ]
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="duplicate harness"):
        write_fix_acceptance_decisions(index_path, decisions_path)
