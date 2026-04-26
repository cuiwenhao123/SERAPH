from pathlib import Path

import pytest

from seraph_rag.openai_compatible import (
    build_openai_compatible_messages,
    build_openai_compatible_request_body,
    extract_openai_compatible_content,
    write_openai_compatible_response,
)


def test_build_openai_compatible_messages_for_generation_prompt():
    payload = {
        "version": "seraph.phase3.prompt.v1",
        "system_prompt": "system rules",
        "user_prompt": "generate harness",
    }

    messages = build_openai_compatible_messages(payload)

    assert messages == [
        {"role": "system", "content": "system rules"},
        {"role": "user", "content": "generate harness"},
    ]


def test_build_openai_compatible_messages_for_fix_request():
    payload = {
        "version": "seraph.phase3.compile_fixer_request.v1",
        "target_api_id": "api::fixture::danger",
        "harness_source": "fn main() {}",
    }

    messages = build_openai_compatible_messages(payload)

    assert messages[0]["role"] == "system"
    assert "Keep SERAPH markers" in messages[0]["content"]
    assert messages[1]["role"] == "user"
    assert '"target_api_id": "api::fixture::danger"' in messages[1]["content"]


def test_build_openai_compatible_messages_for_runtime_request():
    payload = {
        "version": "seraph.phase3.runtime_diagnose_request.v1",
        "instructions": "Return strict JSON.",
        "target_api_id": "api::fixture::danger",
    }

    messages = build_openai_compatible_messages(payload)

    assert messages == [
        {"role": "system", "content": "Return strict JSON."},
        {"role": "user", "content": '{\n  "instructions": "Return strict JSON.",\n  "target_api_id": "api::fixture::danger",\n  "version": "seraph.phase3.runtime_diagnose_request.v1"\n}'},
    ]


def test_extract_openai_compatible_content_from_string_message():
    response = {
        "choices": [
            {
                "message": {
                    "content": "```rust\nfn main() {}\n```",
                }
            }
        ]
    }

    content = extract_openai_compatible_content(response)

    assert content == "```rust\nfn main() {}\n```"


def test_extract_openai_compatible_content_from_text_parts():
    response = {
        "choices": [
            {
                "message": {
                    "content": [
                        {"type": "text", "text": "first"},
                        {"type": "text", "text": "\nsecond"},
                    ]
                }
            }
        ]
    }

    content = extract_openai_compatible_content(response)

    assert content == "first\nsecond"


def test_build_openai_compatible_request_body_for_responses_api():
    payload = {
        "version": "seraph.phase3.prompt.v1",
        "system_prompt": "system rules",
        "user_prompt": "generate harness",
    }

    body = build_openai_compatible_request_body(
        payload,
        "gpt-5.4",
        wire_api="responses",
        temperature=0.0,
    )

    assert body == {
        "model": "gpt-5.4",
        "instructions": "system rules",
        "input": "generate harness",
        "temperature": 0.0,
    }


def test_extract_openai_compatible_content_from_responses_output():
    response = {
        "output": [
            {
                "type": "message",
                "content": [
                    {"type": "output_text", "text": "fn main() "},
                    {"type": "output_text", "text": "{ println!(\"ok\"); }"},
                ],
            }
        ]
    }

    content = extract_openai_compatible_content(response, wire_api="responses")

    assert content == 'fn main() { println!("ok"); }'


def test_write_openai_compatible_response_posts_and_writes_output(tmp_path, monkeypatch):
    captured = {}

    class FakeResponse:
        def __init__(self, payload: bytes):
            self.payload = payload

        def read(self) -> bytes:
            return self.payload

        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return False

    def fake_urlopen(request, timeout):
        captured["url"] = request.full_url
        captured["timeout"] = timeout
        captured["headers"] = dict(request.header_items())
        captured["body"] = request.data.decode("utf-8")
        return FakeResponse(
            b'{"choices":[{"message":{"content":"fn main() { println!(\\"ok\\"); }"}}]}'
        )

    monkeypatch.setattr("seraph_rag.openai_compatible.urlopen", fake_urlopen)

    input_path = tmp_path / "harness_prompt_001.json"
    output_path = tmp_path / "llm_response_001.md"
    input_path.write_text(
        '{"version":"seraph.phase3.prompt.v1","system_prompt":"system","user_prompt":"user"}\n',
        encoding="utf-8",
    )

    result = write_openai_compatible_response(
        input_path,
        output_path,
        base_url="https://example.invalid/v1",
        api_key="secret",
        model="demo-model",
        extra_headers={"X-Test": "yes"},
        extra_body={"temperature": 0.1},
        timeout_seconds=12.5,
    )

    assert result["status"] == "ok"
    assert captured["url"] == "https://example.invalid/v1/chat/completions"
    assert captured["timeout"] == 12.5
    assert "Bearer secret" in captured["headers"]["Authorization"]
    assert captured["headers"]["X-test"] == "yes"
    assert '"model": "demo-model"' in captured["body"]
    assert '"temperature": 0.1' in captured["body"]
    assert output_path.read_text(encoding="utf-8") == 'fn main() { println!("ok"); }'


def test_write_openai_compatible_response_supports_responses_api_without_api_key(tmp_path, monkeypatch):
    captured = {}

    class FakeResponse:
        def __init__(self, payload: bytes):
            self.payload = payload

        def read(self) -> bytes:
            return self.payload

        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return False

    def fake_urlopen(request, timeout):
        captured["url"] = request.full_url
        captured["headers"] = dict(request.header_items())
        captured["body"] = request.data.decode("utf-8")
        captured["timeout"] = timeout
        return FakeResponse(
            b'{"output":[{"type":"message","content":[{"type":"output_text","text":"ok-from-responses"}]}]}'
        )

    monkeypatch.setattr("seraph_rag.openai_compatible.urlopen", fake_urlopen)

    input_path = tmp_path / "harness_prompt_001.json"
    output_path = tmp_path / "llm_response_001.md"
    input_path.write_text(
        '{"version":"seraph.phase3.prompt.v1","system_prompt":"system","user_prompt":"user"}\n',
        encoding="utf-8",
    )

    result = write_openai_compatible_response(
        input_path,
        output_path,
        base_url="http://127.0.0.1:8080",
        api_key="",
        model="gpt-5.4",
        wire_api="responses",
        timeout_seconds=3.0,
    )

    assert result["status"] == "ok"
    assert result["wire_api"] == "responses"
    assert captured["url"] == "http://127.0.0.1:8080/responses"
    assert "Authorization" not in captured["headers"]
    assert '"model": "gpt-5.4"' in captured["body"]
    assert '"instructions": "system"' in captured["body"]
    assert '"input": "user"' in captured["body"]
    assert output_path.read_text(encoding="utf-8") == "ok-from-responses"


def test_build_openai_compatible_messages_rejects_unknown_version():
    with pytest.raises(ValueError):
        build_openai_compatible_messages({"version": "unknown"})
