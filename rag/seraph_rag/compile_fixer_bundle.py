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
            "Prefer Required Setup APIs for opaque wrappers and borrowed handles.",
            "After the target API succeeds, stop unless explicit cleanup is required.",
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
