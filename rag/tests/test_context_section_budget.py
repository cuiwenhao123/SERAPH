from pathlib import Path

from seraph_rag.graph_builder import build_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import _fit_context_budget, rank_unsafe_targets, render_context_markdown

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_fit_context_budget_truncates_variant_before_related_and_compile():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]
    markdown = render_context_markdown(knowledge, graph, target, max_context_chars=10_000)

    compact = _fit_context_budget(markdown, 1150)
    variant_section = compact.split("## Variant Opportunities", 1)[1].split(
        "## Similar API Usage", 1
    )[0]
    related_section = compact.split("## Related APIs", 1)[1].split(
        "## Variant Opportunities", 1
    )[0]
    compile_section = compact.split("## Compile-Time Facts", 1)[1].split(
        "## Related APIs", 1
    )[0]

    assert len(compact) <= 1150
    assert "- [Section truncated to fit budget]" in variant_section
    assert "- [Section truncated to fit budget]" in compact.split("## Similar API Usage", 1)[1].split(
        "## Rust Idioms", 1
    )[0]
    assert "- [Section truncated to fit budget]" in compact.split("## Rust Idioms", 1)[1]
    assert "- [Section truncated to fit budget]" not in related_section
    assert "- [Section truncated to fit budget]" not in compile_section
    assert "fn::fixture_crate::Buffer::get_unchecked" in compact
    assert "fixture_crate::Buffer::new" in compact


def test_fit_context_budget_truncates_related_and_reachability_before_compile():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]
    markdown = render_context_markdown(knowledge, graph, target, max_context_chars=10_000)

    compact = _fit_context_budget(markdown, 1000)
    reachable_section = compact.split("## Known Reachable Paths", 1)[1].split(
        "## Compile-Time Facts", 1
    )[0]
    related_section = compact.split("## Related APIs", 1)[1].split(
        "## Variant Opportunities", 1
    )[0]
    compile_section = compact.split("## Compile-Time Facts", 1)[1].split(
        "## Related APIs", 1
    )[0]

    assert len(compact) <= 1000
    assert "- [Section truncated to fit budget]" in reachable_section
    assert "- [Section truncated to fit budget]" in related_section
    assert "- [Section truncated to fit budget]" in compact.split("## Variant Opportunities", 1)[1].split(
        "## Similar API Usage", 1
    )[0]
    assert "- [Section truncated to fit budget]" not in compile_section
