# Phase 2 RAG Quickstart

This note records the runtime requirements for the SERAPH v5.1 Phase 2 RAG prototype so another developer can set it up quickly.

## Scope

- Phase 1 stays Rust: `s3-extract` writes `workspace/knowledge.json`.
- Phase 2 RAG is Python: `rag/seraph_rag` writes `workspace/vectordb/` and `workspace/graph.pkl`.
- Legacy static Phase 2 artifacts are not required for this flow.

## System Requirements

- Python 3.8 or newer.
- Rust toolchain from `rust-toolchain.toml` for the unchanged Rust crates.
- Network access for initial Python dependency installation.
- On Linux distributions with old system SQLite, install `pysqlite3-binary`; the package automatically switches ChromaDB to it when needed.

## Python Dependencies

From the repository root:

```bash
python3 -m pip install --user -e 'rag[test]'
```

If editable extras are not available in the environment, install the runtime dependencies explicitly:

```bash
python3 -m pip install --user \
  'chromadb>=0.5,<0.6' \
  'networkx>=3.1' \
  'numpy>=1.24' \
  'scikit-learn>=1.3' \
  'pysqlite3-binary>=0.5.3' \
  'protobuf>=5.29,<6' \
  'wrapt<2,>=1.14' \
  'click>=8.1,<9' \
  'posthog<4' \
  'pytest>=8'
```

Why these pins exist:

- `chromadb` requires SQLite `>= 3.35`; `pysqlite3-binary` provides a modern SQLite on older systems.
- ChromaDB's default telemetry path imports `posthog`; SERAPH disables telemetry with `seraph_rag.chroma_telemetry.NoopProductTelemetryClient`.
- Python 3.8 environments need conservative dependency versions because some latest packages use newer typing syntax.

## Embedding Model Configuration

No external embedding model is required for the current Phase 2 prototype.

By default, SERAPH uses `HashingEmbedder(dimensions=128)`, a deterministic local fallback implemented in `rag/seraph_rag/embeddings.py`. This means Phase 2 can run without configuring UniXcoder, Transformers, PyTorch, GPU drivers, or model download credentials.

The backend is selected with `SERAPH_EMBEDDER`:

```bash
SERAPH_EMBEDDER=hashing PYTHONPATH="$PWD/rag" scripts/run.sh \
  --knowledge rag/tests/fixtures/minimal_knowledge.json \
  --workspace-dir /tmp/seraph-rag-smoke \
  --round 1
```

Supported values today:

| Value | Status | Notes |
|-------|--------|-------|
| `hashing` | default, supported | 128-dimensional deterministic local vectors |

The vector DB stores the backend name in Chroma collection metadata. If `SERAPH_EMBEDDER` changes between indexing and retrieval, SERAPH raises an embedding backend mismatch error. Rebuild `workspace/vectordb/` whenever changing embedding backends.

Trade-off:

- The default hashing embedder is fast, deterministic, offline-friendly, and suitable for smoke tests and pipeline development.
- Retrieval quality is weaker than a code-aware neural embedding model.
- Future UniXcoder or other model-backed embedders should use the same embedding backend for both indexing and querying.

Important: do not call ChromaDB `query_texts` directly against SERAPH's current collections. The current prototype indexes with 128-dimensional hashing embeddings, while ChromaDB's default text embedding function may use a different dimensionality. Use `seraph_rag.retrieve` or `seraph_rag.cli retrieve`, which query with explicit matching embeddings.

## Smoke Test

Run the RAG unit and integration tests:

```bash
cd rag
pytest -q
```

Expected result:

```text
16 passed
```

Run the CLI on the minimal fixture:

```bash
cd rag
rm -rf /tmp/seraph_vectordb /tmp/seraph_graph.pkl /tmp/seraph_context.md
python3 -m seraph_rag.cli index \
  --knowledge tests/fixtures/minimal_knowledge.json \
  --vectordb /tmp/seraph_vectordb
python3 -m seraph_rag.cli graph \
  --knowledge tests/fixtures/minimal_knowledge.json \
  --graph /tmp/seraph_graph.pkl
python3 -m seraph_rag.cli targets \
  --graph /tmp/seraph_graph.pkl
python3 -m seraph_rag.cli retrieve \
  --knowledge tests/fixtures/minimal_knowledge.json \
  --graph /tmp/seraph_graph.pkl \
  --vectordb /tmp/seraph_vectordb \
  --output /tmp/seraph_context.md
grep 'SERAPH_STEP_OK' /tmp/seraph_context.md
test -f /tmp/seraph_vectordb/chroma.sqlite3 && echo vectordb-ok
```

Expected output includes:

```text
fn::fixture_crate::Buffer::get_unchecked
SERAPH_STEP_OK
vectordb-ok
```

## Real Crate Workflow

After Phase 1 extraction creates `workspace/knowledge.json`:

```bash
python3 -m seraph_rag.cli index \
  --knowledge workspace/knowledge.json \
  --vectordb workspace/vectordb

python3 -m seraph_rag.cli graph \
  --knowledge workspace/knowledge.json \
  --graph workspace/graph.pkl

python3 -m seraph_rag.cli targets \
  --graph workspace/graph.pkl

python3 -m seraph_rag.cli retrieve \
  --knowledge workspace/knowledge.json \
  --graph workspace/graph.pkl \
  --vectordb workspace/vectordb \
  --round 1 \
  --output workspace/contexts/rag_target_001.md
```

`workspace/contexts/rag_target_001.md` is the input context for `harness-codegen`.

The active Phase 3 skill contract is documented in `skills/harness-codegen/README.md`. It consumes the RAG markdown directly; it does not require `scenario.json`, `api_mapping.json`, or `api_plan.json`.

## Orchestration Script

`scripts/run.sh` wraps the active Phase 2 path:

```bash
PYTHONPATH="$PWD/rag" scripts/run.sh \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1
```

To run Phase 1 extraction first:

```bash
PYTHONPATH="$PWD/rag" scripts/run.sh \
  --manifest-path /path/to/target/Cargo.toml \
  --workspace-dir workspace \
  --round 1
```

To inspect commands without executing them:

```bash
scripts/run.sh \
  --knowledge rag/tests/fixtures/minimal_knowledge.json \
  --workspace-dir /tmp/seraph-run-sh-test \
  --round 7 \
  --dry-run
```

## Generated Artifacts

These are runtime artifacts and should not be committed:

- `workspace/vectordb/`
- `workspace/graph.pkl`
- `workspace/contexts/rag_target_*.md`
- Python caches such as `__pycache__/` and `.pytest_cache/`

The repository `.gitignore` already excludes these paths.

## Troubleshooting

### ChromaDB reports old SQLite

Symptom:

```text
Chroma requires sqlite3 >= 3.35.0
```

Fix:

```bash
python3 -m pip install --user 'pysqlite3-binary>=0.5.3'
```

The SERAPH RAG client calls `ensure_chromadb_sqlite()` before importing ChromaDB and swaps in `pysqlite3` when the system SQLite is too old.

### ChromaDB imports telemetry dependencies that fail on Python 3.8

Use the pinned dependencies above. SERAPH also passes a no-op telemetry implementation through Chroma settings.

### Query dimension mismatch

All indexing and querying in the current prototype must use `HashingEmbedder(dimensions=128)`. Do not call Chroma `query_texts` against these collections because Chroma's default embedding function uses a different dimension. Use SERAPH retrieval helpers, which query with explicit embeddings.
