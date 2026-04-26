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

Phase 2 now requires an OpenAI-compatible embedding endpoint.

Load one of the example env files before running Phase 2 commands:

```bash
set -a && source configs/environments/.env.seraph-local && set +a
```

Phase 2 RAG reads embedding configuration only. It does not read Phase 3 LLM configuration.

Preferred embedding environment variables:

- `SERAPH_EMBEDDING_BACKEND`
- `SERAPH_EMBEDDING_BASE_URL`
- `SERAPH_EMBEDDING_MODEL`
- `SERAPH_EMBEDDING_API_KEY`

Backward-compatible legacy alias:

- `SERAPH_EMBEDDER` behaves the same as `SERAPH_EMBEDDING_BACKEND`

The backend is selected with `SERAPH_EMBEDDING_BACKEND`:

```bash
cargo run -p seraph-cli -- run \
  --knowledge rag/tests/fixtures/minimal_knowledge.json \
  --workspace-dir /tmp/seraph-rag-smoke \
  --round 1
```

Supported values today:

| Value | Status | Notes |
|-------|--------|-------|
| `openai_compatible` | supported | Calls an OpenAI-compatible `/embeddings` endpoint |

Provider note:

- `siliconflow` is accepted as an alias of `openai_compatible`.
- `SERAPH_EMBEDDING_API_PATH` defaults to `/embeddings`.
- `SERAPH_EMBEDDING_TIMEOUT_SECONDS` defaults to `180`.
- `SERAPH_EMBEDDING_EXTRA_HEADERS` and `SERAPH_EMBEDDING_EXTRA_BODY` accept JSON objects for provider-specific overrides.
- If SiliconFlow returns `401 Api key is invalid`, first check whether the copied key accidentally contains the same `sk-...` token twice. SERAPH now rejects this duplicated-token pattern early with a local configuration error.

The vector DB stores the backend name in Chroma collection metadata. If `SERAPH_EMBEDDING_BACKEND` changes between indexing and retrieval, SERAPH raises an embedding backend mismatch error. Rebuild `workspace/vectordb/` whenever changing embedding backends.

Trade-off:

- OpenAI-compatible embedding providers keep the same Phase 2 interface while improving semantic retrieval quality over the removed local fallback.
- Any model-backed embedder must use the same backend for both indexing and querying.

Important: do not call ChromaDB `query_texts` directly against SERAPH's current collections. Use `cargo run -p seraph-cli -- phase2 retrieve` or the `seraph_rag.retrieve` library helpers, which query with explicit matching embeddings and backend metadata checks.

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
cargo run -p seraph-cli -- phase2 index \
  --knowledge tests/fixtures/minimal_knowledge.json \
  --vectordb /tmp/seraph_vectordb
cargo run -p seraph-cli -- phase2 graph \
  --knowledge tests/fixtures/minimal_knowledge.json \
  --graph /tmp/seraph_graph.pkl
cargo run -p seraph-cli -- phase2 targets \
  --graph /tmp/seraph_graph.pkl
cargo run -p seraph-cli -- phase2 retrieve \
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
cargo run -p seraph-cli -- phase2 index \
  --knowledge workspace/knowledge.json \
  --vectordb workspace/vectordb

cargo run -p seraph-cli -- phase2 graph \
  --knowledge workspace/knowledge.json \
  --graph workspace/graph.pkl

cargo run -p seraph-cli -- phase2 targets \
  --graph workspace/graph.pkl

cargo run -p seraph-cli -- phase2 retrieve \
  --knowledge workspace/knowledge.json \
  --graph workspace/graph.pkl \
  --vectordb workspace/vectordb \
  --round 1 \
  --output workspace/contexts/rag_target_001.md
```

`workspace/contexts/rag_target_001.md` is the input context for `harness-codegen`.

Target selection behavior:

- `--round 1` selects the top-ranked unsafe target, `--round 2` selects the next one, and so on.
- `--target-api-id <api_id>` forces retrieval for a specific unsafe target when you want to retry or compare a particular API.

Context layout behavior:

- `## Required Setup APIs` contains producer/setup-chain APIs that are structurally needed to reach the target.
- `## Related APIs` remains the broader exploratory neighborhood.
- This split is intentional for deep targets such as decoder entrypoints, where setup APIs must stay visible even when the broader graph is noisy.

The active Phase 3 skill contract is documented in `skills/harness-codegen/README.md`. It consumes the RAG markdown directly; it does not require `scenario.json`, `api_mapping.json`, or `api_plan.json`.

## Orchestration Script

`seraph-cli run` wraps the active Phase 2 path:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1
```

To run Phase 1 extraction first:

```bash
cargo run -p seraph-cli -- run \
  --manifest-path /path/to/target/Cargo.toml \
  --workspace-dir workspace \
  --round 1
```

To inspect commands without executing them:

```bash
cargo run -p seraph-cli -- run \
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
