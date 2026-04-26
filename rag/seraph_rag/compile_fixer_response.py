from __future__ import annotations

import json
from pathlib import Path
from typing import Union

from seraph_rag.harness_codegen import extract_rust_code_blocks


def write_fixed_harness(
    fix_request_path: Union[str, Path],
    response_path: Union[str, Path],
    output_dir: Union[str, Path],
    attempt: int,
) -> Path:
    request = json.loads(Path(fix_request_path).read_text(encoding="utf-8"))
    round_no = int(request["round"])
    variant = int(request["variant"])
    target_api_id = request["target_api_id"]
    response = Path(response_path).read_text(encoding="utf-8")
    blocks = extract_rust_code_blocks(response)
    if not blocks:
        raise ValueError("fixer response did not contain Rust harness code")
    source = blocks[0]
    _ensure_target_markers(source, target_api_id)
    output = Path(output_dir)
    output.mkdir(parents=True, exist_ok=True)
    path = output / "harness_{:03d}_{:02d}_fixed_{:02d}.rs".format(round_no, variant, attempt)
    path.write_text(source, encoding="utf-8")
    return path


def _ensure_target_markers(source: str, target_api_id: str) -> None:
    if "SERAPH_STEP_ENTER:" not in source or "SERAPH_STEP_OK:" not in source or target_api_id not in source:
        raise ValueError("fixed harness missing SERAPH markers for {}".format(target_api_id))
