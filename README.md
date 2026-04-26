# SERAPH

Semantic Rust Agent-driven Program Harness Synthesis.

This repository is the SERAPH monorepo. The current active prototype keeps Phase 1 extraction in Rust, implements the v5 Phase 2 RAG knowledge layer in Python, and drives Phase 3 through an automatic compile-fix → smoke-run → runtime-diagnose loop.

## Status

- Monorepo with Rust workspace boundaries
- Runtime workspace separated from source tree
- OpenHarness integrated as an external dependency via git submodule
- Phase 2 RAG prototype under `rag/`
- `seraph-cli run` builds Phase 1/2 RAG artifacts and active Phase 3 artifacts

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

The active v5 boundary is:

- `s3-extract` remains the Phase 1 source of `knowledge.json`.
- Phase 2 uses the Python RAG path in the active prototype.
- Phase 2 RAG writes `workspace/vectordb/` and `workspace/graph.pkl`.
- Phase 3 consumes `workspace/contexts/rag_target_*.md` as LLM-readable context.
- Phase 3 active path is: retrieve → harness prompt → model response → harness write → compile-check → fix-loop → smoke-run → runtime-diagnose → coverage update.
- Phase 3 active harness style is `aflpp`: generated harnesses are normal Rust binaries with `fn main()` that read stdin or an optional file argument, so they can be used with AFL++ file/stdin workflows.
- Runtime findings are written to `workspace/reports/runtime_error_*.json`.
- `skills/harness-codegen` is the active Phase 3 generation contract; old scenario/mapping/planning skills are historical only.

## Model Separation

SERAPH now separates embedding-model configuration from LLM configuration:

- Phase 2 RAG uses the embedding backend configuration.
- Phase 3 generation, compile-fix, and runtime diagnosis use the LLM configuration.

Preferred embedding environment variables:

- `SERAPH_EMBEDDING_BACKEND`
- `SERAPH_EMBEDDING_BASE_URL`
- `SERAPH_EMBEDDING_MODEL`
- `SERAPH_EMBEDDING_API_KEY`

Backward-compatible legacy alias:

- `SERAPH_EMBEDDER` still works as an alias for `SERAPH_EMBEDDING_BACKEND`

Phase 3 LLM environment variables:

- `SERAPH_LLM_BASE_URL`
- `SERAPH_LLM_API_KEY`
- `SERAPH_LLM_MODEL`

Supported embedding backends today:

- `openai_compatible`: OpenAI-compatible `/embeddings` endpoint

`siliconflow` is accepted as an alias of `openai_compatible`.

The backend defaults to `openai_compatible` when `SERAPH_EMBEDDING_BACKEND` is unset. Load an env file such as `configs/environments/.env.seraph-local` or `configs/examples/seraph.third_party.env.example` before running Phase 2.

## Current Execution Flow

Use the single public entrypoint:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1 \
  --phase3-style aflpp \
  --model-command 'python3 scripts/fake_model.py --input {input}' \
  --compile-check \
  --fix-loop \
  --smoke-command 'python3 scripts/fake_runtime.py {harness}' \
  --runtime-model-command 'python3 scripts/fake_runtime_diagnose.py --input {input} --output {output}'
```

Runtime diagnosis command fallback order is:

1. `--runtime-model-command`
2. `--fix-model-command`
3. `--model-command`

Generated Phase 3 harnesses should avoid `libfuzzer_sys` and `afl::fuzz!`; the active path now asks the model for stdlib-only Rust binaries that are easy to compile, smoke-run, repair, and feed to AFL++.

## Third-Party Model Template

SERAPH includes a reusable Phase 3 model adapter template for OpenAI-compatible third-party gateways:

- script entrypoint: `scripts/third_party_openai_compatible.py`
- reusable helper logic: `rag/seraph_rag/openai_compatible.py`
- transport: Python stdlib only

Required environment variables:

- `SERAPH_LLM_BASE_URL`
- `SERAPH_LLM_MODEL`

Optional environment variables:

- `SERAPH_LLM_WIRE_API` defaults to `chat_completions`; set it to `responses` for Responses API gateways
- `SERAPH_LLM_API_KEY` is optional for local proxies that do not require bearer auth
- `SERAPH_LLM_API_PATH` defaults to `/chat/completions`
- `SERAPH_LLM_TEMPERATURE` defaults to `0.2`
- `SERAPH_LLM_TIMEOUT_SECONDS` defaults to `180`
- `SERAPH_LLM_EXTRA_HEADERS` JSON object for provider-specific headers
- `SERAPH_LLM_EXTRA_BODY` JSON object for provider-specific request fields

Local `responses` example:

```bash
export SERAPH_LLM_BASE_URL='http://127.0.0.1:8080'
export SERAPH_LLM_MODEL='gpt-5.4'
export SERAPH_LLM_WIRE_API='responses'

cargo run -p seraph-cli -- run \
  --manifest-path /path/to/Cargo.toml \
  --workspace-dir workspace \
  --round 1 \
  --model-command 'python3 scripts/third_party_openai_compatible.py --input {input} --output {output}' \
  --compile-check \
  --fix-loop \
  --fix-model-command 'python3 scripts/third_party_openai_compatible.py --input {input} --output {output}' \
  --smoke-command 'python3 scripts/your_smoke_runner.py {harness}' \
  --runtime-model-command 'python3 scripts/third_party_openai_compatible.py --input {input} --output {output}'
```

If your gateway requires bearer auth, also export `SERAPH_LLM_API_KEY`.

The same template script works for initial generation, fix-loop responses, and runtime diagnosis.

## Real Crate Compile/Smoke Helper

SERAPH also includes a reusable Phase 3 helper for real crate compilation and smoke execution:

- script entrypoint: `scripts/phase3_real_crate.py`
- reusable helper logic: `rag/seraph_rag/real_crate_runner.py`

It uses `workspace/crate_config.json` plus the generated harness path to:

- create `workspace/_cargo_projects/<harness_stem>/`
- write `Cargo.toml` with a path dependency on the real target crate
- copy the harness into `src/main.rs`
- run `cargo build` for compile-check or `cargo run` for smoke-run

Example:

```bash
cargo run -p seraph-cli -- run \
  --manifest-path /path/to/target/Cargo.toml \
  --workspace-dir workspace \
  --round 1 \
  --phase3-style aflpp \
  --model-command 'python3 scripts/third_party_openai_compatible.py --input {input} --output {output}' \
  --compile-check \
  --compile-command 'python3 scripts/phase3_real_crate.py compile {harness}' \
  --fix-loop \
  --fix-model-command 'python3 scripts/third_party_openai_compatible.py --input {input} --output {output}' \
  --smoke-command 'python3 scripts/phase3_real_crate.py smoke {harness}'
```

## Phase 2 RAG Quickstart

Install Python dependencies:

```bash
python3 -m pip install --user -e 'rag[test]'
```

Run the minimal smoke path:

```bash
cargo run -p seraph-cli -- run \
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
