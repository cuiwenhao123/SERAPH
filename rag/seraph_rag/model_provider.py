from __future__ import annotations

import shlex
import subprocess
import time
from pathlib import Path
from typing import Any, Dict, Optional, Union

TRANSIENT_MODEL_ERROR_MARKERS = (
    "HTTP Error 502",
    "HTTP Error 503",
    "HTTP Error 504",
    "Bad Gateway",
    "Gateway Timeout",
)
DEFAULT_MODEL_COMMAND_ATTEMPTS = 3


def write_model_response(
    input_path: Union[str, Path],
    output_path: Union[str, Path],
    command_template: str,
    attempt: Optional[int] = None,
    max_command_attempts: int = DEFAULT_MODEL_COMMAND_ATTEMPTS,
) -> Dict[str, Any]:
    input_file = Path(input_path)
    output_file = Path(output_path)
    output_file.parent.mkdir(parents=True, exist_ok=True)
    command = render_model_command(
        command_template,
        input_file,
        output_file,
        attempt=attempt,
    )
    last_process = None
    for command_attempt in range(1, max(1, max_command_attempts) + 1):
        process = subprocess.run(
            command,
            shell=True,
            text=True,
            capture_output=True,
            check=False,
        )
        last_process = process
        if process.returncode == 0:
            break
        if command_attempt >= max(1, max_command_attempts) or not _is_transient_model_failure(process.stderr):
            raise RuntimeError(
                "model command failed with exit code {}: {}".format(
                    process.returncode,
                    process.stderr.strip(),
                )
            )
        time.sleep(0.2)
    process = last_process
    if process is None:
        raise RuntimeError("model command did not execute")

    if "{output}" in command_template:
        if not output_file.exists():
            if process.stdout:
                output_file.write_text(process.stdout, encoding="utf-8")
            else:
                raise RuntimeError("model command did not write output file: {}".format(output_file))
    else:
        output_file.write_text(process.stdout, encoding="utf-8")

    return {
        "status": "ok",
        "input": str(input_file),
        "output": str(output_file),
        "command": command,
    }


def render_model_command(
    command_template: str,
    input_path: Path,
    output_path: Path,
    attempt: Optional[int] = None,
) -> str:
    command = command_template
    command = command.replace("{input}", shlex.quote(str(input_path)))
    command = command.replace("{output}", shlex.quote(str(output_path)))
    command = command.replace("{attempt}", str(attempt if attempt is not None else 0))
    return command


def _is_transient_model_failure(stderr: str) -> bool:
    text = (stderr or "").strip()
    return any(marker in text for marker in TRANSIENT_MODEL_ERROR_MARKERS)
