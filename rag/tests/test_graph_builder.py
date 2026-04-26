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


def test_graph_links_deref_wrapper_type_to_target_type():
    knowledge = {
        "crate_meta": {"crate_import_name": "deref_fixture"},
        "modules": [{"module_id": "mod::deref_fixture", "canonical_path": "deref_fixture"}],
        "types": [
            {
                "type_id": "type::deref_fixture::Wrapper",
                "name": "Wrapper",
                "canonical_path": "deref_fixture::Wrapper",
                "public_anchor_module_id": "mod::deref_fixture",
            },
            {
                "type_id": "type::deref_fixture::Target",
                "name": "Target",
                "canonical_path": "deref_fixture::Target",
                "public_anchor_module_id": "mod::deref_fixture",
            },
        ],
        "apis": [],
        "trait_registry": [
            {
                "trait_id": "trait::core::ops::deref::Deref",
                "name": "Deref",
                "canonical_path": "core::ops::deref::Deref",
                "is_unsafe": False,
            }
        ],
        "trait_impl_registry": [
            {
                "trait_impl_id": "impl::Wrapper->Deref",
                "target_type_id": "type::deref_fixture::Wrapper",
                "trait_id": "trait::core::ops::deref::Deref",
                "trait_name": "Deref",
                "associated_type_bindings": [
                    {
                        "name": "Target",
                        "assigned_type": "Target",
                    }
                ],
            }
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }

    graph = build_graph(knowledge)

    assert graph.has_edge(
        "type::deref_fixture::Wrapper",
        "type::deref_fixture::Target",
    )
    assert (
        graph.edges["type::deref_fixture::Wrapper", "type::deref_fixture::Target"]["kind"]
        == "type_deref_target"
    )


def test_graph_synthesizes_external_deref_trait_node_from_impl_registry():
    knowledge = {
        "crate_meta": {"crate_import_name": "deref_fixture"},
        "modules": [{"module_id": "mod::deref_fixture", "canonical_path": "deref_fixture"}],
        "types": [
            {
                "type_id": "type::deref_fixture::Wrapper",
                "name": "Wrapper",
                "canonical_path": "deref_fixture::Wrapper",
                "public_anchor_module_id": "mod::deref_fixture",
            },
            {
                "type_id": "type::deref_fixture::Target",
                "name": "Target",
                "canonical_path": "deref_fixture::Target",
                "public_anchor_module_id": "mod::deref_fixture",
            },
        ],
        "apis": [],
        "trait_registry": [],
        "trait_impl_registry": [
            {
                "trait_impl_id": "impl::Wrapper->Deref",
                "target_type_id": "type::deref_fixture::Wrapper",
                "trait_id": "trait::core::ops::deref::Deref",
                "trait_name": "Deref",
                "trait_canonical_path": "core::ops::deref::Deref",
                "associated_type_bindings": [
                    {
                        "name": "Target",
                        "assigned_type": "Target",
                    }
                ],
            }
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }

    graph = build_graph(knowledge)

    assert graph.has_node("trait::core::ops::deref::Deref")
    assert graph.has_edge("type::deref_fixture::Wrapper", "trait::core::ops::deref::Deref")
    assert graph.has_edge("type::deref_fixture::Wrapper", "type::deref_fixture::Target")
