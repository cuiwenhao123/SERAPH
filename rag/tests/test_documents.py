from pathlib import Path

from seraph_rag.documents import build_api_documents, return_shape
from seraph_rag.knowledge_loader import load_knowledge

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_load_knowledge_reads_crate_meta():
    knowledge = load_knowledge(FIXTURE)
    assert knowledge["crate_meta"]["crate_import_name"] == "fixture_crate"


def test_return_shape_classifies_common_returns():
    assert return_shape("Result<(), Error>") == "Result"
    assert return_shape("Option<&str>") == "Option"
    assert return_shape("Self") == "Self"
    assert return_shape("()") == "Void"
    assert return_shape("usize") == "Primitive"
    assert return_shape("Buffer") == "Other"


def test_build_api_documents_marks_unsafe_metadata():
    knowledge = load_knowledge(FIXTURE)
    docs = build_api_documents(knowledge)
    target = next(doc for doc in docs if doc.api_id.endswith("get_unchecked"))
    assert target.doc_id == "api::fn::fixture_crate::Buffer::get_unchecked"
    assert "Safety: The index must be in bounds." in target.text
    assert "WARNING: contains unsafe code block" in target.text
    assert target.metadata["has_unsafe"] is True
    assert target.metadata["owner_type_id"] == "type::fixture_crate::Buffer"
    assert target.metadata["return_shape"] == "Primitive"
