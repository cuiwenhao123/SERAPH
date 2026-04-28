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
_DIRECT_TAKE_SLICE_RE = re.compile(r"data\[\s*1\s*\.\.\s*1\s*\+\s*take\s*\]")
_DIRECT_MIN_SLICE_RE = re.compile(r"data\[\s*1\s*\.\.\s*(?:core::cmp::)?min\([^]]+\)\s*\]")
_SATURATING_PLUS_ONE_SLICE_RE = re.compile(
    r"data\[\s*(?P<var>[A-Za-z_][A-Za-z0-9_]*)\.saturating_add\(\s*1\s*\)\s*\.\.[^]]*\]"
)
_IDX_PLUS_TAKE_SLICE_RE = re.compile(r"data\[\s*idx\s*\.\.\s*idx\s*\+\s*take\s*\]")
_FN_ITEM_PLACEHOLDER_POINTEE_RE = re.compile(
    r"fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*\([^)]*(?P<snippet>\*mut\s*_|\*const\s*_)[^)]*\)",
    re.DOTALL,
)
_HARNESS_ROUND_RE = re.compile(r"harness_(?P<round>\d{3})_\d{2}(?:_fixed_\d{2})?\.rs$")


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
    input_slice_failure = _unsafe_input_slice_failure(source)
    if input_slice_failure is not None:
        return input_slice_failure
    callback_placeholder_failure = _invalid_named_fn_placeholder_failure(source)
    if callback_placeholder_failure is not None:
        return callback_placeholder_failure
    marker_failure = _missing_target_call_between_markers(harness, source)
    if marker_failure is not None:
        return marker_failure
    return None


def _unsafe_fabrication_marker(source: str) -> str | None:
    for marker in _UNSAFE_FABRICATION_MARKERS:
        if marker in source:
            return marker
    return None


def _unsafe_input_slice_failure(source: str) -> str | None:
    for match in _DIRECT_TAKE_SLICE_RE.finditer(source):
        prefix = source[: match.start()]
        if _has_early_return_on_empty(prefix) or _has_local_positive_offset_guard(prefix):
            continue
        return _input_slice_failure_message(match.group(0))

    for match in _DIRECT_MIN_SLICE_RE.finditer(source):
        prefix = source[: match.start()]
        if _has_early_return_on_empty(prefix) or _has_local_positive_offset_guard(prefix):
            continue
        return _input_slice_failure_message(match.group(0))

    for match in _SATURATING_PLUS_ONE_SLICE_RE.finditer(source):
        prefix = source[: match.start()]
        guard_var = match.group("var")
        if _has_local_len_guard_for_var(prefix, guard_var):
            continue
        return _input_slice_failure_message(match.group(0))

    for match in _IDX_PLUS_TAKE_SLICE_RE.finditer(source):
        prefix = source[: match.start()]
        if _has_early_return_on_empty(prefix) or _has_local_idx_guard(prefix):
            continue
        return _input_slice_failure_message(match.group(0))

    return None


def _input_slice_failure_message(snippet: str) -> str:
    return (
        "SERAPH semantic guard: input-derived slice "
        f"`{snippet}` can panic before reaching the target on short inputs; "
        "prefer `get`, `split_first`, `split_at`, or an early return with an explicit local bounds check.\n"
    )


def _invalid_named_fn_placeholder_failure(source: str) -> str | None:
    match = _FN_ITEM_PLACEHOLDER_POINTEE_RE.search(source)
    if match is None:
        return None
    return (
        "SERAPH semantic guard: placeholder `_` in named function item signature "
        f"`{match.group('name')}` is invalid Rust; keep raw-pointer placeholders in closures "
        "or call-site casts/turbofish, or make the helper generic over the pointee type.\n"
    )


def _has_early_return_on_empty(prefix: str) -> bool:
    return bool(re.search(r"if\s+data\.is_empty\(\)\s*\{[^{}]{0,200}?return\b", prefix, re.DOTALL))


def _has_local_positive_offset_guard(prefix: str) -> bool:
    window = prefix[-240:]
    patterns = (
        r"if\s+data\.len\(\)\s*>\s*1",
        r"if\s+data\.len\(\)\s*>=\s*2",
        r"if\s*!data\.is_empty\(\)",
    )
    return any(re.search(pattern, window) for pattern in patterns)


def _has_local_len_guard_for_var(prefix: str, var_name: str) -> bool:
    window = prefix[-240:]
    return bool(
        re.search(
            rf"if\s+{re.escape(var_name)}\s*<\s*data\.len\(\)",
            window,
        )
    )


def _has_local_idx_guard(prefix: str) -> bool:
    window = prefix[-240:]
    return bool(re.search(r"if\s+idx\s*<\s*data\.len\(\)", window))


def _missing_target_call_between_markers(harness: Path, source: str) -> str | None:
    enter_match = _ENTER_MARKER_RE.search(source)
    ok_match = _OK_MARKER_RE.search(source)
    if enter_match is None or ok_match is None:
        return None
    target_api_id = enter_match.group(1)
    if ok_match.group(1) != target_api_id:
        return None
    between = source[enter_match.end() : ok_match.start()]
    for method_name in _target_call_names(harness, target_api_id):
        for match in re.finditer(rf"(\.|::|\b){re.escape(method_name)}", between):
            if _call_suffix_starts_at(between, match.end()):
                return None
    return (
        "SERAPH semantic guard: expected a real target call between "
        f"SERAPH_STEP_ENTER and SERAPH_STEP_OK for `{target_api_id}`.\n"
    )


def _target_call_names(harness: Path, target_api_id: str) -> List[str]:
    names: List[str] = []
    api_leaf = target_api_id.rsplit("::", 1)[-1]
    if api_leaf:
        names.append(api_leaf)
    context_leaf = _context_target_path_leaf(harness, target_api_id)
    if context_leaf and context_leaf not in names:
        names.append(context_leaf)
    return names


def _context_target_path_leaf(harness: Path, target_api_id: str) -> str | None:
    match = _HARNESS_ROUND_RE.search(harness.name)
    if match is None:
        return None
    if harness.parent.name != "fuzz":
        return None
    context_path = harness.parent.parent / "contexts" / "rag_target_{}.md".format(match.group("round"))
    if not context_path.exists():
        return None
    api_id = None
    target_path = None
    for line in context_path.read_text(encoding="utf-8").splitlines():
        if line.startswith("- api_id:"):
            api_id = line.split(":", 1)[1].strip()
        elif line.startswith("- path:"):
            target_path = line.split(":", 1)[1].strip()
    if api_id != target_api_id or not target_path:
        return None
    return target_path.rsplit("::", 1)[-1]


def _call_suffix_starts_at(source: str, index: int) -> bool:
    cursor = _skip_whitespace(source, index)
    if cursor < len(source) and source[cursor] == "(":
        return True
    if not source.startswith("::", cursor):
        return False
    cursor = _skip_whitespace(source, cursor + 2)
    if cursor >= len(source) or source[cursor] != "<":
        return False
    cursor = _skip_turbofish(source, cursor)
    if cursor is None:
        return False
    cursor = _skip_whitespace(source, cursor)
    return cursor < len(source) and source[cursor] == "("


def _skip_whitespace(source: str, index: int) -> int:
    while index < len(source) and source[index].isspace():
        index += 1
    return index


def _skip_turbofish(source: str, index: int) -> int | None:
    if index >= len(source) or source[index] != "<":
        return None
    depth = 0
    cursor = index
    while cursor < len(source):
        char = source[cursor]
        if char == "<":
            depth += 1
        elif char == ">":
            depth -= 1
            if depth == 0:
                return cursor + 1
        cursor += 1
    return None
