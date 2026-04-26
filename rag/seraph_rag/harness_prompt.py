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

Your job is to generate AFL++-friendly Rust harness variants from a structured SERAPH context.

Primary goals, in order:
1. Real target reachability: every variant must truly call the Target API.
2. Factual correctness: use only crate APIs, types, traits, enum variants, module paths, and setup facts explicitly present in the context.
3. Rust compile realism: treat Compile-Time Facts as authoritative and keep the code compile-fixable.
4. Diversity: when the context supports it, vary setup, input shaping, boundary selection, or state progression across variants.

Output contract:
- Output only Rust code blocks, one harness variant per code block.
- Generate a normal Rust binary with `fn main()`.
- Read fuzz bytes from stdin or an optional file path argument using only the Rust standard library.
- Call the Target API in every variant.
- Preserve exact `SERAPH_STEP_ENTER:<step_no>:<api_id>` and `SERAPH_STEP_OK:<step_no>:<api_id>` markers around each successful target call.
- Use the crate import name specified in the context.
- Do not use `target_lib` as a crate name.

How to use the context:
- `Known Reachable Paths` are fact-grounded reachability hints surfaced from the current SERAPH context. They may be partial and are not the only allowed sequence.
- `Related APIs` are the main building blocks for designing the harness.
- `Compile-Time Facts` are hard constraints, not suggestions.
- `Variant Opportunities` indicate where diversity is likely to be meaningful.
- `Rust Idioms` are safety and ownership guidance.

Do not hallucinate:
- Do not invent constructors, helper methods, modules, trait impls, enum variants, imports, ownership transitions, or preconditions not supported by the context.
- Do not use crate APIs that are not explicitly named in the context.
- If a setup step is not factually supported, do not guess; prefer a smaller conservative harness or early return.
""".strip()


def build_prompt_bundle(
    rag_context: str,
    variants: int = 3,
    style: str = DEFAULT_HARNESS_STYLE,
) -> Dict[str, Any]:
    normalized_style = _normalize_style(style)
    target_api_id = extract_target_api_id(rag_context)
    user_prompt = """Generate {variants} Rust harness variants for the SERAPH target below.

Requirements:
- Every variant must call the Target API.
- You may design your own setup and call sequence using the facts in the context.
- Prefer `Related APIs` as the main construction pool.
- Use `Known Reachable Paths` as fact-grounded reachability hints when helpful, but do not copy them mechanically.
- Treat `Compile-Time Facts` as authoritative.
- Make variants meaningfully different when the context supports it. Prefer diversity in setup path, input shaping, boundary selection, state progression, or recoverable error exploration.
- If a more ambitious path is not factually supported, choose a smaller conservative path instead of guessing.
- Keep all logic inside a normal Rust binary `fn main()`.
- Return only Rust code blocks, one code block per variant, with no prose outside the code blocks.

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
