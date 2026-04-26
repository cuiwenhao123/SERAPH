from pathlib import Path

from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import render_context_from_stores
from seraph_rag.vector_index import index_knowledge, persistent_client

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_index_knowledge_writes_api_docs_and_idioms(tmp_path):
    vectordb = tmp_path / "vectordb"
    knowledge = load_knowledge(FIXTURE)

    index_knowledge(knowledge, vectordb)

    client = persistent_client(vectordb)
    api_docs = client.get_collection("api_docs")
    rust_idioms = client.get_collection("rust_idioms")
    api_result = api_docs.get(
        ids=["api::fn::fixture_crate::Buffer::get_unchecked"],
        include=["documents", "metadatas"],
    )
    assert api_docs.count() == 3
    assert rust_idioms.count() >= 6
    assert api_docs.metadata["seraph:embedder"] == "openai_compatible"
    assert rust_idioms.metadata["seraph:embedder"] == "openai_compatible"
    assert "WARNING: contains unsafe code block" in api_result["documents"][0]
    assert api_result["metadatas"][0]["has_unsafe"] is True


def test_render_context_from_stores_includes_vector_idioms(tmp_path):
    vectordb = tmp_path / "vectordb"
    graph_path = tmp_path / "graph.pkl"
    knowledge = load_knowledge(FIXTURE)
    index_knowledge(knowledge, vectordb)

    from seraph_rag.graph_builder import build_graph, write_graph

    write_graph(build_graph(knowledge), graph_path)

    markdown = render_context_from_stores(knowledge, vectordb, graph_path)

    assert "## Rust Idioms" in markdown
    assert "unsafe" in markdown.lower()
    assert "fixture_crate::Buffer::get_unchecked" in markdown
