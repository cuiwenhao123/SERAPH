from __future__ import annotations

import json
import re
import shlex
import subprocess
from pathlib import Path
from typing import Any, Dict, List, Sequence, Union

DEFAULT_COMMAND_TEMPLATE = "rustc --edition=2021 --crate-type bin {harness}"
SEMANTIC_GUARD_EXIT_CODE = 2
_DIVERGING_PLACEHOLDER_RE = re.compile(
    r"fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*<[^>]*T[^>]*>\s*\([^)]*\)\s*->\s*T\s*\{(?P<body>.*?)\}",
    re.DOTALL,
)
_DIVERGING_BODY_MARKERS = (
    "std::process::exit(",
    "process::exit(",
    "panic!(",
    "unreachable!(",
    "todo!(",
    "unimplemented!(",
    "std::hint::unreachable_unchecked(",
)
_UNSAFE_FABRICATION_MARKERS = (
    "std::mem::zeroed(",
    "core::mem::zeroed(",
    "std::mem::transmute(",
    "core::mem::transmute(",
    "assume_init(",
    "Box::into_raw(",
)
_ENTER_MARKER_RE = re.compile(r"SERAPH_STEP_ENTER:\d+:(api::[A-Za-z0-9_:]+)")
_OK_MARKER_RE = re.compile(r"SERAPH_STEP_OK:\d+:(api::[A-Za-z0-9_:]+)")


def run_compile_check(
    harness_path: Union[str, Path],
    report_path: Union[str, Path],
    command_template: str = DEFAULT_COMMAND_TEMPLATE,
) -> Dict[str, Any]:
    harness = Path(harness_path)
    command = _render_command(command_template, harness)
    semantic_guard_error = _semantic_guard_failure(harness)
    if semantic_guard_error is None:
        process = subprocess.run(
            command,
            shell=True,
            text=True,
            capture_output=True,
            check=False,
        )
        status = "ok" if process.returncode == 0 else "failed"
        exit_code = process.returncode
        stdout = process.stdout
        stderr = process.stderr
    else:
        status = "failed"
        exit_code = SEMANTIC_GUARD_EXIT_CODE
        stdout = ""
        stderr = semantic_guard_error
    result = {
        "version": "seraph.phase3.compile_check.v1",
        "harness": str(harness),
        "command": command,
        "status": status,
        "exit_code": exit_code,
        "stdout": stdout,
        "stderr": stderr,
    }
    output = Path(report_path)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def run_compile_checks(
    harness_paths: Sequence[Union[str, Path]],
    report_dir: Union[str, Path],
    round_no: int,
    command_template: str = DEFAULT_COMMAND_TEMPLATE,
) -> Dict[str, Any]:
    output_dir = Path(report_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    reports: List[Dict[str, Any]] = []
    for index, harness_path in enumerate(harness_paths, start=1):
        report_path = output_dir / "compile_{:03d}_{:02d}.json".format(round_no, index)
        report = run_compile_check(
            harness_path,
            report_path,
            command_template=command_template,
        )
        reports.append({
            "harness": report["harness"],
            "report": str(report_path),
            "status": report["status"],
            "exit_code": report["exit_code"],
        })

    status = "ok" if all(report["status"] == "ok" for report in reports) else "failed"
    index = {
        "version": "seraph.phase3.compile_check_index.v1",
        "round": round_no,
        "status": status,
        "reports": reports,
    }
    index_path = output_dir / "compile_{:03d}_index.json".format(round_no)
    index_path.write_text(json.dumps(index, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return index


def _render_command(command_template: str, harness: Path) -> str:
    quoted_harness = shlex.quote(str(harness))
    return command_template.format(harness=quoted_harness)


def _semantic_guard_failure(harness: Path) -> str | None:
    source = harness.read_text(encoding="utf-8")
    for match in _DIVERGING_PLACEHOLDER_RE.finditer(source):
        body = match.group("body")
        if any(marker in body for marker in _DIVERGING_BODY_MARKERS):
            return (
                "SERAPH semantic guard: diverging placeholder helper "
                f"`{match.group('name')}` can fake arbitrary target types; "
                "replace it with a real public setup chain or an early return.\n"
            )
    unsafe_fabrication_marker = _unsafe_fabrication_marker(source)
    if unsafe_fabrication_marker is not None:
        return (
            "SERAPH semantic guard: unsafe initialization trick "
            f"`{unsafe_fabrication_marker}` can fabricate missing target state; "
            "replace it with a real public setup chain or an early return.\n"
        )
    marker_failure = _missing_target_call_between_markers(source)
    if marker_failure is not None:
        return marker_failure
    return None


def _unsafe_fabrication_marker(source: str) -> str | None:
    for marker in _UNSAFE_FABRICATION_MARKERS:
        if marker in source:
            return marker
    return None


def _missing_target_call_between_markers(source: str) -> str | None:
    enter_match = _ENTER_MARKER_RE.search(source)
    ok_match = _OK_MARKER_RE.search(source)
    if enter_match is None or ok_match is None:
        return None
    target_api_id = enter_match.group(1)
    if ok_match.group(1) != target_api_id:
        return None
    between = source[enter_match.end() : ok_match.start()]
    method_name = target_api_id.rsplit("::", 1)[-1]
    if re.search(rf"(\.|::|\b){re.escape(method_name)}\s*\(", between):
        return None
    return (
        "SERAPH semantic guard: expected a real target call between "
        f"SERAPH_STEP_ENTER and SERAPH_STEP_OK for `{target_api_id}`.\n"
    )
