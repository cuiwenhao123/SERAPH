from __future__ import annotations

import re
from collections import defaultdict, deque
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Union

import networkx as nx

from seraph_rag.embeddings import build_embedder, embedder_backend_name
from seraph_rag.documents import api_doc_id
from seraph_rag.schema import UnsafeTarget


def unsafe_priority(api_id: str, graph: nx.Graph) -> float:
    node = graph.nodes[api_id]
    name = str(node.get("path", api_id)).lower()
    score = 0.0
    if node.get("has_ffi"):
        score += 3.0
    if node.get("has_unsafe"):
        score += 2.0
    if "unchecked" in name or "unsafe" in name:
        score += 1.5
    if node.get("has_safety_docs"):
        score += 1.25
    if node.get("has_raw_pointer_args"):
        score += 1.0
    if node.get("has_panic_points"):
        score += 1.0
    score += graph.degree(api_id) * 0.1
    return score


def rank_unsafe_targets(
    graph: nx.Graph,
    excluded: Optional[Set[str]] = None,
) -> List[UnsafeTarget]:
    excluded = excluded or set()
    targets = []
    for node_id, data in graph.nodes(data=True):
        if data.get("kind") == "api" and data.get("has_unsafe") and node_id not in excluded:
            score = unsafe_priority(node_id, graph)
            targets.append(
                UnsafeTarget(
                    api_id=node_id,
                    path=data.get("path", node_id),
                    score=score,
                    reason="unsafe API or safe API containing unsafe block",
                )
            )
    return sorted(targets, key=lambda target: (-target.score, target.api_id))


def target_seed_type_ids(graph: nx.Graph, api_id: str) -> List[str]:
    if not graph.has_node(api_id):
        return []
    api = graph.nodes[api_id]
    if api.get("kind") != "api":
        return []
    owner_type_id = api.get("owner_type_id") or _owner_type_id(graph, api_id)
    receiver = str(api.get("receiver") or "").strip().lower()
    if not owner_type_id or not receiver or "self" not in receiver:
        return []
    if _api_is_constructor_like(api):
        return []
    return [owner_type_id]


def target_missing_seed_type_ids(graph: nx.Graph, api_id: str) -> List[str]:
    return [
        type_id
        for type_id in target_seed_type_ids(graph, api_id)
        if not type_has_public_producer(graph, type_id)
    ]


def is_target_constructible(graph: nx.Graph, api_id: str) -> bool:
    return not target_missing_seed_type_ids(graph, api_id)


def type_has_public_producer(
    graph: nx.Graph,
    type_id: str,
    seen_types: Optional[Set[str]] = None,
    seen_apis: Optional[Set[str]] = None,
) -> bool:
    if not graph.has_node(type_id):
        return False
    seen_types = set(seen_types or set())
    seen_apis = set(seen_apis or set())
    if type_id in seen_types:
        return False
    seen_types.add(type_id)

    for producer_api_id in _graph_in_neighbor_ids(graph, type_id, "api_returns_type"):
        if producer_api_id in seen_apis:
            continue
        next_seen_apis = set(seen_apis)
        next_seen_apis.add(producer_api_id)
        if _api_has_constructible_seed_chain(graph, producer_api_id, seen_types, next_seen_apis):
            return True

    for wrapper_type_id in _graph_in_neighbor_ids(graph, type_id, "type_deref_target"):
        if type_has_public_producer(graph, wrapper_type_id, seen_types, seen_apis):
            return True

    return False


def select_unsafe_target(
    graph: nx.Graph,
    round_no: int = 1,
    target_api_id: Optional[str] = None,
    excluded: Optional[Set[str]] = None,
) -> UnsafeTarget:
    targets = rank_unsafe_targets(graph, excluded=excluded)
    if not targets:
        raise ValueError("no unsafe targets available")
    if target_api_id:
        for target in targets:
            if target.api_id == target_api_id:
                return target
        raise ValueError("requested target API not found among unsafe targets: {}".format(target_api_id))
    constructible_targets = [target for target in targets if is_target_constructible(graph, target.api_id)]
    if constructible_targets:
        targets = constructible_targets
    target_index = max(round_no - 1, 0)
    if target_index >= len(targets):
        raise ValueError(
            "round {} requested target index {}, but only {} unsafe targets are available".format(
                round_no,
                target_index + 1,
                len(targets),
            )
        )
    return targets[target_index]


def render_context_markdown(
    knowledge: Dict[str, Any],
    graph: nx.Graph,
    target: UnsafeTarget,
    idioms: Optional[List[str]] = None,
    similar_api_docs: Optional[List[str]] = None,
    max_context_chars: int = 12000,
    max_setup_apis: int = 12,
    max_related_apis: int = 24,
    max_similar_docs: int = 5,
    max_idioms: int = 5,
) -> str:
    idioms = idioms or []
    similar_api_docs = similar_api_docs or []
    attempts = [
        (max_setup_apis, max_related_apis, max_similar_docs, max_idioms),
        (max_setup_apis, min(max_related_apis, 12), min(max_similar_docs, 3), min(max_idioms, 3)),
        (min(max_setup_apis, 8), min(max_related_apis, 6), min(max_similar_docs, 2), min(max_idioms, 2)),
        (min(max_setup_apis, 6), min(max_related_apis, 3), min(max_similar_docs, 1), min(max_idioms, 1)),
        (min(max_setup_apis, 4), 0, 0, 0),
    ]
    last_markdown = ""
    for setup_limit, related_limit, similar_limit, idiom_limit in attempts:
        markdown = _render_context_markdown_unbudgeted(
            knowledge,
            graph,
            target,
            idioms=idioms,
            similar_api_docs=similar_api_docs,
            max_setup_apis=setup_limit,
            max_related_apis=related_limit,
            max_similar_docs=similar_limit,
            max_idioms=idiom_limit,
        )
        last_markdown = markdown
        if max_context_chars <= 0 or len(markdown) <= max_context_chars:
            return markdown
    return _fit_context_budget(last_markdown, max_context_chars)


def _render_context_markdown_unbudgeted(
    knowledge: Dict[str, Any],
    graph: nx.Graph,
    target: UnsafeTarget,
    idioms: List[str],
    similar_api_docs: List[str],
    max_setup_apis: int,
    max_related_apis: int,
    max_similar_docs: int,
    max_idioms: int,
) -> str:
    apis = {api["api_id"]: api for api in knowledge.get("apis", [])}
    type_index = {type_info["type_id"]: type_info for type_info in knowledge.get("types", [])}
    trait_index = {trait["trait_id"]: trait for trait in knowledge.get("trait_registry", [])}
    crate_import_name = knowledge["crate_meta"]["crate_import_name"]
    target_api = apis[target.api_id]
    setup_entries = _collect_required_setup_entries(knowledge, graph, target_api)
    compile_hints = _collect_compile_hints(
        knowledge,
        graph,
        target_api,
        setup_entries[:max_setup_apis],
        type_index,
        trait_index,
    )
    setup_api_ids = {entry["api"]["api_id"] for entry in setup_entries[:max_setup_apis]}
    related_apis = _collect_related_apis(
        knowledge,
        graph,
        target_api,
        setup_api_ids,
        type_index,
        max_related_apis=max_related_apis,
    )
    lines = [
        "# SERAPH Rust Harness Context",
        "",
        "## Crate Facts",
        "- crate_name: {}".format(knowledge["crate_meta"].get("crate_name", crate_import_name)),
        "- crate_import_name: {}".format(crate_import_name),
        "- target_crate_kind: library",
        "",
        "## Target API",
        "- api_id: {}".format(target.api_id),
        "- path: {}".format(_api_display_path(target_api)),
        "- signature: {}".format(_api_signature_display(target_api)),
        "- target_kind: {}".format(target_api.get("api_kind", "")),
        "- owner_type: {}".format(_type_display_name(type_index.get(target_api.get("owner_type_id")), "")),
        "- owner_trait: {}".format(
            _trait_display_name(trait_index.get(target_api.get("owner_trait_id")), graph, "")
        ),
        "- receiver: {}".format(target_api.get("receiver", "")),
        "- return_shape: {}".format(target_api.get("return_type", "")),
        "- safety_summary: {}".format((target_api.get("doc_sections") or {}).get("safety", "")),
        "- errors_summary: {}".format((target_api.get("doc_sections") or {}).get("errors", "")),
        "- panics_summary: {}".format((target_api.get("doc_sections") or {}).get("panics", "")),
        "",
        "## Known Reachable Paths",
    ]
    for entry in setup_entries[:max_setup_apis]:
        api = entry["api"]
        lines.append(
            "- {}: {} — {} [goal=reach_target basis=producer_chain produces={} depth={}]".format(
                api["api_id"],
                _api_display_path(api),
                _api_signature_display(api),
                ", ".join(entry["produced_types"]),
                entry["upstream_depth"],
            )
        )
    if (
        compile_hints["imports"]
        or compile_hints["traits"]
        or compile_hints["trait_methods"]
        or compile_hints["enums"]
    ):
        lines.extend(["", "## Compile-Time Facts"])
    if compile_hints["imports"]:
        lines.extend(["### Exact Import Paths"])
        for item in compile_hints["imports"]:
            lines.append(
                "- {} => {} [kind={}]".format(
                    item["id"],
                    item["path"],
                    item["kind"],
                )
            )
    if compile_hints["traits"]:
        lines.extend(["", "### Required Traits"])
        for item in compile_hints["traits"]:
            lines.append(
                "- {}: required_methods={}; provided_methods={}".format(
                    item["path"],
                    ", ".join(item["required_methods"]) or "(none)",
                    ", ".join(item["provided_methods"]) or "(none)",
                )
            )
    if compile_hints["trait_methods"]:
        lines.extend(["", "### Trait Method Signatures"])
        for item in compile_hints["trait_methods"]:
            lines.append(
                "- {}: {} [{}]".format(
                    item["path"],
                    item["signature"],
                    item["classification"],
                )
            )
    if compile_hints["enums"]:
        lines.extend(["", "### Enum Variants"])
        for item in compile_hints["enums"]:
            suffix = " [non_exhaustive]" if item["is_non_exhaustive"] else ""
            lines.append(
                "- {}: {}{}".format(
                    item["path"],
                    " | ".join(item["variants"]),
                    suffix,
                )
            )
    lines.extend([
        "",
        "## Related APIs",
    ])
    for api in related_apis:
        lines.append(
            "- {}: {} — {} [{}]".format(
                api["api_id"],
                _api_display_path(api),
                _api_signature_display(api),
                _related_api_role(api, target_api),
            )
        )
    lines.extend(["", "## Similar API Usage"])
    target_path = target_api.get("canonical_path", target.api_id)
    excluded_paths = {
        target_path,
        *[entry["api"].get("canonical_path", entry["api"]["api_id"]) for entry in setup_entries[:max_setup_apis]],
        *[api.get("canonical_path", api["api_id"]) for api in related_apis],
    }
    excluded_api_ids = {target.api_id, *setup_api_ids, *[api["api_id"] for api in related_apis]}
    for doc in _filter_similar_docs(
        similar_api_docs,
        target.api_id,
        target_path,
        excluded_api_ids=excluded_api_ids,
        excluded_paths=excluded_paths,
    )[:max_similar_docs]:
        lines.append("- {}".format(_one_line(doc)))
    lines.extend(["", "## Rust Idioms"])
    for idiom in idioms[:max_idioms]:
        lines.append("- {}".format(idiom))
    return "\n".join(lines)


def _one_line(text: str) -> str:
    return " ".join(text.split())


def _query_documents(collection, query_text: str, n_results: int, query_embedding) -> List[str]:
    if n_results <= 0:
        return []
    count = collection.count()
    if count <= 0:
        return []
    result = collection.query(
        query_embeddings=[query_embedding],
        n_results=min(n_results, count),
    )
    documents = result.get("documents") or [[]]
    return list(documents[0])


def render_context_from_stores(
    knowledge: Dict[str, Any],
    vectordb: Union[str, Path],
    graph_path: Union[str, Path],
    excluded: Optional[Set[str]] = None,
    idiom_results: int = 5,
    similar_results: int = 5,
    max_context_chars: int = 12000,
    round_no: int = 1,
    target_api_id: Optional[str] = None,
) -> str:
    from seraph_rag.graph_builder import read_graph
    from seraph_rag.vector_index import persistent_client

    graph = read_graph(graph_path)
    target = select_unsafe_target(
        graph,
        round_no=round_no,
        target_api_id=target_api_id,
        excluded=excluded,
    )
    client = persistent_client(vectordb)
    api_docs = client.get_collection("api_docs")
    rust_idioms = client.get_collection("rust_idioms")
    _ensure_collection_embedder_matches(api_docs)
    _ensure_collection_embedder_matches(rust_idioms)
    target_doc_result = api_docs.get(
        ids=[api_doc_id(target.api_id)],
        include=["documents"],
    )
    target_doc = target_doc_result["documents"][0]
    embedder = build_embedder()
    target_embedding = embedder.encode([target_doc])[0]
    idiom_embedding = embedder.encode(["unsafe usage: {}".format(target_doc)])[0]
    similar_docs = _query_documents(api_docs, target_doc, similar_results, target_embedding)
    idioms = _query_documents(
        rust_idioms,
        "unsafe usage: {}".format(target_doc),
        idiom_results,
        idiom_embedding,
    )
    return render_context_markdown(
        knowledge,
        graph,
        target,
        idioms=idioms,
        similar_api_docs=similar_docs,
        max_context_chars=max_context_chars,
    )


def _ensure_collection_embedder_matches(collection) -> None:
    expected = embedder_backend_name()
    actual = (collection.metadata or {}).get("seraph:embedder", "hashing")
    if actual != expected:
        raise ValueError(
            "embedding backend mismatch: collection {!r} was indexed with {!r}, "
            "but SERAPH_EMBEDDING_BACKEND is {!r}. Rebuild the vector DB or switch backends.".format(
                collection.name,
                actual,
                expected,
            )
        )


def _filter_similar_docs(
    docs: List[str],
    target_api_id: str,
    target_path: str,
    excluded_api_ids: Optional[Set[str]] = None,
    excluded_paths: Optional[Set[str]] = None,
) -> List[str]:
    excluded_api_ids = excluded_api_ids or set()
    excluded_paths = excluded_paths or set()
    filtered = []
    seen = set()
    for doc in docs:
        one_line = _one_line(doc)
        if target_api_id in one_line or target_path in one_line:
            continue
        if any(api_id and api_id in one_line for api_id in excluded_api_ids):
            continue
        if any(path and path in one_line for path in excluded_paths):
            continue
        if one_line in seen:
            continue
        seen.add(one_line)
        filtered.append(doc)
    return filtered


def _rank_related_apis(apis: List[Dict[str, Any]], target_api: Dict[str, Any]) -> List[Dict[str, Any]]:
    target_owner = target_api.get("owner_type_id") or ""

    def score(api: Dict[str, Any]) -> tuple:
        name = api.get("name", "").lower()
        path = api.get("canonical_path", "").lower()
        api_kind = api.get("api_kind", "")
        owner = api.get("owner_type_id") or ""
        value = 0
        if owner and owner == target_owner:
            value += 30
        if api_kind in {"constructor", "associated_constructor"} or name in {"new", "default", "with_capacity"}:
            value += 25
        if api.get("is_unsafe") or api.get("contains_unsafe_block"):
            value += 18
        if "unsafe" in name or "unchecked" in name or "unsafe" in path or "unchecked" in path:
            value += 20
        if api.get("doc_sections", {}).get("safety"):
            value += 15
        if api.get("receiver") == "&mut Self" or api.get("receiver") == "&mut self":
            value += 10
        if path.endswith("::error") or "::error::" in path:
            value -= 30
        return (-value, api.get("canonical_path", api.get("api_id", "")))

    return sorted(apis, key=score)


def _collect_related_apis(
    knowledge: Dict[str, Any],
    graph: nx.Graph,
    target_api: Dict[str, Any],
    setup_api_ids: Set[str],
    type_index: Dict[str, Dict[str, Any]],
    max_related_apis: int,
) -> List[Dict[str, Any]]:
    apis = {api["api_id"]: api for api in knowledge.get("apis", [])}
    local_related_ids = graph.nodes[target_api["api_id"]].get("unsafe_context_subgraph", [])
    local_related = []
    seen_api_ids = set()
    for api_id in local_related_ids:
        if (
            api_id not in apis
            or api_id == target_api["api_id"]
            or api_id in setup_api_ids
            or _is_low_signal_related_api(apis[api_id], type_index)
        ):
            continue
        local_related.append(apis[api_id])
        seen_api_ids.add(api_id)

    companion_related = _collect_target_companion_apis(
        knowledge,
        graph,
        target_api["api_id"],
        setup_api_ids,
        seen_api_ids,
    )
    return _merge_related_api_lists(
        _rank_related_apis(local_related, target_api),
        companion_related,
        max_related_apis=max_related_apis,
    )


def _collect_target_companion_apis(
    knowledge: Dict[str, Any],
    graph: nx.Graph,
    target_api_id: str,
    setup_api_ids: Set[str],
    seen_api_ids: Set[str],
) -> List[Dict[str, Any]]:
    apis = {api["api_id"]: api for api in knowledge.get("apis", [])}
    ranked_target_ids = [item.api_id for item in rank_unsafe_targets(graph)]
    if not ranked_target_ids:
        return []
    if target_api_id in ranked_target_ids:
        current_index = ranked_target_ids.index(target_api_id)
        rotated_target_ids = ranked_target_ids[current_index + 1 :] + ranked_target_ids[:current_index]
    else:
        rotated_target_ids = ranked_target_ids

    companions = []
    for api_id in rotated_target_ids:
        if (
            api_id == target_api_id
            or api_id in setup_api_ids
            or api_id in seen_api_ids
            or api_id not in apis
        ):
            continue
        companions.append(apis[api_id])
    return companions


def _merge_related_api_lists(
    ranked_local: List[Dict[str, Any]],
    companion_related: List[Dict[str, Any]],
    max_related_apis: int,
) -> List[Dict[str, Any]]:
    if max_related_apis <= 0:
        return []
    companion_slots = min(
        len(companion_related),
        max(1, min(4, max_related_apis // 6 or 1)),
    )
    local_slots = max(0, max_related_apis - companion_slots)
    selected: List[Dict[str, Any]] = []
    seen_api_ids = set()

    for api in ranked_local[:local_slots]:
        if api["api_id"] in seen_api_ids:
            continue
        selected.append(api)
        seen_api_ids.add(api["api_id"])
    for api in companion_related[:companion_slots]:
        if api["api_id"] in seen_api_ids:
            continue
        selected.append(api)
        seen_api_ids.add(api["api_id"])
    for api in ranked_local[local_slots:]:
        if len(selected) >= max_related_apis:
            break
        if api["api_id"] in seen_api_ids:
            continue
        selected.append(api)
        seen_api_ids.add(api["api_id"])
    for api in companion_related[companion_slots:]:
        if len(selected) >= max_related_apis:
            break
        if api["api_id"] in seen_api_ids:
            continue
        selected.append(api)
        seen_api_ids.add(api["api_id"])
    return selected


def _related_api_role(api: Dict[str, Any], target_api: Dict[str, Any]) -> str:
    name = api.get("name", "").lower()
    api_kind = api.get("api_kind", "")
    roles = []
    if api_kind in {"constructor", "associated_constructor"} or name in {"new", "default", "with_capacity"}:
        roles.append("constructor")
    if api.get("owner_type_id") and api.get("owner_type_id") == target_api.get("owner_type_id"):
        roles.append("same_owner")
    if api.get("receiver") in {"&mut Self", "&mut self"}:
        roles.append("mutator")
    if api.get("doc_sections", {}).get("safety"):
        roles.append("safety_docs")
    if "unchecked" in name or "unsafe" in name:
        roles.append("unsafe_related")
    return "role=" + ",".join(roles or ["context"])


def _fit_context_budget(markdown: str, max_context_chars: int) -> str:
    if max_context_chars <= 0 or len(markdown) <= max_context_chars:
        return markdown
    title, sections = _split_markdown_sections(markdown)
    if not sections:
        return markdown[:max_context_chars]

    compression_order = [
        "## Similar API Usage",
        "## Rust Idioms",
        "## Related APIs",
        "## Compile-Time Facts",
        "## Known Reachable Paths",
        "## Target API",
        "## Crate Facts",
    ]
    compact = _render_markdown_sections(title, sections)
    for header in compression_order:
        compact = _compress_markdown_section(title, sections, header)
        if len(compact) <= max_context_chars:
            return compact

    if len(compact) <= max_context_chars:
        return compact
    return compact[:max_context_chars]


def _split_markdown_sections(markdown: str) -> tuple[str, List[Dict[str, Any]]]:
    title_lines: List[str] = []
    sections: List[Dict[str, Any]] = []
    current_header: Optional[str] = None
    current_lines: List[str] = []

    for line in markdown.splitlines():
        if line.startswith("## "):
            if current_header is None:
                pass
            else:
                sections.append({"header": current_header, "lines": current_lines[:]})
            current_header = line
            current_lines = []
            continue
        if current_header is None:
            title_lines.append(line)
        else:
            current_lines.append(line)

    if current_header is not None:
        sections.append({"header": current_header, "lines": current_lines[:]})

    return "\n".join(title_lines).rstrip(), sections


def _render_markdown_sections(title: str, sections: List[Dict[str, Any]]) -> str:
    parts = [title.rstrip()]
    for section in sections:
        parts.extend(["", section["header"]])
        if section["lines"]:
            parts.extend(section["lines"])
    return "\n".join(part for part in parts if part is not None).rstrip()


def _compress_markdown_section(
    title: str,
    sections: List[Dict[str, Any]],
    header: str,
) -> str:
    for section in sections:
        if section["header"] != header:
            continue
        if section["lines"] == ["- [Section truncated to fit budget]"]:
            break
        section["lines"] = ["- [Section truncated to fit budget]"]
        break
    return _render_markdown_sections(title, sections)


def _collect_required_setup_entries(
    knowledge: Dict[str, Any],
    graph: nx.Graph,
    target_api: Dict[str, Any],
    max_depth: int = 5,
) -> List[Dict[str, Any]]:
    apis = {api["api_id"]: api for api in knowledge.get("apis", [])}
    type_index = {type_info["type_id"]: type_info for type_info in knowledge.get("types", [])}
    producers_by_type = defaultdict(list)
    for api in knowledge.get("apis", []):
        for type_id in _expanded_produced_type_ids(graph, api["api_id"]):
            producers_by_type[type_id].append(api)

    seed_types = []
    owner_type_id = target_api.get("owner_type_id")
    if owner_type_id and not _is_low_signal_type(type_index.get(owner_type_id)):
        seed_types.append(owner_type_id)
    owner_trait_id = target_api.get("owner_trait_id")
    if owner_trait_id and _api_receiver_mentions_self(target_api):
        for type_id in _trait_implementor_type_ids(graph, owner_trait_id):
            if not _is_low_signal_type(type_index.get(type_id)):
                seed_types.append(type_id)
    for type_id in _graph_neighbor_ids(graph, target_api["api_id"], "api_accepts_type"):
        if not _is_low_signal_type(type_index.get(type_id)):
            seed_types.append(type_id)

    queue = deque((type_id, 0) for type_id in seed_types)
    best_type_depth = {type_id: 0 for type_id in seed_types}
    candidates: Dict[str, Dict[str, Any]] = {}

    while queue:
        type_id, depth = queue.popleft()
        if depth > max_depth:
            continue
        for api in producers_by_type.get(type_id, []):
            api_id = api["api_id"]
            if api_id == target_api["api_id"]:
                continue
            entry = candidates.setdefault(
                api_id,
                {
                    "api": api,
                    "upstream_depth": depth,
                    "produced_types": set(),
                },
            )
            entry["upstream_depth"] = max(entry["upstream_depth"], depth)
            entry["produced_types"].add(_type_display_name(type_index.get(type_id), type_id))
            if depth >= max_depth:
                continue
            for next_type_id in _api_setup_dependency_type_ids(graph, api):
                if _is_low_signal_type(type_index.get(next_type_id)):
                    continue
                next_depth = depth + 1
                if best_type_depth.get(next_type_id, -1) >= next_depth:
                    continue
                best_type_depth[next_type_id] = next_depth
                queue.append((next_type_id, next_depth))

    def score(item: Dict[str, Any]) -> tuple:
        api = item["api"]
        name = api.get("name", "").lower()
        api_kind = api.get("api_kind", "")
        value = 0
        value += item["upstream_depth"] * 100
        value += len(item["produced_types"]) * 15
        if api_kind in {"constructor", "associated_constructor"} or name in {"new", "default", "with_capacity"}:
            value += 20
        if name.startswith(("new_", "from_", "open", "read", "load", "parse")):
            value += 15
        if api.get("doc_sections", {}).get("safety"):
            value += 10
        if api.get("receiver") in {"&mut Self", "&mut self"}:
            value += 5
        return (-value, api.get("canonical_path", api["api_id"]))

    entries = sorted(candidates.values(), key=score)
    for entry in entries:
        entry["produced_types"] = sorted(entry["produced_types"])
    return entries


def _collect_compile_hints(
    knowledge: Dict[str, Any],
    graph: nx.Graph,
    target_api: Dict[str, Any],
    setup_entries: List[Dict[str, Any]],
    type_index: Dict[str, Dict[str, Any]],
    trait_index: Dict[str, Dict[str, Any]],
) -> Dict[str, List[Dict[str, Any]]]:
    apis_by_id = {api["api_id"]: api for api in knowledge.get("apis", [])}
    relevant_api_ids = [target_api["api_id"], *[entry["api"]["api_id"] for entry in setup_entries]]
    relevant_type_ids: Set[str] = set()
    relevant_trait_ids: Set[str] = set()

    for api_id in relevant_api_ids:
        api = apis_by_id.get(api_id)
        if not graph.has_node(api_id):
            continue
        api_node = graph.nodes[api_id]
        owner_type_id = api_node.get("owner_type_id")
        if owner_type_id:
            relevant_type_ids.add(owner_type_id)
        if api and api.get("owner_trait_id"):
            relevant_trait_ids.add(api["owner_trait_id"])
        relevant_type_ids.update(_graph_neighbor_ids(graph, api_id, "api_accepts_type"))
        relevant_type_ids.update(_expanded_produced_type_ids(graph, api_id))
        relevant_trait_ids.update(_graph_neighbor_ids(graph, api_id, "api_accepts_trait"))

    for api in knowledge.get("apis", []):
        owner_trait_id = api.get("owner_trait_id")
        if owner_trait_id not in relevant_trait_ids:
            continue
        relevant_type_ids.update(_graph_neighbor_ids(graph, api["api_id"], "api_accepts_type"))
        relevant_type_ids.update(_expanded_produced_type_ids(graph, api["api_id"]))

    for trait_id in list(relevant_trait_ids):
        relevant_type_ids.update(_trait_implementor_type_ids(graph, trait_id))

    imports = []
    for type_id in sorted(relevant_type_ids, key=lambda value: _type_display_name(type_index.get(value), value)):
        type_info = type_index.get(type_id)
        if not type_info:
            continue
        imports.append(
            {
                "id": type_id,
                "path": _type_display_name(type_info, type_id),
                "kind": "type",
            }
        )

    for trait_id in sorted(relevant_trait_ids, key=lambda value: _trait_display_name(trait_index.get(value), graph, value)):
        path = _trait_display_name(trait_index.get(trait_id), graph, trait_id)
        imports.append(
            {
                "id": trait_id,
                "path": path,
                "kind": "trait",
            }
        )

    traits = []
    trait_methods = []
    for trait_id in sorted(relevant_trait_ids, key=lambda value: _trait_display_name(trait_index.get(value), graph, value)):
        trait_info = trait_index.get(trait_id)
        if not trait_info:
            continue
        required_methods = sorted(trait_info.get("required_methods") or [])
        provided_methods = sorted(trait_info.get("provided_methods") or [])
        traits.append(
            {
                "path": _trait_display_name(trait_info, graph, trait_id),
                "required_methods": required_methods,
                "provided_methods": provided_methods,
            }
        )
        required_method_names = set(required_methods)
        provided_method_names = set(provided_methods)
        trait_method_apis = sorted(
            [
                api
                for api in knowledge.get("apis", [])
                if api.get("owner_trait_id") == trait_id
            ],
            key=lambda api: api.get("canonical_path", api["api_id"]),
        )
        for api in trait_method_apis:
            method_name = api.get("name", "")
            classification = "method"
            if method_name in required_method_names:
                classification = "required"
            elif method_name in provided_method_names:
                classification = "provided"
            trait_methods.append(
                {
                    "path": api.get("canonical_path", api["api_id"]),
                    "signature": _trait_method_impl_signature(api),
                    "classification": classification,
                }
            )

    enums = []
    for type_id in sorted(relevant_type_ids, key=lambda value: _type_display_name(type_index.get(value), value)):
        type_info = type_index.get(type_id)
        if not type_info or type_info.get("kind") != "enum":
            continue
        variants = [
            variant.get("name")
            for variant in (type_info.get("variants") or [])
            if variant.get("name")
        ]
        if not variants:
            continue
        enums.append(
            {
                "path": _type_display_name(type_info, type_id),
                "variants": variants,
                "is_non_exhaustive": bool(type_info.get("is_non_exhaustive") or type_info.get("has_hidden_variants")),
            }
        )

    return {
        "imports": imports,
        "traits": traits,
        "trait_methods": trait_methods,
        "enums": enums,
    }


def _graph_neighbor_ids(graph: nx.Graph, node_id: str, edge_kind: str) -> List[str]:
    if not graph.has_node(node_id):
        return []
    neighbor_ids = []
    for _, neighbor_id, data in graph.out_edges(node_id, data=True):
        if data.get("kind") == edge_kind:
            neighbor_ids.append(neighbor_id)
    return neighbor_ids


def _graph_in_neighbor_ids(graph: nx.Graph, node_id: str, edge_kind: str) -> List[str]:
    if not graph.has_node(node_id):
        return []
    neighbor_ids = []
    for neighbor_id, _, data in graph.in_edges(node_id, data=True):
        if data.get("kind") == edge_kind:
            neighbor_ids.append(neighbor_id)
    return neighbor_ids


def _expanded_produced_type_ids(graph: nx.Graph, api_id: str) -> List[str]:
    produced = []
    seen = set()
    queue = deque(_graph_neighbor_ids(graph, api_id, "api_returns_type"))
    while queue:
        type_id = queue.popleft()
        if type_id in seen:
            continue
        seen.add(type_id)
        produced.append(type_id)
        for deref_target_id in _graph_neighbor_ids(graph, type_id, "type_deref_target"):
            if deref_target_id not in seen:
                queue.append(deref_target_id)
    return produced


def _api_setup_dependency_type_ids(graph: nx.Graph, api: Dict[str, Any]) -> List[str]:
    dependency_ids = []
    owner_type_id = api.get("owner_type_id")
    if owner_type_id:
        dependency_ids.append(owner_type_id)
    dependency_ids.extend(_graph_neighbor_ids(graph, api["api_id"], "api_accepts_type"))
    return list(dict.fromkeys(dependency_ids))


def _type_display_name(type_info: Optional[Dict[str, Any]], fallback: str) -> str:
    if not type_info:
        return fallback
    public_paths = type_info.get("public_paths") or []
    if public_paths:
        return public_paths[0]
    return type_info.get("canonical_path") or type_info.get("name") or fallback


def _trait_display_name(
    trait_info: Optional[Dict[str, Any]],
    graph: nx.Graph,
    fallback: str,
) -> str:
    if trait_info:
        public_paths = trait_info.get("public_paths") or []
        if public_paths:
            return public_paths[0]
        return trait_info.get("canonical_path") or trait_info.get("name") or fallback
    if graph.has_node(fallback):
        return graph.nodes[fallback].get("path") or fallback
    return fallback


def _api_display_path(api: Dict[str, Any]) -> str:
    public_paths = api.get("public_paths") or []
    if public_paths:
        return public_paths[0]
    return api.get("canonical_path", api["api_id"])


def _api_signature_display(api: Dict[str, Any]) -> str:
    signature = str(api.get("signature") or api.get("signature_text") or "").strip()
    if not signature:
        return signature
    if not api.get("is_unsafe"):
        return signature
    if re.match(r"^(?:pub\s+)?unsafe\b", signature):
        return signature
    return signature.replace("fn ", "unsafe fn ", 1)


def _trait_method_impl_signature(api: Dict[str, Any]) -> str:
    signature = _api_signature_display(api)
    if not signature:
        return signature
    receiver = str(api.get("receiver") or "").strip()
    if not receiver:
        return signature
    normalized_receiver = _normalize_impl_receiver(receiver)
    if not normalized_receiver:
        return signature
    return signature.replace(receiver, normalized_receiver, 1)


def _normalize_impl_receiver(receiver: str) -> str:
    normalized = receiver.strip()
    if not normalized:
        return normalized
    if normalized == "Self":
        return "self"
    if normalized.endswith("Self"):
        return normalized[: -len("Self")] + "self"
    return normalized


def _api_receiver_mentions_self(api: Dict[str, Any]) -> bool:
    receiver = str(api.get("receiver") or "").strip().lower()
    return "self" in receiver


def _trait_implementor_type_ids(graph: nx.Graph, trait_id: str) -> List[str]:
    return list(dict.fromkeys(_graph_in_neighbor_ids(graph, trait_id, "impl_connects_type_trait")))


def _owner_type_id(graph: nx.Graph, api_id: str) -> Optional[str]:
    owners = _graph_in_neighbor_ids(graph, api_id, "type_owns_method")
    if owners:
        return owners[0]
    return None


def _api_is_constructor_like(api: Dict[str, Any]) -> bool:
    api_kind = str(api.get("api_kind", "")).lower()
    name = str(api.get("name", "")).lower()
    receiver = str(api.get("receiver", "")).strip()
    if api_kind in {"constructor", "associated_constructor"}:
        return True
    if not receiver and name in {"new", "default", "with_capacity"}:
        return True
    return False


def _api_has_constructible_seed_chain(
    graph: nx.Graph,
    api_id: str,
    seen_types: Set[str],
    seen_apis: Set[str],
) -> bool:
    seed_type_ids = target_seed_type_ids(graph, api_id)
    if not seed_type_ids:
        return True
    return all(
        type_has_public_producer(
            graph,
            seed_type_id,
            seen_types=seen_types,
            seen_apis=seen_apis,
        )
        for seed_type_id in seed_type_ids
    )


def _is_low_signal_type(type_info: Optional[Dict[str, Any]]) -> bool:
    if not type_info:
        return False
    name = (type_info.get("name") or "").lower()
    path = (type_info.get("canonical_path") or "").lower()
    return name == "error" or path.endswith("::error")


def _is_low_signal_related_api(api: Dict[str, Any], type_index: Dict[str, Dict[str, Any]]) -> bool:
    owner_type_id = api.get("owner_type_id")
    owner_type = type_index.get(owner_type_id) if owner_type_id else None
    if owner_type and _is_low_signal_type(owner_type):
        return True
    return False
