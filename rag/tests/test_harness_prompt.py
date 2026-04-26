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

    assert bundle["version"] == "seraph.phase3.prompt.v1"
    assert bundle["target_api_id"] == "api::fixture::Buffer::get_unchecked"
    assert bundle["style"] == "aflpp"
    assert bundle["variants"] == 2
    assert bundle["system_prompt"].startswith("You are SERAPH's Rust fuzz harness generation expert.")
    assert (
        "`Known Reachable Paths` are fact-grounded reachability hints surfaced from the current SERAPH "
        "context. They may be partial and are not the only allowed sequence."
        in bundle["system_prompt"]
    )
    assert "`Related APIs` are the main building blocks" in bundle["system_prompt"]
    assert "Generate a normal Rust binary with `fn main()`." in bundle["system_prompt"]
    assert (
        "Read fuzz bytes from stdin or an optional file path argument using only the Rust standard library."
        in bundle["system_prompt"]
    )
    assert (
        "Preserve exact `SERAPH_STEP_ENTER:<step_no>:<api_id>` and "
        "`SERAPH_STEP_OK:<step_no>:<api_id>` markers around each successful target call."
        in bundle["system_prompt"]
    )
    assert "Do not use `target_lib` as a crate name." in bundle["system_prompt"]
    assert "You may design your own setup and call sequence using the facts in the context." in bundle["user_prompt"]
    assert "Prefer `Related APIs` as the main construction pool." in bundle["user_prompt"]
    assert (
        "Use `Known Reachable Paths` as fact-grounded reachability hints when helpful, but do not copy them mechanically."
        in bundle["user_prompt"]
    )
    assert "Keep all logic inside a normal Rust binary `fn main()`." in bundle["user_prompt"]
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
