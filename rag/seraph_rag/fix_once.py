from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, Union

from seraph_rag.compile_check import DEFAULT_COMMAND_TEMPLATE, run_compile_check
from seraph_rag.compile_fixer_response import write_fixed_harness


def run_fix_once(
    fix_request_path: Union[str, Path],
    response_path: Union[str, Path],
    output_dir: Union[str, Path],
    report_dir: Union[str, Path],
    attempt: int,
    command_template: str = DEFAULT_COMMAND_TEMPLATE,
) -> Dict[str, Any]:
    request = json.loads(Path(fix_request_path).read_text(encoding="utf-8"))
    round_no = int(request["round"])
    variant = int(request["variant"])
    fixed_harness = write_fixed_harness(
        fix_request_path,
        response_path,
        output_dir,
        attempt=attempt,
    )
    report_path = Path(report_dir) / "compile_{:03d}_{:02d}_fixed_{:02d}.json".format(
        round_no,
        variant,
        attempt,
    )
    return run_compile_check(
        fixed_harness,
        report_path,
        command_template=command_template,
    )
