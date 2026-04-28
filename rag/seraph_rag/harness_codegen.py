from __future__ import annotations

import json
import re
from pathlib import Path
from typing import List, Union

_CODE_FENCE_RE = re.compile(r"```(?:rust|rs)?\s*\n(.*?)```", re.DOTALL | re.IGNORECASE)
_RUN_CASE_RE = re.compile(r"(?m)^pub\s+fn\s+run_case\s*\(\s*input\s*:\s*&\[\s*u8\s*\]\s*\)")


def extract_rust_code_blocks(response_text: str) -> List[str]:
    blocks = [block.strip() + "\n" for block in _CODE_FENCE_RE.findall(response_text)]
    if blocks:
        return blocks
    stripped = response_text.strip()
    return [stripped + "\n"] if stripped else []


def write_harnesses_from_response(
    prompt_path: Union[str, Path],
    response_path: Union[str, Path],
    output_dir: Union[str, Path],
    round_no: int,
) -> List[Path]:
    prompt = json.loads(Path(prompt_path).read_text(encoding="utf-8"))
    target_api_id = prompt.get("target_api_id")
    if not target_api_id:
        raise ValueError("prompt bundle missing target_api_id")

    response_text = Path(response_path).read_text(encoding="utf-8")
    blocks = extract_rust_code_blocks(response_text)
    if not blocks:
        raise ValueError("LLM response did not contain Rust harness code")

    output = Path(output_dir)
    output.mkdir(parents=True, exist_ok=True)
    written = []
    for index, source in enumerate(blocks, start=1):
        _ensure_case_contract(source, target_api_id)
        path = output / "harness_{:03d}_{:02d}.rs".format(round_no, index)
        path.write_text(source, encoding="utf-8")
        written.append(path)
    return written


def _ensure_case_contract(source: str, target_api_id: str) -> None:
    if "fn main" in source:
        raise ValueError("generated case must define `run_case`, not `fn main`")
    if not _RUN_CASE_RE.search(source):
        raise ValueError("generated case missing `pub fn run_case(input: &[u8])`")
    _ensure_target_markers(source, target_api_id)


def _ensure_target_markers(source: str, target_api_id: str) -> None:
    enter = "SERAPH_STEP_ENTER:"
    ok = "SERAPH_STEP_OK:"
    if enter not in source or ok not in source or target_api_id not in source:
        raise ValueError("generated harness missing SERAPH markers for {}".format(target_api_id))
