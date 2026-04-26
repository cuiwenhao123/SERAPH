import pytest

from seraph_rag.embeddings import (
    HashingEmbedder,
    OpenAICompatibleEmbedder,
    build_embedder,
    embedder_backend_name,
    embedding_model_name,
)


def test_build_embedder_defaults_to_hashing(monkeypatch):
    monkeypatch.delenv("SERAPH_EMBEDDER", raising=False)
    monkeypatch.delenv("SERAPH_EMBEDDING_BACKEND", raising=False)
    monkeypatch.delenv("SERAPH_EMBEDDING_MODEL", raising=False)
    embedder = build_embedder()
    assert isinstance(embedder, HashingEmbedder)
    assert embedder_backend_name() == "hashing"
    assert embedding_model_name() is None


def test_build_embedder_accepts_explicit_hashing(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDING_BACKEND", "hashing")
    embedder = build_embedder()
    assert isinstance(embedder, HashingEmbedder)
    assert embedder_backend_name() == "hashing"


def test_build_embedder_rejects_unknown_backend(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDING_BACKEND", "unknown")
    with pytest.raises(ValueError, match="unsupported SERAPH_EMBEDDING_BACKEND"):
        build_embedder()


def test_build_embedder_accepts_legacy_backend_alias(monkeypatch):
    monkeypatch.delenv("SERAPH_EMBEDDING_BACKEND", raising=False)
    monkeypatch.setenv("SERAPH_EMBEDDER", "hashing")

    embedder = build_embedder()

    assert isinstance(embedder, HashingEmbedder)
    assert embedder_backend_name() == "hashing"


def test_build_embedder_prefers_new_backend_name_over_legacy_alias(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDING_BACKEND", "hashing")
    monkeypatch.setenv("SERAPH_EMBEDDER", "unknown")

    embedder = build_embedder()

    assert isinstance(embedder, HashingEmbedder)
    assert embedder_backend_name() == "hashing"


def test_build_embedder_ignores_llm_model_env(monkeypatch):
    monkeypatch.delenv("SERAPH_EMBEDDING_BACKEND", raising=False)
    monkeypatch.delenv("SERAPH_EMBEDDER", raising=False)
    monkeypatch.delenv("SERAPH_EMBEDDING_MODEL", raising=False)
    monkeypatch.setenv("SERAPH_LLM_MODEL", "some-llm")

    embedder = build_embedder()

    assert isinstance(embedder, HashingEmbedder)
    assert embedder_backend_name() == "hashing"
    assert embedding_model_name() is None


def test_embedding_model_name_comes_from_embedding_config(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDING_MODEL", "unixcoder")
    monkeypatch.setenv("SERAPH_LLM_MODEL", "gpt-like-model")

    assert embedding_model_name() == "unixcoder"


def test_build_embedder_accepts_openai_compatible_backend(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDING_BACKEND", "openai_compatible")
    monkeypatch.setenv("SERAPH_EMBEDDING_BASE_URL", "https://example.invalid/v1")
    monkeypatch.setenv("SERAPH_EMBEDDING_MODEL", "embed-model")

    embedder = build_embedder()

    assert isinstance(embedder, OpenAICompatibleEmbedder)
    assert embedder_backend_name() == "openai_compatible"
    assert embedding_model_name() == "embed-model"


def test_build_embedder_rejects_duplicated_openai_compatible_api_key(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDING_BACKEND", "openai_compatible")
    monkeypatch.setenv("SERAPH_EMBEDDING_BASE_URL", "https://api.siliconflow.cn/v1")
    monkeypatch.setenv("SERAPH_EMBEDDING_MODEL", "Qwen/Qwen3-Embedding-8B")
    monkeypatch.setenv(
        "SERAPH_EMBEDDING_API_KEY",
        "sk-demo-token-123sk-demo-token-123",
    )

    with pytest.raises(ValueError, match="SERAPH_EMBEDDING_API_KEY appears duplicated"):
        build_embedder()


def test_build_embedder_accepts_siliconflow_backend_alias(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDING_BACKEND", "siliconflow")
    monkeypatch.setenv("SERAPH_EMBEDDING_BASE_URL", "https://api.siliconflow.cn/v1")
    monkeypatch.setenv("SERAPH_EMBEDDING_MODEL", "Qwen/Qwen3-Embedding-8B")

    embedder = build_embedder()

    assert isinstance(embedder, OpenAICompatibleEmbedder)
    assert embedder_backend_name() == "openai_compatible"


def test_build_embedder_accepts_single_openai_compatible_api_key(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDING_BACKEND", "openai_compatible")
    monkeypatch.setenv("SERAPH_EMBEDDING_BASE_URL", "https://api.siliconflow.cn/v1")
    monkeypatch.setenv("SERAPH_EMBEDDING_MODEL", "Qwen/Qwen3-Embedding-8B")
    monkeypatch.setenv("SERAPH_EMBEDDING_API_KEY", "sk-demo-token-123")

    embedder = build_embedder()

    assert isinstance(embedder, OpenAICompatibleEmbedder)
    assert embedder.api_key == "sk-demo-token-123"


def test_build_embedder_requires_base_url_for_openai_compatible(monkeypatch):
    monkeypatch.setenv("SERAPH_EMBEDDING_BACKEND", "openai_compatible")
    monkeypatch.delenv("SERAPH_EMBEDDING_BASE_URL", raising=False)
    monkeypatch.setenv("SERAPH_EMBEDDING_MODEL", "embed-model")

    with pytest.raises(ValueError, match="SERAPH_EMBEDDING_BASE_URL"):
        build_embedder()
