from seraph_rag.embeddings import HashingEmbedder
from seraph_rag.idioms import bundled_idioms


def test_hashing_embedder_is_deterministic_and_normalized():
    embedder = HashingEmbedder(dimensions=16)
    first = embedder.encode(["unsafe pointer aliasing"])[0]
    second = embedder.encode(["unsafe pointer aliasing"])[0]
    assert first == second
    assert len(first) == 16
    assert abs(sum(value * value for value in first) - 1.0) < 0.000001


def test_bundled_idioms_include_unsafe_and_result_guidance():
    idioms = bundled_idioms()
    categories = {idiom.metadata["category"] for idiom in idioms}
    text = "\n".join(idiom.text for idiom in idioms)
    assert "unsafe_semantics" in categories
    assert "error_handling" in categories
    assert "Result" in text
