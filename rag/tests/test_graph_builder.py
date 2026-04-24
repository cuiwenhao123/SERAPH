from pathlib import Path

from seraph_rag.graph_builder import build_graph
from seraph_rag.knowledge_loader import load_knowledge

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_graph_links_type_to_methods_and_marks_unsafe_context():
    graph = build_graph(load_knowledge(FIXTURE))
    target = "fn::fixture_crate::Buffer::get_unchecked"
    assert graph.nodes[target]["kind"] == "api"
    assert graph.nodes[target]["has_unsafe"] is True
    assert graph.has_edge("type::fixture_crate::Buffer", target)
    assert graph.edges["type::fixture_crate::Buffer", target]["kind"] == "type_owns_method"
    context = set(graph.nodes[target]["unsafe_context_subgraph"])
    assert "fn::fixture_crate::Buffer::new" in context
    assert "fn::fixture_crate::Buffer::push" in context
