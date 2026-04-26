#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any, Dict

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "rag"))

from seraph_rag.openai_compatible import (  # noqa: E402
    DEFAULT_OPENAI_COMPATIBLE_API_PATH,
    DEFAULT_OPENAI_WIRE_API,
    DEFAULT_TEMPERATURE,
    DEFAULT_TIMEOUT_SECONDS,
    build_openai_compatible_request_body,
    extract_openai_compatible_content,
    request_openai_compatible_completion,
    write_openai_compatible_response,
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--output")
    parser.add_argument("--attempt", type=int)
    parser.add_argument("--temperature", type=float)
    args = parser.parse_args()

    base_url = _required_env("SERAPH_LLM_BASE_URL")
    api_key = os.environ.get("SERAPH_LLM_API_KEY", "")
    model = _required_env("SERAPH_LLM_MODEL")
    wire_api = os.environ.get("SERAPH_LLM_WIRE_API", DEFAULT_OPENAI_WIRE_API)
    api_path = os.environ.get("SERAPH_LLM_API_PATH")
    if api_path is None and wire_api == DEFAULT_OPENAI_WIRE_API:
        api_path = DEFAULT_OPENAI_COMPATIBLE_API_PATH
    timeout_seconds = float(os.environ.get("SERAPH_LLM_TIMEOUT_SECONDS", DEFAULT_TIMEOUT_SECONDS))
    temperature = args.temperature
    if temperature is None:
        temperature = float(os.environ.get("SERAPH_LLM_TEMPERATURE", DEFAULT_TEMPERATURE))
    extra_headers = _json_object_env("SERAPH_LLM_EXTRA_HEADERS")
    extra_body = _json_object_env("SERAPH_LLM_EXTRA_BODY")

    if args.output:
        write_openai_compatible_response(
            args.input,
            args.output,
            base_url=base_url,
            api_key=api_key,
            model=model,
            wire_api=wire_api,
            api_path=api_path,
            temperature=temperature,
            extra_headers=extra_headers,
            extra_body=extra_body,
            timeout_seconds=timeout_seconds,
        )
        return 0

    payload = json.loads(Path(args.input).read_text(encoding="utf-8"))
    request_body = build_openai_compatible_request_body(
        payload,
        model,
        wire_api=wire_api,
        temperature=temperature,
        extra_body=extra_body,
    )
    response = request_openai_compatible_completion(
        base_url=base_url,
        api_key=api_key,
        request_body=request_body,
        wire_api=wire_api,
        api_path=api_path,
        extra_headers=extra_headers,
        timeout_seconds=timeout_seconds,
    )
    sys.stdout.write(extract_openai_compatible_content(response, wire_api=wire_api))
    return 0


def _required_env(name: str) -> str:
    value = os.environ.get(name)
    if value:
        return value
    raise SystemExit(f"missing required environment variable: {name}")


def _json_object_env(name: str) -> Dict[str, Any]:
    raw = os.environ.get(name)
    if not raw:
        return {}
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"{name} must be valid JSON") from exc
    if not isinstance(value, dict):
        raise SystemExit(f"{name} must be a JSON object")
    return value


if __name__ == "__main__":
    raise SystemExit(main())
