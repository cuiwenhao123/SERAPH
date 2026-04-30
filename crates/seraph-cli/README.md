# seraph-cli

Unified SERAPH CLI entrypoint.

`seraph-cli` is the user-facing command layer. It currently delegates the active Phase 2 RAG path and active Phase 3 orchestration steps to the Python `seraph_rag` implementation modules. The Python CLI remains available for development/debugging, but scripts and documentation should prefer this crate.

Configuration boundary:

- Phase 2 RAG uses the embedding configuration: `SERAPH_EMBEDDING_BACKEND` and optional `SERAPH_EMBEDDING_MODEL`
- Phase 3 uses LLM commands and LLM environment variables such as `SERAPH_LLM_BASE_URL`, `SERAPH_LLM_API_KEY`, and `SERAPH_LLM_MODEL`
- Legacy `SERAPH_EMBEDDER` remains accepted as a backward-compatible alias for `SERAPH_EMBEDDING_BACKEND`

Phase 2 currently uses the `openai_compatible` embedding backend through:

- `SERAPH_EMBEDDING_BASE_URL`
- `SERAPH_EMBEDDING_MODEL`
- optional `SERAPH_EMBEDDING_API_KEY`
- optional `SERAPH_EMBEDDING_API_PATH`, `SERAPH_EMBEDDING_TIMEOUT_SECONDS`

If `SERAPH_EMBEDDING_BACKEND` is unset, SERAPH defaults it to `openai_compatible`.

Examples:

```bash
cargo run -p seraph-cli -- phase2 index \
  --knowledge workspace/knowledge.json \
  --vectordb workspace/vectordb

cargo run -p seraph-cli -- phase3 harness-prompt \
  --context workspace/contexts/rag_target_001.md \
  --output workspace/prompts/harness_prompt_001.json \
  --variants 3 \
  --style aflpp

cargo run -p seraph-cli -- phase3 compile-check \
  --harness-glob 'workspace/fuzz/harness_001_*.rs' \
  --report-dir workspace/reports \
  --round 1

cargo run -p seraph-cli -- phase3 smoke-run \
  --compile-index workspace/reports/compile_001_index.json \
  --report-dir workspace/reports \
  --round 1 \
  --command-template 'python3 scripts/fake_runtime.py {harness}'

cargo run -p seraph-cli -- phase3 merge-harnesses \
  --workspace-dir workspace \
  --round 1

cargo run -p seraph-cli -- phase3 runtime-diagnose \
  --context workspace/contexts/rag_target_001.md \
  --smoke-index workspace/reports/smoke_001_index.json \
  --output-dir workspace/reports \
  --round 1 \
  --command-template 'python3 scripts/fake_runtime_diagnose.py --input {input} --output {output}'

cargo run -p seraph-cli -- phase3 fix-loop \
  --request workspace/fixes/fix_request_001_01.json \
  --responses-dir workspace/fixes \
  --output-dir workspace/fuzz \
  --report-dir workspace/reports \
  --max-attempts 3

cargo run -p seraph-cli -- phase3 model-response \
  --input workspace/prompts/harness_prompt_001.json \
  --output workspace/prompts/llm_response_001.md \
  --command-template 'python3 scripts/fake_model.py --input {input}'

cargo run -p seraph-cli -- phase3 afl-bootstrap \
  --workspace-dir /tmp/seraph-phase3-real-aflpp-localresp/arrayvec \
  --merge-report /tmp/seraph-phase3-real-aflpp-localresp/arrayvec/reports/merge_arrayvec.json \
  --dry-run

cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1 \
  --phase3-style aflpp \
  --model-command 'python3 scripts/fake_model.py --input {input}' \
  --fix-loop \
  --fix-max-attempts 2
```

Use `--dry-run` to print the delegated command without executing it.

Phase 2 target selection rules:

- `seraph-cli run` now iterates all ranked unsafe targets by default.
- `--round N` is the batch start index, so `--round 4` starts at the 4th ranked target and continues through the remaining targets.
- `--target-api-id <api_id>` switches back to single-target mode and retrieves context for that exact unsafe target.
- `--llm-response <path>` is single-target only, so pair it with `--target-api-id`.
- `--afl-bootstrap` is also single-target only, and unified runs require `--smoke-command` so SERAPH can merge only passing cases.
- Retrieved RAG context now includes a `Required Setup APIs` section before exploratory related APIs so deep targets keep their setup chain in view.

The active `aflpp` style asks the model for `pub fn run_case(input: &[u8])` case modules, not final binaries. SERAPH wraps those cases in one-shot executables for compile/smoke screening, then merges passing cases into a crate-level AFL++ target with a tool-generated `afl::fuzz!` entrypoint.

For smoke-successful Phase 3 workspaces, `phase3 merge-harnesses` selects the passing cases, synthesizes a merged Cargo target, and writes selector-safe default seeds under `workspace/afl/<target_name>/corpus`. `phase3 afl-bootstrap` then forwards that merged target to `scripts/bootstrap-fuzz-target.sh`, which consumes the merge report's `default_corpus_dir` automatically, builds regular, ASan, and CmpLog variants, and renders or launches the paired AFL++ campaign. Passing `--afl-corpus-dir` still overrides that default when you want a manual corpus path.

When Phase 3 compile, smoke, or fix steps run, `seraph-cli run` also writes `workspace/coverage.json`.

Runtime smoke can be enabled from the unified run entry:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1 \
  --target-api-id api::your_crate::Type::unsafe_target \
  --llm-response workspace/prompts/llm_response_001.md \
  --compile-check \
  --smoke-command 'python3 scripts/fake_runtime.py {harness}'
```

`--smoke-command` automatically enables compile-check. The smoke stage reads successful harnesses from `compile_RRR_index.json` and, when `--fix-loop` is enabled, also includes repaired harnesses that compiled successfully in `fix_loop_RRR_index.json`.

When `--smoke-command` is enabled, `run` executes smoke for every compile-successful harness. If runtime diagnosis is configured, it also writes `runtime_error_RRR_index.json`. Coverage still treats compile-successful, smoke-reaching harnesses as covered.

When `--afl-bootstrap` is enabled, `run` also executes `merge-harnesses` after smoke-run, then boots a merged AFL++ campaign from the passing cases.

If you rerun the same target in the same workspace, `coverage.json` now follows the latest Phase 3 result for that target. Older `validated`, `bug`, and stale harness entries for that target are replaced instead of accumulated.

Runtime diagnosis command selection follows this fallback order:

- `--runtime-model-command`
- `--fix-model-command`
- `--model-command`

Third-party OpenAI-compatible gateways can reuse:

- `scripts/third_party_openai_compatible.py`

Required environment variables:

- `SERAPH_LLM_BASE_URL`
- `SERAPH_LLM_MODEL`

Optional environment variables:

- `SERAPH_LLM_WIRE_API` with `responses` for Responses API gateways
- `SERAPH_LLM_API_KEY` for gateways that require bearer auth

Recommended unified run wiring:

```bash
export SERAPH_LLM_BASE_URL='http://127.0.0.1:8080'
export SERAPH_LLM_MODEL='gpt-5.4'
export SERAPH_LLM_WIRE_API='responses'

cargo run -p seraph-cli -- run \
  --manifest-path /path/to/Cargo.toml \
  --workspace-dir workspace \
  --round 1 \
  --phase3-style aflpp \
  --model-command 'python3 scripts/third_party_openai_compatible.py --input {input} --output {output}' \
  --compile-check \
  --fix-loop \
  --fix-model-command 'python3 scripts/third_party_openai_compatible.py --input {input} --output {output}' \
  --smoke-command 'python3 scripts/your_smoke_runner.py {harness}' \
  --runtime-model-command 'python3 scripts/third_party_openai_compatible.py --input {input} --output {output}'
```

`phase3 fix-acceptance-write` and `phase3 fix-acceptance-write-batch` remain available as compatibility-only manual override commands for older review artifacts.

For real crate compile/smoke execution, the unified CLI can reuse:

- `scripts/phase3_real_crate.py`

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
  --smoke-command 'python3 scripts/phase3_real_crate.py smoke {harness}' \
  --afl-bootstrap \
  --afl-build-only
```

Compatibility examples:

```bash
cargo run -p seraph-cli -- phase3 fix-acceptance-write \
  --index workspace/reports/fix_acceptance_001_index.json \
  --harness workspace/fuzz/harness_001_01_fixed_01.rs \
  --status accepted \
  --reason manual_triage_ok \
  --coverage workspace/coverage.json \
  --context workspace/contexts/rag_target_001.md

cargo run -p seraph-cli -- phase3 fix-acceptance-write-batch \
  --index workspace/reports/fix_acceptance_001_index.json \
  --decisions workspace/reports/fix_acceptance_001_decisions.json \
  --coverage workspace/coverage.json \
  --context workspace/contexts/rag_target_001.md
```

If you also pass `--coverage` and `--context`, the same command immediately folds the updated review result back into `coverage.json`. This sync mode is intentionally narrow: it replays acceptance review state only, and assumes the target already has a coverage entry from earlier compile/fix stages.

For reviewer batches, `phase3 fix-acceptance-write-batch` applies multiple decisions from one JSON file. The file may be either a top-level list or an object with a `decisions` list:

```json
{
  "decisions": [
    {
      "harness": "workspace/fuzz/harness_001_01_fixed_01.rs",
      "status": "accepted",
      "reason": "manual_triage_ok"
    },
    {
      "harness": "workspace/fuzz/harness_001_02_fixed_01.rs",
      "status": "bug",
      "reason": "manual_bug_confirmed",
      "source": "reviewer:alice"
    }
  ]
}
```

Full unified run:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1 \
  --phase3-prompt
```
