# SERAPH Phase 2 RAG Redesign

> Based on `semantic_harness_agent_design_v5.md` v5.1. This document resets Phase 2 from the old `s3-model` static modeling pipeline to a retrieval-augmented knowledge layer. Phase 1 extraction remains unchanged.

## Scope

### Keep unchanged

- `crates/s3-extract`: continues producing `knowledge.json`.
- `seraph-types::Knowledge`: remains the authoritative Phase 1 schema.
- Stable IDs: `api_id`, `type_id`, `trait_id`, `module_id` stay as the only cross-phase join keys.
- Coverage semantics: `targeted`, `attempted`, `validated` remain the state machine vocabulary.
- Existing compile repair, smoke run, and fuzz workspace bootstrap concepts remain valid.

### Reset completely

- `crates/s3-model`: no longer represents Phase 2 architecture.
- `models.json`: removed from the active pipeline.
- `seraph-types::Models`, FCG, SLM, static API contracts, static risk-surface map, and trait-surface summary are no longer the Phase 2 handoff artifact.
- Old Phase 3 stages `scenario-generator`, `scenario-api-mapper`, and `api-planner` are removed from the main path.

## New Architecture

Phase 2 is a Python RAG layer with two build steps and one retrieval step:

1. Build API document vectors from `knowledge.json`.
2. Build a semantic relation graph from deterministic schema facts, vector clusters, and optional LLM relation inference.
3. Retrieve per-target unsafe context for Phase 3 harness generation.

```text
knowledge.json
  -> workspace/vectordb/api_docs
  -> workspace/vectordb/rust_idioms
  -> workspace/graph.pkl
  -> workspace/contexts/rag_target_<round>.md
  -> harness-codegen
```

## Proposed Files

### Python RAG package

- `rag/seraph_rag/__init__.py`: package marker and version.
- `rag/seraph_rag/schema.py`: typed dict/dataclass views over `knowledge.json`, graph nodes, edges, and retrieval output.
- `rag/seraph_rag/knowledge_loader.py`: loads `seraph-types::Knowledge` JSON without changing Phase 1.
- `rag/seraph_rag/documents.py`: converts APIs, types, traits, examples, and risk facts into embedding documents.
- `rag/seraph_rag/embeddings.py`: provides pluggable embedding backends with a deterministic local fallback.
- `rag/seraph_rag/vector_index.py`: builds and queries ChromaDB collections `api_docs` and `rust_idioms`.
- `rag/seraph_rag/idioms.py`: builds or loads cross-crate Rust idiom documents.
- `rag/seraph_rag/graph_builder.py`: constructs `workspace/graph.pkl` using NetworkX.
- `rag/seraph_rag/retrieve.py`: ranks unsafe targets and assembles Phase 3 context markdown/JSON.
- `rag/seraph_rag/cli.py`: internal Python implementation entrypoint used by `seraph-cli`.

### Tests and fixtures

- `rag/tests/fixtures/minimal_knowledge.json`: small Phase 1 fixture with at least one unsafe API.
- `rag/tests/test_documents.py`: verifies API document construction and metadata filters.
- `rag/tests/test_graph_builder.py`: verifies deterministic graph edges and unsafe ego-subgraph marking.
- `rag/tests/test_retrieve.py`: verifies unsafe target ranking and context assembly.

### Legacy cleanup

- `Cargo.toml`: remove `crates/s3-model` from active workspace once no code depends on it.
- `crates/seraph-types/src/lib.rs`: stop re-exporting `models` after migration.
- `crates/seraph-types/src/models.rs`: delete or move to an archival compatibility module only if needed by old artifacts.
- `skills/scenario-generator`, `skills/scenario-api-mapper`, `skills/api-planner`: mark deprecated or remove from the default orchestration path.

## Data Contracts

### API vector document

Each public callable API becomes one document in ChromaDB collection `api_docs`.

```json
{
  "id": "api::<stable_api_id>",
  "document": "path, signature, receiver, docs, panic/errors/safety text, risk facts",
  "metadata": {
    "api_id": "fn_003",
    "module_id": "mod_001",
    "owner_type_id": "type_001",
    "has_unsafe": true,
    "has_ffi": false,
    "has_panic_points": true,
    "receiver": "&mut self",
    "return_shape": "Result",
    "risk_level": "high"
  }
}
```

The metadata must contain only Chroma-compatible scalar values. Missing stable IDs should be encoded as an empty string rather than JSON `null` when required by the backend.

### Rust idiom document

Each idiom becomes one reusable document in ChromaDB collection `rust_idioms`.

```json
{
  "id": "idiom::unsafe::aliasing::001",
  "document": "A short 2-5 sentence Rust usage rule with fuzz-harness implications.",
  "metadata": {
    "category": "unsafe_semantics",
    "source": "rustonomicon",
    "applies_to": "raw_pointer,aliasing,unsafe_block"
  }
}
```

### Semantic graph

`workspace/graph.pkl` is a NetworkX graph serialized with pickle. Nodes use stable IDs.

Node kinds:

- `api`: callable API from `knowledge.apis`.
- `type`: public nominal type from `knowledge.types`.
- `trait`: trait node from `knowledge.trait_registry`.
- `module`: module node from `knowledge.modules`.
- `cluster`: semantic cluster created from vector similarity.

Edge kinds:

- `module_contains_api`: module to API.
- `type_owns_method`: type to method API.
- `api_returns_type`: API to returned type.
- `api_accepts_type`: API to parameter type.
- `trait_has_method`: trait to method API when Phase 1 exposes it.
- `impl_connects_type_trait`: type to trait through `trait_impl_registry`.
- `semantic_similar`: API to API from vector similarity.
- `same_cluster`: API to semantic cluster.
- `llm_relation`: optional API to API relation inferred inside a bounded cluster.

Unsafe API nodes must include:

```json
{
  "has_unsafe": true,
  "unsafe_context_subgraph": ["fn_001", "fn_003", "type_001"]
}
```

## Retrieval Output

`rag retrieve` produces a context artifact for `harness-codegen`. The recommended first format is markdown because it is directly LLM-readable and easy to inspect.

```markdown
# SERAPH RAG Harness Context

## Target API
- api_id: fn_003
- path: crate::Type::dangerous
- signature: pub fn dangerous(&mut self, input: &[u8]) -> Result<(), Error>
- unsafe evidence: unsafe block in source, Safety docs if available

## Related APIs
- constructor/setup APIs from graph neighbors
- mutation/query/finalization APIs from graph neighbors
- vector-similar APIs not already present in graph context

## Relevant Types and Traits
- stable type and trait IDs required to understand construction constraints

## Rust Idioms
- retrieved ownership, Result handling, unsafe, FFI, Drop, or trait-safety guidance

## Generation Rules
- Call the target API.
- Emit `SERAPH_STEP_ENTER:<step_no>:<api_id>` before each targeted API call.
- Emit `SERAPH_STEP_OK:<step_no>:<api_id>` after successful return.
- Use early return for recoverable `Result`/`Option` paths.
- Do not invent crate names; use `crate_import_name` from `knowledge.json`.
```

## CLI Contract

The RAG package should expose these commands:

```bash
cargo run -p seraph-cli -- phase2 index \
  --knowledge workspace/knowledge.json \
  --vectordb workspace/vectordb

cargo run -p seraph-cli -- phase2 graph \
  --knowledge workspace/knowledge.json \
  --vectordb workspace/vectordb \
  --graph workspace/graph.pkl

cargo run -p seraph-cli -- phase2 targets \
  --graph workspace/graph.pkl \
  --coverage workspace/coverage.json

cargo run -p seraph-cli -- phase2 retrieve \
  --knowledge workspace/knowledge.json \
  --vectordb workspace/vectordb \
  --graph workspace/graph.pkl \
  --coverage workspace/coverage.json \
  --round 1 \
  --output workspace/contexts/rag_target_001.md
```

## Orchestration Changes

The new main path is:

```bash
s3-extract --manifest-path <target>/Cargo.toml --output workspace/knowledge.json
cargo run -p seraph-cli -- phase2 index --knowledge workspace/knowledge.json --vectordb workspace/vectordb
cargo run -p seraph-cli -- phase2 graph --knowledge workspace/knowledge.json --vectordb workspace/vectordb --graph workspace/graph.pkl
cargo run -p seraph-cli -- phase2 retrieve --knowledge workspace/knowledge.json --vectordb workspace/vectordb --graph workspace/graph.pkl --coverage workspace/coverage.json --round 1 --output workspace/contexts/rag_target_001.md
oh run --skill harness-codegen --context workspace/contexts/rag_target_001.md --output-dir workspace/fuzz/fuzz_targets
```

No `s3-model`, `models.json`, scenario artifact, mapping artifact, or API plan artifact should appear in this path.

## Migration Strategy

1. Add the Python RAG package without deleting old Rust code.
2. Add tests around document construction, graph construction, unsafe ranking, and retrieval output.
3. Update orchestration scripts to use RAG artifacts instead of `models.json`.
4. Update `harness-codegen` prompt contract to consume RAG context directly.
5. Deprecate old Phase 2 and old Phase 3 stage artifacts after the new smoke path works.
6. Remove `s3-model` from the workspace when no command, test, or documentation references it as an active component.

## Non-Goals

- Do not modify `s3-extract` for this reset unless a concrete missing Phase 1 fact blocks RAG retrieval.
- Do not preserve `models.json` as a compatibility output in the new pipeline.
- Do not make LLM output an ordered API plan before code generation.
- Do not treat safe-only crates as primary targets in this version.
