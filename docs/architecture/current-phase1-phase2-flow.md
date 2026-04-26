# Current Phase 1 / Phase 2 Execution Flow

This document describes the current SERAPH execution flow after the v5.1 Phase 2 reset. The active scope is Phase 1 extraction plus Phase 2 RAG knowledge assembly.

## Active Components

| Phase | Component | Language | Input | Output |
|-------|-----------|----------|-------|--------|
| Phase 1 | `s3-extract` | Rust | target crate `Cargo.toml` | `workspace/knowledge.json` |
| Phase 2A | `seraph-cli phase2 index` | Python | `knowledge.json` | `workspace/vectordb/` |
| Phase 2B | `seraph-cli phase2 graph` | Python | `knowledge.json` | `workspace/graph.pkl` |
| Phase 2C | `seraph-cli phase2 targets` | Python | `graph.pkl` | ranked unsafe target list |
| Phase 2D | `seraph-cli phase2 retrieve` | Python | `knowledge.json`, `vectordb/`, `graph.pkl` | `workspace/contexts/rag_target_RRR.md` |

## Removed From Active Path

The static Rust Phase 2 modeling path has been removed from the active workspace. The current Phase 2 does not produce or consume static model artifacts.

## Setup

Install Python dependencies from the repository root:

```bash
python3 -m pip install --user -e 'rag[test]'
```

Load an embedding env file before Phase 2 commands:

```bash
set -a && source configs/environments/.env.seraph-local && set +a
export PYTHONPATH="$PWD/rag"
```

Phase 2 RAG reads the embedding configuration only:

- `SERAPH_EMBEDDING_BACKEND`
- `SERAPH_EMBEDDING_BASE_URL`
- `SERAPH_EMBEDDING_MODEL`

Legacy `SERAPH_EMBEDDER` remains accepted as a backward-compatible alias.

## One-Command Fixture Smoke Test

```bash
cargo run -p seraph-cli -- run \
  --knowledge rag/tests/fixtures/s3_audit_fixture_knowledge.json \
  --workspace-dir /tmp/seraph-phase2-smoke \
  --round 1
```

Expected artifacts:

```text
/tmp/seraph-phase2-smoke/vectordb/chroma.sqlite3
/tmp/seraph-phase2-smoke/graph.pkl
/tmp/seraph-phase2-smoke/contexts/rag_target_001.md
```

## Real Crate Flow

### 1. Extract Phase 1 knowledge

```bash
cargo run -p s3-extract -- \
  --manifest-path /path/to/target/Cargo.toml \
  --output workspace/knowledge.json
```

### 2. Build the vector DB

```bash
cargo run -p seraph-cli -- phase2 index \
  --knowledge workspace/knowledge.json \
  --vectordb workspace/vectordb
```

The vector DB contains:

- `api_docs`: API signature, docs, unsafe flags, risk metadata, return shape.
- `rust_idioms`: bundled Rust safety and usage guidance.

### 3. Build the semantic graph

```bash
cargo run -p seraph-cli -- phase2 graph \
  --knowledge workspace/knowledge.json \
  --graph workspace/graph.pkl
```

The graph is a NetworkX `DiGraph` with stable ID nodes and structured edges:

- `module_contains_api`
- `module_contains_type`
- `type_owns_method`
- `api_returns_type`
- `api_accepts_type`
- `api_accepts_trait`
- `impl_connects_type_trait`

Unsafe API nodes also receive an `unsafe_context_subgraph` based on 2-hop undirected neighborhood expansion.

### 4. Inspect unsafe targets

```bash
cargo run -p seraph-cli -- phase2 targets \
  --graph workspace/graph.pkl
```

Targets are ranked by:

- FFI signal
- unsafe signal from `is_unsafe` or `contains_unsafe_block`
- panic signal
- graph degree

### 5. Retrieve RAG context

```bash
cargo run -p seraph-cli -- phase2 retrieve \
  --knowledge workspace/knowledge.json \
  --graph workspace/graph.pkl \
  --vectordb workspace/vectordb \
  --round 1 \
  --output workspace/contexts/rag_target_001.md
```

The context includes:

- target unsafe API
- graph-related APIs
- semantically similar API docs from ChromaDB
- Rust idiom docs from ChromaDB
- deterministic generation rules and SERAPH marker requirements

## Convenience Wrapper

`seraph-cli run` wraps the current Phase 1/2 path:

```bash
cargo run -p seraph-cli -- run \
  --manifest-path /path/to/target/Cargo.toml \
  --workspace-dir workspace \
  --round 1
```

If `knowledge.json` already exists:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1
```

Use `--dry-run` to inspect commands without executing them.

## Validation Commands

```bash
cargo test --workspace
cd rag && pytest -q
cargo test -p seraph-cli --test run_dry_run
```

For the real fixture smoke:

```bash
rm -rf /tmp/seraph-real-fixture-rag
cargo run -p seraph-cli -- run \
  --knowledge rag/tests/fixtures/s3_audit_fixture_knowledge.json \
  --workspace-dir /tmp/seraph-real-fixture-rag \
  --round 4
```

Expected target output includes:

```text
api::s3_audit_fixture::uses_unsafe_block
```

## Notes

- Rebuild `workspace/vectordb/` whenever `SERAPH_EMBEDDING_BACKEND` changes.
- Do not call ChromaDB `query_texts` directly against SERAPH collections; use SERAPH retrieval helpers so query embeddings match indexed embeddings.
- Runtime artifacts under `workspace/` are ignored by git.

## Unified Run Entry

Use the single unified run entry:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1
```

The Python `seraph_rag.cli` commands remain internal implementation entrypoints for development and debugging.
