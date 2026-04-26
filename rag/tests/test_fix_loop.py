import json

from seraph_rag.fix_loop import run_fix_loop, run_fix_loops

VALID_FIXED_DANGER_SOURCE = """fn danger() {}

fn main() {
    println!("SERAPH_STEP_ENTER:1:api::fixture::danger");
    danger();
    println!("SERAPH_STEP_OK:1:api::fixture::danger");
}
"""

VALID_FIXED_DANGER_RESPONSE = f"```rust\n{VALID_FIXED_DANGER_SOURCE}```"


def test_run_fix_loop_stops_after_first_success(tmp_path):
    request_path = tmp_path / "fix_request_004_02.json"
    responses_dir = tmp_path / "fixes"
    fuzz_dir = tmp_path / "fuzz"
    report_dir = tmp_path / "reports"
    request_path.write_text(
        json.dumps({"round": 4, "variant": 2, "target_api_id": "api::fixture::danger"}),
        encoding="utf-8",
    )
    responses_dir.mkdir()
    for attempt in (1, 2, 3):
        (responses_dir / "fix_response_004_02_{:02d}.md".format(attempt)).write_text(
            VALID_FIXED_DANGER_RESPONSE,
            encoding="utf-8",
        )

    result = run_fix_loop(
        request_path,
        responses_dir,
        fuzz_dir,
        report_dir,
        max_attempts=3,
        command_template="python3 -c \"import pathlib, sys; sys.exit(0 if 'fixed_02' in pathlib.Path(sys.argv[1]).name else 1)\" {harness}",
    )

    assert result["status"] == "ok"
    assert result["stop_reason"] == "compiled"
    assert result["successful_attempt"] == 2
    assert [attempt["attempt"] for attempt in result["attempts"]] == [1, 2]
    assert (fuzz_dir / "harness_004_02_fixed_01.rs").exists()
    assert (fuzz_dir / "harness_004_02_fixed_02.rs").exists()
    assert not (fuzz_dir / "harness_004_02_fixed_03.rs").exists()
    assert (report_dir / "fix_loop_004_02.json").exists()


def test_run_fix_loop_reports_missing_next_response(tmp_path):
    request_path = tmp_path / "fix_request_005_01.json"
    responses_dir = tmp_path / "fixes"
    fuzz_dir = tmp_path / "fuzz"
    report_dir = tmp_path / "reports"
    request_path.write_text(
        json.dumps({"round": 5, "variant": 1, "target_api_id": "api::fixture::danger"}),
        encoding="utf-8",
    )
    responses_dir.mkdir()
    (responses_dir / "fix_response_005_01_01.md").write_text(
        VALID_FIXED_DANGER_RESPONSE,
        encoding="utf-8",
    )

    result = run_fix_loop(
        request_path,
        responses_dir,
        fuzz_dir,
        report_dir,
        max_attempts=3,
        command_template="python3 -c 'import sys; sys.exit(1)'",
    )

    assert result["status"] == "failed"
    assert result["stop_reason"] == "missing_response"
    assert result["successful_attempt"] is None
    assert len(result["attempts"]) == 1
    assert result["missing_response"].endswith("fix_response_005_01_02.md")
    assert (fuzz_dir / "harness_005_01_fixed_01.rs").exists()
    assert not (fuzz_dir / "harness_005_01_fixed_02.rs").exists()


def test_run_fix_loops_writes_batch_index(tmp_path):
    requests_dir = tmp_path / "fixes"
    responses_dir = tmp_path / "fixes"
    fuzz_dir = tmp_path / "fuzz"
    report_dir = tmp_path / "reports"
    index_path = report_dir / "fix_loop_008_index.json"
    requests_dir.mkdir()

    (requests_dir / "fix_request_008_01.json").write_text(
        json.dumps({"round": 8, "variant": 1, "target_api_id": "api::fixture::danger"}),
        encoding="utf-8",
    )
    (requests_dir / "fix_request_008_02.json").write_text(
        json.dumps({"round": 8, "variant": 2, "target_api_id": "api::fixture::danger"}),
        encoding="utf-8",
    )
    (responses_dir / "fix_response_008_01_01.md").write_text(
        VALID_FIXED_DANGER_RESPONSE,
        encoding="utf-8",
    )

    result = run_fix_loops(
        requests_dir / "fix_request_008_*.json",
        responses_dir,
        fuzz_dir,
        report_dir,
        index_path,
        max_attempts=2,
        command_template="python3 -c 'import sys; sys.exit(0)'",
    )

    assert result["status"] == "failed"
    assert result["request_count"] == 2
    assert result["successful_requests"] == 1
    assert result["failed_requests"] == 1
    assert len(result["loops"]) == 2
    assert result["loops"][0]["status"] == "ok"
    assert result["loops"][1]["stop_reason"] == "missing_response"
    assert index_path.exists()


def test_run_fix_loop_generates_missing_response_with_command_template(tmp_path):
    request_path = tmp_path / "fix_request_011_01.json"
    responses_dir = tmp_path / "fixes"
    fuzz_dir = tmp_path / "fuzz"
    report_dir = tmp_path / "reports"
    request_path.write_text(
        json.dumps({"round": 11, "variant": 1, "target_api_id": "api::fixture::danger"}),
        encoding="utf-8",
    )

    result = run_fix_loop(
        request_path,
        responses_dir,
        fuzz_dir,
        report_dir,
        max_attempts=1,
        command_template="python3 -c 'import sys; sys.exit(0)'",
        response_command_template=f'python3 -c "print({VALID_FIXED_DANGER_SOURCE!r})"',
    )

    assert result["status"] == "ok"
    assert (responses_dir / "fix_response_011_01_01.md").exists()
