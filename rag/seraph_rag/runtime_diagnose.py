from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple, Union

from seraph_rag.model_provider import write_model_response

SANITIZER_MARKERS = (
    "AddressSanitizer",
    "UndefinedBehaviorSanitizer",
    "MemorySanitizer",
    "ThreadSanitizer",
    "LeakSanitizer",
    "Sanitizer:",
)


def run_runtime_diagnosis(
    context_path: Union[str, Path],
    smoke_index_path: Union[str, Path],
    output_dir: Union[str, Path],
    round_no: int,
    command_template: str,
) -> Dict[str, Any]:
    context_file = Path(context_path)
    smoke_index_file = Path(smoke_index_path)
    out_dir = Path(output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    target_api_id = _extract_target_api_id(context_file)
    context_markdown = context_file.read_text(encoding="utf-8")
    smoke_index = json.loads(smoke_index_file.read_text(encoding="utf-8"))

    reports: List[Dict[str, Any]] = []
    bug_count = 0

    for smoke_ref in smoke_index.get("reports", []):
        if smoke_ref.get("status") == "ok":
            continue

        smoke_report_path = Path(smoke_ref["report"])
        smoke_report = json.loads(smoke_report_path.read_text(encoding="utf-8"))
        harness_path = Path(smoke_report["harness"])
        variant, attempt = _parse_harness_identity(harness_path)
        suffix = _report_suffix(harness_path, round_no)

        request_path = out_dir / f"runtime_request_{round_no:03d}_{suffix}.json"
        response_path = out_dir / f"runtime_response_{round_no:03d}_{suffix}.json"
        detail_path = out_dir / f"runtime_error_{round_no:03d}_{suffix}.json"

        request = _build_prompt(
            round_no=round_no,
            variant=variant,
            attempt=attempt,
            target_api_id=target_api_id,
            context_markdown=context_markdown,
            harness_path=harness_path,
            smoke_report=smoke_report,
        )
        request_path.write_text(
            json.dumps(request, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        write_model_response(
            request_path,
            response_path,
            command_template,
            attempt=attempt,
        )

        diagnosis = _normalize_diagnosis(
            json.loads(response_path.read_text(encoding="utf-8")),
            smoke_report,
        )
        detail = {
            "version": "seraph.phase3.runtime_error.v1",
            "round": round_no,
            "variant": variant,
            "attempt": attempt,
            "target_api_id": target_api_id,
            "harness": str(harness_path),
            "compile_report": smoke_ref.get("compile_report"),
            "smoke_report": str(smoke_report_path),
            "request": str(request_path),
            "response": str(response_path),
            "status": "bug" if diagnosis["is_bug"] else "runtime_error",
            "classification": diagnosis["classification"],
            "summary": diagnosis["summary"],
            "evidence": {
                "exit_code": smoke_report.get("exit_code"),
                "stdout": smoke_report.get("stdout", "")[:2000],
                "stderr": smoke_report.get("stderr", "")[:2000],
            },
            "bug": diagnosis["is_bug"],
        }
        detail_path.write_text(
            json.dumps(detail, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        reports.append(
            {
                "harness": detail["harness"],
                "report": str(detail_path),
                "status": detail["status"],
                "classification": detail["classification"],
                "summary": detail["summary"],
                "bug": detail["bug"],
            }
        )
        if detail["bug"]:
            bug_count += 1

    index = {
        "version": "seraph.phase3.runtime_error_index.v1",
        "round": round_no,
        "status": "bug" if bug_count else ("runtime_error" if reports else "ok"),
        "report_count": len(reports),
        "bug_count": bug_count,
        "reports": reports,
    }
    index_path = out_dir / f"runtime_error_{round_no:03d}_index.json"
    index_path.write_text(
        json.dumps(index, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    index["report"] = str(index_path)
    return index


def _build_prompt(
    *,
    round_no: int,
    variant: int,
    attempt: Optional[int],
    target_api_id: str,
    context_markdown: str,
    harness_path: Path,
    smoke_report: Dict[str, Any],
) -> Dict[str, Any]:
    return {
        "version": "seraph.phase3.runtime_diagnose_request.v1",
        "round": round_no,
        "variant": variant,
        "attempt": attempt,
        "target_api_id": target_api_id,
        "context_markdown": context_markdown,
        "harness": str(harness_path),
        "harness_source": harness_path.read_text(encoding="utf-8"),
        "smoke_status": smoke_report.get("status"),
        "smoke_classification": smoke_report.get("classification"),
        "exit_code": smoke_report.get("exit_code"),
        "stdout": smoke_report.get("stdout", ""),
        "stderr": smoke_report.get("stderr", ""),
        "instructions": (
            "Summarize the runtime issue as strict JSON with keys: "
            "classification, summary, is_bug. "
            "classification must be one of: "
            "asan_bug, panic_or_crash, invalid_input_or_precondition, other_runtime_issue."
        ),
    }


def _normalize_diagnosis(
    diagnosis: Dict[str, Any],
    smoke_report: Dict[str, Any],
) -> Dict[str, Any]:
    combined_output = "\n".join(
        part for part in (smoke_report.get("stdout", ""), smoke_report.get("stderr", "")) if part
    )
    sanitizer_hit = _looks_like_sanitizer_issue(combined_output)
    classification = diagnosis.get("classification")
    if classification not in {
        "asan_bug",
        "panic_or_crash",
        "invalid_input_or_precondition",
        "other_runtime_issue",
    }:
        classification = "asan_bug" if sanitizer_hit else "other_runtime_issue"
    summary = diagnosis.get("summary")
    if not isinstance(summary, str) or not summary.strip():
        summary = "runtime issue recorded from smoke-run output"
    if (
        not sanitizer_hit
        and classification in {"panic_or_crash", "other_runtime_issue"}
        and _looks_like_capacity_precondition_panic(summary, combined_output)
    ):
        classification = "invalid_input_or_precondition"
        is_bug = False
    else:
        is_bug = bool(diagnosis.get("is_bug")) or classification == "asan_bug" or sanitizer_hit
    return {
        "classification": classification,
        "summary": summary.strip(),
        "is_bug": is_bug,
    }


def _looks_like_sanitizer_issue(output: str) -> bool:
    return any(marker in output for marker in SANITIZER_MARKERS)


def _looks_like_capacity_precondition_panic(summary: str, output: str) -> bool:
    text = f"{summary}\n{output}".lower()
    return (
        "arena overflow:" in text
        or "exceed arena capacity" in text
        or "exceeded arena capacity" in text
    )


def _extract_target_api_id(context_path: Path) -> str:
    for line in context_path.read_text(encoding="utf-8").splitlines():
        if line.strip().startswith("- api_id:"):
            return line.split(":", 1)[1].strip()
    raise ValueError("context missing target api_id")


def _parse_harness_identity(harness_path: Path) -> Tuple[int, Optional[int]]:
    parts = harness_path.stem.split("_")
    if len(parts) >= 3 and parts[0] == "harness":
        variant = int(parts[2])
        if len(parts) >= 5 and parts[3] == "fixed":
            return variant, int(parts[4])
        return variant, None
    raise ValueError(f"unexpected harness path: {harness_path}")


def _report_suffix(harness_path: Path, round_no: int) -> str:
    prefix = f"harness_{round_no:03d}_"
    stem = harness_path.stem
    if stem.startswith(prefix):
        return stem[len(prefix):]
    return stem
