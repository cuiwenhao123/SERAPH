import json

from seraph_rag.compile_fixer_response import write_fixed_harness


def test_write_fixed_harness_from_markdown_response(tmp_path):
    request_path = tmp_path / "fix_request_009_02.json"
    response_path = tmp_path / "fix_response.md"
    output_dir = tmp_path / "fuzz"
    request_path.write_text(
        json.dumps({
            "round": 9,
            "variant": 2,
            "target_api_id": "api::fixture::danger",
        }),
        encoding="utf-8",
    )
    response_path.write_text(
        """```rust
fn fuzz() {
    println!("SERAPH_STEP_ENTER:1:api::fixture::danger");
    println!("SERAPH_STEP_OK:1:api::fixture::danger");
}
```""",
        encoding="utf-8",
    )

    output = write_fixed_harness(request_path, response_path, output_dir, attempt=1)

    assert output.name == "harness_009_02_fixed_01.rs"
    assert "api::fixture::danger" in output.read_text(encoding="utf-8")


def test_write_fixed_harness_rejects_missing_target_marker(tmp_path):
    request_path = tmp_path / "fix_request_001_01.json"
    response_path = tmp_path / "fix_response.md"
    output_dir = tmp_path / "fuzz"
    request_path.write_text(
        json.dumps({"round": 1, "variant": 1, "target_api_id": "api::fixture::danger"}),
        encoding="utf-8",
    )
    response_path.write_text("```rust\nfn fuzz() {}\n```", encoding="utf-8")

    try:
        write_fixed_harness(request_path, response_path, output_dir, attempt=1)
    except ValueError as exc:
        assert "missing SERAPH markers" in str(exc)
    else:
        raise AssertionError("expected ValueError")
