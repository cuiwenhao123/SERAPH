#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "rag"))

from seraph_rag.real_crate_runner import compile_harness, smoke_harness  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["compile", "smoke"])
    parser.add_argument("harness")
    args = parser.parse_args()

    try:
        if args.command == "compile":
            result = compile_harness(args.harness)
        else:
            stdin_bytes = _stdin_bytes()
            result = smoke_harness(args.harness, stdin_bytes=stdin_bytes)
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        return 2

    _write_output(result)
    return int(result.returncode)


def _stdin_bytes() -> bytes:
    input_path = os.environ.get("SERAPH_SMOKE_INPUT_PATH")
    if input_path:
        return Path(input_path).read_bytes()
    input_hex = os.environ.get("SERAPH_SMOKE_STDIN_HEX")
    if input_hex:
        return bytes.fromhex(input_hex)
    input_text = os.environ.get("SERAPH_SMOKE_STDIN")
    if input_text is not None:
        return input_text.encode("utf-8")
    return b""


def _write_output(result) -> None:
    stdout = result.stdout
    stderr = result.stderr
    if isinstance(stdout, bytes):
        sys.stdout.buffer.write(stdout)
    elif stdout:
        sys.stdout.write(stdout)
    if isinstance(stderr, bytes):
        sys.stderr.buffer.write(stderr)
    elif stderr:
        sys.stderr.write(stderr)


if __name__ == "__main__":
    raise SystemExit(main())
