from pathlib import Path

from seraph_rag.documents import build_api_documents
from seraph_rag.graph_builder import build_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import rank_unsafe_targets, render_context_markdown

FIXTURE = Path(__file__).parent / "fixtures" / "s3_audit_fixture_knowledge.json"


def test_phase2_reads_real_phase1_signature_and_module_fields():
    knowledge = load_knowledge(FIXTURE)
    docs = build_api_documents(knowledge)
    unsafe_doc = next(doc for doc in docs if doc.api_id.endswith("uses_unsafe_block"))
    assert unsafe_doc.doc_id == "api::s3_audit_fixture::uses_unsafe_block"
    assert "Signature: fn uses_unsafe_block(*const u8) -> Option<u8>" in unsafe_doc.text
    assert unsafe_doc.metadata["module_id"] == "mod::s3_audit_fixture"

    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]
    markdown = render_context_markdown(knowledge, graph, target)
    assert "fn uses_unsafe_block(*const u8) -> Option<u8>" in markdown
