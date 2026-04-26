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
