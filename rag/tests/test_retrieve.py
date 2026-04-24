from pathlib import Path

from seraph_rag.graph_builder import build_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import rank_unsafe_targets, render_context_markdown

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_rank_unsafe_targets_returns_only_unsafe_api():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    targets = rank_unsafe_targets(graph)
    assert [target.api_id for target in targets] == ["fn::fixture_crate::Buffer::get_unchecked"]
    assert targets[0].score > 0


def test_render_context_markdown_contains_codegen_rules():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]
    markdown = render_context_markdown(
        knowledge,
        graph,
        target,
        idioms=["Handle Result with early return."],
    )
    assert "# SERAPH RAG Harness Context" in markdown
    assert "fixture_crate::Buffer::get_unchecked" in markdown
    assert "fixture_crate::Buffer::new" in markdown
    assert "SERAPH_STEP_ENTER" in markdown
    assert "fixture_crate" in markdown
