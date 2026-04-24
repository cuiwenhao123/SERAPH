from pathlib import Path

from seraph_rag.graph_builder import build_graph
from seraph_rag.knowledge_loader import load_knowledge

FIXTURE = Path(__file__).parent / "fixtures" / "s3_audit_fixture_knowledge.json"


def test_graph_adds_return_arg_and_trait_impl_edges_for_real_fixture():
    graph = build_graph(load_knowledge(FIXTURE))
    example_type = "type::s3_audit_fixture::ExampleType"
    new_api = "api::s3_audit_fixture::ExampleType::new"
    wrapped_api = "api::s3_audit_fixture::ExampleType::wrapped"
    trait_object_api = "api::s3_audit_fixture::use_trait_object"
    trait_id = "trait::s3_audit_fixture::ExampleTrait"

    assert graph.has_edge(new_api, example_type)
    assert graph.edges[new_api, example_type]["kind"] == "api_returns_type"
    assert graph.has_edge(wrapped_api, example_type)
    assert graph.edges[wrapped_api, example_type]["kind"] == "api_returns_type"
    assert graph.has_edge(trait_object_api, trait_id)
    assert graph.edges[trait_object_api, trait_id]["kind"] == "api_accepts_trait"
    assert graph.has_edge(example_type, trait_id)
    assert graph.edges[example_type, trait_id]["kind"] == "impl_connects_type_trait"
