from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Union

from seraph_rag.fix_acceptance import summarize_fix_acceptance_entries

VALID_STATUSES = {"accepted", "needs_review", "bug"}


def write_fix_acceptance_decision(
    index_path: Union[str, Path],
    harness_path: Union[str, Path],
    status: str,
    reason: str,
    source: str = "manual",
) -> Dict[str, Any]:
    path = Path(index_path)
    data = json.loads(path.read_text(encoding="utf-8"))
    _apply_decision(data, harness_path, status, reason, source)
    _finalize_index_data(data)
    _write_index(path, data)
    return data


def write_fix_acceptance_decisions(
    index_path: Union[str, Path],
    decisions_path: Union[str, Path],
    source: str = "manual",
) -> Dict[str, Any]:
    path = Path(index_path)
    data = json.loads(path.read_text(encoding="utf-8"))
    decisions = _load_decisions(decisions_path)
    seen = set()

    for idx, decision in enumerate(decisions):
        harness = decision.get("harness")
        if not harness:
            raise ValueError("decision missing harness at index {}".format(idx))
        harness_key = str(Path(harness))
        if harness_key in seen:
            raise ValueError("duplicate harness in decisions: {}".format(harness_key))
        seen.add(harness_key)
        _apply_decision(
            data,
            harness_key,
            decision.get("status"),
            decision.get("reason"),
            decision.get("source", source),
        )

    _finalize_index_data(data)
    _write_index(path, data)
    return data


def _load_decisions(decisions_path: Union[str, Path]) -> List[Dict[str, Any]]:
    raw = json.loads(Path(decisions_path).read_text(encoding="utf-8"))
    if isinstance(raw, dict):
        raw = raw.get("decisions")
    if not isinstance(raw, list):
        raise ValueError("decisions file must be a list or an object with a decisions list")
    for idx, item in enumerate(raw):
        if not isinstance(item, dict):
            raise ValueError("decision at index {} must be an object".format(idx))
    return raw


def _apply_decision(
    data: Dict[str, Any],
    harness_path: Union[str, Path],
    status: Any,
    reason: Any,
    source: str,
) -> None:
    if status not in VALID_STATUSES:
        raise ValueError("invalid status: {}".format(status))
    if not isinstance(reason, str) or not reason.strip():
        raise ValueError("reason must be a non-empty string")

    harness = str(Path(harness_path))

    entry = None
    for item in data.get("entries", []):
        if item.get("harness") == harness:
            entry = item
            break

    if entry is None:
        raise ValueError("harness not found in acceptance index: {}".format(harness))

    entry["status"] = status
    entry["reason"] = reason.strip()
    entry["source"] = source


def _finalize_index_data(data: Dict[str, Any]) -> None:
    data["entry_count"] = len(data.get("entries", []))
    data["status"] = summarize_fix_acceptance_entries(data.get("entries", []))


def _write_index(path: Path, data: Dict[str, Any]) -> None:
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
