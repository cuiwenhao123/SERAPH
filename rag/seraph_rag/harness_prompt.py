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
    return """You are a Rust fuzz harness expert.

Your task is to generate AFL++-friendly Rust harness variants from SERAPH RAG context.

Rules:
1. Generate a normal Rust binary harness with `fn main()`.
2. Read fuzz bytes from stdin or an optional input file path argument using only the Rust standard library.
3. Call the Target API in every harness variant.
4. Use public APIs from Related APIs to construct meaningful pre-state.
5. Keep unsafe blocks as narrow as possible.
6. Use early return for recoverable Result and Option paths; do not unwrap recoverable failures.
7. Emit SERAPH_STEP_ENTER:<step_no>:<api_id> immediately before each targeted API call.
8. Emit SERAPH_STEP_OK:<step_no>:<api_id> immediately after the targeted API call returns successfully.
9. Preserve exact stable api_id strings from the RAG context in SERAPH markers.
10. Use the crate import name specified by the RAG context.
11. Do not use `target_lib` as a crate name.
12. Do not use `libfuzzer_sys`, `#![no_main]`, `fuzz_target!`, or `afl::fuzz!`.
13. If perfect setup is impossible, emit the smallest compile-fixable harness that still reaches the target API.
14. Use only crate APIs explicitly named in the SERAPH RAG context.
15. Do not invent constructors, helper methods, trait impls, modules, or functions that are not present in the context.
16. If setup is incomplete, prefer a conservative harness that returns early over invented API calls.
17. Do not rename modules or types from the context.
18. Prefer Required Setup APIs when an opaque wrapper, borrowed handle, or deref-backed owner value is needed.
19. After the target API succeeds, stop unless an explicit cleanup step is required.
20. Do not add extra post-target exercise calls.
21. Do not fabricate enum constructors, transmute arbitrary integers, or use unsafe initialization tricks to invent missing values.
22. Use exact canonical module paths from `Exact Import Paths`; do not shorten imports to the crate root unless that exact root path appears there.
23. If you implement a trait from `Required Traits`, implement every listed required method. Listed provided methods are optional overrides, not mandatory.
24. If you override a trait method listed in `Trait Method Signatures`, copy the exact implementation-ready signature from `Trait Method Signatures`.
25. For trait-method targets with a `Self` receiver, call the target on a concrete implementor surfaced by `Required Setup APIs`, `Exact Import Paths`, or `Required Traits`; do not assume similarly named wrapper types implement the trait.
26. If you match or construct an enum from `Enum Variants`, cover every listed variant unless the context marks that enum as non-exhaustive.
27. Do not create typed function-pointer bindings, `std::mem::size_of` placeholders, `PhantomData`, or dead helper functions merely to reference APIs or lifetime-bearing types.
28. If setup is unavailable, return early; do not fake reachability by mentioning APIs without calling them.
29. Do not use `std::process::exit`, `panic!`, `unreachable!`, `todo!`, or `unimplemented!` inside placeholder helpers to fabricate missing values or references.
30. Do not use `MaybeUninit`, `mem::zeroed`, `transmute`, `Box::into_raw`, or similar unsafe initialization tricks to fabricate missing target state unless the target or setup signatures explicitly require them.
31. Treat `Exact Import Paths`, `Required Traits`, `Trait Method Signatures`, and `Enum Variants` as authoritative compile-time facts.
32. Before creating an `&mut` borrow, mutable slice view, or wrapper over some owner value, first compute any indexes, lengths, cloned source buffers, or read-only bytes you still need from that owner.
33. After creating an `&mut` borrow into a value, do not read, slice, or immutably borrow the original owner again until that mutable borrow is no longer used.
34. For constructors or adapters like `Type::new(owner.as_mut_slice())` that return an `&mut` wrapper, compute lengths, indexes, and any source bytes before the call, and do not read `owner` again until that wrapper is no longer used.
""".strip()


def build_prompt_bundle(
    rag_context: str,
    variants: int = 3,
    style: str = DEFAULT_HARNESS_STYLE,
) -> Dict[str, Any]:
    normalized_style = _normalize_style(style)
    target_api_id = extract_target_api_id(rag_context)
    user_prompt = """Generate {variants} harness variants from the SERAPH RAG context below.

Each variant must use a different setup, input, or boundary strategy when the context supports it. Return only Rust code blocks, one per variant. Do not explain the code outside comments that belong in the harness source.
Treat `Exact Import Paths`, `Required Traits`, `Trait Method Signatures`, and `Enum Variants` in the RAG context as authoritative compile-time facts.

Target API id: {target_api_id}
Harness style: {style}

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
