# SERAPH

Semantic Rust Agent-driven Program Harness Synthesis.

This repository is the SERAPH monorepo. The current active prototype keeps Phase 1 extraction in Rust and implements the v5.1 Phase 2 RAG knowledge layer in Python.

## Status

- Monorepo with Rust workspace boundaries
- Runtime workspace separated from source tree
- OpenHarness integrated as an external dependency via git submodule
- Phase 2 RAG prototype under `rag/`
- `scripts/run.sh` can build RAG artifacts from an existing `knowledge.json`

## Repository Map

- `crates/`: Rust workspace members for core SERAPH components
- `scripts/`: orchestration entrypoints and developer helper scripts
- `skills/`: SERAPH-owned skill definitions and references
- `configs/`: long-lived configuration and environment layout
- `templates/`: schemas and reusable structural templates
- `workspace/`: runtime-only artifacts, logs, plans, fuzz workspaces
- `third_party/openharness/`: upstream OpenHarness submodule
- `integrations/openharness/`: SERAPH-specific OpenHarness integration notes
- `docs/`: architecture, specs, decisions, and research notes

## Key Design Docs

- `semantic_harness_agent_design_v5.md`
- `semantic_harness_agent_design_v4.md`
- `docs/integration/openharness.md`
- `docs/architecture/phase2-rag-redesign-v5.md`
- `docs/architecture/phase2-rag-quickstart.md`
- `docs/architecture/current-phase1-phase2-flow.md`

## Current Boundary

The active v5.1 boundary is:

- `s3-extract` remains the Phase 1 source of `knowledge.json`.
- Phase 2 uses the Python RAG path in the active prototype.
- Phase 2 RAG writes `workspace/vectordb/` and `workspace/graph.pkl`.
- Phase 3 consumes `workspace/contexts/rag_target_*.md` as LLM-readable context.
- `skills/harness-codegen` is the active Phase 3 generation contract; old scenario/mapping/planning skills are historical only.

## Phase 2 RAG Quickstart

Install Python dependencies:

```bash
python3 -m pip install --user -e 'rag[test]'
```

Run the minimal smoke path:

```bash
PYTHONPATH="$PWD/rag" scripts/run.sh \
  --knowledge rag/tests/fixtures/minimal_knowledge.json \
  --workspace-dir /tmp/seraph-rag-smoke \
  --round 1
```

For full setup notes and troubleshooting, see `docs/architecture/phase2-rag-quickstart.md`.

## Next Recommended Implementation Order

1. `crates/seraph-types`
2. `crates/s3-extract`
3. `rag/seraph_rag` retrieval quality improvements
4. `crates/s3-coverage`
5. `skills/harness-codegen` RAG context consumption
6. `crates/seraph-cli`
