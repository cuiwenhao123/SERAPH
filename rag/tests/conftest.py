import json
import math
from typing import List

import pytest


class _FakeResponse:
    def __init__(self, payload):
        self._payload = payload

    def read(self):
        return json.dumps(self._payload).encode("utf-8")

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        return False


def _project_token(token: str, dimensions: int) -> int:
    total = 0
    for index, char in enumerate(token):
        total += (index + 1) * ord(char)
    return total % dimensions


def _fake_embedding(text: str, dimensions: int = 24) -> List[float]:
    vector = [0.0] * dimensions
    tokens = [
        token
        for token in text.lower().replace("::", " ").replace("(", " ").replace(")", " ").split()
        if token
    ]
    if not tokens:
        vector[0] = 1.0
        return vector
    for token in tokens:
        primary = _project_token(token, dimensions)
        secondary = (primary + len(token)) % dimensions
        vector[primary] += 1.0
        vector[secondary] += 0.5
    norm = math.sqrt(sum(value * value for value in vector)) or 1.0
    return [value / norm for value in vector]


@pytest.fixture(autouse=True)
def use_fake_openai_compatible_embeddings(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDING_BACKEND", "openai_compatible")
    monkeypatch.setenv("SERAPH_EMBEDDING_BASE_URL", "https://example.invalid/v1")
    monkeypatch.setenv("SERAPH_EMBEDDING_MODEL", "test-embedding-model")

    def fake_urlopen(request, timeout):
        body = json.loads(request.data.decode("utf-8"))
        inputs = body.get("input", [])
        if isinstance(inputs, str):
            inputs = [inputs]
        return _FakeResponse(
            {
                "data": [
                    {"index": index, "embedding": _fake_embedding(text)}
                    for index, text in enumerate(inputs)
                ]
            }
        )

    monkeypatch.setattr("seraph_rag.embeddings.urlopen", fake_urlopen)
