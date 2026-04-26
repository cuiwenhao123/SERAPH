import json

from seraph_rag.fix_once import run_fix_once

VALID_FIXED_DANGER_SOURCE = """fn danger() {}

fn main() {
    println!("SERAPH_STEP_ENTER:1:api::fixture::danger");
    danger();
    println!("SERAPH_STEP_OK:1:api::fixture::danger");
}
"""

VALID_FIXED_DANGER_RESPONSE = f"```rust\n{VALID_FIXED_DANGER_SOURCE}```"


def test_run_fix_once_writes_fixed_harness_and_compile_report(tmp_path):
    request_path = tmp_path / "fix_request_004_02.json"
    response_path = tmp_path / "fix_response_004_02.md"
    fuzz_dir = tmp_path / "fuzz"
    report_dir = tmp_path / "reports"
    request_path.write_text(
        json.dumps({"round": 4, "variant": 2, "target_api_id": "api::fixture::danger"}),
        encoding="utf-8",
    )
    response_path.write_text(
        VALID_FIXED_DANGER_RESPONSE,
        encoding="utf-8",
    )

    result = run_fix_once(
        request_path,
        response_path,
        fuzz_dir,
        report_dir,
        attempt=1,
        command_template="python3 -c 'import sys; sys.exit(0)'",
    )

    assert result["status"] == "ok"
    assert (fuzz_dir / "harness_004_02_fixed_01.rs").exists()
    report_path = report_dir / "compile_004_02_fixed_01.json"
    assert report_path.exists()
    report = json.loads(report_path.read_text(encoding="utf-8"))
    assert report["harness"].endswith("harness_004_02_fixed_01.rs")
    assert report["status"] == "ok"
