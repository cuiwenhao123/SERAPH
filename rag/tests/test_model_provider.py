from pathlib import Path

from seraph_rag.model_provider import write_model_response


def test_write_model_response_writes_stdout_to_output_file(tmp_path):
    input_path = tmp_path / "harness_prompt_001.json"
    output_path = tmp_path / "llm_response_001.md"
    input_path.write_text('{"target_api_id":"api::fixture::danger"}\n', encoding="utf-8")

    result = write_model_response(
        input_path,
        output_path,
        "python3 -c \"print('fn main() { println!(\\\"SERAPH_STEP_ENTER:1:api::fixture::danger\\\"); println!(\\\"SERAPH_STEP_OK:1:api::fixture::danger\\\"); }')\"",
    )

    assert result["status"] == "ok"
    assert output_path.exists()
    assert "SERAPH_STEP_ENTER:1:api::fixture::danger" in output_path.read_text(encoding="utf-8")


def test_write_model_response_retries_transient_gateway_failures(tmp_path):
    input_path = tmp_path / "harness_prompt_001.json"
    output_path = tmp_path / "llm_response_001.md"
    state_path = tmp_path / "attempt_count.txt"
    script_path = tmp_path / "flaky_model.py"
    input_path.write_text('{"target_api_id":"api::fixture::danger"}\n', encoding="utf-8")
    script_path.write_text(
        "\n".join([
            "from pathlib import Path",
            "import sys",
            f"state = Path({str(state_path)!r})",
            "count = int(state.read_text()) if state.exists() else 0",
            "state.write_text(str(count + 1))",
            "if count == 0:",
            "    sys.stderr.write('HTTP Error 502: Bad Gateway\\n')",
            "    raise SystemExit(1)",
            "sys.stdout.write('fn main() { println!(\"SERAPH_STEP_ENTER:1:api::fixture::danger\"); println!(\"SERAPH_STEP_OK:1:api::fixture::danger\"); }')",
        ]) + "\n",
        encoding="utf-8",
    )

    result = write_model_response(
        input_path,
        output_path,
        f"python3 {script_path} {{output}}",
    )

    assert result["status"] == "ok"
    assert state_path.read_text(encoding="utf-8") == "2"
    assert output_path.exists()
