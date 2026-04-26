import json

from seraph_rag.harness_codegen import write_harnesses_from_response


def test_write_harnesses_from_markdown_code_fences(tmp_path):
    prompt_path = tmp_path / "harness_prompt_004.json"
    response_path = tmp_path / "llm_response.md"
    output_dir = tmp_path / "fuzz"
    prompt_path.write_text(
        json.dumps({"target_api_id": "api::fixture::Buffer::get_unchecked"}),
        encoding="utf-8",
    )
    response_path.write_text(
        """Here are variants.

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    println!("SERAPH_STEP_ENTER:1:api::fixture::Buffer::get_unchecked");
    println!("SERAPH_STEP_OK:1:api::fixture::Buffer::get_unchecked");
});
```

```rust
fn fuzz(data: &[u8]) {
    println!("SERAPH_STEP_ENTER:1:api::fixture::Buffer::get_unchecked");
    println!("SERAPH_STEP_OK:1:api::fixture::Buffer::get_unchecked");
}
```
""",
        encoding="utf-8",
    )

    written = write_harnesses_from_response(prompt_path, response_path, output_dir, round_no=4)

    assert [path.name for path in written] == ["harness_004_01.rs", "harness_004_02.rs"]
    assert "#![no_main]" in written[0].read_text(encoding="utf-8")
    assert "fn fuzz" in written[1].read_text(encoding="utf-8")


def test_write_harnesses_rejects_response_without_target_marker(tmp_path):
    prompt_path = tmp_path / "harness_prompt_001.json"
    response_path = tmp_path / "llm_response.md"
    output_dir = tmp_path / "fuzz"
    prompt_path.write_text(json.dumps({"target_api_id": "api::fixture::danger"}), encoding="utf-8")
    response_path.write_text("```rust\nfn fuzz() {}\n```", encoding="utf-8")

    try:
        write_harnesses_from_response(prompt_path, response_path, output_dir, round_no=1)
    except ValueError as exc:
        assert "missing SERAPH markers" in str(exc)
    else:
        raise AssertionError("expected ValueError")
