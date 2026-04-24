from pathlib import Path

from seraph_rag.graph_builder import build_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import rank_unsafe_targets

FIXTURE = Path(__file__).parent / "fixtures" / "s3_audit_fixture_knowledge.json"


def test_real_extract_fixture_has_rag_unsafe_target():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    targets = rank_unsafe_targets(graph)
    assert targets
    assert any("uses_unsafe_block" in target.path for target in targets)
