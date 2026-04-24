from __future__ import annotations

import pickle
from pathlib import Path
from typing import Any, Dict, Union

import networkx as nx


def _string_ids(values):
    return {value for value in values if isinstance(value, str)}


def build_graph(knowledge: Dict[str, Any]) -> nx.DiGraph:
    graph = nx.DiGraph()
    risk_facts = knowledge.get("risk_facts", {})
    unsafe_api_ids = _string_ids(risk_facts.get("unsafe_functions", []))
    ffi_api_ids = _string_ids(risk_facts.get("ffi_functions", []))
    panic_api_ids = _string_ids(risk_facts.get("panic_sites", []))

    for module in knowledge.get("modules", []):
        module_id = module["module_id"]
        graph.add_node(
            module_id,
            kind="module",
            path=module.get("canonical_path", module_id),
        )

    for type_info in knowledge.get("types", []):
        type_id = type_info["type_id"]
        graph.add_node(
            type_id,
            kind="type",
            path=type_info.get("canonical_path", type_id),
        )
        module_id = type_info.get("public_anchor_module_id")
        if module_id:
            graph.add_edge(module_id, type_id, kind="module_contains_type")

    for trait in knowledge.get("trait_registry", []):
        trait_id = trait["trait_id"]
        graph.add_node(
            trait_id,
            kind="trait",
            path=trait.get("canonical_path", trait_id),
            is_unsafe=bool(trait.get("is_unsafe")),
        )

    for api in knowledge.get("apis", []):
        api_id = api["api_id"]
        has_unsafe = (
            bool(api.get("is_unsafe"))
            or bool(api.get("contains_unsafe_block"))
            or api_id in unsafe_api_ids
        )
        graph.add_node(
            api_id,
            kind="api",
            path=api.get("canonical_path", api_id),
            signature=api.get("signature") or api.get("signature_text", ""),
            has_unsafe=has_unsafe,
            has_ffi=api_id in ffi_api_ids,
            has_panic_points=api_id in panic_api_ids,
            has_safety_docs=bool((api.get("doc_sections") or {}).get("safety")),
            has_raw_pointer_args=any("*const" in arg or "*mut" in arg for arg in (api.get("arg_types") or [])),
        )
        module_id = api.get("module_id") or api.get("public_anchor_module_id")
        if module_id:
            graph.add_edge(module_id, api_id, kind="module_contains_api")
        owner_type_id = api.get("owner_type_id")
        if owner_type_id:
            graph.add_edge(owner_type_id, api_id, kind="type_owns_method")
        for type_id in _return_type_ids(api, knowledge):
            graph.add_edge(api_id, type_id, kind="api_returns_type")
        for type_id in _arg_type_ids(api, knowledge):
            graph.add_edge(api_id, type_id, kind="api_accepts_type")
        for trait_id in _arg_trait_ids(api, knowledge):
            graph.add_edge(api_id, trait_id, kind="api_accepts_trait")

    for impl_info in knowledge.get("trait_impl_registry", []):
        type_id = (
            impl_info.get("type_id")
            or impl_info.get("for_type_id")
            or impl_info.get("target_type_id")
        )
        trait_id = impl_info.get("trait_id")
        if type_id and trait_id and graph.has_node(type_id) and graph.has_node(trait_id):
            graph.add_edge(type_id, trait_id, kind="impl_connects_type_trait")

    for node_id, data in list(graph.nodes(data=True)):
        if data.get("kind") == "api" and data.get("has_unsafe"):
            subgraph = nx.ego_graph(graph.to_undirected(), node_id, radius=2)
            graph.nodes[node_id]["unsafe_context_subgraph"] = sorted(subgraph.nodes())

    return graph


def _return_type_ids(api: Dict[str, Any], knowledge: Dict[str, Any]) -> set:
    return_type = api.get("return_type") or ""
    owner_type_id = api.get("owner_type_id")
    if owner_type_id and _type_text_mentions_self(return_type):
        return {owner_type_id}
    return _matching_type_ids(return_type, knowledge)


def _arg_type_ids(api: Dict[str, Any], knowledge: Dict[str, Any]) -> set:
    matched = set()
    for arg_type in api.get("arg_types", []) or []:
        matched.update(_matching_type_ids(arg_type, knowledge))
    return matched


def _arg_trait_ids(api: Dict[str, Any], knowledge: Dict[str, Any]) -> set:
    matched = set()
    arg_text = "\n".join(api.get("arg_types", []) or [])
    for trait in knowledge.get("trait_registry", []):
        name = trait.get("name", "")
        path = trait.get("canonical_path", "")
        if name and ("dyn {}".format(name) in arg_text or path in arg_text):
            matched.add(trait["trait_id"])
    return matched


def _matching_type_ids(type_text: str, knowledge: Dict[str, Any]) -> set:
    matched = set()
    for type_info in knowledge.get("types", []):
        name = type_info.get("name", "")
        path = type_info.get("canonical_path", "")
        if not name:
            continue
        if _type_text_mentions(type_text, name) or (path and path in type_text):
            matched.add(type_info["type_id"])
    return matched


def _type_text_mentions_self(type_text: str) -> bool:
    return _type_text_mentions(type_text, "Self")


def _type_text_mentions(type_text: str, name: str) -> bool:
    normalized = (
        type_text.replace("::", " ")
        .replace("<", " ")
        .replace(">", " ")
        .replace(",", " ")
        .replace("&", " ")
        .replace("'", " ")
        .replace("*", " ")
        .replace("(", " ")
        .replace(")", " ")
    )
    return name in normalized.split()


def write_graph(graph: nx.Graph, path: Union[str, Path]) -> None:
    output = Path(path)
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("wb") as handle:
        pickle.dump(graph, handle)


def read_graph(path: Union[str, Path]) -> nx.Graph:
    with Path(path).open("rb") as handle:
        return pickle.load(handle)
