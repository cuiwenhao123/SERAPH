from seraph_rag.graph_builder import build_graph
from seraph_rag.retrieve import rank_unsafe_targets, render_context_markdown


def ranking_knowledge():
    return {
        "crate_meta": {"crate_import_name": "rank_fixture"},
        "modules": [
            {"module_id": "mod::rank_fixture", "canonical_path": "rank_fixture"}
        ],
        "types": [
            {
                "type_id": "type::rank_fixture::Bag",
                "name": "Bag",
                "canonical_path": "rank_fixture::Bag",
                "public_anchor_module_id": "mod::rank_fixture",
            }
        ],
        "apis": [
            {
                "api_id": "api::rank_fixture::Bag::new",
                "name": "new",
                "canonical_path": "rank_fixture::Bag::new",
                "public_anchor_module_id": "mod::rank_fixture",
                "owner_type_id": "type::rank_fixture::Bag",
                "api_kind": "constructor",
                "signature_text": "fn new() -> Bag",
                "return_type": "Self",
                "arg_types": [],
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::rank_fixture::Bag::iter",
                "name": "iter",
                "canonical_path": "rank_fixture::Bag::iter",
                "public_anchor_module_id": "mod::rank_fixture",
                "owner_type_id": "type::rank_fixture::Bag",
                "signature_text": "fn iter(&Self)",
                "receiver": "&Self",
                "return_type": "Iter",
                "arg_types": [],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::rank_fixture::Bag::get_unchecked",
                "name": "get_unchecked",
                "canonical_path": "rank_fixture::Bag::get_unchecked",
                "public_anchor_module_id": "mod::rank_fixture",
                "owner_type_id": "type::rank_fixture::Bag",
                "signature_text": "unsafe fn get_unchecked(&Self, usize) -> u8",
                "receiver": "&Self",
                "return_type": "u8",
                "arg_types": ["usize"],
                "is_unsafe": True,
                "doc_sections": {"safety": "index must be in bounds"},
            },
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }


def test_target_ranking_prefers_explicit_unchecked_safety_api():
    graph = build_graph(ranking_knowledge())
    targets = rank_unsafe_targets(graph)
    assert targets[0].api_id == "api::rank_fixture::Bag::get_unchecked"


def test_related_api_rendering_includes_roles_and_prioritizes_constructor():
    knowledge = ranking_knowledge()
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]
    markdown = render_context_markdown(knowledge, graph, target)
    related = markdown.split("## Related APIs", 1)[1].split(
        "## Semantically Similar API Docs", 1
    )[0]
    assert "role=constructor" in related
    assert related.index("Bag::new") < related.index("Bag::iter")
