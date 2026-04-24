from pathlib import Path

import pytest

from seraph_rag.graph_builder import build_graph, write_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import render_context_from_stores
from seraph_rag.vector_index import index_knowledge

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_retrieve_rejects_mismatched_embedder_backend(tmp_path, monkeypatch):
    vectordb = tmp_path / "vectordb"
    graph_path = tmp_path / "graph.pkl"
    knowledge = load_knowledge(FIXTURE)
    monkeypatch.setenv("SERAPH_EMBEDDER", "hashing")
    index_knowledge(knowledge, vectordb)
    write_graph(build_graph(knowledge), graph_path)

    monkeypatch.setenv("SERAPH_EMBEDDER", "future-model")
    with pytest.raises(ValueError, match="embedding backend mismatch"):
        render_context_from_stores(knowledge, vectordb, graph_path)
