import json

from seraph_rag.harness_prompt import build_prompt_bundle


def test_build_prompt_bundle_uses_fact_grounded_prompt_contract():
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

    assert "SERAPH's Rust fuzz harness generation expert" in bundle["system_prompt"]
    assert "`Known Reachable Paths` are validated, fact-grounded examples" in bundle["system_prompt"]
    assert "`Related APIs` are the main building blocks" in bundle["system_prompt"]
    assert "You may design your own setup and call sequence using the facts in the context." in bundle["user_prompt"]
    assert "Prefer `Related APIs` as the main construction pool." in bundle["user_prompt"]
    assert "Use `Known Reachable Paths` as validated anchors when helpful" in bundle["user_prompt"]


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
