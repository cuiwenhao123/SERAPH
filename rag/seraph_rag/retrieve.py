from __future__ import annotations

import re
from collections import defaultdict, deque
from functools import lru_cache
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Union

import networkx as nx

from seraph_rag.embeddings import build_embedder, embedder_backend_name
from seraph_rag.documents import api_doc_id
from seraph_rag.schema import UnsafeTarget

_SIGNATURE_TYPE_KEYWORDS = {
    "Self",
    "as",
    "const",
    "crate",
    "dyn",
    "extern",
    "fn",
    "for",
    "impl",
    "mut",
    "pub",
    "self",
    "static",
    "super",
    "unsafe",
    "where",
}
_SIGNATURE_BUILTIN_TYPE_TOKENS = {
    "Arc",
    "BinaryHeap",
    "Box",
    "Bound",
    "CStr",
    "CString",
    "Cell",
    "Cow",
    "Duration",
    "HashMap",
    "HashSet",
    "Instant",
    "LinkedList",
    "ManuallyDrop",
    "MaybeUninit",
    "Mutex",
    "NonNull",
    "Option",
    "OsStr",
    "OsString",
    "Path",
    "PathBuf",
    "PhantomData",
    "Pin",
    "Range",
    "RangeFrom",
    "RangeFull",
    "RangeInclusive",
    "RangeTo",
    "RangeToInclusive",
    "Rc",
    "RefCell",
    "Result",
    "RwLock",
    "String",
    "UnsafeCell",
    "Vec",
    "VecDeque",
    "bool",
    "c_char",
    "c_double",
    "c_float",
    "c_int",
    "c_long",
    "c_longlong",
    "c_schar",
    "c_short",
    "c_uchar",
    "c_uint",
    "c_ulong",
    "c_ulonglong",
    "c_ushort",
    "c_void",
    "char",
    "f32",
    "f64",
    "i128",
    "i16",
    "i32",
    "i64",
    "i8",
    "isize",
    "str",
    "u128",
    "u16",
    "u32",
    "u64",
    "u8",
    "usize",
}
_OWNER_HINT_ALLOWED_UPPER_TOKENS = {
    "Default",
}


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
        if (
            data.get("kind") == "api"
            and data.get("has_unsafe")
            and node_id not in excluded
            and not target_missing_signature_type_names(graph, node_id)
        ):
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


def target_missing_signature_type_names(graph: nx.Graph, api_id: str) -> List[str]:
    if not graph.has_node(api_id):
        return []
    api = graph.nodes[api_id]
    if api.get("kind") != "api":
        return []
    arg_types = [str(arg).strip() for arg in (api.get("arg_types") or []) if str(arg).strip()]
    if not arg_types:
        return []

    generic_param_names = _generic_param_names(api.get("generic_params") or [])
    accessible_names: Set[str] = set()
    for type_id in _graph_neighbor_ids(graph, api_id, "api_accepts_type"):
        if _graph_type_is_externally_usable(graph, type_id):
            accessible_names.update(_graph_type_names(graph.nodes[type_id]))
    for trait_id in _graph_neighbor_ids(graph, api_id, "api_accepts_trait"):
        if _graph_type_is_publicly_nameable(graph, trait_id):
            accessible_names.update(_graph_type_names(graph.nodes[trait_id]))

    missing: List[str] = []
    for arg_type in arg_types:
        for token in _custom_type_tokens(arg_type, generic_param_names):
            if token in accessible_names or token in missing:
                continue
            missing.append(token)
    return missing


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
    visible_setup_entries = _visible_setup_entries(setup_entries, max_setup_apis)
    compile_hints = _collect_compile_hints(
        knowledge,
        graph,
        target_api,
        visible_setup_entries,
        type_index,
        trait_index,
    )
    display_setup_entries = _preferred_display_setup_entries(
        visible_setup_entries,
        target_api,
        compile_hints,
        type_index,
    )
    setup_api_ids = {entry["api"]["api_id"] for entry in display_setup_entries}
    related_apis = _collect_related_apis(
        knowledge,
        graph,
        target_api,
        setup_api_ids,
        type_index,
        max_related_apis=max_related_apis,
    )
    variant_opportunities = _collect_variant_opportunities(
        target_api,
        display_setup_entries,
        related_apis,
        compile_hints,
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
        "- generic_bounds: {}".format(_api_generic_bounds_display(target_api)),
        "- safety_summary: {}".format((target_api.get("doc_sections") or {}).get("safety", "")),
        "- errors_summary: {}".format((target_api.get("doc_sections") or {}).get("errors", "")),
        "- panics_summary: {}".format((target_api.get("doc_sections") or {}).get("panics", "")),
        "",
    ]
    target_usage_hints = _collect_target_usage_hints(target_api)
    owner_type_usage_hints = _collect_owner_type_usage_hints(target_api, type_index, knowledge)
    if target_usage_hints:
        lines.extend(["## Target Usage Hints"])
        for hint in target_usage_hints:
            lines.append("- `{}`".format(hint))
        lines.append("")
    lines.extend([
        "## Known Reachable Paths",
    ])
    for entry in display_setup_entries:
        api = entry["api"]
        lines.append(
            "- {}: {} — {} [goal={} basis={} produces={} depth={}]".format(
                api["api_id"],
                _api_display_path(api),
                _api_signature_display(api),
                entry.get("goal", "reach_target"),
                entry.get("basis", "producer_chain"),
                ", ".join(entry["produced_types"]),
                entry["upstream_depth"],
            )
        )
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
    if compile_hints["trait_implementor_facts"]:
        lines.extend(["", "### Trait Implementor Facts"])
        for item in compile_hints["trait_implementor_facts"]:
            lines.append(
                "- {}: public_implementors={}".format(
                    item["path"],
                    ", ".join(item["public_implementors"]),
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
    if compile_hints["type_traits"]:
        lines.extend(["", "### Type Trait Facts"])
        for item in compile_hints["type_traits"]:
            lines.append(
                "- {} [kind={}]: Copy={}; Clone={}; other_explicit_impls={}".format(
                    item["path"],
                    item["kind"],
                    "yes" if item["is_copy"] else "no",
                    "yes" if item["is_clone"] else "no",
                    ", ".join(item["other_explicit_impls"]) or "(none)",
                )
            )
    if compile_hints["owner_type_facts"]:
        lines.extend(["", "### Owner Type Facts"])
        for item in compile_hints["owner_type_facts"]:
            lines.append(
                "- {}: generic_params={}; where_clauses={}".format(
                    item["path"],
                    ", ".join(item["generic_params"]) or "(none)",
                    "; ".join(item["where_clauses"]) or "(none)",
                )
            )
    if compile_hints["owner_doc_facts"]:
        lines.extend(["", "### Owner Documentation Facts"])
        for item in compile_hints["owner_doc_facts"]:
            lines.append("- {}: {}".format(item["path"], item["fact"]))
    if compile_hints["owner_construction_bridges"]:
        lines.extend(["", "### Owner Construction Bridges"])
        for item in compile_hints["owner_construction_bridges"]:
            where_suffix = ""
            if item["where_clauses"]:
                where_suffix = " [where={}]".format("; ".join(item["where_clauses"]))
            lines.append(
                "- {}: {} via impl {}{}".format(
                    item["path"],
                    item["call"],
                    item["impl"],
                    where_suffix,
                )
            )
    if compile_hints["output_initializers"]:
        lines.extend(["", "### Output Initialization Facts"])
        for item in compile_hints["output_initializers"]:
            lines.append("- {}: prefer `{}`".format(item["path"], item["statement"]))
    if owner_type_usage_hints:
        lines.extend(["", "## Owner Type Usage Hints"])
        for hint in owner_type_usage_hints:
            lines.append("- `{}`".format(hint))
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
    lines.extend(["", "## Variant Opportunities", "### Setup Choices"])
    for item in variant_opportunities["setup"]:
        lines.append("- {}".format(item))
    lines.extend(["", "### Input Shaping Choices"])
    for item in variant_opportunities["input"]:
        lines.append("- {}".format(item))
    lines.extend(["", "### State Progression Choices"])
    for item in variant_opportunities["state"]:
        lines.append("- {}".format(item))
    lines.extend(["", "### Boundary Choices"])
    for item in variant_opportunities["boundary"]:
        lines.append("- {}".format(item))
    lines.extend(["", "## Similar API Usage"])
    target_path = target_api.get("canonical_path", target.api_id)
    excluded_paths = {
        target_path,
        *[entry["api"].get("canonical_path", entry["api"]["api_id"]) for entry in display_setup_entries],
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


def _collect_target_usage_hints(target_api: Dict[str, Any], max_hints: int = 3) -> List[str]:
    examples = str((target_api.get("doc_sections") or {}).get("examples") or "").strip()
    method_name = str(target_api.get("name") or "").strip()
    if not examples or not method_name or max_hints <= 0:
        return []

    blocks = _markdown_code_blocks(examples)
    if not blocks:
        blocks = [examples]

    hints: List[str] = []
    seen: Set[str] = set()
    for block in blocks:
        raw_lines = [line.strip() for line in block.splitlines() if line.strip()]
        for index, line in enumerate(raw_lines):
            if not _line_mentions_method_call(line, method_name):
                continue
            setup_line = ""
            for prev_index in range(index - 1, -1, -1):
                candidate = raw_lines[prev_index]
                if candidate.startswith("use ") or candidate.startswith("//") or candidate.startswith("#"):
                    continue
                setup_line = candidate
                break
            snippet_parts = [part for part in [setup_line, line] if part]
            snippet = _one_line(" ".join(snippet_parts))
            if not snippet or snippet in seen:
                continue
            seen.add(snippet)
            hints.append(snippet)
            if len(hints) >= max_hints:
                return hints
    return hints


def _collect_owner_type_usage_hints(
    target_api: Dict[str, Any],
    type_index: Dict[str, Dict[str, Any]],
    knowledge: Dict[str, Any],
    max_hints: int = 3,
) -> List[str]:
    owner_type_id = target_api.get("owner_type_id")
    owner_type = type_index.get(owner_type_id or "")
    if not owner_type or max_hints <= 0:
        return []

    owner_name = str(owner_type.get("name") or "").strip()
    examples = str((owner_type.get("doc_sections") or {}).get("examples") or "").strip()
    if not owner_name or not examples:
        return []

    blocks = _markdown_code_blocks(examples)
    if not blocks:
        blocks = [examples]

    hints: List[str] = []
    seen: Set[str] = set()
    for block in blocks:
        for raw_line in [line.strip() for line in block.splitlines() if line.strip()]:
            if raw_line.startswith(("use ", "//", "#")):
                continue
            if owner_name not in raw_line:
                continue
            if not _owner_type_usage_hint_is_publicly_nameable(raw_line, owner_name, knowledge):
                continue
            hint = _one_line(raw_line)
            if not hint or hint in seen:
                continue
            seen.add(hint)
            hints.append(hint)
            if len(hints) >= max_hints:
                return hints
    return hints


def _owner_type_usage_hint_is_publicly_nameable(
    snippet: str,
    owner_name: str,
    knowledge: Dict[str, Any],
) -> bool:
    if not re.search(r"\b{}\b".format(re.escape(owner_name)), snippet):
        return False
    if not any(token in snippet for token in ("::", "=", ";")):
        return False

    known_upper_tokens = set(_SIGNATURE_BUILTIN_TYPE_TOKENS)
    known_upper_tokens.update(_OWNER_HINT_ALLOWED_UPPER_TOKENS)
    for type_info in knowledge.get("types", []):
        type_name = str(type_info.get("name") or "").strip()
        if type_name:
            known_upper_tokens.add(type_name)
        canonical_tail = str(type_info.get("canonical_path") or "").rsplit("::", 1)[-1].strip()
        if canonical_tail:
            known_upper_tokens.add(canonical_tail)
        for path in type_info.get("public_paths") or []:
            tail = str(path).rsplit("::", 1)[-1].strip()
            if tail:
                known_upper_tokens.add(tail)

    for token in re.findall(r"\b[A-Z][A-Za-z0-9_]*\b", snippet):
        if token not in known_upper_tokens:
            return False
    return True


def _markdown_code_blocks(text: str) -> List[str]:
    return re.findall(r"```(?:[^\n`]*)\n(.*?)```", text, flags=re.DOTALL)


def _line_mentions_method_call(line: str, method_name: str) -> bool:
    return ".{}(".format(method_name) in line or "::{}(".format(method_name) in line


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
    actual = (collection.metadata or {}).get("seraph:embedder", "unknown")
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
        "## Variant Opportunities",
        "## Related APIs",
        "## Known Reachable Paths",
        "## Owner Type Usage Hints",
        "## Compile-Time Facts",
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
    for type_id in _owner_specialization_seed_type_ids(target_api, knowledge, type_index):
        if not _is_low_signal_type(type_index.get(type_id)):
            seed_types.append(type_id)
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
                    "goal": "reach_target",
                    "basis": "producer_chain",
                    "upstream_depth": depth,
                    "produced_types": set(),
                },
            )
            entry["upstream_depth"] = max(entry["upstream_depth"], depth)
            entry["produced_types"].add(_type_display_name(type_index.get(type_id), type_id))
            basis = _setup_entry_basis(api, target_api, type_id)
            if basis == "owner_bridge":
                entry["basis"] = basis
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


def _visible_setup_entries(entries: List[Dict[str, Any]], limit: int) -> List[Dict[str, Any]]:
    if limit <= 0:
        return []
    if len(entries) <= limit:
        return entries[:]

    selected = list(entries[:limit])
    hidden_owner_bridges = [
        entry for entry in entries[limit:] if entry.get("basis") == "owner_bridge"
    ]
    for hidden in hidden_owner_bridges:
        replace_index = next(
            (
                index
                for index in range(len(selected) - 1, -1, -1)
                if selected[index].get("basis") != "owner_bridge"
            ),
            None,
        )
        if replace_index is None:
            break
        selected[replace_index] = hidden
    return selected


def _preferred_display_setup_entries(
    entries: List[Dict[str, Any]],
    target_api: Dict[str, Any],
    compile_hints: Dict[str, List[Dict[str, Any]]],
    type_index: Dict[str, Dict[str, Any]],
) -> List[Dict[str, Any]]:
    owner_type_id = str(target_api.get("owner_type_id") or "").strip()
    owner_type = type_index.get(owner_type_id)
    owner_path = _type_display_name(owner_type, owner_type_id) if owner_type_id else ""
    has_direct_owner_conversion = any(
        item.get("path") == owner_path for item in compile_hints.get("owner_construction_bridges", [])
    )
    if not has_direct_owner_conversion:
        return entries
    return [entry for entry in entries if entry.get("basis") != "owner_bridge"]


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
    type_trait_candidate_ids: Set[str] = set()

    for api_id in relevant_api_ids:
        api = apis_by_id.get(api_id)
        if not graph.has_node(api_id):
            continue
        api_node = graph.nodes[api_id]
        owner_type_id = api_node.get("owner_type_id")
        if owner_type_id:
            relevant_type_ids.add(owner_type_id)
            owner_type = type_index.get(owner_type_id)
            if owner_type:
                relevant_trait_ids.update(_where_clause_trait_ids(owner_type, knowledge))
                relevant_type_ids.update(_where_clause_type_ids(owner_type, knowledge))
        if api and api.get("owner_trait_id"):
            relevant_trait_ids.add(api["owner_trait_id"])
        accepted_type_ids = _graph_neighbor_ids(graph, api_id, "api_accepts_type")
        relevant_type_ids.update(accepted_type_ids)
        type_trait_candidate_ids.update(accepted_type_ids)
        if api:
            relevant_type_ids.update(_where_clause_type_ids(api, knowledge))
            relevant_trait_ids.update(_where_clause_trait_ids(api, knowledge))
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
    trait_implementor_facts = []
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
        public_implementors = list(
            dict.fromkeys(
                _type_display_name(type_index.get(type_id), type_id)
                for type_id in _trait_implementor_type_ids(graph, trait_id)
                if _graph_type_is_publicly_nameable(graph, type_id) and type_index.get(type_id)
            )
        )
        if public_implementors:
            trait_implementor_facts.append(
                {
                    "path": _trait_display_name(trait_info, graph, trait_id),
                    "public_implementors": public_implementors,
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

    trait_impls_by_type: Dict[str, List[Dict[str, Any]]] = defaultdict(list)
    for impl_info in knowledge.get("trait_impl_registry", []):
        target_type_id = impl_info.get("target_type_id")
        if target_type_id:
            trait_impls_by_type[str(target_type_id)].append(impl_info)

    type_traits = []
    for type_id in sorted(
        type_trait_candidate_ids,
        key=lambda value: _type_display_name(type_index.get(value), value),
    ):
        type_info = type_index.get(type_id)
        if not type_info:
            continue
        impl_paths = sorted(
            dict.fromkeys(
                path
                for path in (
                    _trait_impl_display_path(impl_info)
                    for impl_info in trait_impls_by_type.get(type_id, [])
                )
                if path
            )
        )
        type_traits.append(
            {
                "path": _type_display_name(type_info, type_id),
                "kind": type_info.get("kind", "unknown"),
                "is_copy": any(_is_copy_trait_path(path) for path in impl_paths),
                "is_clone": any(_is_clone_trait_path(path) for path in impl_paths),
                "other_explicit_impls": [
                    path
                    for path in impl_paths
                    if not _is_copy_trait_path(path) and not _is_clone_trait_path(path)
                ],
            }
        )

    output_initializers = []
    for type_id in sorted(
        _target_mut_output_type_ids(target_api, graph, type_index),
        key=lambda value: _type_display_name(type_index.get(value), value),
    ):
        type_info = type_index.get(type_id)
        if not type_info:
            continue
        initializer_expr = _output_initializer_expr(type_info, knowledge, type_index)
        if not initializer_expr:
            continue
        path = _type_display_name(type_info, type_id)
        if initializer_expr.startswith(path + " {"):
            statement = "let mut value = {};".format(initializer_expr)
        else:
            statement = "let mut value: {} = {};".format(path, initializer_expr)
        output_initializers.append(
            {
                "path": path,
                "statement": statement,
            }
        )

    owner_type_facts = []
    owner_type_ids = {
        str(apis_by_id[api_id].get("owner_type_id") or "")
        for api_id in relevant_api_ids
        if api_id in apis_by_id
    }
    owner_type_ids.discard("")
    for type_id in sorted(owner_type_ids, key=lambda value: _type_display_name(type_index.get(value), value)):
        type_info = type_index.get(type_id)
        if not type_info:
            continue
        generic_params = [str(item).strip() for item in (type_info.get("generic_params") or []) if str(item).strip()]
        where_clauses = [str(item).strip() for item in (type_info.get("where_clauses") or []) if str(item).strip()]
        if not generic_params and not where_clauses:
            continue
        owner_type_facts.append(
            {
                "path": _type_display_name(type_info, type_id),
                "generic_params": generic_params,
                "where_clauses": where_clauses,
            }
        )

    owner_doc_facts = _collect_owner_documentation_facts(
        knowledge,
        owner_type_ids,
        type_index,
    )
    owner_construction_bridges = _collect_owner_construction_bridges(
        knowledge,
        owner_type_ids,
        type_index,
    )

    return {
        "imports": imports,
        "traits": traits,
        "trait_implementor_facts": trait_implementor_facts,
        "trait_methods": trait_methods,
        "enums": enums,
        "type_traits": type_traits,
        "owner_type_facts": owner_type_facts,
        "owner_doc_facts": owner_doc_facts,
        "owner_construction_bridges": owner_construction_bridges,
        "output_initializers": output_initializers,
    }


def _where_clause_type_ids(api: Dict[str, Any], knowledge: Dict[str, Any]) -> Set[str]:
    matched: Set[str] = set()
    for clause in api.get("where_clauses", []) or []:
        matched.update(_matching_type_ids_in_knowledge(str(clause), knowledge))
    return matched


def _where_clause_trait_ids(api: Dict[str, Any], knowledge: Dict[str, Any]) -> Set[str]:
    matched: Set[str] = set()
    for clause in api.get("where_clauses", []) or []:
        matched.update(_matching_trait_ids_in_knowledge(str(clause), knowledge))
    return matched


def _collect_owner_documentation_facts(
    knowledge: Dict[str, Any],
    owner_type_ids: Set[str],
    type_index: Dict[str, Dict[str, Any]],
) -> List[Dict[str, str]]:
    modules_by_id = {module.get("module_id"): module for module in knowledge.get("modules", [])}
    facts: List[Dict[str, str]] = []
    seen: Set[tuple[str, str]] = set()
    for type_id in sorted(owner_type_ids, key=lambda value: _type_display_name(type_index.get(value), value)):
        type_info = type_index.get(type_id)
        if not type_info:
            continue
        candidates = [
            str((knowledge.get("crate_meta") or {}).get("root_docs") or ""),
            str(((knowledge.get("crate_meta") or {}).get("root_doc_sections") or {}).get("summary") or ""),
            str(type_info.get("docs") or ""),
            str((type_info.get("doc_sections") or {}).get("summary") or ""),
            str((type_info.get("doc_sections") or {}).get("examples") or ""),
        ]
        module_info = modules_by_id.get(type_info.get("public_anchor_module_id") or "")
        if module_info:
            candidates.extend(
                [
                    str(module_info.get("docs") or ""),
                    str((module_info.get("doc_sections") or {}).get("summary") or ""),
                ]
            )

        for text in candidates:
            fact = _owner_documentation_fact_from_text(text, type_info)
            if not fact:
                continue
            key = (_type_display_name(type_info, type_id), fact)
            if key in seen:
                continue
            seen.add(key)
            facts.append(
                {
                    "path": _type_display_name(type_info, type_id),
                    "fact": fact,
                }
            )
            break
    return facts


def _owner_documentation_fact_from_text(text: str, type_info: Dict[str, Any]) -> Optional[str]:
    normalized_text = str(text).strip()
    if not normalized_text:
        return None
    owner_name = str(type_info.get("name") or "").strip()
    owner_paths = [owner_name, *[str(path).strip() for path in (type_info.get("public_paths") or []) if str(path).strip()]]
    owner_paths = [path for path in owner_paths if path]
    keywords = ("default", "defaults", "omit", "omitted")

    for fragment in _documentation_fragments(normalized_text):
        one_line = _one_line(fragment)
        lowered = one_line.lower()
        if not one_line or not any(keyword in lowered for keyword in keywords):
            continue
        if not any(path in one_line or "{}<".format(path.rsplit("::", 1)[-1]) in one_line for path in owner_paths):
            continue
        return _summarize_owner_documentation_fact(one_line)
    return None


def _documentation_fragments(text: str) -> List[str]:
    normalized = " ".join(str(text).replace("\r\n", "\n").split())
    fragments = re.split(r"(?<=[.!?])\s+", normalized)
    return [fragment.strip() for fragment in fragments if fragment.strip()]


def _summarize_owner_documentation_fact(fact: str) -> str:
    one_line = _one_line(fact)
    lowered = one_line.lower()
    if "omit" in lowered and "default" in lowered:
        owner_shape_match = re.search(r"`([^`]+)`", one_line)
        default_match = re.search(r"default(?:s)? to (?:a size of |size of |capacity of )?([A-Za-z0-9_]+)", lowered)
        owner_shape = owner_shape_match.group(1).strip() if owner_shape_match else "owner type"
        default_value = default_match.group(1).strip() if default_match else ""
        if default_value:
            return "omit the size; `{}` defaults to size {}".format(owner_shape, default_value)
        return "omit the size; `{}` has a documented default size".format(owner_shape)
    return one_line


def _collect_owner_construction_bridges(
    knowledge: Dict[str, Any],
    owner_type_ids: Set[str],
    type_index: Dict[str, Dict[str, Any]],
) -> List[Dict[str, Any]]:
    trait_impls_by_type: Dict[str, List[Dict[str, Any]]] = defaultdict(list)
    for impl_info in knowledge.get("trait_impl_registry", []):
        target_type_id = str(impl_info.get("target_type_id") or "").strip()
        if target_type_id:
            trait_impls_by_type[target_type_id].append(impl_info)

    bridges: List[Dict[str, Any]] = []
    seen: Set[tuple[str, str, str]] = set()
    for type_id in sorted(owner_type_ids, key=lambda value: _type_display_name(type_index.get(value), value)):
        type_info = type_index.get(type_id)
        if not type_info:
            continue
        for impl_info in trait_impls_by_type.get(type_id, []):
            bridge = _owner_construction_bridge_from_impl(impl_info, type_info, type_id)
            if not bridge:
                continue
            key = (bridge["path"], bridge["call"], bridge["impl"])
            if key in seen:
                continue
            seen.add(key)
            bridges.append(bridge)
    return bridges


def _owner_construction_bridge_from_impl(
    impl_info: Dict[str, Any],
    type_info: Dict[str, Any],
    type_id: str,
) -> Optional[Dict[str, Any]]:
    trait_path = _trait_impl_display_path(impl_info)
    owner_path = _type_display_name(type_info, type_id)
    owner_display = _public_type_display_with_generics(type_info, str(impl_info.get("for_type_text") or ""))
    trait_ref = str(impl_info.get("trait_ref_text") or "").strip() or trait_path
    where_clauses = [
        str(clause).strip() for clause in (impl_info.get("where_clauses") or []) if str(clause).strip()
    ]

    if _is_from_trait_path(trait_path) and _trait_conversion_source_text(impl_info):
        return {
            "path": owner_path,
            "call": "{}::from(...)".format(owner_path),
            "impl": "{} for {}".format(trait_ref, owner_display),
            "where_clauses": where_clauses,
        }
    if _is_try_from_trait_path(trait_path) and _trait_conversion_source_text(impl_info):
        return {
            "path": owner_path,
            "call": "{}::try_from(...)".format(owner_path),
            "impl": "{} for {}".format(trait_ref, owner_display),
            "where_clauses": where_clauses,
        }
    return None


def _public_type_display_with_generics(type_info: Dict[str, Any], for_type_text: str) -> str:
    path = _type_display_name(
        type_info,
        type_info.get("type_id") or type_info.get("canonical_path") or type_info.get("name") or "",
    )
    owner_name = str(type_info.get("name") or "").strip()
    normalized_for_type = str(for_type_text).strip()
    if owner_name and normalized_for_type.startswith(owner_name):
        return "{}{}".format(path, normalized_for_type[len(owner_name) :])
    generic_params = [str(item).strip() for item in (type_info.get("generic_params") or []) if str(item).strip()]
    if generic_params:
        return "{}<{}>".format(path, ", ".join(generic_params))
    return path


def _trait_conversion_source_text(impl_info: Dict[str, Any]) -> str:
    trait_ref = str(impl_info.get("trait_ref_text") or "").strip()
    match = re.search(r"(?:^|::)(?:From|TryFrom)<(.+)>$", trait_ref)
    if not match:
        return ""
    return match.group(1).strip()


def _is_from_trait_path(path: str) -> bool:
    normalized = str(path).strip()
    return normalized == "From" or normalized.endswith("::From")


def _is_try_from_trait_path(path: str) -> bool:
    normalized = str(path).strip()
    return normalized == "TryFrom" or normalized.endswith("::TryFrom")


def _matching_type_ids_in_knowledge(type_text: str, knowledge: Dict[str, Any]) -> Set[str]:
    matched: Set[str] = set()
    for type_info in knowledge.get("types", []):
        name = str(type_info.get("name") or "").strip()
        canonical_path = str(type_info.get("canonical_path") or "").strip()
        public_paths = [str(path).strip() for path in (type_info.get("public_paths") or []) if str(path).strip()]
        candidate_names = {
            name,
            canonical_path.rsplit("::", 1)[-1].strip(),
            *[path.rsplit("::", 1)[-1].strip() for path in public_paths],
        }
        candidate_names.discard("")
        if any(_text_mentions_token(type_text, candidate) for candidate in candidate_names):
            matched.add(type_info["type_id"])
            continue
        if canonical_path and canonical_path in type_text:
            matched.add(type_info["type_id"])
            continue
        if any(path and path in type_text for path in public_paths):
            matched.add(type_info["type_id"])
    return matched


def _matching_trait_ids_in_knowledge(type_text: str, knowledge: Dict[str, Any]) -> Set[str]:
    matched: Set[str] = set()
    for trait_info in knowledge.get("trait_registry", []):
        name = str(trait_info.get("name") or "").strip()
        canonical_path = str(trait_info.get("canonical_path") or "").strip()
        public_paths = [str(path).strip() for path in (trait_info.get("public_paths") or []) if str(path).strip()]
        candidate_names = {
            name,
            canonical_path.rsplit("::", 1)[-1].strip(),
            *[path.rsplit("::", 1)[-1].strip() for path in public_paths],
        }
        candidate_names.discard("")
        if any(_text_mentions_token(type_text, candidate) for candidate in candidate_names):
            matched.add(trait_info["trait_id"])
            continue
        if canonical_path and canonical_path in type_text:
            matched.add(trait_info["trait_id"])
            continue
        if any(path and path in type_text for path in public_paths):
            matched.add(trait_info["trait_id"])
    return matched


def _text_mentions_token(type_text: str, token: str) -> bool:
    normalized = (
        str(type_text)
        .replace("::", " ")
        .replace("<", " ")
        .replace(">", " ")
        .replace(",", " ")
        .replace("&", " ")
        .replace("'", " ")
        .replace("*", " ")
        .replace("(", " ")
        .replace(")", " ")
    )
    return token in normalized.split()


def _collect_variant_opportunities(
    target_api: Dict[str, Any],
    setup_entries: List[Dict[str, Any]],
    related_apis: List[Dict[str, Any]],
    compile_hints: Dict[str, List[Dict[str, Any]]],
) -> Dict[str, List[str]]:
    setup_api_choices = [
        "setup API {} produces {}".format(
            _api_display_path(entry["api"]),
            ", ".join(entry["produced_types"]),
        )
        for entry in setup_entries[:3]
        if entry["produced_types"]
    ]
    derived_setup_choices = [
        "documented owner setup shape: {}".format(item["fact"])
        for item in compile_hints.get("owner_doc_facts", [])[:2]
    ]
    derived_setup_choices.extend(
        [
            "public conversion constructor available: {} via impl {}".format(
                item["call"],
                item["impl"],
            )
            for item in compile_hints.get("owner_construction_bridges", [])[:2]
        ]
    )
    setup_choices = [
        *setup_api_choices[:2],
        *derived_setup_choices[:2],
        *setup_api_choices[2:],
    ]

    input_choices = [
        "target signature includes argument type {}".format(arg_type)
        for arg_type in _target_argument_types(target_api)
    ]

    target_owner_type_id = target_api.get("owner_type_id")
    state_choices = [
        "same-owner mutator surfaced in related APIs: {}".format(_api_display_path(api))
        for api in related_apis
        if api.get("receiver") in {"&mut Self", "&mut self"}
        and target_owner_type_id
        and api.get("owner_type_id") == target_owner_type_id
    ]
    if not state_choices:
        state_choices = ["no same-owner mutator surfaced in related APIs"]

    boundary_choices: List[str] = []
    doc_sections = target_api.get("doc_sections") or {}
    if doc_sections.get("safety"):
        boundary_choices.append(
            "documented safety precondition: {}".format(_one_line(str(doc_sections["safety"])))
        )
    if doc_sections.get("errors"):
        boundary_choices.append(
            "documented error condition: {}".format(_one_line(str(doc_sections["errors"])))
        )
    if doc_sections.get("panics"):
        boundary_choices.append(
            "documented panic condition: {}".format(_one_line(str(doc_sections["panics"])))
        )

    return {
        "setup": list(dict.fromkeys(setup_choices))[:3],
        "input": list(dict.fromkeys(input_choices))[:3],
        "state": list(dict.fromkeys(state_choices))[:3],
        "boundary": list(dict.fromkeys(boundary_choices))[:3],
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


def _api_generic_bounds_display(api: Dict[str, Any]) -> str:
    clauses = [str(clause).strip() for clause in (api.get("where_clauses") or []) if str(clause).strip()]
    return "; ".join(clauses)


def _target_argument_types(api: Dict[str, Any]) -> List[str]:
    structured_arg_types = [str(arg_type).strip() for arg_type in (api.get("arg_types") or []) if str(arg_type).strip()]
    if structured_arg_types:
        return structured_arg_types

    signature = str(api.get("signature") or api.get("signature_text") or "").strip()
    if not signature or "(" not in signature:
        return []

    start = signature.find("(")
    depth = 0
    end = -1
    for index in range(start, len(signature)):
        char = signature[index]
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                end = index
                break
    if end == -1:
        return []

    args_block = signature[start + 1 : end].strip()
    if not args_block:
        return []

    args: List[str] = []
    current: List[str] = []
    nested_depth = 0
    for char in args_block:
        if char == "," and nested_depth == 0:
            arg = "".join(current).strip()
            if arg:
                args.append(arg)
            current = []
            continue
        current.append(char)
        if char in "(<[":
            nested_depth += 1
        elif char in ")>]":
            nested_depth = max(0, nested_depth - 1)
    tail = "".join(current).strip()
    if tail:
        args.append(tail)

    extracted = []
    for arg in args:
        normalized = arg.strip()
        if normalized in {"self", "&self", "&mut self"}:
            continue
        if ":" in normalized:
            normalized = normalized.split(":", 1)[1].strip()
        if normalized:
            extracted.append(normalized)
    return extracted


def _target_mut_output_type_ids(
    target_api: Dict[str, Any],
    graph: nx.Graph,
    type_index: Dict[str, Dict[str, Any]],
) -> List[str]:
    arg_types = _target_argument_types(target_api)
    matched: List[str] = []
    for type_id in _graph_neighbor_ids(graph, target_api["api_id"], "api_accepts_type"):
        type_info = type_index.get(type_id)
        if not type_info:
            continue
        if any(_arg_type_is_mut_ref_to_type(arg_type, type_info) for arg_type in arg_types):
            matched.append(type_id)
    return list(dict.fromkeys(matched))


def _arg_type_is_mut_ref_to_type(arg_type: str, type_info: Dict[str, Any]) -> bool:
    normalized = " ".join(str(arg_type).split())
    if not normalized.startswith("&mut "):
        return False
    candidate_names = {
        str(type_info.get("name") or "").strip(),
        *[
            str(path).rsplit("::", 1)[-1].strip()
            for path in (type_info.get("public_paths") or [])
            if str(path).strip()
        ],
        str(type_info.get("canonical_path") or "").rsplit("::", 1)[-1].strip(),
    }
    candidate_names.discard("")
    return any(re.search(r"\b{}\b".format(re.escape(name)), normalized) for name in candidate_names)


def _output_initializer_expr(
    type_info: Dict[str, Any],
    knowledge: Dict[str, Any],
    type_index: Dict[str, Dict[str, Any]],
) -> Optional[str]:
    kind = str(type_info.get("kind") or "")
    if kind == "type_alias":
        type_expr = _type_alias_target_expr(type_info, knowledge)
        if not type_expr:
            return None
        return _zero_initializer_for_type_expr(type_expr)
    if kind != "struct":
        return None
    if type_info.get("has_hidden_fields"):
        return None
    fields = type_info.get("fields") or []
    if not fields:
        return None
    field_initializers = []
    for field in fields:
        field_name = str(field.get("name") or "").strip()
        if not field_name:
            return None
        field_type_expr = _field_type_expr(field, knowledge)
        if not field_type_expr:
            return None
        field_initializer = _zero_initializer_for_type_expr(field_type_expr)
        if field_initializer is None:
            return None
        field_initializers.append("{}: {}".format(field_name, field_initializer))
    path = _type_display_name(type_info, type_info.get("type_id") or type_info.get("canonical_path") or type_info.get("name") or "")
    return "{} {{ {} }}".format(path, ", ".join(field_initializers))


def _type_alias_target_expr(type_info: Dict[str, Any], knowledge: Dict[str, Any]) -> Optional[str]:
    code_ref = type_info.get("code_ref") or {}
    snippet = _read_source_span(
        knowledge,
        str(code_ref.get("file") or ""),
        int(code_ref.get("start_line") or 0),
        int(code_ref.get("end_line") or 0),
    )
    if not snippet:
        return None
    line = " ".join(snippet.split())
    if "=" not in line or ";" not in line:
        return None
    return line.split("=", 1)[1].rsplit(";", 1)[0].strip()


def _owner_specialization_seed_type_ids(
    target_api: Dict[str, Any],
    knowledge: Dict[str, Any],
    type_index: Dict[str, Dict[str, Any]],
) -> List[str]:
    owner_type_id = str(target_api.get("owner_type_id") or "").strip()
    owner_type = type_index.get(owner_type_id)
    if not owner_type:
        return []

    seed_type_ids: List[str] = []
    for type_info in knowledge.get("types", []):
        if str(type_info.get("kind") or "") != "type_alias":
            continue
        type_expr = _type_alias_target_expr(type_info, knowledge)
        if not type_expr or not _type_expr_mentions_specific_type(type_expr, owner_type):
            continue
        for type_id in _matching_type_ids_in_knowledge(type_expr, knowledge):
            if type_id == owner_type_id:
                continue
            if not type_index.get(type_id):
                continue
            seed_type_ids.append(type_id)
    return list(dict.fromkeys(seed_type_ids))


def _type_expr_mentions_specific_type(type_expr: str, type_info: Dict[str, Any]) -> bool:
    normalized = str(type_expr).strip()
    if not normalized:
        return False

    path_candidates = _type_path_candidates(type_info)
    for candidate in path_candidates:
        if "::" in candidate and candidate in normalized:
            return True

    tail_candidates = {
        str(type_info.get("name") or "").strip(),
        *[candidate.rsplit("::", 1)[-1].strip() for candidate in path_candidates],
    }
    tail_candidates.discard("")
    return any(_text_mentions_token(normalized, candidate) for candidate in tail_candidates)


def _type_path_candidates(type_info: Dict[str, Any]) -> List[str]:
    candidates: List[str] = []
    seen: Set[str] = set()

    def add(candidate: str) -> None:
        normalized = str(candidate).strip()
        if not normalized or normalized in seen:
            return
        seen.add(normalized)
        candidates.append(normalized)

    canonical_path = str(type_info.get("canonical_path") or "").strip()
    if canonical_path:
        add(canonical_path)
        if "::" in canonical_path:
            add("::".join(canonical_path.split("::")[1:]))

    for public_path in type_info.get("public_paths") or []:
        normalized = str(public_path).strip()
        if not normalized:
            continue
        add(normalized)
        if "::" in normalized:
            add("::".join(normalized.split("::")[1:]))

    return candidates


def _field_type_expr(field: Dict[str, Any], knowledge: Dict[str, Any]) -> Optional[str]:
    type_text = str(field.get("type_text") or "").strip()
    if type_text and "_" not in type_text:
        return type_text
    source = field.get("source") or {}
    snippet = _read_source_span(
        knowledge,
        str(source.get("file") or ""),
        int(source.get("start_line") or 0),
        int(source.get("end_line") or 0),
    )
    if not snippet:
        return type_text or None
    line = " ".join(snippet.split())
    if ":" not in line:
        return type_text or None
    return line.split(":", 1)[1].rstrip(",").strip()


def _zero_initializer_for_type_expr(type_expr: str) -> Optional[str]:
    normalized = " ".join(str(type_expr).strip().rstrip(",").split())
    if not normalized:
        return None
    array_parts = _split_array_type_expr(normalized)
    if array_parts is not None:
        element_expr, length_expr = array_parts
        element_initializer = _zero_initializer_for_type_expr(element_expr)
        if element_initializer is None:
            return None
        return "[{}; {}]".format(element_initializer, _normalize_array_length(length_expr))
    scalar = normalized.replace(" ", "")
    if scalar == "bool":
        return "false"
    if _is_zero_float_type(scalar):
        return "0.0"
    if _is_zero_scalar_type(scalar):
        return "0"
    return None


def _split_array_type_expr(type_expr: str) -> Optional[tuple[str, str]]:
    if not (type_expr.startswith("[") and type_expr.endswith("]")):
        return None
    inner = type_expr[1:-1]
    depth = 0
    for index, char in enumerate(inner):
        if char in "[<(":
            depth += 1
        elif char in "]>)":
            depth = max(0, depth - 1)
        elif char == ";" and depth == 0:
            return inner[:index].strip(), inner[index + 1 :].strip()
    return None


def _normalize_array_length(length_expr: str) -> str:
    normalized = " ".join(length_expr.split())
    if re.fullmatch(r"\d+[A-Za-z0-9_]*", normalized):
        return re.sub(r"[A-Za-z_][A-Za-z0-9_]*$", "", normalized)
    return normalized


def _is_zero_float_type(type_expr: str) -> bool:
    return type_expr in {"f32", "f64"} or type_expr.endswith("::c_float") or type_expr.endswith("::c_double")


def _is_zero_scalar_type(type_expr: str) -> bool:
    builtin_scalars = {
        "u8",
        "u16",
        "u32",
        "u64",
        "u128",
        "usize",
        "i8",
        "i16",
        "i32",
        "i64",
        "i128",
        "isize",
    }
    alias_scalars = {
        "byte",
        "word",
        "longword",
        "shortint",
        "smallint",
        "integer",
        "longint",
        "char",
    }
    return (
        type_expr in builtin_scalars
        or type_expr.lower() in alias_scalars
        or type_expr.endswith("::c_char")
        or type_expr.endswith("::c_schar")
        or type_expr.endswith("::c_uchar")
        or type_expr.endswith("::c_short")
        or type_expr.endswith("::c_ushort")
        or type_expr.endswith("::c_int")
        or type_expr.endswith("::c_uint")
        or type_expr.endswith("::c_long")
        or type_expr.endswith("::c_ulong")
        or type_expr.endswith("::c_longlong")
        or type_expr.endswith("::c_ulonglong")
    )


def _read_source_span(
    knowledge: Dict[str, Any],
    source_file: str,
    start_line: int,
    end_line: int,
) -> str:
    if not source_file or start_line <= 0 or end_line <= 0 or end_line < start_line:
        return ""
    resolved = _resolve_source_path(knowledge, source_file)
    if resolved is None or not resolved.exists():
        return ""
    lines = _read_source_lines(str(resolved))
    start_index = max(start_line - 1, 0)
    end_index = min(end_line, len(lines))
    return "\n".join(lines[start_index:end_index])


def _resolve_source_path(knowledge: Dict[str, Any], source_file: str) -> Optional[Path]:
    if not source_file:
        return None
    path = Path(source_file)
    if path.is_absolute():
        return path
    crate_meta = knowledge.get("crate_meta") or {}
    manifest_path = crate_meta.get("manifest_path")
    if manifest_path:
        return Path(manifest_path).resolve().parent / path
    lib_rs_path = crate_meta.get("lib_rs_path")
    if lib_rs_path:
        return Path(lib_rs_path).resolve().parent.parent / path
    return path


@lru_cache(maxsize=128)
def _read_source_lines(path: str) -> List[str]:
    return Path(path).read_text(encoding="utf-8", errors="ignore").splitlines()


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


def _trait_impl_display_path(impl_info: Dict[str, Any]) -> str:
    return (
        str(impl_info.get("trait_canonical_path") or "").strip()
        or str(impl_info.get("trait_ref_text") or "").strip()
        or str(impl_info.get("trait_name") or "").strip()
        or str(impl_info.get("trait_id") or "").strip()
    )


def _is_copy_trait_path(path: str) -> bool:
    normalized = path.strip()
    return normalized == "core::marker::Copy" or normalized.endswith("::Copy") or normalized == "Copy"


def _is_clone_trait_path(path: str) -> bool:
    normalized = path.strip()
    return normalized == "core::clone::Clone" or normalized.endswith("::Clone") or normalized == "Clone"


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


def _generic_param_names(generic_params: List[Any]) -> Set[str]:
    names: Set[str] = set()
    for item in generic_params:
        if isinstance(item, dict):
            candidate = item.get("name") or item.get("param") or item.get("text") or ""
        else:
            candidate = str(item)
        token = str(candidate).split(":", 1)[0].split("=", 1)[0].strip()
        if token:
            names.add(token)
    return names


def _custom_type_tokens(type_text: str, generic_param_names: Set[str]) -> List[str]:
    tokens: List[str] = []
    for token in re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*\b", type_text):
        if token in _SIGNATURE_TYPE_KEYWORDS or token in generic_param_names:
            continue
        if token in _SIGNATURE_BUILTIN_TYPE_TOKENS:
            continue
        if not _looks_like_custom_type_token(token):
            continue
        if token not in tokens:
            tokens.append(token)
    return tokens


def _looks_like_custom_type_token(token: str) -> bool:
    return any(char.isupper() for char in token)


def _graph_type_is_externally_usable(graph: nx.Graph, type_id: str) -> bool:
    return _graph_type_is_publicly_nameable(graph, type_id) or type_has_public_producer(graph, type_id)


def _graph_type_is_publicly_nameable(graph: nx.Graph, node_id: str) -> bool:
    if not graph.has_node(node_id):
        return False
    node = graph.nodes[node_id]
    return bool(node.get("public_paths") or node.get("public_anchor_module_id"))


def _graph_type_names(node: Dict[str, Any]) -> Set[str]:
    names = {str(node.get("name") or "").strip()}
    path = str(node.get("path") or "").strip()
    if path:
        names.add(path.rsplit("::", 1)[-1].strip())
    for public_path in node.get("public_paths") or []:
        names.add(str(public_path).rsplit("::", 1)[-1].strip())
    names.discard("")
    return names


def _setup_entry_basis(
    api: Dict[str, Any],
    target_api: Dict[str, Any],
    produced_type_id: str,
) -> str:
    target_owner_type_id = target_api.get("owner_type_id")
    if (
        target_owner_type_id
        and produced_type_id == target_owner_type_id
        and api.get("owner_type_id")
        and api.get("owner_type_id") != target_owner_type_id
        and _api_receiver_mentions_self(api)
    ):
        return "owner_bridge"
    return "producer_chain"


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
