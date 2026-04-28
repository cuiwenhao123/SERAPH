from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, Union

from seraph_rag.compile_check import DEFAULT_COMMAND_TEMPLATE, run_compile_check
from seraph_rag.compile_fixer_response import write_fixed_harness

FIXER_RESPONSE_VALIDATION_EXIT_CODE = 3


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
    report_path = Path(report_dir) / "compile_{:03d}_{:02d}_fixed_{:02d}.json".format(
        round_no,
        variant,
        attempt,
    )
    try:
        fixed_harness = write_fixed_harness(
            fix_request_path,
            response_path,
            output_dir,
            attempt=attempt,
        )
    except ValueError as exc:
        failed_harness = Path(output_dir) / "harness_{:03d}_{:02d}_fixed_{:02d}.rs".format(
            round_no,
            variant,
            attempt,
        )
        result = {
            "version": "seraph.phase3.compile_check.v1",
            "harness": str(failed_harness),
            "command": "<fixer_response_validation>",
            "status": "failed",
            "exit_code": FIXER_RESPONSE_VALIDATION_EXIT_CODE,
            "stdout": "",
            "stderr": str(exc),
        }
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return result
    return run_compile_check(
        fixed_harness,
        report_path,
        command_template=command_template,
    )
