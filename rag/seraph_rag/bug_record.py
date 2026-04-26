from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Union

_ROUND_FILE_RE = re.compile(r"_(\d{3})(?:_index)?\.")
_COMPILE_ERROR_RE = re.compile(r"error\[(E\d{4})\]")


def render_bug_record_entry(
    workspace_dir: Union[str, Path],
    *,
    crate_name: str,
    phase: str = "Phase 3",
    round_no: Optional[int] = None,
    title: Optional[str] = None,
    symptom: Optional[Sequence[str]] = None,
    rust_feature: Optional[Sequence[str]] = None,
    root_cause: Optional[Sequence[str]] = None,
    tool_fix: Optional[Sequence[str]] = None,
    capability_gain: Optional[Sequence[str]] = None,
) -> str:
    workspace = Path(workspace_dir)
    resolved_round = round_no or _discover_round_no(workspace)
    context_path = workspace / "contexts" / f"rag_target_{resolved_round:03d}.md"
    compile_index_path = workspace / "reports" / f"compile_{resolved_round:03d}_index.json"
    fix_loop_index_path = workspace / "reports" / f"fix_loop_{resolved_round:03d}_index.json"
    smoke_index_path = workspace / "reports" / f"smoke_{resolved_round:03d}_index.json"
    runtime_index_path = workspace / "reports" / f"runtime_error_{resolved_round:03d}_index.json"
    coverage_path = workspace / "coverage.json"

    target_api_id = _extract_target_api_id(context_path)
    compile_index = _load_json_if_exists(compile_index_path)
    fix_loop_index = _load_json_if_exists(fix_loop_index_path)
    smoke_index = _load_json_if_exists(smoke_index_path)
    runtime_index = _load_json_if_exists(runtime_index_path)
    coverage = _load_json_if_exists(coverage_path)
    compile_error_codes = _collect_compile_error_codes(compile_index)
    effective_compile_status = _effective_compile_status(compile_index, fix_loop_index)
    runtime_summaries = _runtime_summaries(runtime_index)

    related_total = len(_list_field(coverage, "related_total_api_ids"))
    related_covered = len(_list_field(coverage, "related_covered_api_ids"))
    target_total = len(_list_field(coverage, "total_api_ids"))
    target_covered = len(_list_field(coverage, "covered_api_ids"))

    lines = [
        f"### {title or _default_title(crate_name, target_api_id)}",
        "",
        f"- **crate / 阶段**：`{crate_name}`，{phase}",
        f"- **workspace**：`{workspace}`",
        f"- **round**：`{resolved_round}`",
        f"- **target API**：`{target_api_id}`",
        "- **状态摘要**：",
        f"  - compile：`{effective_compile_status}`",
        f"  - smoke：`{_index_status(smoke_index)}`",
        f"  - runtime diagnose：`{_index_status(runtime_index)}`",
    ]
    if fix_loop_index is not None:
        lines.append(f"  - compile 初始状态：`{_index_status(compile_index)}`")
        lines.append(f"  - fix-loop：`{_index_status(fix_loop_index)}`")
    if target_total:
        lines.append(f"  - target 覆盖：`{target_covered} / {target_total}`")
    if related_total:
        lines.append(f"  - related API 覆盖：`{related_covered} / {related_total}`")
    if compile_error_codes:
        lines.extend(["- **编译错误码**："])
        for error_code in compile_error_codes:
            lines.append(f"  - `{error_code}`")
    if runtime_summaries:
        lines.extend(["- **运行时诊断**："])
        for item in runtime_summaries:
            lines.append(f"  - `{item['classification']}`：{item['summary']}")
    _append_section(lines, "现象", symptom)
    _append_section(lines, "Rust 特性", rust_feature)
    _append_section(lines, "根因", root_cause)
    _append_section(lines, "工具修复", tool_fix)
    _append_section(lines, "能力提升", capability_gain)
    lines.extend(["- **证据**：", f"  - context：`{context_path}`"])
    if compile_index_path.exists():
        lines.append(f"  - compile index：`{compile_index_path}`")
    if fix_loop_index_path.exists():
        lines.append(f"  - fix-loop index：`{fix_loop_index_path}`")
    if smoke_index_path.exists():
        lines.append(f"  - smoke index：`{smoke_index_path}`")
    if runtime_index_path.exists():
        lines.append(f"  - runtime index：`{runtime_index_path}`")
    if coverage_path.exists():
        lines.append(f"  - coverage：`{coverage_path}`")
    return "\n".join(lines) + "\n"


def append_bug_record_entry(doc_path: Union[str, Path], markdown: str) -> Path:
    path = Path(doc_path)
    existing = path.read_text(encoding="utf-8") if path.exists() else ""
    separator = ""
    if existing and not existing.endswith("\n\n"):
        separator = "\n" if existing.endswith("\n") else "\n\n"
    path.write_text(existing + separator + markdown.rstrip() + "\n", encoding="utf-8")
    return path


def _append_section(lines: List[str], label: str, items: Optional[Sequence[str]]) -> None:
    lines.append(f"- **{label}**：")
    normalized_items = _normalize_items(items)
    if not normalized_items:
        normalized_items = ["待补充"]
    for item in normalized_items:
        lines.append(f"  - {item}")


def _normalize_items(items: Optional[Sequence[str]]) -> List[str]:
    if not items:
        return []
    normalized: List[str] = []
    for item in items:
        text = str(item).strip()
        if text:
            normalized.append(text)
    return normalized


def _default_title(crate_name: str, target_api_id: str) -> str:
    return f"`{crate_name}`：{target_api_id.rsplit('::', 1)[-1]}"


def _discover_round_no(workspace: Path) -> int:
    candidates: List[int] = []
    for pattern in (
        "contexts/rag_target_*.md",
        "reports/compile_*_index.json",
        "reports/smoke_*_index.json",
        "reports/runtime_error_*_index.json",
    ):
        for path in workspace.glob(pattern):
            match = _ROUND_FILE_RE.search(path.name)
            if match:
                candidates.append(int(match.group(1)))
    if not candidates:
        raise ValueError(f"unable to discover round number under {workspace}")
    return max(candidates)


def _extract_target_api_id(context_path: Path) -> str:
    for line in context_path.read_text(encoding="utf-8").splitlines():
        if line.strip().startswith("- api_id:"):
            return line.split(":", 1)[1].strip()
    raise ValueError(f"context missing target api_id: {context_path}")


def _collect_compile_error_codes(compile_index: Optional[Dict[str, Any]]) -> List[str]:
    if not compile_index:
        return []
    codes = set()
    for report_ref in compile_index.get("reports", []):
        report_path = report_ref.get("report")
        if not report_path:
            continue
        report = _load_json_if_exists(Path(report_path))
        if not report:
            continue
        for field in ("stderr", "stdout"):
            for error_code in _COMPILE_ERROR_RE.findall(str(report.get(field, ""))):
                codes.add(error_code)
    return sorted(codes)


def _index_status(index: Optional[Dict[str, Any]]) -> str:
    if not index:
        return "missing"
    return str(index.get("status") or "missing")


def _effective_compile_status(
    compile_index: Optional[Dict[str, Any]],
    fix_loop_index: Optional[Dict[str, Any]],
) -> str:
    compile_status = _index_status(compile_index)
    if compile_status == "ok":
        return "ok"
    if _index_status(fix_loop_index) == "ok":
        return "ok"
    return compile_status


def _list_field(value: Optional[Dict[str, Any]], key: str) -> List[Any]:
    if not isinstance(value, dict):
        return []
    field = value.get(key)
    if isinstance(field, list):
        return field
    return []


def _runtime_summaries(runtime_index: Optional[Dict[str, Any]]) -> List[Dict[str, str]]:
    if not isinstance(runtime_index, dict):
        return []
    summaries: List[Dict[str, str]] = []
    for report in runtime_index.get("reports", []):
        classification = str(report.get("classification") or "").strip()
        summary = str(report.get("summary") or "").strip()
        if classification or summary:
            summaries.append(
                {
                    "classification": classification or "runtime_issue",
                    "summary": summary or "runtime issue recorded",
                }
            )
    return summaries


def _load_json_if_exists(path: Path) -> Optional[Dict[str, Any]]:
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))
