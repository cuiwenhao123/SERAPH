# Phase 2 RAG Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the old `s3-model`/`models.json` Phase 2 with a Python RAG knowledge layer that builds API vectors, a semantic graph, and per-unsafe-target harness contexts.

**Architecture:** Phase 1 remains Rust and continues writing `knowledge.json`. Phase 2 becomes a Python package that indexes `knowledge.json` into ChromaDB, builds `workspace/graph.pkl` with NetworkX, and retrieves markdown contexts for Phase 3. Phase 3 consumes RAG context directly; LLM codegen decides call sequences without scenario/mapping/planning artifacts.

**Tech Stack:** Rust workspace for existing extraction/coverage, Python 3.11+, ChromaDB, scikit-learn, NetworkX, pytest, optional UniXcoder/Transformers embedding backend with deterministic fallback for tests.

---

## File Structure

- Create: `rag/pyproject.toml` — Python package metadata and pytest config.
- Create: `rag/seraph_rag/__init__.py` — package version.
- Create: `rag/seraph_rag/schema.py` — dataclasses for API docs, metadata, graph nodes, retrieval context.
- Create: `rag/seraph_rag/knowledge_loader.py` — JSON loader and minimal validation for `seraph-types::Knowledge`.
- Create: `rag/seraph_rag/documents.py` — API document text and scalar metadata construction.
- Create: `rag/seraph_rag/embeddings.py` — embedding interface, hashing fallback, optional UniXcoder backend.
- Create: `rag/seraph_rag/vector_index.py` — ChromaDB collection creation and API/idiom indexing.
- Create: `rag/seraph_rag/idioms.py` — bundled Rust idiom corpus builder.
- Create: `rag/seraph_rag/graph_builder.py` — NetworkX graph construction, clustering, unsafe subgraph marking.
- Create: `rag/seraph_rag/retrieve.py` — unsafe ranking and context assembly.
- Create: `rag/seraph_rag/cli.py` — `index`, `graph`, `targets`, `retrieve` commands.
- Create: `rag/tests/fixtures/minimal_knowledge.json` — fixture with one constructor, one safe mutator, one unsafe target.
- Create: `rag/tests/test_documents.py` — document and metadata tests.
- Create: `rag/tests/test_graph_builder.py` — graph edge and unsafe context tests.
- Create: `rag/tests/test_retrieve.py` — target ranking and markdown context tests.
- Modify: `scripts/run.sh` — replace `s3-model` invocation with RAG commands after package is working.
- Modify: `Cargo.toml` — remove `crates/s3-model` from workspace after migration.
- Modify: `README.md` and `docs/architecture/README.md` — document the new Phase 2 path.

---

### Task 1: Python Package Scaffold

**Files:**
- Create: `rag/pyproject.toml`
- Create: `rag/seraph_rag/__init__.py`
- Create: `rag/seraph_rag/schema.py`
- Create: `rag/tests/fixtures/minimal_knowledge.json`

- [ ] **Step 1: Create package config**

Create `rag/pyproject.toml`:

```toml
[build-system]
requires = ["setuptools>=68"]
build-backend = "setuptools.build_meta"

[project]
name = "seraph-rag"
version = "0.1.0"
description = "SERAPH Phase 2 RAG knowledge layer"
requires-python = ">=3.11"
dependencies = [
  "chromadb>=0.5",
  "networkx>=3.2",
  "numpy>=1.26",
  "scikit-learn>=1.4",
]

[project.optional-dependencies]
embeddings = [
  "torch>=2.2",
  "transformers>=4.40",
]
test = [
  "pytest>=8",
]

[project.scripts]
seraph-rag = "seraph_rag.cli:main"

[tool.pytest.ini_options]
testpaths = ["tests"]
pythonpath = ["."]
```

- [ ] **Step 2: Add package marker**

Create `rag/seraph_rag/__init__.py`:

```python
__version__ = "0.1.0"
```

- [ ] **Step 3: Add schema dataclasses**

Create `rag/seraph_rag/schema.py`:

```python
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Literal

NodeKind = Literal["api", "type", "trait", "module", "cluster"]
EdgeKind = Literal[
    "module_contains_api",
    "type_owns_method",
    "api_returns_type",
    "api_accepts_type",
    "trait_has_method",
    "impl_connects_type_trait",
    "semantic_similar",
    "same_cluster",
    "llm_relation",
]


@dataclass(frozen=True)
class ApiDocument:
    doc_id: str
    api_id: str
    text: str
    metadata: dict[str, str | int | float | bool]


@dataclass(frozen=True)
class IdiomDocument:
    doc_id: str
    text: str
    metadata: dict[str, str | int | float | bool]


@dataclass(frozen=True)
class UnsafeTarget:
    api_id: str
    path: str
    score: float
    reason: str


@dataclass(frozen=True)
class RetrievalContext:
    target: UnsafeTarget
    markdown: str
    api_ids: list[str] = field(default_factory=list)
```

- [ ] **Step 4: Add minimal fixture**

Create `rag/tests/fixtures/minimal_knowledge.json`:

```json
{
  "crate_meta": {
    "package_name": "fixture_crate",
    "lib_target_name": "fixture_crate",
    "crate_import_name": "fixture_crate",
    "version": "0.1.0",
    "edition": "2021",
    "rust_version": null,
    "repository": null,
    "manifest_path": "/tmp/fixture/Cargo.toml",
    "lib_rs_path": "/tmp/fixture/src/lib.rs",
    "default_features": [],
    "cargo_description": "Fixture crate",
    "root_docs": "Fixture crate docs.",
    "root_doc_sections": {"summary": "Fixture crate docs.", "panics": "", "errors": "", "safety": "", "examples": ""}
  },
  "modules": [
    {"module_id": "mod::fixture_crate", "name": "fixture_crate", "canonical_path": "fixture_crate", "public_paths": ["fixture_crate"], "parent_module_id": null, "code_ref": {"file": "src/lib.rs", "line": 1, "column": 1}, "docs": "", "doc_sections": {"summary": "", "panics": "", "errors": "", "safety": "", "examples": ""}}
  ],
  "types": [
    {"type_id": "type::fixture_crate::Buffer", "name": "Buffer", "canonical_path": "fixture_crate::Buffer", "public_paths": ["fixture_crate::Buffer"], "public_anchor_module_id": "mod::fixture_crate", "code_ref": {"file": "src/lib.rs", "line": 3, "column": 1}, "docs": "Mutable byte buffer.", "doc_sections": {"summary": "Mutable byte buffer.", "panics": "", "errors": "", "safety": "", "examples": ""}, "kind": "struct", "generic_params": [], "where_clauses": [], "is_non_exhaustive": false, "fields": [], "variants": [], "has_hidden_fields": true, "has_hidden_variants": false}
  ],
  "apis": [
    {"api_id": "fn::fixture_crate::Buffer::new", "name": "new", "canonical_path": "fixture_crate::Buffer::new", "public_paths": ["fixture_crate::Buffer::new"], "module_id": "mod::fixture_crate", "owner_type_id": "type::fixture_crate::Buffer", "code_ref": {"file": "src/lib.rs", "line": 7, "column": 5}, "docs": "Creates an empty buffer.", "doc_sections": {"summary": "Creates an empty buffer.", "panics": "", "errors": "", "safety": "", "examples": ""}, "signature": "pub fn new() -> Buffer", "receiver": null, "return_type": "Buffer", "generic_params": [], "where_clauses": [], "is_unsafe": false},
    {"api_id": "fn::fixture_crate::Buffer::push", "name": "push", "canonical_path": "fixture_crate::Buffer::push", "public_paths": ["fixture_crate::Buffer::push"], "module_id": "mod::fixture_crate", "owner_type_id": "type::fixture_crate::Buffer", "code_ref": {"file": "src/lib.rs", "line": 11, "column": 5}, "docs": "Pushes bytes into the buffer.", "doc_sections": {"summary": "Pushes bytes into the buffer.", "panics": "", "errors": "", "safety": "", "examples": ""}, "signature": "pub fn push(&mut self, bytes: &[u8])", "receiver": "&mut self", "return_type": "()", "generic_params": [], "where_clauses": [], "is_unsafe": false},
    {"api_id": "fn::fixture_crate::Buffer::get_unchecked", "name": "get_unchecked", "canonical_path": "fixture_crate::Buffer::get_unchecked", "public_paths": ["fixture_crate::Buffer::get_unchecked"], "module_id": "mod::fixture_crate", "owner_type_id": "type::fixture_crate::Buffer", "code_ref": {"file": "src/lib.rs", "line": 15, "column": 5}, "docs": "Reads a byte without bounds checks.\n\n# Safety\nThe index must be in bounds.", "doc_sections": {"summary": "Reads a byte without bounds checks.", "panics": "", "errors": "", "safety": "The index must be in bounds.", "examples": ""}, "signature": "pub unsafe fn get_unchecked(&self, index: usize) -> u8", "receiver": "&self", "return_type": "u8", "generic_params": [], "where_clauses": [], "is_unsafe": true}
  ],
  "symbols": [],
  "trait_registry": [],
  "trait_impl_registry": [],
  "examples": [],
  "risk_facts": {"unsafe_functions": ["fn::fixture_crate::Buffer::get_unchecked"], "unsafe_blocks": [], "ffi_functions": [], "panic_sites": []}
}
```

- [ ] **Step 5: Run scaffold import check**

Run: `cd rag && python -c "import seraph_rag; print(seraph_rag.__version__)"`

Expected: prints `0.1.0`.

---

### Task 2: Knowledge Loading and API Documents

**Files:**
- Create: `rag/seraph_rag/knowledge_loader.py`
- Create: `rag/seraph_rag/documents.py`
- Create: `rag/tests/test_documents.py`

- [ ] **Step 1: Write failing tests**

Create `rag/tests/test_documents.py`:

```python
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
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd rag && pytest tests/test_documents.py -q`

Expected: FAIL with missing modules/functions.

- [ ] **Step 3: Implement loader**

Create `rag/seraph_rag/knowledge_loader.py`:

```python
from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def load_knowledge(path: str | Path) -> dict[str, Any]:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    for key in ("crate_meta", "modules", "types", "apis", "risk_facts"):
        if key not in data:
            raise ValueError(f"knowledge.json missing required key: {key}")
    return data
```

- [ ] **Step 4: Implement document builder**

Create `rag/seraph_rag/documents.py`:

```python
from __future__ import annotations

from typing import Any

from seraph_rag.schema import ApiDocument

PRIMITIVE_RETURNS = {
    "bool", "char", "str", "u8", "u16", "u32", "u64", "u128", "usize",
    "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64",
}


def return_shape(return_type: str | None) -> str:
    text = (return_type or "").strip()
    if text.startswith("Result"):
        return "Result"
    if text.startswith("Option"):
        return "Option"
    if text in {"Self", "self"}:
        return "Self"
    if text in {"", "()"}:
        return "Void"
    if text in PRIMITIVE_RETURNS:
        return "Primitive"
    return "Other"


def build_api_documents(knowledge: dict[str, Any]) -> list[ApiDocument]:
    risk_facts = knowledge.get("risk_facts", {})
    unsafe_api_ids = set(risk_facts.get("unsafe_functions", []))
    ffi_api_ids = set(risk_facts.get("ffi_functions", []))
    panic_api_ids = set(risk_facts.get("panic_sites", []))
    documents: list[ApiDocument] = []
    for api in knowledge.get("apis", []):
        api_id = api["api_id"]
        sections = api.get("doc_sections", {}) or {}
        has_unsafe = bool(api.get("is_unsafe")) or api_id in unsafe_api_ids
        parts = [
            f"{api.get('canonical_path', api_id)}: {sections.get('summary') or api.get('docs', '')}",
            f"Signature: {api.get('signature', '')}",
        ]
        if api.get("receiver"):
            parts.append(f"Receiver: {api['receiver']}")
        if api.get("where_clauses"):
            parts.append(f"Generic bounds: {', '.join(api['where_clauses'])}")
        if sections.get("panics"):
            parts.append(f"Panics: {sections['panics']}")
        if sections.get("errors"):
            parts.append(f"Errors: {sections['errors']}")
        if sections.get("safety"):
            parts.append(f"Safety: {sections['safety']}")
        if has_unsafe:
            parts.append("WARNING: contains unsafe code block")
        metadata: dict[str, str | int | float | bool] = {
            "api_id": api_id,
            "module_id": api.get("module_id", ""),
            "owner_type_id": api.get("owner_type_id") or "",
            "path": api.get("canonical_path", api_id),
            "has_unsafe": has_unsafe,
            "has_ffi": api_id in ffi_api_ids,
            "has_panic_points": api_id in panic_api_ids,
            "receiver": api.get("receiver") or "",
            "return_shape": return_shape(api.get("return_type")),
            "risk_level": "high" if has_unsafe or api_id in ffi_api_ids else "medium" if api_id in panic_api_ids else "low",
        }
        documents.append(ApiDocument(f"api::{api_id}", api_id, "\n".join(parts), metadata))
    return documents
```

- [ ] **Step 5: Run document tests**

Run: `cd rag && pytest tests/test_documents.py -q`

Expected: PASS.

---

### Task 3: Embeddings and Vector Index

**Files:**
- Create: `rag/seraph_rag/embeddings.py`
- Create: `rag/seraph_rag/idioms.py`
- Create: `rag/seraph_rag/vector_index.py`

- [ ] **Step 1: Implement deterministic embedding fallback**

Create `rag/seraph_rag/embeddings.py`:

```python
from __future__ import annotations

import hashlib
import math
from dataclasses import dataclass
from typing import Protocol


class Embedder(Protocol):
    def encode(self, texts: list[str]) -> list[list[float]]:
        ...


@dataclass(frozen=True)
class HashingEmbedder:
    dimensions: int = 128

    def encode(self, texts: list[str]) -> list[list[float]]:
        return [self._encode_one(text) for text in texts]

    def _encode_one(self, text: str) -> list[float]:
        vector = [0.0] * self.dimensions
        for token in text.lower().replace("::", " ").split():
            digest = hashlib.sha256(token.encode("utf-8")).digest()
            index = int.from_bytes(digest[:4], "big") % self.dimensions
            sign = 1.0 if digest[4] % 2 == 0 else -1.0
            vector[index] += sign
        norm = math.sqrt(sum(value * value for value in vector)) or 1.0
        return [value / norm for value in vector]
```

- [ ] **Step 2: Implement idiom corpus**

Create `rag/seraph_rag/idioms.py`:

```python
from __future__ import annotations

from seraph_rag.schema import IdiomDocument


def bundled_idioms() -> list[IdiomDocument]:
    rows = [
        ("idiom::unsafe::aliasing::001", "unsafe_semantics", "Raw pointers and references created inside unsafe code must still respect Rust aliasing and validity rules. Fuzz harnesses should construct valid inputs before crossing unsafe boundaries."),
        ("idiom::ownership::mut-ref::001", "ownership", "An &mut reference must be exclusive for its scope. Harnesses should avoid keeping shared references alive while calling mutating APIs."),
        ("idiom::error::result::001", "error_handling", "Recoverable Result errors should be handled with match or early return in fuzz harnesses. Do not unwrap fallible parsing or construction paths."),
        ("idiom::ffi::pointer::001", "ffi", "Pointer arguments crossing FFI or unsafe boundaries must be non-null and aligned unless documentation explicitly allows otherwise."),
        ("idiom::drop::cleanup::001", "drop", "Types with Drop may encode cleanup invariants. Harnesses should prefer public constructors and finalizers instead of fabricating internal states."),
        ("idiom::trait::unsafe::001", "trait_safety", "Unsafe trait implementations rely on documented invariants. Harnesses should use existing implementations rather than inventing unsound custom implementations."),
    ]
    return [IdiomDocument(doc_id, text, {"category": category, "source": "bundled"}) for doc_id, category, text in rows]
```

- [ ] **Step 3: Implement vector index helper**

Create `rag/seraph_rag/vector_index.py`:

```python
from __future__ import annotations

from pathlib import Path

import chromadb

from seraph_rag.documents import build_api_documents
from seraph_rag.embeddings import Embedder, HashingEmbedder
from seraph_rag.idioms import bundled_idioms


def persistent_client(path: str | Path):
    return chromadb.PersistentClient(path=str(path))


def index_knowledge(knowledge: dict, vectordb: str | Path, embedder: Embedder | None = None) -> None:
    embedder = embedder or HashingEmbedder()
    client = persistent_client(vectordb)
    api_collection = client.get_or_create_collection(name="api_docs", metadata={"hnsw:space": "cosine"})
    api_docs = build_api_documents(knowledge)
    if api_docs:
        api_collection.upsert(
            ids=[doc.doc_id for doc in api_docs],
            embeddings=embedder.encode([doc.text for doc in api_docs]),
            documents=[doc.text for doc in api_docs],
            metadatas=[doc.metadata for doc in api_docs],
        )
    idiom_collection = client.get_or_create_collection(name="rust_idioms", metadata={"hnsw:space": "cosine"})
    idioms = bundled_idioms()
    idiom_collection.upsert(
        ids=[doc.doc_id for doc in idioms],
        embeddings=embedder.encode([doc.text for doc in idioms]),
        documents=[doc.text for doc in idioms],
        metadatas=[doc.metadata for doc in idioms],
    )
```

- [ ] **Step 4: Run package import check**

Run: `cd rag && python -c "from seraph_rag.vector_index import index_knowledge; print(index_knowledge.__name__)"`

Expected: prints `index_knowledge`.

---

### Task 4: Semantic Graph Builder

**Files:**
- Create: `rag/seraph_rag/graph_builder.py`
- Create: `rag/tests/test_graph_builder.py`

- [ ] **Step 1: Write graph tests**

Create `rag/tests/test_graph_builder.py`:

```python
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
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd rag && pytest tests/test_graph_builder.py -q`

Expected: FAIL with missing `graph_builder`.

- [ ] **Step 3: Implement graph builder**

Create `rag/seraph_rag/graph_builder.py`:

```python
from __future__ import annotations

import pickle
from pathlib import Path
from typing import Any

import networkx as nx


def build_graph(knowledge: dict[str, Any]) -> nx.Graph:
    graph = nx.Graph()
    risk_facts = knowledge.get("risk_facts", {})
    unsafe_api_ids = set(risk_facts.get("unsafe_functions", []))
    ffi_api_ids = set(risk_facts.get("ffi_functions", []))
    panic_api_ids = set(risk_facts.get("panic_sites", []))

    for module in knowledge.get("modules", []):
        graph.add_node(module["module_id"], kind="module", path=module.get("canonical_path", module["module_id"]))

    for type_info in knowledge.get("types", []):
        type_id = type_info["type_id"]
        graph.add_node(type_id, kind="type", path=type_info.get("canonical_path", type_id))
        module_id = type_info.get("public_anchor_module_id")
        if module_id:
            graph.add_edge(module_id, type_id, kind="module_contains_type")

    for trait in knowledge.get("trait_registry", []):
        graph.add_node(trait["trait_id"], kind="trait", path=trait.get("canonical_path", trait["trait_id"]), is_unsafe=bool(trait.get("is_unsafe")))

    for api in knowledge.get("apis", []):
        api_id = api["api_id"]
        has_unsafe = bool(api.get("is_unsafe")) or api_id in unsafe_api_ids
        graph.add_node(
            api_id,
            kind="api",
            path=api.get("canonical_path", api_id),
            signature=api.get("signature", ""),
            has_unsafe=has_unsafe,
            has_ffi=api_id in ffi_api_ids,
            has_panic_points=api_id in panic_api_ids,
        )
        module_id = api.get("module_id")
        if module_id:
            graph.add_edge(module_id, api_id, kind="module_contains_api")
        owner_type_id = api.get("owner_type_id")
        if owner_type_id:
            graph.add_edge(owner_type_id, api_id, kind="type_owns_method")

    for impl_info in knowledge.get("trait_impl_registry", []):
        type_id = impl_info.get("type_id") or impl_info.get("for_type_id")
        trait_id = impl_info.get("trait_id")
        if type_id and trait_id and graph.has_node(type_id) and graph.has_node(trait_id):
            graph.add_edge(type_id, trait_id, kind="impl_connects_type_trait")

    for node_id, data in list(graph.nodes(data=True)):
        if data.get("kind") == "api" and data.get("has_unsafe"):
            subgraph = nx.ego_graph(graph, node_id, radius=2)
            graph.nodes[node_id]["unsafe_context_subgraph"] = sorted(subgraph.nodes())

    return graph


def write_graph(graph: nx.Graph, path: str | Path) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    with Path(path).open("wb") as handle:
        pickle.dump(graph, handle)


def read_graph(path: str | Path) -> nx.Graph:
    with Path(path).open("rb") as handle:
        return pickle.load(handle)
```

- [ ] **Step 4: Run graph tests**

Run: `cd rag && pytest tests/test_graph_builder.py -q`

Expected: PASS.

---

### Task 5: Retrieval and Context Assembly

**Files:**
- Create: `rag/seraph_rag/retrieve.py`
- Create: `rag/tests/test_retrieve.py`

- [ ] **Step 1: Write retrieval tests**

Create `rag/tests/test_retrieve.py`:

```python
from pathlib import Path

from seraph_rag.graph_builder import build_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import rank_unsafe_targets, render_context_markdown

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_rank_unsafe_targets_returns_only_unsafe_api():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    targets = rank_unsafe_targets(graph)
    assert [target.api_id for target in targets] == ["fn::fixture_crate::Buffer::get_unchecked"]
    assert targets[0].score > 0


def test_render_context_markdown_contains_codegen_rules():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]
    markdown = render_context_markdown(knowledge, graph, target, idioms=["Handle Result with early return."])
    assert "# SERAPH RAG Harness Context" in markdown
    assert "fixture_crate::Buffer::get_unchecked" in markdown
    assert "fixture_crate::Buffer::new" in markdown
    assert "SERAPH_STEP_ENTER" in markdown
    assert "fixture_crate" in markdown
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd rag && pytest tests/test_retrieve.py -q`

Expected: FAIL with missing retrieval functions.

- [ ] **Step 3: Implement retrieval**

Create `rag/seraph_rag/retrieve.py`:

```python
from __future__ import annotations

from typing import Any

import networkx as nx

from seraph_rag.schema import UnsafeTarget


def unsafe_priority(api_id: str, graph: nx.Graph) -> float:
    node = graph.nodes[api_id]
    score = 0.0
    if node.get("has_ffi"):
        score += 3.0
    if node.get("has_unsafe"):
        score += 2.0
    if node.get("has_panic_points"):
        score += 1.0
    score += graph.degree(api_id) * 0.1
    return score


def rank_unsafe_targets(graph: nx.Graph, excluded: set[str] | None = None) -> list[UnsafeTarget]:
    excluded = excluded or set()
    targets = []
    for node_id, data in graph.nodes(data=True):
        if data.get("kind") == "api" and data.get("has_unsafe") and node_id not in excluded:
            score = unsafe_priority(node_id, graph)
            targets.append(UnsafeTarget(node_id, data.get("path", node_id), score, "unsafe API priority"))
    return sorted(targets, key=lambda target: (-target.score, target.api_id))


def render_context_markdown(
    knowledge: dict[str, Any],
    graph: nx.Graph,
    target: UnsafeTarget,
    idioms: list[str] | None = None,
) -> str:
    idioms = idioms or []
    apis = {api["api_id"]: api for api in knowledge.get("apis", [])}
    crate_import_name = knowledge["crate_meta"]["crate_import_name"]
    target_api = apis[target.api_id]
    related_ids = graph.nodes[target.api_id].get("unsafe_context_subgraph", [])
    related_apis = [apis[api_id] for api_id in related_ids if api_id in apis and api_id != target.api_id]
    lines = [
        "# SERAPH RAG Harness Context",
        "",
        "## Target API",
        f"- api_id: {target.api_id}",
        f"- path: {target_api.get('canonical_path', target.api_id)}",
        f"- signature: {target_api.get('signature', '')}",
        f"- safety: {(target_api.get('doc_sections') or {}).get('safety', '')}",
        "",
        "## Related APIs",
    ]
    for api in related_apis:
        lines.append(f"- {api['api_id']}: {api.get('canonical_path', api['api_id'])} — {api.get('signature', '')}")
    lines.extend(["", "## Rust Idioms"])
    for idiom in idioms:
        lines.append(f"- {idiom}")
    lines.extend([
        "",
        "## Generation Rules",
        f"- Use crate import name `{crate_import_name}`.",
        "- Call the target API in every harness variant.",
        "- Emit `SERAPH_STEP_ENTER:<step_no>:<api_id>` before each targeted API call.",
        "- Emit `SERAPH_STEP_OK:<step_no>:<api_id>` after successful return.",
        "- Use early return for recoverable `Result` and `Option` paths.",
        "- Do not use `target_lib` as a crate name.",
        "",
    ])
    return "\n".join(lines)
```

- [ ] **Step 4: Run retrieval tests**

Run: `cd rag && pytest tests/test_retrieve.py -q`

Expected: PASS.

---

### Task 6: CLI Wiring

**Files:**
- Create: `rag/seraph_rag/cli.py`

- [ ] **Step 1: Implement CLI**

Create `rag/seraph_rag/cli.py`:

```python
from __future__ import annotations

import argparse
from pathlib import Path

from seraph_rag.graph_builder import build_graph, read_graph, write_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import rank_unsafe_targets, render_context_markdown
from seraph_rag.vector_index import index_knowledge


def main() -> None:
    parser = argparse.ArgumentParser(prog="seraph-rag")
    subparsers = parser.add_subparsers(dest="command", required=True)

    index_parser = subparsers.add_parser("index")
    index_parser.add_argument("--knowledge", required=True)
    index_parser.add_argument("--vectordb", required=True)

    graph_parser = subparsers.add_parser("graph")
    graph_parser.add_argument("--knowledge", required=True)
    graph_parser.add_argument("--graph", required=True)

    targets_parser = subparsers.add_parser("targets")
    targets_parser.add_argument("--graph", required=True)


    retrieve_parser = subparsers.add_parser("retrieve")
    retrieve_parser.add_argument("--knowledge", required=True)
    retrieve_parser.add_argument("--graph", required=True)
    retrieve_parser.add_argument("--output", required=True)
    retrieve_parser.add_argument("--round", type=int, default=1)

    args = parser.parse_args()
    if args.command == "index":
        index_knowledge(load_knowledge(args.knowledge), args.vectordb)
        return
    if args.command == "graph":
        write_graph(build_graph(load_knowledge(args.knowledge)), args.graph)
        return
    if args.command == "targets":
        for target in rank_unsafe_targets(read_graph(args.graph)):
            print(f"{target.api_id}\t{target.score:.2f}\t{target.path}")
        return
    if args.command == "retrieve":
        knowledge = load_knowledge(args.knowledge)
        graph = read_graph(args.graph)
        targets = rank_unsafe_targets(graph)
        if not targets:
            raise SystemExit("no unsafe targets available")
        markdown = render_context_markdown(knowledge, graph, targets[0])
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(markdown, encoding="utf-8")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run CLI smoke path**

Run:

```bash
cd rag
python -m seraph_rag.cli graph --knowledge tests/fixtures/minimal_knowledge.json --graph /tmp/seraph_graph.pkl
python -m seraph_rag.cli targets --graph /tmp/seraph_graph.pkl
python -m seraph_rag.cli retrieve --knowledge tests/fixtures/minimal_knowledge.json --graph /tmp/seraph_graph.pkl --output /tmp/seraph_context.md
grep 'SERAPH_STEP_ENTER' /tmp/seraph_context.md
```

Expected: target list includes `fn::fixture_crate::Buffer::get_unchecked`; grep finds `SERAPH_STEP_ENTER`.

---

### Task 7: Orchestration Migration

**Files:**
- Modify: `scripts/run.sh`
- Modify: `README.md`
- Modify: `docs/architecture/README.md`

- [ ] **Step 1: Locate old `s3-model` calls**

Run: `rg -n "s3-model|models.json|scenario-generator|scenario-api-mapper|api-planner" scripts README.md docs skills crates`

Expected: list of old active-path references.

- [ ] **Step 2: Update run script Phase 2 commands**

Modify `scripts/run.sh` so the active Phase 2 flow is:

```bash
python -m seraph_rag.cli index \
  --knowledge ./workspace/knowledge.json \
  --vectordb ./workspace/vectordb

python -m seraph_rag.cli graph \
  --knowledge ./workspace/knowledge.json \
  --graph ./workspace/graph.pkl
```

- [ ] **Step 3: Update retrieval loop command**

Modify `scripts/run.sh` so each synthesis round creates context with:

```bash
python -m seraph_rag.cli retrieve \
  --knowledge ./workspace/knowledge.json \
  --graph ./workspace/graph.pkl \
  --round "$ROUND" \
  --output "$CONTEXT_FILE"
```

- [ ] **Step 4: Update docs**

Document that Phase 2 now produces `workspace/vectordb/` and `workspace/graph.pkl`, not `models.json`.

- [ ] **Step 5: Run text search verification**

Run: `rg -n "active.*models.json|s3-model.*active|scenario-generator.*active|api-planner.*active" README.md docs scripts`

Expected: no references describing old Phase 2/3 as active.

---

### Task 8: Legacy Rust Workspace Cleanup

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/seraph-types/src/lib.rs`
- Delete or archive: `crates/seraph-types/src/models.rs`
- Delete or archive: `crates/s3-model/**`

- [ ] **Step 1: Verify no active dependency remains**

Run: `rg -n "seraph_types::Models|build_models_from_knowledge|s3-model|models::|crate::models|models.json" crates scripts rag README.md docs`

Expected: only archival docs or old plan references remain.

- [ ] **Step 2: Remove workspace member**

Modify `Cargo.toml` and remove:

```toml
"crates/s3-model",
```

- [ ] **Step 3: Stop exporting old model schema**

Modify `crates/seraph-types/src/lib.rs` and remove:

```rust
pub mod models;
pub use models::*;
```

- [ ] **Step 4: Delete old model files**

Remove `crates/s3-model/` and `crates/seraph-types/src/models.rs` only after Step 1 confirms no active Rust code uses them.

- [ ] **Step 5: Run Rust verification**

Run: `cargo test --workspace`

Expected: PASS for remaining Rust workspace members.

---

## Final Verification

- [ ] Run Python tests: `cd rag && pytest -q`
- [ ] Run CLI smoke path on `rag/tests/fixtures/minimal_knowledge.json`.
- [ ] Run Rust tests: `cargo test --workspace`.
- [ ] Run search: `rg -n "models.json|scenario-generator|scenario-api-mapper|api-planner|s3-model" scripts README.md docs/architecture skills/harness-codegen` and confirm any remaining mentions are explicitly deprecated or historical.
- [ ] Confirm `semantic_harness_agent_design_v5.md` Phase 1 promises remain unchanged by the implementation.
