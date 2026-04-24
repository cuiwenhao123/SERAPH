import pytest

from seraph_rag.embeddings import HashingEmbedder, build_embedder, embedder_backend_name


def test_build_embedder_defaults_to_hashing(monkeypatch):
    monkeypatch.delenv("SERAPH_EMBEDDER", raising=False)
    embedder = build_embedder()
    assert isinstance(embedder, HashingEmbedder)
    assert embedder_backend_name() == "hashing"


def test_build_embedder_accepts_explicit_hashing(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDER", "hashing")
    embedder = build_embedder()
    assert isinstance(embedder, HashingEmbedder)
    assert embedder_backend_name() == "hashing"


def test_build_embedder_rejects_unknown_backend(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDER", "unknown")
    with pytest.raises(ValueError, match="unsupported SERAPH_EMBEDDER"):
        build_embedder()
