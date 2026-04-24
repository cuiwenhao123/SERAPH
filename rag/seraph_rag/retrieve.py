from __future__ import annotations

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


def render_context_markdown(
    knowledge: Dict[str, Any],
    graph: nx.Graph,
    target: UnsafeTarget,
    idioms: Optional[List[str]] = None,
    similar_api_docs: Optional[List[str]] = None,
    max_context_chars: int = 12000,
    max_related_apis: int = 24,
    max_similar_docs: int = 5,
    max_idioms: int = 5,
) -> str:
    idioms = idioms or []
    similar_api_docs = similar_api_docs or []
    attempts = [
        (max_related_apis, max_similar_docs, max_idioms),
        (min(max_related_apis, 12), min(max_similar_docs, 3), min(max_idioms, 3)),
        (min(max_related_apis, 6), min(max_similar_docs, 2), min(max_idioms, 2)),
        (min(max_related_apis, 3), min(max_similar_docs, 1), min(max_idioms, 1)),
        (0, 0, 0),
    ]
    last_markdown = ""
    for related_limit, similar_limit, idiom_limit in attempts:
        markdown = _render_context_markdown_unbudgeted(
            knowledge,
            graph,
            target,
            idioms=idioms,
            similar_api_docs=similar_api_docs,
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
    max_related_apis: int,
    max_similar_docs: int,
    max_idioms: int,
) -> str:
    apis = {api["api_id"]: api for api in knowledge.get("apis", [])}
    crate_import_name = knowledge["crate_meta"]["crate_import_name"]
    target_api = apis[target.api_id]
    related_ids = graph.nodes[target.api_id].get("unsafe_context_subgraph", [])
    related_apis = [
        apis[api_id]
        for api_id in related_ids
        if api_id in apis and api_id != target.api_id
    ]
    lines = [
        "# SERAPH RAG Harness Context",
        "",
        "## Target API",
        "- api_id: {}".format(target.api_id),
        "- path: {}".format(target_api.get("canonical_path", target.api_id)),
        "- signature: {}".format(target_api.get("signature") or target_api.get("signature_text", "")),
        "- safety: {}".format((target_api.get("doc_sections") or {}).get("safety", "")),
        "",
        "## Related APIs",
    ]
    for api in _rank_related_apis(related_apis, target_api)[:max_related_apis]:
        lines.append(
            "- {}: {} — {} [{}]".format(
                api["api_id"],
                api.get("canonical_path", api["api_id"]),
                api.get("signature") or api.get("signature_text", ""),
                _related_api_role(api, target_api),
            )
        )
    lines.extend(["", "## Semantically Similar API Docs"])
    target_path = target_api.get("canonical_path", target.api_id)
    for doc in _filter_similar_docs(similar_api_docs, target.api_id, target_path)[:max_similar_docs]:
        lines.append("- {}".format(_one_line(doc)))
    lines.extend(["", "## Rust Idioms"])
    for idiom in idioms[:max_idioms]:
        lines.append("- {}".format(idiom))
    lines.extend(
        [
            "",
            "## Generation Rules",
            "- Use crate import name `{}`.".format(crate_import_name),
            "- Call the target API in every harness variant.",
            "- Emit `SERAPH_STEP_ENTER:<step_no>:<api_id>` before each targeted API call.",
            "- Emit `SERAPH_STEP_OK:<step_no>:<api_id>` after successful return.",
            "- Use early return for recoverable `Result` and `Option` paths.",
            "- Do not use `target_lib` as a crate name.",
            "",
        ]
    )
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
) -> str:
    from seraph_rag.graph_builder import read_graph
    from seraph_rag.vector_index import persistent_client

    graph = read_graph(graph_path)
    targets = rank_unsafe_targets(graph, excluded=excluded)
    if not targets:
        raise ValueError("no unsafe targets available")
    target = targets[0]
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
            "but SERAPH_EMBEDDER is {!r}. Rebuild the vector DB or switch backends.".format(
                collection.name,
                actual,
                expected,
            )
        )


def _filter_similar_docs(docs: List[str], target_api_id: str, target_path: str) -> List[str]:
    filtered = []
    seen = set()
    for doc in docs:
        one_line = _one_line(doc)
        if target_api_id in one_line or target_path in one_line:
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
        if "unsafe" in name or "unchecked" in name or "unsafe" in path or "unchecked" in path:
            value += 20
        if api.get("doc_sections", {}).get("safety"):
            value += 15
        if api.get("receiver") == "&mut Self" or api.get("receiver") == "&mut self":
            value += 10
        return (-value, api.get("canonical_path", api.get("api_id", "")))

    return sorted(apis, key=score)


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
    required_marker = "## Generation Rules"
    marker_index = markdown.find(required_marker)
    if marker_index < 0:
        return markdown[:max_context_chars]
    prefix_budget = max_context_chars - (len(markdown) - marker_index) - len("\n\n[Context truncated to fit budget]\n")
    if prefix_budget <= 0:
        return markdown[marker_index:][:max_context_chars]
    return (
        markdown[:prefix_budget].rstrip()
        + "\n\n[Context truncated to fit budget]\n"
        + markdown[marker_index:]
    )[:max_context_chars]
