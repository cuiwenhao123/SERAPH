import json

from seraph_rag.harness_prompt import build_prompt_bundle


def test_build_prompt_bundle_preserves_rag_context_and_marker_rules():
    context = """# SERAPH RAG Harness Context

## Target API
- api_id: api::fixture::Buffer::get_unchecked
- path: fixture::Buffer::get_unchecked
- signature: unsafe fn get_unchecked(&self, index: usize) -> u8

## Exact Import Paths
- type::fixture::avutil::Dictionary => fixture::avutil::Dictionary [kind=type]
- trait::fixture::io::IoContext => fixture::io::IoContext [kind=trait]
- type::fixture::io::Whence => fixture::io::Whence [kind=type]

## Required Traits
- fixture::io::IoContext: required_methods=buf_len; provided_methods=read, seek

## Trait Method Signatures
- fixture::io::IoContext::seek: fn seek(&mut self, i64, Whence, bool) -> Result<u64, Error> [provided]

## Enum Variants
- fixture::io::Whence: Size | Set | Cur | End

## Related APIs
- api::fixture::Buffer::new: fixture::Buffer::new — fn new() -> Buffer [role=constructor]

## Generation Rules
- Use crate import name `fixture`.
- Call the target API in every harness variant.
- Emit `SERAPH_STEP_ENTER:<step_no>:<api_id>` before each targeted API call.
- Emit `SERAPH_STEP_OK:<step_no>:<api_id>` after successful return.
"""

    bundle = build_prompt_bundle(context, variants=3)

    assert bundle["version"] == "seraph.phase3.prompt.v1"
    assert bundle["target_api_id"] == "api::fixture::Buffer::get_unchecked"
    assert bundle["style"] == "aflpp"
    assert bundle["variants"] == 3
    assert "Rust fuzz harness expert" in bundle["system_prompt"]
    assert "AFL++" in bundle["system_prompt"]
    assert "normal Rust binary harness with `fn main()`" in bundle["system_prompt"]
    assert "stdin or an optional input file path argument" in bundle["system_prompt"]
    assert "Do not use `libfuzzer_sys`" in bundle["system_prompt"]
    assert "Use only crate APIs explicitly named in the SERAPH RAG context" in bundle["system_prompt"]
    assert "Do not invent constructors" in bundle["system_prompt"]
    assert "Do not rename modules or types from the context" in bundle["system_prompt"]
    assert "Use exact canonical module paths from `Exact Import Paths`" in bundle["system_prompt"]
    assert "implement every listed required method" in bundle["system_prompt"]
    assert "copy the exact implementation-ready signature from `Trait Method Signatures`" in bundle["system_prompt"]
    assert "cover every listed variant" in bundle["system_prompt"]
    assert "Do not create typed function-pointer" in bundle["system_prompt"]
    assert "Do not use `std::process::exit`, `panic!`, `unreachable!`, `todo!`, or `unimplemented!`" in bundle["system_prompt"]
    assert "After the target API succeeds, stop unless an explicit cleanup step is required" in bundle["system_prompt"]
    assert "Before creating an `&mut` borrow" in bundle["system_prompt"]
    assert "do not read, slice, or immutably borrow the original owner again" in bundle["system_prompt"]
    assert "For trait-method targets with a `Self` receiver" in bundle["system_prompt"]
    assert "Do not use `MaybeUninit`" in bundle["system_prompt"]
    assert "unless the target or setup signatures explicitly require them" in bundle["system_prompt"]
    assert "owner.as_mut_slice()" in bundle["system_prompt"]
    assert "compute lengths, indexes, and any source bytes before the call" in bundle["system_prompt"]
    assert "Generate 3 harness variants" in bundle["user_prompt"]
    assert "api::fixture::Buffer::get_unchecked" in bundle["user_prompt"]
    assert "SERAPH_STEP_ENTER:<step_no>:<api_id>" in bundle["user_prompt"]
    assert "authoritative compile-time facts" in bundle["user_prompt"]
    assert "Do not use `target_lib`" in bundle["system_prompt"]


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
