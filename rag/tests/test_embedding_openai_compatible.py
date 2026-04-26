import io
import json

from seraph_rag.embeddings import OpenAICompatibleEmbedder


class _FakeResponse:
    def __init__(self, payload):
        self._payload = payload

    def read(self):
        return json.dumps(self._payload).encode("utf-8")

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        return False


def test_openai_compatible_embedder_requests_embeddings(monkeypatch):
    captured = {}

    def fake_urlopen(request, timeout):
        captured["url"] = request.full_url
        captured["headers"] = dict(request.header_items())
        captured["body"] = json.loads(request.data.decode("utf-8"))
        captured["timeout"] = timeout
        return _FakeResponse(
            {
                "data": [
                    {"index": 1, "embedding": [0.3, 0.4]},
                    {"index": 0, "embedding": [0.1, 0.2]},
                ]
            }
        )

    monkeypatch.setattr("seraph_rag.embeddings.urlopen", fake_urlopen)
    embedder = OpenAICompatibleEmbedder(
        base_url="https://example.invalid/v1",
        api_key="secret",
        model="embed-model",
    )

    vectors = embedder.encode(["unsafe pointer", "ffi boundary"])

    assert vectors == [[0.1, 0.2], [0.3, 0.4]]
    assert captured["url"] == "https://example.invalid/v1/embeddings"
    assert captured["body"] == {
        "model": "embed-model",
        "input": ["unsafe pointer", "ffi boundary"],
    }
    assert captured["headers"]["Authorization"] == "Bearer secret"
    assert captured["timeout"] == 180.0
