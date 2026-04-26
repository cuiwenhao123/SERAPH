from pathlib import Path

from seraph_rag.graph_builder import build_graph, write_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import render_context_from_stores
from seraph_rag.vector_index import index_knowledge

FIXTURE = Path(__file__).parent / "fixtures" / "s3_audit_fixture_knowledge.json"


def test_context_budget_preserves_all_core_sections(tmp_path):
    knowledge = load_knowledge(FIXTURE)
    vectordb = tmp_path / "vectordb"
    graph_path = tmp_path / "graph.pkl"
    index_knowledge(knowledge, vectordb)
    write_graph(build_graph(knowledge), graph_path)

    markdown = render_context_from_stores(
        knowledge,
        vectordb,
        graph_path,
        max_context_chars=1800,
    )

    assert len(markdown) <= 1800
    for section in [
        "## Crate Facts",
        "## Target API",
        "## Known Reachable Paths",
        "## Related APIs",
        "## Similar API Usage",
        "## Rust Idioms",
    ]:
        assert section in markdown
    assert "## Generation Rules" not in markdown
