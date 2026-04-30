import io
import json
import ssl
from http.client import RemoteDisconnected
from http.client import IncompleteRead
from urllib.error import HTTPError, URLError

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


def test_openai_compatible_embedder_retries_incomplete_read(monkeypatch):
    calls = {"count": 0}

    class _BrokenResponse:
        def read(self):
            raise IncompleteRead(b'{"data": [', 32)

        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return False

    def fake_urlopen(request, timeout):
        calls["count"] += 1
        if calls["count"] == 1:
            return _BrokenResponse()
        return _FakeResponse({"data": [{"index": 0, "embedding": [0.5, 0.6]}]})

    monkeypatch.setattr("seraph_rag.embeddings.urlopen", fake_urlopen)
    embedder = OpenAICompatibleEmbedder(
        base_url="https://example.invalid/v1",
        api_key="secret",
        model="embed-model",
    )

    vectors = embedder.encode(["ffi boundary"])

    assert vectors == [[0.5, 0.6]]
    assert calls["count"] == 2


def test_openai_compatible_embedder_retries_urlerror(monkeypatch):
    calls = {"count": 0}

    def fake_urlopen(request, timeout):
        calls["count"] += 1
        if calls["count"] == 1:
            raise URLError("EOF occurred in violation of protocol")
        return _FakeResponse({"data": [{"index": 0, "embedding": [0.7, 0.8]}]})

    monkeypatch.setattr("seraph_rag.embeddings.urlopen", fake_urlopen)
    embedder = OpenAICompatibleEmbedder(
        base_url="https://example.invalid/v1",
        api_key="secret",
        model="embed-model",
    )

    vectors = embedder.encode(["ffi boundary"])

    assert vectors == [[0.7, 0.8]]
    assert calls["count"] == 2


def test_openai_compatible_embedder_retries_http_500(monkeypatch):
    calls = {"count": 0}

    def fake_urlopen(request, timeout):
        calls["count"] += 1
        if calls["count"] == 1:
            raise HTTPError(request.full_url, 500, "Internal Server Error", hdrs=None, fp=None)
        return _FakeResponse({"data": [{"index": 0, "embedding": [0.9, 1.0]}]})

    monkeypatch.setattr("seraph_rag.embeddings.urlopen", fake_urlopen)
    embedder = OpenAICompatibleEmbedder(
        base_url="https://example.invalid/v1",
        api_key="secret",
        model="embed-model",
    )

    vectors = embedder.encode(["ffi boundary"])

    assert vectors == [[0.9, 1.0]]
    assert calls["count"] == 2


def test_openai_compatible_embedder_retries_remote_disconnected(monkeypatch):
    calls = {"count": 0}

    def fake_urlopen(request, timeout):
        calls["count"] += 1
        if calls["count"] == 1:
            raise RemoteDisconnected("Remote end closed connection without response")
        return _FakeResponse({"data": [{"index": 0, "embedding": [1.1, 1.2]}]})

    monkeypatch.setattr("seraph_rag.embeddings.urlopen", fake_urlopen)
    embedder = OpenAICompatibleEmbedder(
        base_url="https://example.invalid/v1",
        api_key="secret",
        model="embed-model",
    )

    vectors = embedder.encode(["ffi boundary"])

    assert vectors == [[1.1, 1.2]]
    assert calls["count"] == 2


def test_openai_compatible_embedder_retries_ssl_eof(monkeypatch):
    calls = {"count": 0}

    def fake_urlopen(request, timeout):
        calls["count"] += 1
        if calls["count"] == 1:
            raise ssl.SSLEOFError(8, "EOF occurred in violation of protocol")
        return _FakeResponse({"data": [{"index": 0, "embedding": [1.3, 1.4]}]})

    monkeypatch.setattr("seraph_rag.embeddings.urlopen", fake_urlopen)
    embedder = OpenAICompatibleEmbedder(
        base_url="https://example.invalid/v1",
        api_key="secret",
        model="embed-model",
    )

    vectors = embedder.encode(["ffi boundary"])

    assert vectors == [[1.3, 1.4]]
    assert calls["count"] == 2
