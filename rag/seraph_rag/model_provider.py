from __future__ import annotations

import shlex
import subprocess
from pathlib import Path
from typing import Any, Dict, Optional, Union


def write_model_response(
    input_path: Union[str, Path],
    output_path: Union[str, Path],
    command_template: str,
    attempt: Optional[int] = None,
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
    process = subprocess.run(
        command,
        shell=True,
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0:
        raise RuntimeError(
            "model command failed with exit code {}: {}".format(
                process.returncode,
                process.stderr.strip(),
            )
        )

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
