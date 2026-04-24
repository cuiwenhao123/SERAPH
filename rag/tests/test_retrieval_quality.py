from pathlib import Path

from seraph_rag.graph_builder import build_graph, write_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import render_context_from_stores
from seraph_rag.vector_index import index_knowledge

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_render_context_from_stores_dedupes_target_and_honors_budget(tmp_path):
    vectordb = tmp_path / "vectordb"
    graph_path = tmp_path / "graph.pkl"
    knowledge = load_knowledge(FIXTURE)
    index_knowledge(knowledge, vectordb)
    write_graph(build_graph(knowledge), graph_path)

    markdown = render_context_from_stores(
        knowledge,
        vectordb,
        graph_path,
        max_context_chars=900,
    )

    similar_section = markdown.split("## Semantically Similar API Docs", 1)[1].split(
        "## Rust Idioms", 1
    )[0]
    assert "fixture_crate::Buffer::get_unchecked" not in similar_section
    assert len(markdown) <= 900
    assert "## Generation Rules" in markdown
    assert "SERAPH_STEP_OK" in markdown
