#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import os
import re
import shutil
import subprocess
import sys
import time
from collections import Counter
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_WORKUNITS = REPO_ROOT / "docs" / "research" / "deepsurf-rust-workunits.csv"
_CARGO_COMPILE_FAILED_RE = re.compile(
    r"error:\s+could not compile `(?P<crate>[^`]+)`(?: \((?P<kind>[^)]+)\))? due to"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Evaluate deepSURF Rust work units with SERAPH.")
    parser.add_argument(
        "--workunits-csv",
        default=str(DEFAULT_WORKUNITS),
        help="CSV describing reproducible deepSURF work units.",
    )
    parser.add_argument(
        "--dataset-root",
        default="/tmp/deepSURF-main/dataset",
        help="Root directory that contains erasan_crates/, rustsan_crates/, rug_crates/, crabtree_crates/.",
    )
    parser.add_argument(
        "--workspace-root",
        default=f"/tmp/seraph-deepsurf-round1-{time.strftime('%Y%m%d-%H%M%S')}",
        help="Directory that will hold one SERAPH workspace per work unit.",
    )
    parser.add_argument(
        "--results-json",
        default=str(REPO_ROOT / "docs" / "research" / "deepsurf-round1-results.json"),
    )
    parser.add_argument(
        "--results-csv",
        default=str(REPO_ROOT / "docs" / "research" / "deepsurf-round1-results.csv"),
    )
    parser.add_argument(
        "--results-md",
        default=str(REPO_ROOT / "docs" / "research" / "deepsurf-round1-results.md"),
    )
    parser.add_argument("--limit", type=int, default=0, help="Only evaluate the first N work units.")
    parser.add_argument(
        "--match",
        default="",
        help="Only evaluate work units whose source_group, dataset_entry, unit_name, or manifest path contains this substring.",
    )
    parser.add_argument(
        "--skip-phase3",
        action="store_true",
        help="Stop after Phase 1/2 target discovery instead of running Phase 3 round-1.",
    )
    parser.add_argument(
        "--resume",
        action="store_true",
        help="Resume from an existing results JSON and skip completed work units.",
    )
    parser.add_argument("--variants", type=int, default=1)
    parser.add_argument("--fix-max-attempts", type=int, default=2)
    parser.add_argument("--extract-timeout", type=int, default=1800)
    parser.add_argument("--index-timeout", type=int, default=1800)
    parser.add_argument("--graph-timeout", type=int, default=1800)
    parser.add_argument("--phase3-timeout", type=int, default=1800)
    return parser.parse_args()


def ensure_env(skip_phase3: bool) -> None:
    required = [
        "SERAPH_EMBEDDING_BASE_URL",
        "SERAPH_EMBEDDING_MODEL",
    ]
    if not skip_phase3:
        required.extend(
            [
                "SERAPH_LLM_BASE_URL",
                "SERAPH_LLM_MODEL",
            ]
        )
    missing = [name for name in required if not os.environ.get(name)]
    if missing:
        raise SystemExit(
            "missing required environment variables: "
            + ", ".join(missing)
            + "\nload configs/environments/.env.seraph-local before running"
        )


def load_workunits(path: Path, match: str, limit: int) -> List[Dict[str, str]]:
    rows: List[Dict[str, str]] = []
    needle = match.lower().strip()
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if needle:
                haystack = " ".join(
                    [
                        row.get("source_group", ""),
                        row.get("dataset_entry", ""),
                        row.get("unit_name", ""),
                        row.get("manifest_rel_path", ""),
                    ]
                ).lower()
                if needle not in haystack:
                    continue
            rows.append(row)
    if limit > 0:
        rows = rows[:limit]
    return rows


def load_previous_results(path: Path) -> Dict[str, Dict[str, object]]:
    if not path.exists():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return {}
    if not isinstance(data, list):
        return {}
    previous: Dict[str, Dict[str, object]] = {}
    for row in data:
        if isinstance(row, dict):
            key = str(row.get("workunit_key", ""))
            if key:
                previous[key] = row
    return previous


def write_json(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_csv(path: Path, rows: Sequence[Dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fieldnames: List[str] = []
    for row in rows:
        for key in row.keys():
            if key not in fieldnames:
                fieldnames.append(key)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow({key: normalize_csv_value(row.get(key)) for key in fieldnames})


def normalize_csv_value(value: object) -> str:
    if value is None:
        return ""
    if isinstance(value, (str, int, float, bool)):
        return str(value)
    return json.dumps(value, ensure_ascii=True, sort_keys=True)


def write_markdown(path: Path, rows: Sequence[Dict[str, object]], args: argparse.Namespace) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    outcome_counts = Counter(str(row.get("outcome", "unknown")) for row in rows)
    group_counts = Counter(str(row.get("source_group", "")) for row in rows)
    lines: List[str] = []
    lines.append("# deepSURF Round-1 SERAPH Evaluation")
    lines.append("")
    lines.append(f"- Generated at: `{time.strftime('%Y-%m-%d %H:%M:%S')}`")
    lines.append(f"- Workspaces: `{args.workspace_root}`")
    lines.append(f"- Dataset root: `{args.dataset_root}`")
    lines.append(f"- Work units attempted: `{len(rows)}`")
    lines.append(f"- Phase 3 enabled: `{'no' if args.skip_phase3 else 'yes'}`")
    lines.append("")
    lines.append("## Outcome Counts")
    lines.append("")
    for outcome, count in sorted(outcome_counts.items()):
        lines.append(f"- `{outcome}`: `{count}`")
    lines.append("")
    lines.append("## Group Counts")
    lines.append("")
    for group, count in sorted(group_counts.items()):
        lines.append(f"- `{group}`: `{count}`")
    lines.append("")
    lines.append("## Results")
    lines.append("")
    lines.append(
        "| Group | Unit | Unsafe Targets | Top Target | Outcome | Compile/Fix/Smoke | Error |"
    )
    lines.append("|-------|------|----------------|------------|---------|-------------------|-------|")
    for row in rows:
        lines.append(
            "| {group} | {unit} | {targets} | `{target}` | `{outcome}` | `{c}/{f}/{s}` | {error} |".format(
                group=row.get("source_group", ""),
                unit=row.get("unit_name", ""),
                targets=row.get("unsafe_target_count", ""),
                target=row.get("top_target_api_id", "") or "-",
                outcome=row.get("outcome", ""),
                c=row.get("compile_status", "-"),
                f=row.get("fix_status", "-"),
                s=row.get("smoke_status", "-"),
                error=(str(row.get("error_stage", "")) + ":" + str(row.get("error_message", ""))).strip(":")
                or "-",
            )
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def run_command(
    args: Sequence[str],
    *,
    cwd: Path,
    env: Dict[str, str],
    timeout_seconds: int,
    log_prefix: Path,
) -> Tuple[str, int, float, str, str]:
    start = time.time()
    stdout = ""
    stderr = ""
    status = "ok"
    return_code = 0
    try:
        completed = subprocess.run(
            list(args),
            cwd=str(cwd),
            env=env,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
        )
        stdout = completed.stdout
        stderr = completed.stderr
        return_code = completed.returncode
        if return_code != 0:
            status = "failed"
    except subprocess.TimeoutExpired as exc:
        status = "timeout"
        return_code = 124
        stdout = exc.stdout or ""
        stderr = exc.stderr or ""
    elapsed = time.time() - start
    log_prefix.parent.mkdir(parents=True, exist_ok=True)
    (log_prefix.with_suffix(".stdout.log")).write_text(stdout, encoding="utf-8")
    (log_prefix.with_suffix(".stderr.log")).write_text(stderr, encoding="utf-8")
    return status, return_code, elapsed, stdout, stderr


def parse_targets(stdout: str) -> List[Dict[str, object]]:
    targets: List[Dict[str, object]] = []
    for line in stdout.splitlines():
        line = line.strip()
        if not line or "\t" not in line:
            continue
        parts = line.split("\t", 2)
        if len(parts) != 3:
            continue
        api_id, score_text, path = parts
        try:
            score = float(score_text)
        except ValueError:
            score = 0.0
        targets.append(
            {
                "api_id": api_id,
                "score": score,
                "path": path,
            }
        )
    return targets


def load_optional_json(path: Path) -> Optional[Dict[str, object]]:
    if not path.exists():
        return None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None
    if isinstance(data, dict):
        return data
    return None


def summarize_phase3(workspace: Path, top_target_api_id: str) -> Dict[str, object]:
    reports_dir = workspace / "reports"
    compile_index = load_optional_json(reports_dir / "compile_001_index.json") or {}
    fix_index = load_optional_json(reports_dir / "fix_loop_001_index.json") or {}
    smoke_index = load_optional_json(reports_dir / "smoke_001_index.json") or {}
    runtime_index = load_optional_json(reports_dir / "runtime_error_001_index.json") or {}
    coverage = load_optional_json(workspace / "coverage.json") or {}

    compile_reports = compile_index.get("reports", [])
    smoke_reports = smoke_index.get("reports", [])
    api_status = coverage.get("api_status", {})
    top_status = api_status.get(top_target_api_id)
    compile_failure = summarize_compile_failures(workspace, compile_index, fix_index)

    bug_count = int(smoke_index.get("bug_count", 0) or 0)
    review_count = int(smoke_index.get("review_count", 0) or 0)
    runtime_reports = runtime_index.get("reports", [])
    runtime_issue_count = len(runtime_reports) if isinstance(runtime_reports, list) else 0
    runtime_status = runtime_index.get("status")

    if runtime_status == "bug":
        outcome = "bug"
    elif runtime_status == "runtime_error":
        outcome = "runtime_error"
    elif bug_count > 0:
        outcome = "bug"
    elif review_count > 0:
        outcome = "needs_review"
    elif top_status:
        outcome = str(top_status)
        if outcome == "attempted" and compile_failure["all_crate_build_failed"]:
            outcome = "crate_build_failed"
    elif smoke_index:
        outcome = "phase3_completed_without_coverage_status"
    else:
        outcome = "phase3_missing_reports"

    return {
        "coverage_rate": coverage.get("coverage_rate"),
        "related_coverage_rate": coverage.get("related_coverage_rate"),
        "top_api_status": top_status,
        "outcome": outcome,
        "compile_status": compile_index.get("status"),
        "compile_ok_count": sum(1 for report in compile_reports if report.get("status") == "ok")
        if isinstance(compile_reports, list)
        else 0,
        "fix_status": fix_index.get("status"),
        "fix_success_count": fix_index.get("successful_requests"),
        "smoke_status": smoke_index.get("status"),
        "smoke_ok_count": sum(1 for report in smoke_reports if report.get("status") == "ok")
        if isinstance(smoke_reports, list)
        else 0,
        "smoke_bug_count": bug_count,
        "smoke_review_count": review_count,
        "runtime_issue_count": runtime_issue_count,
        "found_bugs": coverage.get("found_bugs"),
        "needs_review": coverage.get("needs_review"),
        "compile_failure_kind": compile_failure["failure_kind"],
        "compile_failed_crates": compile_failure["failed_crates"],
        "compile_failed_target_kinds": compile_failure["failed_target_kinds"],
    }


def summarize_compile_failures(
    workspace: Path,
    compile_index: Dict[str, object],
    fix_index: Dict[str, object],
) -> Dict[str, object]:
    report_paths = []
    for report in compile_index.get("reports", []):
        if isinstance(report, dict):
            path = report.get("report")
            if isinstance(path, str) and path:
                report_paths.append(Path(path))

    for loop in fix_index.get("loops", []):
        if not isinstance(loop, dict):
            continue
        loop_report_path = loop.get("report")
        if not isinstance(loop_report_path, str) or not loop_report_path:
            continue
        loop_report = load_optional_json(Path(loop_report_path)) or {}
        for attempt in loop_report.get("attempts", []):
            if isinstance(attempt, dict):
                path = attempt.get("report")
                if isinstance(path, str) and path:
                    report_paths.append(Path(path))

    failure_records = []
    for path in report_paths:
        report = load_optional_json(path) or {}
        if report.get("status") != "failed":
            continue
        failure_records.append(_classify_compile_failure(report))

    kinds = sorted({record["failure_kind"] for record in failure_records if record["failure_kind"]})
    failed_crates = sorted({record["failed_crate"] for record in failure_records if record["failed_crate"]})
    failed_target_kinds = sorted(
        {record["failed_target_kind"] for record in failure_records if record["failed_target_kind"]}
    )
    all_crate_build_failed = bool(failure_records) and all(
        record["failure_kind"] == "crate_build_failed" for record in failure_records
    )
    if not kinds:
        failure_kind = None
    elif len(kinds) == 1:
        failure_kind = kinds[0]
    else:
        failure_kind = "mixed"
    return {
        "failure_kind": failure_kind,
        "failed_crates": failed_crates,
        "failed_target_kinds": failed_target_kinds,
        "all_crate_build_failed": all_crate_build_failed,
    }


def _classify_compile_failure(report: Dict[str, object]) -> Dict[str, Optional[str]]:
    failure_kind = report.get("failure_kind")
    failed_crate = report.get("failed_crate")
    failed_target_kind = report.get("failed_target_kind")
    if isinstance(failure_kind, str) and failure_kind:
        return {
            "failure_kind": failure_kind,
            "failed_crate": failed_crate if isinstance(failed_crate, str) else None,
            "failed_target_kind": failed_target_kind if isinstance(failed_target_kind, str) else None,
        }

    stderr = str(report.get("stderr") or "")
    exit_code = report.get("exit_code")
    if exit_code == 2 and stderr.startswith("SERAPH semantic guard:"):
        return {
            "failure_kind": "semantic_guard_failed",
            "failed_crate": None,
            "failed_target_kind": None,
        }

    match = None
    for candidate in _CARGO_COMPILE_FAILED_RE.finditer(stderr):
        match = candidate

    if match is None:
        return {
            "failure_kind": "command_failed",
            "failed_crate": None,
            "failed_target_kind": None,
        }

    failed_crate = match.group("crate")
    failed_target_kind = match.group("kind")
    harness_name = Path(str(report.get("harness") or "")).stem
    failure_kind = "harness_compile_failed"
    if failed_crate != harness_name:
        failure_kind = "crate_build_failed"
    return {
        "failure_kind": failure_kind,
        "failed_crate": failed_crate,
        "failed_target_kind": failed_target_kind,
    }


def append_result(results: List[Dict[str, object]], row: Dict[str, object], args: argparse.Namespace) -> None:
    results.append(row)
    write_json(Path(args.results_json), results)
    write_csv(Path(args.results_csv), results)
    write_markdown(Path(args.results_md), results, args)


def build_phase3_commands(workspace: Path) -> Dict[str, str]:
    repo = REPO_ROOT
    cargo_target = workspace / "_cargo_target"
    compile_command = (
        f"timeout 180s bash -lc 'CARGO_TARGET_DIR={cargo_target} "
        f"python3 {repo / 'scripts' / 'phase3_real_crate.py'} compile {{harness}}'"
    )
    smoke_command = (
        f"timeout 30s bash -lc 'CARGO_TARGET_DIR={cargo_target} SERAPH_SMOKE_STDIN=seed "
        f"python3 {repo / 'scripts' / 'phase3_real_crate.py'} smoke {{harness}}'"
    )
    model_command = (
        f"python3 {repo / 'scripts' / 'third_party_openai_compatible.py'} "
        "--input {input} --output {output}"
    )
    return {
        "compile_command": compile_command,
        "smoke_command": smoke_command,
        "model_command": model_command,
    }


def stage_error(stderr: str, stdout: str) -> str:
    text = (stderr or stdout).strip()
    if not text:
        return ""
    lines = text.splitlines()
    return lines[-1][:400]


def evaluate_one(
    workunit: Dict[str, str],
    *,
    args: argparse.Namespace,
    env: Dict[str, str],
) -> Dict[str, object]:
    dataset_root = Path(args.dataset_root)
    manifest_path = dataset_root / workunit["manifest_rel_path"]
    unit_name = workunit["unit_name"]
    workspace = Path(args.workspace_root) / unit_name
    logs_dir = workspace / "logs"
    workunit_key = workunit["manifest_rel_path"]

    if workspace.exists():
        shutil.rmtree(workspace)
    workspace.mkdir(parents=True, exist_ok=True)

    row: Dict[str, object] = {
        "workunit_key": workunit_key,
        "source_group": workunit["source_group"],
        "dataset_entry": workunit["dataset_entry"],
        "unit_name": unit_name,
        "manifest_rel_path": workunit["manifest_rel_path"],
        "manifest_path": str(manifest_path),
        "expanded_from": workunit.get("expanded_from", ""),
        "workspace_dir": str(workspace),
        "outcome": "started",
    }

    start = time.time()

    extract_args = [
        "cargo",
        "run",
        "-p",
        "s3-extract",
        "--",
        "--manifest-path",
        str(manifest_path),
        "--output",
        str(workspace / "knowledge.json"),
    ]
    status, return_code, elapsed, stdout, stderr = run_command(
        extract_args,
        cwd=REPO_ROOT,
        env=env,
        timeout_seconds=args.extract_timeout,
        log_prefix=logs_dir / "phase1_extract",
    )
    row["extract_status"] = status
    row["extract_return_code"] = return_code
    row["extract_elapsed_seconds"] = round(elapsed, 2)
    if status != "ok":
        row["outcome"] = "extract_failed"
        row["error_stage"] = "extract"
        row["error_message"] = stage_error(stderr, stdout)
        row["elapsed_seconds"] = round(time.time() - start, 2)
        return row

    index_args = [
        "cargo",
        "run",
        "-p",
        "seraph-cli",
        "--",
        "phase2",
        "index",
        "--knowledge",
        str(workspace / "knowledge.json"),
        "--vectordb",
        str(workspace / "vectordb"),
    ]
    status, return_code, elapsed, stdout, stderr = run_command(
        index_args,
        cwd=REPO_ROOT,
        env=env,
        timeout_seconds=args.index_timeout,
        log_prefix=logs_dir / "phase2_index",
    )
    row["index_status"] = status
    row["index_return_code"] = return_code
    row["index_elapsed_seconds"] = round(elapsed, 2)
    if status != "ok":
        row["outcome"] = "index_failed"
        row["error_stage"] = "index"
        row["error_message"] = stage_error(stderr, stdout)
        row["elapsed_seconds"] = round(time.time() - start, 2)
        return row

    graph_args = [
        "cargo",
        "run",
        "-p",
        "seraph-cli",
        "--",
        "phase2",
        "graph",
        "--knowledge",
        str(workspace / "knowledge.json"),
        "--graph",
        str(workspace / "graph.pkl"),
    ]
    status, return_code, elapsed, stdout, stderr = run_command(
        graph_args,
        cwd=REPO_ROOT,
        env=env,
        timeout_seconds=args.graph_timeout,
        log_prefix=logs_dir / "phase2_graph",
    )
    row["graph_status"] = status
    row["graph_return_code"] = return_code
    row["graph_elapsed_seconds"] = round(elapsed, 2)
    if status != "ok":
        row["outcome"] = "graph_failed"
        row["error_stage"] = "graph"
        row["error_message"] = stage_error(stderr, stdout)
        row["elapsed_seconds"] = round(time.time() - start, 2)
        return row

    targets_args = [
        "cargo",
        "run",
        "-p",
        "seraph-cli",
        "--",
        "phase2",
        "targets",
        "--graph",
        str(workspace / "graph.pkl"),
    ]
    status, return_code, elapsed, stdout, stderr = run_command(
        targets_args,
        cwd=REPO_ROOT,
        env=env,
        timeout_seconds=120,
        log_prefix=logs_dir / "phase2_targets",
    )
    row["targets_status"] = status
    row["targets_return_code"] = return_code
    row["targets_elapsed_seconds"] = round(elapsed, 2)
    if status != "ok":
        row["outcome"] = "targets_failed"
        row["error_stage"] = "targets"
        row["error_message"] = stage_error(stderr, stdout)
        row["elapsed_seconds"] = round(time.time() - start, 2)
        return row

    targets = parse_targets(stdout)
    row["unsafe_target_count"] = len(targets)
    if not targets:
        row["outcome"] = "no_targets"
        row["elapsed_seconds"] = round(time.time() - start, 2)
        return row

    top_target = targets[0]
    row["top_target_api_id"] = top_target["api_id"]
    row["top_target_score"] = top_target["score"]
    row["top_target_path"] = top_target["path"]

    if args.skip_phase3:
        row["outcome"] = "phase2_only"
        row["elapsed_seconds"] = round(time.time() - start, 2)
        return row

    commands = build_phase3_commands(workspace)
    phase3_args = [
        "cargo",
        "run",
        "-p",
        "seraph-cli",
        "--",
        "run",
        "--manifest-path",
        str(manifest_path),
        "--workspace-dir",
        str(workspace),
        "--round",
        "1",
        "--target-api-id",
        str(top_target["api_id"]),
        "--phase3-style",
        "aflpp",
        "--variants",
        str(args.variants),
        "--model-command",
        commands["model_command"],
        "--compile-check",
        "--compile-command",
        commands["compile_command"],
        "--fix-loop",
        "--fix-max-attempts",
        str(args.fix_max_attempts),
        "--fix-model-command",
        commands["model_command"],
        "--smoke-command",
        commands["smoke_command"],
        "--runtime-model-command",
        commands["model_command"],
    ]
    status, return_code, elapsed, stdout, stderr = run_command(
        phase3_args,
        cwd=REPO_ROOT,
        env=env,
        timeout_seconds=args.phase3_timeout,
        log_prefix=logs_dir / "phase3_round1",
    )
    row["phase3_run_status"] = status
    row["phase3_run_return_code"] = return_code
    row["phase3_run_elapsed_seconds"] = round(elapsed, 2)
    if status != "ok":
        row["outcome"] = "phase3_run_failed"
        row["error_stage"] = "phase3_run"
        row["error_message"] = stage_error(stderr, stdout)
        row["elapsed_seconds"] = round(time.time() - start, 2)
        return row

    row.update(summarize_phase3(workspace, str(top_target["api_id"])))
    row["elapsed_seconds"] = round(time.time() - start, 2)
    return row


def main() -> None:
    args = parse_args()
    ensure_env(args.skip_phase3)

    workunits = load_workunits(Path(args.workunits_csv), args.match, args.limit)
    if not workunits:
        raise SystemExit("no work units matched the requested filters")

    env = dict(os.environ)
    env.setdefault("SERAPH_EMBEDDING_BATCH_SIZE", "2")
    env.setdefault("SERAPH_EMBEDDING_MAX_RETRIES", "3")
    pythonpath = env.get("PYTHONPATH", "")
    rag_path = str(REPO_ROOT / "rag")
    if rag_path not in pythonpath.split(os.pathsep):
        env["PYTHONPATH"] = rag_path if not pythonpath else pythonpath + os.pathsep + rag_path

    previous = load_previous_results(Path(args.results_json)) if args.resume else {}
    results: List[Dict[str, object]] = []
    if previous:
        for key in previous:
            results.append(previous[key])

    completed_keys = set(previous.keys())
    remaining = [row for row in workunits if row["manifest_rel_path"] not in completed_keys]
    total = len(remaining)

    for index, workunit in enumerate(remaining, start=1):
        label = f"{workunit['source_group']}::{workunit['unit_name']}"
        print(f"[{index}/{total}] {label}", flush=True)
        row = evaluate_one(workunit, args=args, env=env)
        results = [item for item in results if item.get("workunit_key") != row["workunit_key"]]
        append_result(results, row, args)
        print(
            f"  outcome={row.get('outcome')} targets={row.get('unsafe_target_count', 0)} "
            f"top={row.get('top_target_api_id', '-')}",
            flush=True,
        )


if __name__ == "__main__":
    main()
