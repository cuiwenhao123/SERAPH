from __future__ import annotations

import json
import shlex
import subprocess
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Union

DEFAULT_SMOKE_COMMAND_TEMPLATE = "{harness}"
REVIEW_MARKER = "SERAPH_NEEDS_REVIEW:"
BUG_MARKER = "SERAPH_FOUND_BUG:"


def run_smoke_check(
    harness_path: Union[str, Path],
    report_path: Union[str, Path],
    command_template: str = DEFAULT_SMOKE_COMMAND_TEMPLATE,
) -> Dict[str, Any]:
    harness = Path(harness_path)
    command = _render_command(command_template, harness)
    process = subprocess.run(
        command,
        shell=True,
        text=True,
        capture_output=True,
        check=False,
    )
    combined_output = "\n".join(part for part in (process.stdout, process.stderr) if part)
    panic_detected = "panicked at" in combined_output.lower()
    review_reason = _extract_marker_reason(combined_output, REVIEW_MARKER)
    bug_reason = _extract_marker_reason(combined_output, BUG_MARKER)
    status = "ok"
    classification = "completed"
    if review_reason:
        status = "review"
        classification = "needs_review"
    elif process.returncode != 0:
        status = "bug"
        classification = "panic_detected" if panic_detected else "nonzero_exit"
    elif bug_reason:
        status = "bug"
        classification = "bug_marker"

    result = {
        "version": "seraph.phase3.smoke_run.v1",
        "harness": str(harness),
        "command": command,
        "status": status,
        "classification": classification,
        "exit_code": process.returncode,
        "stdout": process.stdout,
        "stderr": process.stderr,
        "panic_detected": panic_detected,
        "review_reason": review_reason,
        "bug_reason": bug_reason,
    }
    output = Path(report_path)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def run_smoke_checks(
    harness_paths: Sequence[Union[str, Path]],
    report_dir: Union[str, Path],
    round_no: int,
    command_template: str = DEFAULT_SMOKE_COMMAND_TEMPLATE,
) -> Dict[str, Any]:
    output_dir = Path(report_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    reports: List[Dict[str, Any]] = []
    status = "ok"
    bug_count = 0
    review_count = 0

    for harness_path in harness_paths:
        harness = Path(harness_path)
        report_path = output_dir / "smoke_{:03d}_{}.json".format(
            round_no,
            _report_suffix(harness, round_no),
        )
        report = run_smoke_check(
            harness,
            report_path,
            command_template=command_template,
        )
        reports.append({
            "harness": report["harness"],
            "report": str(report_path),
            "status": report["status"],
            "classification": report["classification"],
            "exit_code": report["exit_code"],
        })
        if report["status"] == "bug":
            bug_count += 1
        elif report["status"] == "review":
            review_count += 1

    if not reports:
        status = "skipped"
    elif bug_count:
        status = "bug"
    elif review_count:
        status = "review"

    index = {
        "version": "seraph.phase3.smoke_run_index.v1",
        "round": round_no,
        "status": status,
        "report_count": len(reports),
        "bug_count": bug_count,
        "review_count": review_count,
        "reports": reports,
    }
    index_path = output_dir / "smoke_{:03d}_index.json".format(round_no)
    index_path.write_text(json.dumps(index, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return index


def collect_successful_harnesses(
    compile_index_path: Optional[Union[str, Path]],
    fix_loop_index_path: Optional[Union[str, Path]] = None,
) -> List[Path]:
    harnesses: List[Path] = []
    seen = set()

    if compile_index_path:
        compile_index = json.loads(Path(compile_index_path).read_text(encoding="utf-8"))
        for report in compile_index.get("reports", []):
            if report.get("status") != "ok":
                continue
            harness = Path(report["harness"])
            key = str(harness)
            if key not in seen:
                seen.add(key)
                harnesses.append(harness)

    if fix_loop_index_path:
        fix_loop_index = json.loads(Path(fix_loop_index_path).read_text(encoding="utf-8"))
        for loop in fix_loop_index.get("loops", []):
            if loop.get("status") != "ok":
                continue
            successful_attempt = loop.get("successful_attempt")
            selected = None
            for attempt in _loop_attempts(loop):
                if attempt.get("attempt") == successful_attempt:
                    selected = attempt
                    break
            if selected is None:
                continue
            harness = Path(selected["harness"])
            key = str(harness)
            if key not in seen:
                seen.add(key)
                harnesses.append(harness)

    return harnesses


def _loop_attempts(loop: Dict[str, Any]) -> List[Dict[str, Any]]:
    attempts = loop.get("attempts")
    if isinstance(attempts, list):
        return attempts
    report_path = loop.get("report")
    if not report_path:
        return []
    report = Path(report_path)
    if not report.exists():
        return []
    payload = json.loads(report.read_text(encoding="utf-8"))
    nested_attempts = payload.get("attempts")
    if isinstance(nested_attempts, list):
        return nested_attempts
    return []


def _render_command(command_template: str, harness: Path) -> str:
    quoted_harness = shlex.quote(str(harness))
    return command_template.format(harness=quoted_harness)


def _extract_marker_reason(output: str, marker: str) -> str:
    for line in output.splitlines():
        if marker in line:
            _, _, suffix = line.partition(marker)
            return suffix.strip()
    return ""


def _report_suffix(harness: Path, round_no: int) -> str:
    prefix = "harness_{:03d}_".format(round_no)
    stem = harness.stem
    if stem.startswith(prefix):
        return stem[len(prefix):]
    return stem
