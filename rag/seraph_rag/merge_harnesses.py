from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Dict, List, Tuple, Union

from seraph_rag.smoke_run import collect_successful_harnesses


_SELECTOR_SAFE_SEED_LIMIT = 65536


def write_merged_harnesses(
    workspace_dir: Union[str, Path],
    round_no: int,
    crate_name: str,
    crate_import_name: str,
) -> Dict[str, object]:
    workspace = Path(workspace_dir)
    reports_dir = workspace / "reports"
    compile_index = reports_dir / "compile_{:03d}_index.json".format(round_no)
    smoke_index = reports_dir / "smoke_{:03d}_index.json".format(round_no)
    fix_loop_index = reports_dir / "fix_loop_{:03d}_index.json".format(round_no)

    selected_cases, excluded_cases = select_merge_cases(
        compile_index,
        smoke_index,
        fix_loop_index if fix_loop_index.exists() else None,
    )
    if not selected_cases:
        raise ValueError("no passing harness cases available for merge")
    if len(selected_cases) > _SELECTOR_SAFE_SEED_LIMIT:
        raise ValueError("selector-safe seed limit exceeded: more than 65536 selected cases")

    crate_dir_name = _sanitize_name(crate_name)
    target_name = "merged_{}".format(crate_dir_name)
    merged_dir = workspace / "fuzz" / crate_dir_name / "merged"
    merged_cases_dir = merged_dir / "cases"
    project_dir = workspace / "_cargo_projects" / target_name
    project_src_dir = project_dir / "src"
    project_cases_dir = project_src_dir / "cases"
    default_corpus_dir = workspace / "afl" / target_name / "corpus"
    default_corpus_meta_dir = workspace / "afl" / target_name / "corpus_meta"

    merged_cases_dir.mkdir(parents=True, exist_ok=True)
    project_cases_dir.mkdir(parents=True, exist_ok=True)

    case_entries = []
    for case_path in selected_cases:
        module_name = _sanitize_name(case_path.stem)
        source = case_path.read_text(encoding="utf-8")
        (merged_cases_dir / case_path.name).write_text(source, encoding="utf-8")
        (project_cases_dir / case_path.name).write_text(source, encoding="utf-8")
        case_entries.append((module_name, case_path.name))

    registry_source = render_registry(case_entries)
    main_source = render_merged_main()
    selected_cases_text = "".join("{}\n".format(path) for path in selected_cases)

    (merged_dir / "registry.rs").write_text(registry_source, encoding="utf-8")
    (merged_dir / "main.rs").write_text(main_source, encoding="utf-8")
    (merged_dir / "selected_cases.txt").write_text(selected_cases_text, encoding="utf-8")

    (project_src_dir / "registry.rs").write_text(registry_source, encoding="utf-8")
    (project_src_dir / "main.rs").write_text(main_source, encoding="utf-8")
    (project_dir / "Cargo.toml").write_text(
        render_merged_manifest(target_name, crate_name, crate_import_name, workspace),
        encoding="utf-8",
    )
    default_seed_files = write_selector_safe_corpus(default_corpus_dir, selected_cases)

    report = {
        "version": "seraph.phase3.merge_harnesses.v2",
        "round": round_no,
        "crate_name": crate_name,
        "crate_import_name": crate_import_name,
        "target_name": target_name,
        "project_dir": str(project_dir),
        "manifest_path": str(project_dir / "Cargo.toml"),
        "merged_source_dir": str(merged_dir),
        "main_rs": str(merged_dir / "main.rs"),
        "registry_rs": str(merged_dir / "registry.rs"),
        "selected_cases_file": str(merged_dir / "selected_cases.txt"),
        "selected_case_count": len(selected_cases),
        "safe_seed_count": len(default_seed_files),
        "default_corpus_dir": str(default_corpus_dir),
        "default_corpus_meta_dir": str(default_corpus_meta_dir),
        "default_seed_files": [str(path) for path in default_seed_files],
        "selected_cases": [str(path) for path in selected_cases],
        "excluded_cases": excluded_cases,
    }
    report_path = reports_dir / "merge_{}.json".format(crate_dir_name)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_selector_safe_corpus_summary(
        summary_path=default_corpus_meta_dir / "summary.json",
        target_name=target_name,
        manifest_path=project_dir / "Cargo.toml",
        merge_report_path=report_path,
        corpus_dir=default_corpus_dir,
        meta_dir=default_corpus_meta_dir,
        selected_case_count=len(selected_cases),
        safe_seed_count=len(default_seed_files),
        seed_files=default_seed_files,
    )
    report["report_path"] = str(report_path)
    return report


def select_merge_cases(
    compile_index_path: Path,
    smoke_index_path: Path,
    fix_loop_index_path: Path | None = None,
) -> Tuple[List[Path], Dict[str, str]]:
    compile_successes = {
        str(path)
        for path in collect_successful_harnesses(compile_index_path, fix_loop_index_path)
    }
    smoke_index = json.loads(smoke_index_path.read_text(encoding="utf-8"))
    smoke_statuses = {
        report["harness"]: report.get("status", "")
        for report in smoke_index.get("reports", [])
    }

    selected = []
    excluded: Dict[str, str] = {}
    for harness in sorted(smoke_statuses):
        if harness not in compile_successes:
            excluded[harness] = "compile_failed_or_missing"
        elif smoke_statuses[harness] != "ok":
            excluded[harness] = "smoke_failed_or_missing"
        else:
            selected.append(Path(harness))

    for harness in sorted(compile_successes):
        if harness not in smoke_statuses:
            excluded[harness] = "smoke_failed_or_missing"
    return selected, excluded


def render_registry(case_entries: List[Tuple[str, str]]) -> str:
    lines = [
        "pub struct CaseEntry {",
        "    pub name: &'static str,",
        "    pub run: fn(&[u8]),",
        "}",
        "",
    ]
    for module_name, filename in case_entries:
        lines.append('#[path = "cases/{}"]'.format(filename))
        lines.append("mod {};".format(module_name))
    lines.append("")
    lines.append("pub static CASES: &[CaseEntry] = &[")
    for module_name, filename in case_entries:
        lines.append(
            '    CaseEntry {{ name: "{}", run: {}::run_case }},'.format(filename, module_name)
        )
    lines.append("];")
    lines.append("")
    return "\n".join(lines)


def render_merged_main() -> str:
    return """mod registry;

use std::fs;
use std::io::{self, Read};

fn read_input() -> Vec<u8> {
    if let Some(path) = std::env::args().nth(1) {
        return fs::read(path).unwrap_or_default();
    }
    let mut data = Vec::new();
    let _ = io::stdin().read_to_end(&mut data);
    data
}

fn dispatch(data: &[u8]) {
    if registry::CASES.is_empty() {
        return;
    }
    let (selector_bytes, payload) = if data.len() >= 2 {
        data.split_at(2)
    } else {
        (data, &[][..])
    };
    let selector = selector_bytes
        .iter()
        .fold(0usize, |acc, byte| (acc << 8) | (*byte as usize));
    let case = &registry::CASES[selector % registry::CASES.len()];
    (case.run)(payload);
}

#[cfg(feature = "seraph_afl")]
fn main() {
    afl::fuzz!(|data: &[u8]| {
        dispatch(data);
    });
}

#[cfg(not(feature = "seraph_afl"))]
fn main() {
    let data = read_input();
    dispatch(&data);
}
"""


def render_merged_manifest(
    target_name: str,
    crate_name: str,
    crate_import_name: str,
    workspace: Path,
) -> str:
    crate_config = json.loads((workspace / "crate_config.json").read_text(encoding="utf-8"))
    return (
        "[package]\n"
        f'name = "{target_name}"\n'
        'version = "0.1.0"\n'
        'edition = "2021"\n'
        "\n"
        "[features]\n"
        "default = []\n"
        'seraph_afl = ["dep:afl"]\n'
        "\n"
        "[dependencies]\n"
        f'{crate_import_name} = {{ package = "{crate_name}", path = "{crate_config["crate_dir"]}" }}\n'
        'afl = { version = "0.15", optional = true }\n'
    )


def encode_selector(selector: int) -> bytes:
    return selector.to_bytes(2, byteorder="big", signed=False)


def write_selector_safe_corpus(corpus_dir: Path, selected_cases: List[Path]) -> List[Path]:
    corpus_dir.mkdir(parents=True, exist_ok=True)
    seed_files = []
    for selector, _case_path in enumerate(selected_cases):
        seed_path = corpus_dir / "selector_{:04d}.bin".format(selector)
        seed_path.write_bytes(encode_selector(selector))
        seed_files.append(seed_path)
    return seed_files


def write_selector_safe_corpus_summary(
    summary_path: Path,
    target_name: str,
    manifest_path: Path,
    merge_report_path: Path,
    corpus_dir: Path,
    meta_dir: Path,
    selected_case_count: int,
    safe_seed_count: int,
    seed_files: List[Path],
) -> None:
    meta_dir.mkdir(parents=True, exist_ok=True)
    summary = {
        "version": "seraph.phase3.selector_safe_corpus.v1",
        "target_name": target_name,
        "manifest_path": str(manifest_path),
        "merge_report": str(merge_report_path),
        "corpus_dir": str(corpus_dir),
        "meta_dir": str(meta_dir),
        "selected_case_count": selected_case_count,
        "safe_seed_count": safe_seed_count,
        "crash_seed_count": 0,
        "seed_files": [str(path) for path in seed_files],
    }
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _sanitize_name(value: str) -> str:
    sanitized = re.sub(r"[^A-Za-z0-9_]+", "_", value)
    if not sanitized:
        sanitized = "seraph_merged"
    if sanitized[0].isdigit():
        sanitized = "seraph_{}".format(sanitized)
    return sanitized
