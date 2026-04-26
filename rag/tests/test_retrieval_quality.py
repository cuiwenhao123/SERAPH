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
        max_context_chars=1025,
    )

    variant_section = markdown.split("## Variant Opportunities", 1)[1].split(
        "## Similar API Usage", 1
    )[0]
    related_section = markdown.split("## Related APIs", 1)[1].split(
        "## Variant Opportunities", 1
    )[0]
    compile_section = markdown.split("## Compile-Time Facts", 1)[1].split(
        "## Related APIs", 1
    )[0]
    similar_section = markdown.split("## Similar API Usage", 1)[1].split(
        "## Rust Idioms", 1
    )[0]
    assert "- [Section truncated to fit budget]" in variant_section
    assert "- [Section truncated to fit budget]" in similar_section
    assert "- [Section truncated to fit budget]" in markdown.split("## Rust Idioms", 1)[1]
    assert "- [Section truncated to fit budget]" not in related_section
    assert "- [Section truncated to fit budget]" not in compile_section
    assert "fixture_crate::Buffer::get_unchecked" not in similar_section
    assert len(markdown) <= 1025
    assert "## Generation Rules" not in markdown
