from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional, Union


def write_fix_acceptance_index(
    fix_loop_index_path: Union[str, Path],
    smoke_index_path: Optional[Union[str, Path]],
    output_path: Union[str, Path],
) -> Dict[str, Any]:
    fix_loop_index = json.loads(Path(fix_loop_index_path).read_text(encoding="utf-8"))
    smoke_reports = _load_smoke_reports(smoke_index_path)
    entries: List[Dict[str, Any]] = []

    for loop in fix_loop_index.get("loops", []):
        if loop.get("status") != "ok":
            continue
        detail_path = loop.get("report")
        if not detail_path:
            continue
        detail = json.loads(Path(detail_path).read_text(encoding="utf-8"))
        successful_attempt = detail.get("successful_attempt")
        selected = None
        for attempt in detail.get("attempts", []):
            if attempt.get("attempt") == successful_attempt:
                selected = attempt
                break
        if selected is None:
            continue

        harness = str(selected["harness"])
        smoke = smoke_reports.get(harness)
        entry = {
            "harness": harness,
            "compile_report": selected.get("report"),
            "smoke_report": smoke.get("report") if smoke else None,
            "status": "accepted",
            "reason": "smoke_ok",
            "source": "auto",
        }
        if smoke is None:
            entry["status"] = "needs_review"
            entry["reason"] = "smoke_missing"
        else:
            entry["runtime_status"] = smoke.get("status")
            entry["runtime_classification"] = smoke.get("classification")
            if smoke.get("status") == "bug":
                entry["status"] = "bug"
                entry["reason"] = smoke.get("classification") or "runtime_bug"
            elif smoke.get("status") == "review":
                entry["status"] = "needs_review"
                entry["reason"] = smoke.get("classification") or "needs_review"
        entries.append(entry)

    result = {
        "version": "seraph.phase3.fix_acceptance_index.v1",
        "round": _derive_round(fix_loop_index, entries),
        "status": summarize_fix_acceptance_entries(entries),
        "entry_count": len(entries),
        "entries": entries,
    }
    output = Path(output_path)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def _load_smoke_reports(smoke_index_path: Optional[Union[str, Path]]) -> Dict[str, Dict[str, Any]]:
    if not smoke_index_path:
        return {}
    path = Path(smoke_index_path)
    if not path.exists():
        return {}
    smoke_index = json.loads(path.read_text(encoding="utf-8"))
    reports: Dict[str, Dict[str, Any]] = {}
    for report in smoke_index.get("reports", []):
        harness = report.get("harness")
        if harness:
            reports[str(harness)] = report
    return reports


def _derive_round(fix_loop_index: Dict[str, Any], entries: List[Dict[str, Any]]) -> int:
    if isinstance(fix_loop_index.get("round"), int):
        return int(fix_loop_index["round"])
    for entry in entries:
        harness = Path(entry["harness"]).stem
        parts = harness.split("_")
        if len(parts) >= 3 and parts[0] == "harness":
            try:
                return int(parts[1])
            except ValueError:
                continue
    return 0


def summarize_fix_acceptance_entries(entries: List[Dict[str, Any]]) -> str:
    if any(entry.get("status") == "bug" for entry in entries):
        return "bug"
    if any(entry.get("status") == "needs_review" for entry in entries):
        return "needs_review"
    if entries:
        return "accepted"
    return "skipped"
