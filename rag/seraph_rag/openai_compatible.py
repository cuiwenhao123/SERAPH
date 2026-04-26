from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Mapping, Optional, Union
from urllib.request import Request, urlopen

DEFAULT_OPENAI_WIRE_API = "chat_completions"
DEFAULT_OPENAI_COMPATIBLE_API_PATH = "/chat/completions"
DEFAULT_RESPONSES_API_PATH = "/responses"
DEFAULT_TEMPERATURE = 0.2
DEFAULT_TIMEOUT_SECONDS = 180.0


def build_openai_compatible_messages(payload: Mapping[str, Any]) -> List[Dict[str, str]]:
    version = payload.get("version")
    if version == "seraph.phase3.prompt.v1":
        return [
            {"role": "system", "content": str(payload["system_prompt"])},
            {"role": "user", "content": str(payload["user_prompt"])},
        ]
    if version == "seraph.phase3.compile_fixer_request.v1":
        return [
            {
                "role": "system",
                "content": (
                    "You are fixing a Rust harness for SERAPH. "
                    "Keep SERAPH markers, keep the target API call, "
                    "and return only Rust code."
                ),
            },
            {
                "role": "user",
                "content": json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True),
            },
        ]
    if version == "seraph.phase3.runtime_diagnose_request.v1":
        return [
            {
                "role": "system",
                "content": str(payload.get("instructions", "Summarize the runtime issue as JSON.")),
            },
            {
                "role": "user",
                "content": json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True),
            },
        ]
    raise ValueError(f"unsupported input version: {version}")


def build_openai_compatible_request_body(
    payload: Mapping[str, Any],
    model: str,
    *,
    wire_api: str = DEFAULT_OPENAI_WIRE_API,
    temperature: float = DEFAULT_TEMPERATURE,
    extra_body: Optional[Mapping[str, Any]] = None,
) -> Dict[str, Any]:
    normalized_wire_api = normalize_wire_api(wire_api)
    messages = build_openai_compatible_messages(payload)
    if normalized_wire_api == "responses":
        system_text = ""
        user_text = ""
        for message in messages:
            if message.get("role") == "system":
                system_text = message.get("content", "")
            elif message.get("role") == "user":
                user_text = message.get("content", "")
        body: Dict[str, Any] = {
            "model": model,
            "instructions": system_text,
            "input": user_text,
            "temperature": temperature,
        }
    else:
        body = {
            "model": model,
            "messages": messages,
            "temperature": temperature,
        }
    if extra_body:
        body.update(dict(extra_body))
    return body


def request_openai_compatible_completion(
    *,
    base_url: str,
    api_key: str = "",
    request_body: Mapping[str, Any],
    wire_api: str = DEFAULT_OPENAI_WIRE_API,
    api_path: Optional[str] = None,
    extra_headers: Optional[Mapping[str, str]] = None,
    timeout_seconds: float = DEFAULT_TIMEOUT_SECONDS,
) -> Dict[str, Any]:
    normalized_wire_api = normalize_wire_api(wire_api)
    url = "{}{}".format(base_url.rstrip("/"), _normalize_api_path(api_path, normalized_wire_api))
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
    }
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    if extra_headers:
        headers.update(dict(extra_headers))
    request = Request(
        url,
        data=json.dumps(request_body, ensure_ascii=False).encode("utf-8"),
        headers=headers,
        method="POST",
    )
    with urlopen(request, timeout=timeout_seconds) as response:
        return json.loads(response.read().decode("utf-8"))


def extract_openai_compatible_content(
    response: Mapping[str, Any],
    *,
    wire_api: str = DEFAULT_OPENAI_WIRE_API,
) -> str:
    normalized_wire_api = normalize_wire_api(wire_api)
    if normalized_wire_api == "responses":
        output_text = response.get("output_text")
        if isinstance(output_text, str) and output_text:
            return output_text
        output = response.get("output")
        if isinstance(output, list):
            text_parts: List[str] = []
            for item in output:
                if not isinstance(item, Mapping):
                    continue
                if item.get("type") != "message":
                    continue
                content = item.get("content")
                if isinstance(content, list):
                    for part in content:
                        if not isinstance(part, Mapping):
                            continue
                        if part.get("type") in {"output_text", "text"}:
                            text = part.get("text")
                            if isinstance(text, str):
                                text_parts.append(text)
                elif isinstance(content, str):
                    text_parts.append(content)
            if text_parts:
                return "".join(text_parts)
        raise ValueError("responses output missing text content")

    choices = response.get("choices")
    if not isinstance(choices, list) or not choices:
        raise ValueError("response missing choices")

    first = choices[0]
    if isinstance(first, Mapping):
        message = first.get("message")
        if isinstance(message, Mapping):
            content = message.get("content")
            if isinstance(content, str):
                return content
            if isinstance(content, list):
                return "".join(_content_part_text(part) for part in content)
            if isinstance(content, Mapping):
                text = content.get("text")
                if isinstance(text, str):
                    return text
        text = first.get("text")
        if isinstance(text, str):
            return text

    raise ValueError("response missing text content")


def write_openai_compatible_response(
    input_path: Union[str, Path],
    output_path: Union[str, Path],
    *,
    base_url: str,
    api_key: str = "",
    model: str,
    wire_api: str = DEFAULT_OPENAI_WIRE_API,
    api_path: Optional[str] = None,
    temperature: float = DEFAULT_TEMPERATURE,
    extra_headers: Optional[Mapping[str, str]] = None,
    extra_body: Optional[Mapping[str, Any]] = None,
    timeout_seconds: float = DEFAULT_TIMEOUT_SECONDS,
) -> Dict[str, Any]:
    input_file = Path(input_path)
    output_file = Path(output_path)
    payload = json.loads(input_file.read_text(encoding="utf-8"))
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
    content = extract_openai_compatible_content(response, wire_api=wire_api)
    output_file.parent.mkdir(parents=True, exist_ok=True)
    output_file.write_text(content, encoding="utf-8")
    return {
        "status": "ok",
        "input": str(input_file),
        "output": str(output_file),
        "model": model,
        "wire_api": normalize_wire_api(wire_api),
        "api_path": _normalize_api_path(api_path, normalize_wire_api(wire_api)),
    }


def normalize_wire_api(wire_api: str) -> str:
    normalized = (wire_api or DEFAULT_OPENAI_WIRE_API).strip().lower().replace("/", "_")
    if normalized in {"chat_completions", "chat_completions_api"}:
        return "chat_completions"
    if normalized == "responses":
        return "responses"
    raise ValueError(f"unsupported wire_api: {wire_api}")


def _normalize_api_path(api_path: Optional[str], wire_api: str) -> str:
    if not api_path:
        return (
            DEFAULT_RESPONSES_API_PATH
            if wire_api == "responses"
            else DEFAULT_OPENAI_COMPATIBLE_API_PATH
        )
    return api_path if api_path.startswith("/") else f"/{api_path}"


def _content_part_text(part: Any) -> str:
    if isinstance(part, str):
        return part
    if isinstance(part, Mapping):
        text = part.get("text")
        if isinstance(text, str):
            return text
    return ""
