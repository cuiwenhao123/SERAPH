from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class CaseProject:
    project_dir: Path
    manifest_path: Path
    main_rs_path: Path
    case_impl_path: Path


def render_case_wrapper_main() -> str:
    return """mod case_impl;

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

fn main() {
    let data = read_input();
    case_impl::run_case(&data);
}
"""


def write_case_project(project_dir: Path, harness: Path, manifest_text: str) -> CaseProject:
    src_dir = project_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = project_dir / "Cargo.toml"
    case_impl_path = src_dir / "case_impl.rs"
    main_rs_path = src_dir / "main.rs"
    manifest_path.write_text(manifest_text, encoding="utf-8")
    case_impl_path.write_text(harness.read_text(encoding="utf-8"), encoding="utf-8")
    main_rs_path.write_text(render_case_wrapper_main(), encoding="utf-8")
    return CaseProject(
        project_dir=project_dir,
        manifest_path=manifest_path,
        main_rs_path=main_rs_path,
        case_impl_path=case_impl_path,
    )
