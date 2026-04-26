from __future__ import annotations

import glob
import json
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

from seraph_rag.compile_check import DEFAULT_COMMAND_TEMPLATE
from seraph_rag.fix_once import run_fix_once
from seraph_rag.model_provider import write_model_response


def run_fix_loops(
    request_glob: Union[str, Path],
    responses_dir: Union[str, Path],
    output_dir: Union[str, Path],
    report_dir: Union[str, Path],
    index_output_path: Union[str, Path],
    max_attempts: int,
    command_template: str = DEFAULT_COMMAND_TEMPLATE,
    response_command_template: Optional[str] = None,
) -> Dict[str, Any]:
    request_paths = [Path(path) for path in sorted(glob.glob(str(request_glob)))]
    loops: List[Dict[str, Any]] = []
    success_count = 0
    for request_path in request_paths:
        summary = run_fix_loop(
            request_path,
            responses_dir,
            output_dir,
            report_dir,
            max_attempts=max_attempts,
            command_template=command_template,
            response_command_template=response_command_template,
        )
        loops.append({
            "request": str(request_path),
            "report": summary["report"],
            "status": summary["status"],
            "stop_reason": summary["stop_reason"],
            "successful_attempt": summary["successful_attempt"],
            "attempt_count": len(summary["attempts"]),
        })
        if summary["status"] == "ok":
            success_count += 1

    failed_count = len(request_paths) - success_count
    summary = {
        "version": "seraph.phase3.fix_loop_index.v1",
        "request_glob": str(request_glob),
        "request_count": len(request_paths),
        "successful_requests": success_count,
        "failed_requests": failed_count,
        "status": "ok" if failed_count == 0 else "failed",
        "loops": loops,
    }
    output = Path(index_output_path)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary["report"] = str(output)
    return summary


def run_fix_loop(
    fix_request_path: Union[str, Path],
    responses_dir: Union[str, Path],
    output_dir: Union[str, Path],
    report_dir: Union[str, Path],
    max_attempts: int,
    command_template: str = DEFAULT_COMMAND_TEMPLATE,
    response_command_template: Optional[str] = None,
) -> Dict[str, Any]:
    if max_attempts < 1:
        raise ValueError("max_attempts must be >= 1")

    request_path = Path(fix_request_path)
    request = json.loads(request_path.read_text(encoding="utf-8"))
    round_no = int(request["round"])
    variant = int(request["variant"])
    responses_root = Path(responses_dir)
    output_root = Path(output_dir)
    report_root = Path(report_dir)

    summary: Dict[str, Any] = {
        "version": "seraph.phase3.fix_loop.v1",
        "request": str(request_path),
        "round": round_no,
        "variant": variant,
        "max_attempts": max_attempts,
        "status": "failed",
        "stop_reason": "max_attempts_exhausted",
        "successful_attempt": None,
        "attempts": [],
    }

    for attempt in range(1, max_attempts + 1):
        response_path = resolve_fix_response_path(round_no, variant, responses_root, attempt)
        if response_path is None and response_command_template:
            expected_path = expected_fix_response_path(round_no, variant, responses_root, attempt)
            write_model_response(
                request_path,
                expected_path,
                response_command_template,
                attempt=attempt,
            )
            response_path = expected_path
        if response_path is None:
            summary["stop_reason"] = "missing_response"
            summary["missing_response"] = str(expected_fix_response_path(round_no, variant, responses_root, attempt))
            break

        result = run_fix_once(
            request_path,
            response_path,
            output_root,
            report_root,
            attempt=attempt,
            command_template=command_template,
        )
        summary["attempts"].append({
            "attempt": attempt,
            "response": str(response_path),
            "harness": result["harness"],
            "report": str(fixed_report_path(round_no, variant, report_root, attempt)),
            "status": result["status"],
            "exit_code": result["exit_code"],
        })

        if result["status"] == "ok":
            summary["status"] = "ok"
            summary["stop_reason"] = "compiled"
            summary["successful_attempt"] = attempt
            break

    summary_path = fix_loop_report_path(round_no, variant, report_root)
    summary_path.parent.mkdir(parents=True, exist_ok=True)
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary["report"] = str(summary_path)
    return summary


def resolve_fix_response_path(round_no: int, variant: int, responses_dir: Path, attempt: int) -> Optional[Path]:
    candidates = [expected_fix_response_path(round_no, variant, responses_dir, attempt)]
    if attempt == 1:
        candidates.append(responses_dir / "fix_response_{:03d}_{:02d}.md".format(round_no, variant))
    for candidate in candidates:
        if candidate.exists():
            return candidate
    return None


def expected_fix_response_path(round_no: int, variant: int, responses_dir: Path, attempt: int) -> Path:
    return responses_dir / "fix_response_{:03d}_{:02d}_{:02d}.md".format(round_no, variant, attempt)


def fixed_report_path(round_no: int, variant: int, report_dir: Path, attempt: int) -> Path:
    return report_dir / "compile_{:03d}_{:02d}_fixed_{:02d}.json".format(round_no, variant, attempt)


def fix_loop_report_path(round_no: int, variant: int, report_dir: Path) -> Path:
    return report_dir / "fix_loop_{:03d}_{:02d}.json".format(round_no, variant)
