from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any, Dict, List, Union


def write_compile_fixer_bundles(
    compile_index_path: Union[str, Path],
    rag_context_path: Union[str, Path],
    output_dir: Union[str, Path],
) -> List[Path]:
    index = json.loads(Path(compile_index_path).read_text(encoding="utf-8"))
    context = Path(rag_context_path).read_text(encoding="utf-8")
    target_api_id = _extract_target_api_id(context)
    written: List[Path] = []
    for position, summary in enumerate(index.get("reports", []), start=1):
        if summary.get("status") == "ok":
            continue
        report_path = Path(summary["report"])
        report = json.loads(report_path.read_text(encoding="utf-8"))
        harness_path = Path(report.get("harness") or summary.get("harness"))
        bundle = _build_bundle(
            round_no=int(index.get("round", 0)),
            variant=position,
            target_api_id=target_api_id,
            context=context,
            harness_path=harness_path,
            report=report,
        )
        output = Path(output_dir)
        output.mkdir(parents=True, exist_ok=True)
        bundle_path = output / "fix_request_{:03d}_{:02d}.json".format(bundle["round"], position)
        bundle_path.write_text(json.dumps(bundle, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        written.append(bundle_path)
    return written


def _build_bundle(
    round_no: int,
    variant: int,
    target_api_id: str,
    context: str,
    harness_path: Path,
    report: Dict[str, Any],
) -> Dict[str, Any]:
    return {
        "version": "seraph.phase3.compile_fixer_request.v1",
        "round": round_no,
        "variant": variant,
        "target_api_id": target_api_id,
        "rag_context": context,
        "harness_path": str(harness_path),
        "harness_source": harness_path.read_text(encoding="utf-8"),
        "diagnostics": {
            "command": report.get("command", ""),
            "exit_code": report.get("exit_code"),
            "stdout": report.get("stdout", ""),
            "stderr": report.get("stderr", ""),
        },
        "rules": [
            "Do not remove the target API call.",
            "Do not remove or rename SERAPH_STEP_ENTER markers.",
            "Do not remove or rename SERAPH_STEP_OK markers.",
            "Preserve exact api_id strings in markers.",
            "Prefer compiler-directed edits over semantic rewrites.",
            "Only use crate APIs explicitly present in the rag_context or harness_source.",
            "Do not invent new crate APIs to satisfy compiler errors.",
            "If the current setup is insufficient, prefer deleting invented calls or returning early.",
            "Do not rename modules or types from the rag_context.",
            "Treat Known Reachable Paths as fact-grounded reachability/setup hints for opaque wrappers and borrowed handles.",
            "Treat Type Trait Facts in the rag_context as authoritative for Copy/Clone and ownership assumptions.",
            "If Output Initialization Facts provide a concrete initializer for a mutable output type, use that factual initializer instead of inventing `Default`, `mem::zeroed`, or `MaybeUninit`.",
            "If a callback or function-pointer type needs a raw-pointer pointee that is not surfaced in Exact Import Paths, preserve compiler inference with `*mut _`, `*const _`, or closure argument `_` annotations instead of guessing a private path or substituting `()`.",
            "Do not write `*mut _` or `*const _` directly in a named `fn` item's parameter types; use a closure, keep the placeholder at the call site, or make the helper generic over the pointee type.",
            "When fixing fuzz input parsing, do not keep direct positive-offset slices like `data[1..]` or `data[idx..idx + take]` unless a local bounds proof is explicit; prefer `get`, `split_first`, `split_at`, or an early return on short input.",
            "If you need lengths, indexes, or read-only slices from an owner or backing buffer, compute them before creating a mutable wrapper or `&mut` view from that owner.",
            "Once a mutable wrapper or `&mut` view is live, do not read, index, or immutably borrow the original owner again until that mutable wrapper is no longer used.",
            "If the rag_context surfaces an `unsafe fn`, wrap only the minimal required call in `unsafe` and shorten the live mutable borrow before reusing the owner.",
            "If a semantic guard reports that the real target call is missing, restore the exact target API named by target_api_id instead of a neighboring same-owner substitute.",
            "Do not keep a read-only slice or reference from the same backing owner alive across constructing a mutable target view; move the source to separate input/backing or end the immutable borrow first.",
            "If a surfaced setup API has a generic or opaque input and the rag_context does not show concrete bounds or accepted shapes, delete guessed tuple/composite owners and prefer an explicit concrete setup API already present.",
            "If the rag_context already provides a safe concrete producer for the target owner, replace unnecessary `unsafe` raw-pointer/raw-parts setup with that safe producer.",
            "Delete `unsafe` raw-pointer/raw-parts setup that was chosen only for diversity when a safe surfaced producer reaches the same owner and the required raw-parts invariants are not explicitly surfaced.",
            "If a safe surfaced producer already reaches the target owner, delete `unsafe` raw-pointer/raw-parts setup unless the rag_context explicitly provides the invariants needed to construct that raw-parts state honestly.",
            "If rustc reports that a helper-returned wrapper or view does not implement the surfaced target trait, delete that wrapper bridge and call the trait method on the simpler concrete backing owner already in scope, unless the rag_context explicitly proves the wrapper implements the trait.",
            "If rustc reports that a trait-based or generic producer leaves an owner or collection type unconstrained, add an explicit concrete owner type annotation or replace it with another surfaced constructor or producer for the same owner whose concrete type can be written honestly.",
            "If rustc reports that a provided associated function on a trait cannot be called without a concrete implementor type, rewrite it using UFCS with the surfaced concrete owner type or replace it with an equivalent standard-library constructor that preserves the same honest shape.",
            "A left-hand-side type annotation alone is not enough to call a provided associated function through a trait path; if rustc still rejects the call, rewrite the constructor with UFCS or replace it with an equivalent concrete constructor.",
            "Do not add `.clone()` to fix move errors unless the rag_context explicitly shows that the moved type implements `Clone`.",
            "For non-`Copy` selector enums or structs, prefer borrow-preserving rewrites, branch-local construction, or consuming the owned value exactly once.",
            "After the target API succeeds, stop unless explicit cleanup is required.",
            "If the target API returns a borrowed view, wrapper, or handle tied to an owner or backing buffer, treat the target call itself as sufficient reachability and delete follow-up same-owner method calls unless explicit cleanup is required.",
            "If rustc reports inconsistent bindings across `|` pattern alternatives on a returned `Cow` or other enum wrapper, delete that post-target destructuring or replace it with a whole-value method call; do not force different variant payload types into one shared binding.",
            "If the target API docs or Boundary Choices describe a panic condition or exact input relation, satisfy that documented relation exactly before the target call.",
            "Do not approximate an exact precondition with `min`, truncation, or a shorter slice; if the documented relation cannot be met honestly, return early.",
            "Delete extra post-target exercise calls that trigger unrelated invariant failures.",
            "Do not fabricate constructors, enum values, transmute hacks, or unsafe initialization tricks.",
            "Do not use std::process::exit, panic!, unreachable!, todo!, or unimplemented! inside placeholder helpers to fabricate missing values or references.",
            "Do not use MaybeUninit, mem::zeroed, transmute, Box::into_raw, or similar unsafe initialization tricks to fabricate missing target state unless the target or setup signatures explicitly require them.",
        ],
    }


def _extract_target_api_id(rag_context: str) -> str:
    match = re.search(r"(?m)^- api_id:\s*(\S+)\s*$", rag_context)
    if not match:
        raise ValueError("RAG context missing Target API api_id line")
    return match.group(1)
