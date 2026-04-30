import json

from seraph_rag.harness_prompt import build_prompt_bundle


def test_build_prompt_bundle_requests_run_case_modules():
    context = """# SERAPH Rust Harness Context

## Crate Facts
- crate_name: fixture
- crate_import_name: fixture
- target_crate_kind: library

## Target API
- api_id: api::fixture::Buffer::get_unchecked
- path: fixture::Buffer::get_unchecked
- signature: unsafe fn get_unchecked(&self, index: usize) -> u8

## Known Reachable Paths
- api::fixture::Buffer::new: fixture::Buffer::new — fn new() -> Buffer [goal=construct_owner basis=producer_chain depth=0]

## Related APIs
- api::fixture::Buffer::len: fixture::Buffer::len — fn len(&self) -> usize [roles=accessor relation=graph_neighbor]

## Compile-Time Facts
### Exact Import Paths
- type::fixture::Buffer => fixture::Buffer [kind=type]
"""

    bundle = build_prompt_bundle(context, variants=2)

    assert bundle["version"] == "seraph.phase3.prompt.v1"
    assert bundle["target_api_id"] == "api::fixture::Buffer::get_unchecked"
    assert bundle["style"] == "aflpp"
    assert bundle["variants"] == 2
    assert bundle["system_prompt"].startswith("You are SERAPH's Rust fuzz harness generation expert.")
    assert "Output only Rust code blocks, one harness variant per code block." in bundle["system_prompt"]
    assert "Emit a Rust case module, not a full executable." in bundle["system_prompt"]
    assert "Define `pub fn run_case(input: &[u8])` in every variant." in bundle["system_prompt"]
    assert "Do not wrap `run_case` inside an extra module." in bundle["system_prompt"]
    assert "Do not generate `fn main()`, `afl::fuzz!`, or crate-level registry code." in bundle["system_prompt"]
    assert "You may design your own setup and call sequence using the facts in the context." in bundle["user_prompt"]
    assert "Prefer `Related APIs` as the main construction pool." in bundle["user_prompt"]
    assert (
        "Use `Known Reachable Paths` as validated anchors when helpful, but do not copy them mechanically."
        in bundle["user_prompt"]
    )
    assert "Define `pub fn run_case(input: &[u8])` in every variant." in bundle["user_prompt"]
    assert "Keep all setup and the target call inside `run_case`." in bundle["user_prompt"]
    assert "If `Owner Type Usage Hints` exist, prefer those documented concrete owner/setup shapes" in bundle["user_prompt"]
    assert "Do not wrap `run_case` inside an extra module like `mod case_1`" in bundle["user_prompt"]
    assert "Return only Rust code blocks, one case module per code block" in bundle["user_prompt"]
    assert context.rstrip() in bundle["user_prompt"]


def test_prompt_bundle_is_json_serializable():
    bundle = build_prompt_bundle("## Target API\n- api_id: api::fixture::danger\n")

    encoded = json.dumps(bundle, sort_keys=True)

    assert "api::fixture::danger" in encoded


def test_build_prompt_bundle_accepts_explicit_aflpp_style():
    bundle = build_prompt_bundle(
        "## Target API\n- api_id: api::fixture::danger\n",
        variants=2,
        style="aflpp",
    )

    assert bundle["style"] == "aflpp"
    assert "AFL++" in bundle["system_prompt"]
