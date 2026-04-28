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
    assert (
        "`Target Usage Hints` are extract-grounded example receiver and setup shapes for the Target API; prefer those documented receiver forms over inferred wrapper bridges."
        in bundle["system_prompt"]
    )
    assert "`Type Trait Facts` inside `Compile-Time Facts` override default Rust ownership assumptions." in bundle["system_prompt"]
    assert (
        "`Output Initialization Facts` inside `Compile-Time Facts` provide the preferred factual way to build "
        "mutable output buffers and structs."
        in bundle["system_prompt"]
    )
    assert (
        "Do not assume enums, array elements, or selector values are `Copy` or `Clone` unless `Type Trait Facts` explicitly support that."
        in bundle["system_prompt"]
    )
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
    assert (
        "If `Type Trait Facts` do not explicitly say a type is `Copy` or `Clone`, do not assume by-value indexing, repeated reuse, or `.clone()` is valid for that type."
        in bundle["user_prompt"]
    )
    assert (
        "If `Output Initialization Facts` provide a concrete initializer for a mutable output argument, use that factual initializer instead of guessing `Default`, `mem::zeroed`, or `MaybeUninit`."
        in bundle["user_prompt"]
    )
    assert (
        "If a callback or function-pointer type needs a raw-pointer pointee that is not surfaced in `Exact Import Paths`, preserve compiler inference with `*mut _`, `*const _`, or closure argument `_` annotations instead of guessing a private path or substituting `()`."
        in bundle["system_prompt"]
    )
    assert (
        "Do not write `*mut _` or `*const _` directly in a named `fn` item's parameter types; keep placeholders in closures or call-site casts/turbofish, or make the helper generic over the pointee type."
        in bundle["system_prompt"]
    )
    assert (
        "When decoding fuzz input, do not slice `data` from a positive offset unless the length check is explicit and local; prefer `get`, `split_first`, `split_at`, delimiter helpers, or an early return on short input."
        in bundle["system_prompt"]
    )
    assert (
        "If you need lengths, indexes, or read-only slices from an owner or backing buffer, compute them before creating a mutable wrapper or `&mut` view from that owner."
        in bundle["system_prompt"]
    )
    assert (
        "Once a mutable wrapper or `&mut` view is live, do not read, index, or immutably borrow the original owner again until that mutable wrapper is no longer used."
        in bundle["system_prompt"]
    )
    assert (
        "After the Target API succeeds, stop unless explicit cleanup is factually required by the context."
        in bundle["system_prompt"]
    )
    assert (
        "If the Target API returns a borrowed view, wrapper, or handle tied to an owner or backing buffer, the target call itself is sufficient reachability; do not add follow-up same-owner method calls after the success marker unless explicit cleanup is required by the context."
        in bundle["system_prompt"]
    )
    assert (
        "If the Target API docs or `Boundary Choices` describe a panic condition or exact input relation, satisfy that documented relation exactly before the target call."
        in bundle["system_prompt"]
    )
    assert (
        "Do not approximate an exact precondition with `min`, truncation, or a shorter slice; if the documented relation cannot be met honestly, return early."
        in bundle["system_prompt"]
    )
    assert (
        "If any Target API, Related API, or surfaced trait method is `unsafe fn`, wrap only the minimal required call in an `unsafe` block and keep the unsafe borrow scope as short as possible."
        in bundle["system_prompt"]
    )
    assert (
        "Call the exact Target API path named in the context between the SERAPH markers. Do not substitute a neighboring same-owner or same-signature API."
        in bundle["system_prompt"]
    )
    assert (
        "Do not keep an immutable slice or reference borrowed from a backing buffer alive across creating a mutable wrapper or `&mut` view from that same backing buffer. If both source and mutable target are needed, derive the source from a separate input buffer or clone."
        in bundle["system_prompt"]
    )
    assert (
        "If a setup API takes a generic or opaque parameter and the context does not surface its concrete trait bounds or accepted shapes, do not guess tuples, wrapper structs, or composite owners; prefer another surfaced setup API with explicit concrete argument shapes."
        in bundle["system_prompt"]
    )
    assert (
        "If the context already surfaces a safe, concrete producer that reaches the target owner, prefer it over `unsafe` raw-pointer/raw-parts setup APIs."
        in bundle["system_prompt"]
    )
    assert (
        "Do not use an `unsafe` raw-pointer/raw-parts setup API merely for diversity when a safe surfaced producer already reaches the same owner, unless the target itself is that raw-parts API or the context explicitly provides the setup invariants needed to satisfy it."
        in bundle["system_prompt"]
    )
    assert (
        "If a safe surfaced producer already reaches the target owner, do not use an `unsafe` raw-pointer/raw-parts setup API unless the target itself is that setup API or the context explicitly provides the invariants required to construct that raw-parts state honestly."
        in bundle["system_prompt"]
    )
    assert (
        "If a surfaced trait method or setup API returns a generic owner or collection and the concrete type parameters are not inferable from the call alone, do not leave the result unconstrained."
        in bundle["system_prompt"]
    )
    assert (
        "Add an explicit concrete owner type annotation, or switch to another surfaced constructor or producer for the same owner whose concrete type can be written honestly from the context."
        in bundle["system_prompt"]
    )
    assert (
        "If a trait-based or generic producer returns an owner or collection whose concrete type is not inferable at the call site, add an explicit concrete type annotation or prefer another surfaced constructor with an honest concrete owner type."
        in bundle["user_prompt"]
    )
    assert (
        "Do not call a provided associated function on a surfaced trait path as if it were an inherent constructor when Rust requires a concrete implementor type."
        in bundle["system_prompt"]
    )
    assert (
        "If a surfaced trait helper returns a concrete owner such as `Vec<u8>`, bind that concrete owner and use UFCS with the concrete implementor, or use an equivalent standard-library constructor when it preserves the same honest shape."
        in bundle["system_prompt"]
    )
    assert (
        "A left-hand-side type annotation alone does not make a provided trait associated function callable through the trait path; write UFCS with the concrete implementor type, or switch to an equivalent concrete constructor."
        in bundle["system_prompt"]
    )
    assert (
        "If a surfaced trait helper is a provided associated function rather than an inherent constructor, do not call it through the trait path unless the concrete implementor type is written explicitly."
        in bundle["user_prompt"]
    )
    assert (
        "If the Target API returns `Cow` or another enum-like wrapper, do not destructure multiple variants after the success marker just to use the value; prefer ending the variant or calling a method on the whole returned wrapper."
        in bundle["system_prompt"]
    )
    assert (
        "When the Target API is a trait method and the context does not explicitly surface a concrete implementor, keep the receiver in the simplest concrete backing form already in hand; do not bridge through helper-returned wrappers like `BStr` unless the context proves that wrapper implements the target trait."
        in bundle["system_prompt"]
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
