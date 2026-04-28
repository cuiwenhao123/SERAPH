from __future__ import annotations

import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Union

from seraph_rag.case_harness import write_case_project


@dataclass(frozen=True)
class CrateConfig:
    crate_dir: Path
    package_name: str
    crate_import_name: str


def ensure_cargo_project(harness_path: Union[str, Path]) -> Path:
    harness = Path(harness_path)
    workspace = _workspace_dir(harness)
    config = _load_crate_config(workspace)
    project_dir = workspace / "_cargo_projects" / harness.stem
    package_name = _cargo_package_name(harness.stem)
    write_case_project(
        project_dir,
        harness,
        _cargo_manifest(package_name, config),
    )
    return project_dir


def compile_harness(harness_path: Union[str, Path]):
    project_dir = ensure_cargo_project(harness_path)
    return subprocess.run(
        ["cargo", "build", "--manifest-path", str(project_dir / "Cargo.toml")],
        capture_output=True,
        text=True,
        check=False,
    )


def smoke_harness(harness_path: Union[str, Path], stdin_bytes: bytes = b""):
    project_dir = ensure_cargo_project(harness_path)
    return subprocess.run(
        ["cargo", "run", "--quiet", "--manifest-path", str(project_dir / "Cargo.toml")],
        input=stdin_bytes,
        capture_output=True,
        check=False,
    )


def _workspace_dir(harness: Path) -> Path:
    return harness.parent.parent


def _load_crate_config(workspace: Path) -> CrateConfig:
    config_path = workspace / "crate_config.json"
    payload = json.loads(config_path.read_text(encoding="utf-8"))
    return CrateConfig(
        crate_dir=Path(payload["crate_dir"]),
        package_name=str(payload["package_name"]),
        crate_import_name=str(payload["crate_import_name"]),
    )


def _cargo_package_name(stem: str) -> str:
    sanitized = re.sub(r"[^A-Za-z0-9_]+", "_", stem)
    if not sanitized:
        sanitized = "seraph_harness"
    if sanitized[0].isdigit():
        sanitized = f"seraph_{sanitized}"
    return sanitized


def _cargo_manifest(package_name: str, config: CrateConfig) -> str:
    return (
        "[package]\n"
        f'name = "{package_name}"\n'
        'version = "0.1.0"\n'
        'edition = "2021"\n'
        "\n"
        "[dependencies]\n"
        f'{config.crate_import_name} = {{ package = "{config.package_name}", path = "{config.crate_dir}" }}\n'
    )
