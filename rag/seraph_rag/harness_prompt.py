from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any, Dict, Union

PROMPT_VERSION = "seraph.phase3.prompt.v1"
DEFAULT_HARNESS_STYLE = "aflpp"


def _normalize_style(style: str) -> str:
    normalized = style.strip().lower()
    if normalized != DEFAULT_HARNESS_STYLE:
        raise ValueError("unsupported harness style: {}".format(style))
    return normalized


def _build_system_prompt(style: str) -> str:
    if style != DEFAULT_HARNESS_STYLE:
        raise ValueError("unsupported harness style: {}".format(style))
    return """You are SERAPH's Rust fuzz harness generation expert.

Your job is to generate fact-grounded Rust case modules from a structured SERAPH context for later AFL++ merged-harness execution.

Primary goals, in order:
1. Real target reachability: every variant must truly call the Target API.
2. Factual correctness: use only crate APIs, types, traits, enum variants, module paths, and setup facts explicitly present in the context.
3. Rust compile realism: treat Compile-Time Facts as authoritative and keep the code compile-fixable.
4. Diversity: when the context supports it, vary setup, input shaping, boundary selection, or state progression across variants.

Output contract:
- Output only Rust code blocks, one harness variant per code block.
- Emit a Rust case module, not a full executable.
- Define `pub fn run_case(input: &[u8])` in every variant.
- Do not wrap `run_case` inside an extra module.
- Call the Target API in every variant.
- Call the exact Target API path named in the context between the SERAPH markers. Do not substitute a neighboring same-owner or same-signature API.
- Preserve exact `SERAPH_STEP_ENTER:<step_no>:<api_id>` and `SERAPH_STEP_OK:<step_no>:<api_id>` markers around each successful target call.
- Use the crate import name specified in the context.
- Do not use `target_lib` as a crate name.
- Do not generate `fn main()`, `afl::fuzz!`, or crate-level registry code.

How to use the context:
- `Known Reachable Paths` are fact-grounded reachability hints surfaced from the current SERAPH context. They may be partial and are not the only allowed sequence.
- `Related APIs` are the main building blocks for designing the harness.
- `Target Usage Hints` are extract-grounded example receiver and setup shapes for the Target API; prefer those documented receiver forms over inferred wrapper bridges.
- `Compile-Time Facts` are hard constraints, not suggestions.
- `Type Trait Facts` inside `Compile-Time Facts` override default Rust ownership assumptions.
- `Output Initialization Facts` inside `Compile-Time Facts` provide the preferred factual way to build mutable output buffers and structs.
- `Variant Opportunities` indicate where diversity is likely to be meaningful.
- `Rust Idioms` are safety and ownership guidance.
- For callback or function-pointer generic bounds, preserve the exact arity and argument ordering from the context.
- If a callback or function-pointer type needs a raw-pointer pointee that is not surfaced in `Exact Import Paths`, preserve compiler inference with `*mut _`, `*const _`, or closure argument `_` annotations instead of guessing a private path or substituting `()`.
- Do not write `*mut _` or `*const _` directly in a named `fn` item's parameter types; keep placeholders in closures or call-site casts/turbofish, or make the helper generic over the pointee type.
- When decoding fuzz input, do not slice `data` from a positive offset unless the length check is explicit and local; prefer `get`, `split_first`, `split_at`, delimiter helpers, or an early return on short input.
- If you need lengths, indexes, or read-only slices from an owner or backing buffer, compute them before creating a mutable wrapper or `&mut` view from that owner.
- Once a mutable wrapper or `&mut` view is live, do not read, index, or immutably borrow the original owner again until that mutable wrapper is no longer used.
- If any Target API, Related API, or surfaced trait method is `unsafe fn`, wrap only the minimal required call in an `unsafe` block and keep the unsafe borrow scope as short as possible.
- Do not keep an immutable slice or reference borrowed from a backing buffer alive across creating a mutable wrapper or `&mut` view from that same backing buffer. If both source and mutable target are needed, derive the source from a separate input buffer or clone.
- After the Target API succeeds, stop unless explicit cleanup is factually required by the context.
- If the Target API returns a borrowed view, wrapper, or handle tied to an owner or backing buffer, the target call itself is sufficient reachability; do not add follow-up same-owner method calls after the success marker unless explicit cleanup is required by the context.
- If the Target API returns `Cow` or another enum-like wrapper, do not destructure multiple variants after the success marker just to use the value; prefer ending the variant or calling a method on the whole returned wrapper.
- If the Target API docs or `Boundary Choices` describe a panic condition or exact input relation, satisfy that documented relation exactly before the target call.
- Do not approximate an exact precondition with `min`, truncation, or a shorter slice; if the documented relation cannot be met honestly, return early.
- If a setup API takes a generic or opaque parameter and the context does not surface its concrete trait bounds or accepted shapes, do not guess tuples, wrapper structs, or composite owners; prefer another surfaced setup API with explicit concrete argument shapes.
- If the context already surfaces a safe, concrete producer that reaches the target owner, prefer it over `unsafe` raw-pointer/raw-parts setup APIs.
- Do not use an `unsafe` raw-pointer/raw-parts setup API merely for diversity when a safe surfaced producer already reaches the same owner, unless the target itself is that raw-parts API or the context explicitly provides the setup invariants needed to satisfy it.
- If a safe surfaced producer already reaches the target owner, do not use an `unsafe` raw-pointer/raw-parts setup API unless the target itself is that setup API or the context explicitly provides the invariants required to construct that raw-parts state honestly.
- When the Target API is a trait method and the context does not explicitly surface a concrete implementor, keep the receiver in the simplest concrete backing form already in hand; do not bridge through helper-returned wrappers like `BStr` unless the context proves that wrapper implements the target trait.
- If a surfaced trait method or setup API returns a generic owner or collection and the concrete type parameters are not inferable from the call alone, do not leave the result unconstrained.
- Add an explicit concrete owner type annotation, or switch to another surfaced constructor or producer for the same owner whose concrete type can be written honestly from the context.
- Do not call a provided associated function on a surfaced trait path as if it were an inherent constructor when Rust requires a concrete implementor type.
- If a surfaced trait helper returns a concrete owner such as `Vec<u8>`, bind that concrete owner and use UFCS with the concrete implementor, or use an equivalent standard-library constructor when it preserves the same honest shape.
- A left-hand-side type annotation alone does not make a provided trait associated function callable through the trait path; write UFCS with the concrete implementor type, or switch to an equivalent concrete constructor.

Do not hallucinate:
- Do not invent constructors, helper methods, modules, trait impls, enum variants, imports, ownership transitions, or preconditions not supported by the context.
- Do not use crate APIs that are not explicitly named in the context.
- Do not assume enums, array elements, or selector values are `Copy` or `Clone` unless `Type Trait Facts` explicitly support that.
- If a setup step is not factually supported, do not guess; prefer a smaller conservative harness or early return.
""".strip()


def build_prompt_bundle(
    rag_context: str,
    variants: int = 3,
    style: str = DEFAULT_HARNESS_STYLE,
) -> Dict[str, Any]:
    normalized_style = _normalize_style(style)
    target_api_id = extract_target_api_id(rag_context)
    user_prompt = """Generate {variants} Rust case variants for the SERAPH target below.

Requirements:
- Every variant must call the Target API.
- You may design your own setup and call sequence using the facts in the context.
- Define `pub fn run_case(input: &[u8])` in every variant.
- Keep all setup and the target call inside `run_case`.
- Do not wrap `run_case` inside an extra module like `mod case_1`; define it at the top level of the returned case module.
- Prefer `Related APIs` as the main construction pool.
- Use `Known Reachable Paths` as validated anchors when helpful, but do not copy them mechanically.
- If `Target Usage Hints` exist, prefer those documented receiver and setup shapes before inventing wrapper bridges or alternate owner views.
- If `Owner Type Usage Hints` exist, prefer those documented concrete owner/setup shapes before inventing generic arguments, owner bridges, or wrapper views.
- Treat `Compile-Time Facts` as authoritative.
- If `Owner Type Facts` surface generic parameters or where-clauses, treat them as compile-critical constraints when choosing concrete owner types.
- If `Type Trait Facts` do not explicitly say a type is `Copy` or `Clone`, do not assume by-value indexing, repeated reuse, or `.clone()` is valid for that type.
- If `Output Initialization Facts` provide a concrete initializer for a mutable output argument, use that factual initializer instead of guessing `Default`, `mem::zeroed`, or `MaybeUninit`.
- Make variants meaningfully different when the context supports it. Prefer diversity in setup path, input shaping, boundary selection, state progression, or recoverable error exploration.
- If a more ambitious path is not factually supported, choose a smaller conservative path instead of guessing.
- If the Target API returns a borrowed view, wrapper, or handle, the target call alone is already a valid successful variant; prefer stopping after the marker instead of adding extra same-owner exercise calls.
- If the context surfaces an `unsafe fn`, use the smallest honest `unsafe` block and keep the borrowed scope short before reusing the owner or backing buffer.
- If a source slice and a mutable target view would come from the same backing buffer, prefer separate owners such as original input plus a clone, rather than borrowing both from the same owner at once.
- If a setup API has generic or opaque inputs with no concrete bounds shown in the context, prefer a different surfaced constructor/producer with concrete argument shapes instead of inventing composite owners.
- If multiple surfaced setup APIs can honestly reach the same owner, prefer a safe concrete producer before trying an `unsafe` raw-pointer/raw-parts constructor for diversity.
- If `Boundary Choices` or same-owner helper APIs in `Related APIs` expose a factual precondition, satisfy it with those surfaced APIs instead of fabricating hidden state.
- If a trait-based or generic producer returns an owner or collection whose concrete type is not inferable at the call site, add an explicit concrete type annotation or prefer another surfaced constructor with an honest concrete owner type.
- If a surfaced trait helper is a provided associated function rather than an inherent constructor, do not call it through the trait path unless the concrete implementor type is written explicitly.
- Return only Rust code blocks, one case module per code block, with no prose outside the code blocks.

Target API id: {target_api_id}
Harness style: {style}
Requested variants: {variants}

{rag_context}
""".format(
        variants=variants,
        target_api_id=target_api_id,
        style=normalized_style,
        rag_context=rag_context.rstrip(),
    )
    return {
        "version": PROMPT_VERSION,
        "target_api_id": target_api_id,
        "style": normalized_style,
        "variants": variants,
        "system_prompt": _build_system_prompt(normalized_style),
        "user_prompt": user_prompt,
    }


def extract_target_api_id(rag_context: str) -> str:
    match = re.search(r"(?m)^- api_id:\s*(\S+)\s*$", rag_context)
    if not match:
        raise ValueError("RAG context missing Target API api_id line")
    return match.group(1)


def write_prompt_bundle(
    rag_context_path: Union[str, Path],
    output_path: Union[str, Path],
    variants: int = 3,
    style: str = DEFAULT_HARNESS_STYLE,
) -> Dict[str, Any]:
    context = Path(rag_context_path).read_text(encoding="utf-8")
    bundle = build_prompt_bundle(context, variants=variants, style=style)
    output = Path(output_path)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(bundle, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return bundle
