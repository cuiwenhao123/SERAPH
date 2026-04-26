# Phase 3 Harness Generation Flow

This document records the first executable Phase 3 steps after the Phase 2 RAG reset.

## Scope

Phase 3 starts from a retrieved RAG context and prepares a deterministic LLM prompt bundle for harness generation. It does not reintroduce the retired scenario/mapping/planning artifacts.

Phase 3 reads LLM configuration and LLM command templates only. It does not use the Phase 2 embedding-model configuration except indirectly through the already-generated RAG context.

Active target scope remains unchanged:

- unsafe APIs
- safe APIs that contain unsafe blocks

## Inputs and Outputs

| Step | Command | Input | Output |
|------|---------|-------|--------|
| RAG context | `seraph-cli phase2 retrieve` | `knowledge.json`, `graph.pkl`, `vectordb/` | `workspace/contexts/rag_target_RRR.md` |
| Prompt bundle | `seraph-cli phase3 harness-prompt` | `rag_target_RRR.md` | `workspace/prompts/harness_prompt_RRR.json` |
| Model response | `seraph-cli phase3 model-response` | prompt bundle or fix request, provider command | `workspace/prompts/llm_response_RRR.md` or `workspace/fixes/fix_response_RRR_SS_AA.md` |
| Harness writer | `seraph-cli phase3 harness-write` | `harness_prompt_RRR.json`, LLM markdown response | `workspace/fuzz/harness_RRR_SS.rs` |
| Compile check | `seraph-cli phase3 compile-check` | generated harness files | `workspace/reports/compile_RRR_SS.json`, `compile_RRR_index.json` |
| Smoke run | `seraph-cli phase3 smoke-run` | successful compile/fix-loop harnesses | `workspace/reports/smoke_RRR_SS*.json`, `smoke_RRR_index.json` |
| Runtime diagnose | `seraph-cli phase3 runtime-diagnose` | failed smoke reports plus target context | `workspace/reports/runtime_error_RRR_SS*.json`, `runtime_error_RRR_index.json` |
| Fixer bundle | `seraph-cli phase3 fixer-bundle` | compile index, RAG context, failing harnesses | `workspace/fixes/fix_request_RRR_SS.json` |
| Fix loop | `seraph-cli phase3 fix-loop` | `fix_request_RRR_SS.json`, ordered fix responses | `workspace/fuzz/harness_RRR_SS_fixed_AA.rs`, `workspace/reports/compile_RRR_SS_fixed_AA.json`, `workspace/reports/fix_loop_RRR_SS.json` |
| Fix loop batch | `seraph-cli phase3 fix-loop-batch` | `fix_request_RRR_*.json`, ordered fix responses | `workspace/reports/fix_loop_RRR_index.json` plus per-request fix-loop reports |

`fix-acceptance*` commands remain available only for backward compatibility. They are no longer part of the active unified `run` flow.

The prompt bundle contains:

- `version`: prompt schema version
- `target_api_id`: exact stable target API ID parsed from the RAG context
- `style`: active harness generation style; currently `aflpp`
- `variants`: requested harness variant count
- `system_prompt`: stable Phase 3 generation rules
- `user_prompt`: RAG context plus task-specific generation request

The active Phase 3 prompt contract is now fact-grounded rather than rule-heavy.

The retrieved markdown context is organized around:

- `Crate Facts`
- `Target API`
- `Known Reachable Paths`
- `Compile-Time Facts`
- `Related APIs`
- `Variant Opportunities`
- `Similar API Usage`
- `Rust Idioms`

`Known Reachable Paths` are fact-grounded reachability/setup hints surfaced from the current SERAPH context. They may be partial, they are not mandatory scripts, and they are not the only allowed sequence. The model may construct a different harness sequence as long as every API, import path, trait fact, and setup assumption is explicitly supported by the context.

## Active Harness Style

The active Phase 3 harness style is `aflpp`.

This does **not** mean SERAPH asks the model to emit `afl::fuzz!` macro targets. Instead, the active contract asks for plain Rust binary harnesses that:

- define a normal `fn main()`
- read fuzz bytes from stdin or an optional input file path argument
- use only the Rust standard library for input ingestion
- preserve SERAPH markers around the target API call
- avoid `libfuzzer_sys`, `#![no_main]`, `fuzz_target!`, and `afl::fuzz!`

This shape keeps the current compile-check, fix-loop, and smoke-run pipeline unchanged while making the generated program directly usable with AFL++ stdin or `@@` file workflows.

## AFL++ Launch

After Phase 3 has produced a compile-successful harness workspace, use:

- `scripts/bootstrap-fuzz-target.sh`

The script reuses the generated `_cargo_projects/<harness_stem>/Cargo.toml`, copies the latest harness source into `src/main.rs`, builds an AFL++-instrumented binary with `cargo afl build`, then launches `afl-fuzz` against either:

- `@@` file mode by default
- stdin mode when `--input-mode stdin` is requested

Example:

```bash
bash scripts/bootstrap-fuzz-target.sh \
  --workspace-dir /tmp/seraph-phase3-real-aflpp-localresp/arrayvec \
  --harness fuzz/harness_001_01.rs
```

Dry-run example:

```bash
bash scripts/bootstrap-fuzz-target.sh \
  --workspace-dir /tmp/seraph-phase3-real-aflpp-localresp/arrayvec \
  --harness fuzz/harness_001_01.rs \
  --dry-run
```

Prerequisites:

- a compile-successful Phase 3 workspace with `_cargo_projects/`
- `cargo afl` available on `PATH`
- `afl-fuzz` available on `PATH`

## Model Provider Adapter

SERAPH now uses a provider-agnostic command adapter for model inference. Instead of hard-coding one vendor SDK, Phase 3 shells out through a command template with these placeholders:

- `{input}`: shell-quoted input file path
- `{output}`: shell-quoted output file path
- `{attempt}`: fix attempt number for repair loops

If the command template does not contain `{output}`, SERAPH captures stdout and writes it to the output file.

SERAPH also ships a reusable third-party OpenAI-compatible template script:

- `scripts/third_party_openai_compatible.py`

This script is intended for providers that expose a `/chat/completions` style API. It reads the same SERAPH JSON request files used by:

- initial harness generation
- compile-fix loop requests
- runtime diagnosis requests

Configuration is environment-driven:

- required: `SERAPH_LLM_BASE_URL`, `SERAPH_LLM_MODEL`
- optional auth: `SERAPH_LLM_API_KEY`
- optional transport: `SERAPH_LLM_WIRE_API`, `SERAPH_LLM_API_PATH`
- optional tuning: `SERAPH_LLM_TEMPERATURE`, `SERAPH_LLM_TIMEOUT_SECONDS`
- optional provider overrides: `SERAPH_LLM_EXTRA_HEADERS`, `SERAPH_LLM_EXTRA_BODY`

Recommended command template:

```bash
python3 scripts/third_party_openai_compatible.py --input {input} --output {output}
```

Example local Responses API wiring:

```bash
export SERAPH_LLM_BASE_URL='http://127.0.0.1:8080'
export SERAPH_LLM_MODEL='gpt-5.4'
export SERAPH_LLM_WIRE_API='responses'
```

Configuration boundary:

- Phase 2 / RAG embedding side: `SERAPH_EMBEDDING_BACKEND`, `SERAPH_EMBEDDING_BASE_URL`, `SERAPH_EMBEDDING_MODEL`
- Phase 3 / LLM side: `SERAPH_LLM_BASE_URL`, `SERAPH_LLM_API_KEY`, `SERAPH_LLM_MODEL`
- Legacy `SERAPH_EMBEDDER` remains accepted only as an alias for the Phase 2 embedding backend name

Load the embedding + LLM env file before unified `run` examples:

```bash
set -a && source configs/environments/.env.seraph-local && set +a
```

## Prompt Bundle CLI Usage

Generate a prompt bundle from an existing RAG context:

```bash
cargo run -p seraph-cli -- phase3 harness-prompt \
  --context workspace/contexts/rag_target_001.md \
  --output workspace/prompts/harness_prompt_001.json \
  --variants 3 \
  --style aflpp
```

Or ask the unified CLI to create it after Phase 1/2 retrieval:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1 \
  --phase3-prompt \
  --phase3-style aflpp \
  --variants 3
```


## Model Response CLI Usage

Generate one response file from a prompt bundle or fix request:

```bash
cargo run -p seraph-cli -- phase3 model-response \
  --input workspace/prompts/harness_prompt_001.json \
  --output workspace/prompts/llm_response_001.md \
  --command-template 'python3 scripts/fake_model.py --input {input}'
```

The response may be fenced Rust markdown or plain Rust source. For harness generation, the writer accepts either form. For fixer generation, `fix-loop` and `fix-loop-batch` can call this adapter lazily when a response file for an attempt is still missing.


## Harness Writer CLI Usage

After an LLM returns markdown or plain Rust source, split the response into harness files:

```bash
cargo run -p seraph-cli -- phase3 harness-write \
  --prompt workspace/prompts/harness_prompt_001.json \
  --response workspace/prompts/llm_response_001.md \
  --output-dir workspace/fuzz \
  --round 1
```

The writer extracts fenced Rust code blocks. If no code fence exists, it treats the full response as one Rust source file. Each generated harness must contain both `SERAPH_STEP_ENTER` and `SERAPH_STEP_OK` markers for the prompt bundle's `target_api_id`; otherwise the writer rejects the response.

Under the active `aflpp` style, the expected source shape is a normal Rust binary, not a libFuzzer- or macro-based fuzz target.

The unified CLI can run retrieval, prompt creation, and response splitting together:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1 \
  --model-command 'python3 scripts/fake_model.py --input {input}'
```


## Compile Check CLI Usage

Run a configurable compile command and store stdout/stderr diagnostics as JSON for one harness:

```bash
cargo run -p seraph-cli -- phase3 compile-check \
  --harness workspace/fuzz/harness_001_01.rs \
  --report workspace/reports/compile_001_01.json \
  --command-template 'rustc --edition=2021 --crate-type bin {harness}'
```

Each per-harness report contains `status`, `exit_code`, `stdout`, `stderr`, the rendered command, and the harness path. The `{harness}` placeholder is shell-quoted before command execution. In a full cargo-fuzz integration, replace the command template with a workspace-specific build command that copies or references the generated harness.

The CLI can also check all generated variants and write `compile_RRR_index.json`:

```bash
cargo run -p seraph-cli -- phase3 compile-check \
  --harness-glob 'workspace/fuzz/harness_001_*.rs' \
  --report-dir workspace/reports \
  --round 1 \
  --command-template 'rustc --edition=2021 --crate-type bin {harness}'
```

The unified CLI runs compile-check across all generated variants:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1 \
  --llm-response workspace/prompts/llm_response_001.md \
  --compile-check \
  --compile-command 'rustc --edition=2021 --crate-type bin {harness}'
```


## Smoke Run Usage

After compile-check succeeds for at least one harness, `smoke-run` executes a runtime command and writes JSON runtime results:

```bash
cargo run -p seraph-cli -- phase3 smoke-run \
  --compile-index workspace/reports/compile_001_index.json \
  --report-dir workspace/reports \
  --round 1 \
  --command-template 'python3 scripts/fake_runtime.py {harness}'
```

If `fix-loop` produced repaired harnesses, pass the batch index too:

```bash
cargo run -p seraph-cli -- phase3 smoke-run \
  --compile-index workspace/reports/compile_001_index.json \
  --fix-loop-index workspace/reports/fix_loop_001_index.json \
  --report-dir workspace/reports \
  --round 1 \
  --command-template 'python3 scripts/fake_runtime.py {harness}'
```

Runtime status is currently classified as:

- `ok`: command exited successfully and emitted no runtime-review markers
- `bug`: command exited non-zero, or emitted `SERAPH_FOUND_BUG: ...`
- `review`: command emitted `SERAPH_NEEDS_REVIEW: ...`

`smoke-run` only selects harnesses that compiled successfully. Failed original variants and failed repairs are ignored for runtime execution.


## Runtime Diagnose Usage

When smoke reports a runtime failure, `runtime-diagnose` asks an LLM-compatible command to summarize the issue and writes a dedicated runtime knowledge artifact:

```bash
cargo run -p seraph-cli -- phase3 runtime-diagnose \
  --context workspace/contexts/rag_target_001.md \
  --smoke-index workspace/reports/smoke_001_index.json \
  --output-dir workspace/reports \
  --round 1 \
  --command-template 'python3 scripts/fake_runtime_diagnose.py --input {input} --output {output}'
```

The active runtime classes are:

- `asan_bug`
- `panic_or_crash`
- `invalid_input_or_precondition`
- `other_runtime_issue`

Non-ASan runtime errors still count as covered once the harness compiled and reached smoke-run.


## Compile-Fixer Bundle Usage

Create repair requests for failed compile reports:

```bash
cargo run -p seraph-cli -- phase3 fixer-bundle \
  --compile-index workspace/reports/compile_001_index.json \
  --context workspace/contexts/rag_target_001.md \
  --output-dir workspace/fixes
```

Each `fix_request_RRR_SS.json` includes the RAG context, original harness source, rustc diagnostics, target API id, and non-removal rules for SERAPH markers and the target API call. Successful compile reports do not produce fixer requests.

The unified CLI can generate fixer requests after compile-check:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1 \
  --llm-response workspace/prompts/llm_response_001.md \
  --fixer-bundle
```


## Compile-Fixer Response Usage

After an LLM or human repair produces a markdown/plain Rust response for a `fix_request_RRR_SS.json`, write the repaired harness without overwriting the original:

```bash
cargo run -p seraph-cli -- phase3 fixer-write \
  --request workspace/fixes/fix_request_001_01.json \
  --response workspace/fixes/fix_response_001_01.md \
  --output-dir workspace/fuzz \
  --attempt 1
```

This writes:

```text
workspace/fuzz/harness_001_01_fixed_01.rs
```

Then run compile-check again against repaired harnesses:

```bash
cargo run -p seraph-cli -- phase3 compile-check \
  --harness-glob 'workspace/fuzz/harness_001_01_fixed_*.rs' \
  --report-dir workspace/reports \
  --round 1 \
  --command-template 'rustc --edition=2021 --crate-type bin {harness}'
```

For multi-attempt repair, the ordered response filenames are:

```text
workspace/fixes/fix_response_RRR_SS_01.md
workspace/fixes/fix_response_RRR_SS_02.md
workspace/fixes/fix_response_RRR_SS_03.md
```

`attempt 1` also accepts the legacy single-response name `fix_response_RRR_SS.md` for compatibility.


## Fix Once Usage

`fix-once` combines `fixer-write` and compile-check for one repair attempt:

```bash
cargo run -p seraph-cli -- phase3 fix-once \
  --request workspace/fixes/fix_request_001_01.json \
  --response workspace/fixes/fix_response_001_01.md \
  --output-dir workspace/fuzz \
  --report-dir workspace/reports \
  --attempt 1 \
  --command-template 'rustc --edition=2021 --crate-type bin {harness}'
```

This writes both:

```text
workspace/fuzz/harness_001_01_fixed_01.rs
workspace/reports/compile_001_01_fixed_01.json
```


## Fix Loop Usage

`fix-loop` consumes repair responses in attempt order and stops as soon as one repaired harness compiles successfully:

```bash
cargo run -p seraph-cli -- phase3 fix-loop \
  --request workspace/fixes/fix_request_001_01.json \
  --responses-dir workspace/fixes \
  --output-dir workspace/fuzz \
  --report-dir workspace/reports \
  --max-attempts 3 \
  --command-template 'rustc --edition=2021 --crate-type bin {harness}' \
  --response-command-template 'python3 scripts/fake_fixer.py --input {input} --attempt {attempt}'
```

This writes per-attempt artifacts:

```text
workspace/fuzz/harness_001_01_fixed_01.rs
workspace/reports/compile_001_01_fixed_01.json
workspace/fuzz/harness_001_01_fixed_02.rs
workspace/reports/compile_001_01_fixed_02.json
```

And one loop-level summary:

```text
workspace/reports/fix_loop_001_01.json
```

The summary records:

- ordered attempts consumed
- each response/harness/report path
- compile status and exit code for each attempt
- `successful_attempt` when a repair compiles
- `stop_reason` as `compiled`, `missing_response`, or `max_attempts_exhausted`


## Fix Loop Batch Usage

`fix-loop-batch` runs `fix-loop` over every failed request for one round and writes an index summary:

```bash
cargo run -p seraph-cli -- phase3 fix-loop-batch \
  --request-glob 'workspace/fixes/fix_request_001_*.json' \
  --responses-dir workspace/fixes \
  --output-dir workspace/fuzz \
  --report-dir workspace/reports \
  --index-output workspace/reports/fix_loop_001_index.json \
  --max-attempts 3 \
  --command-template 'rustc --edition=2021 --crate-type bin {harness}' \
  --response-command-template 'python3 scripts/fake_fixer.py --input {input} --attempt {attempt}'
```

The batch index records:

- matched request count
- successful request count
- failed request count
- one summary row per request, including `stop_reason` and `successful_attempt`


## Compatibility-Only Acceptance Usage

`fix-acceptance` and its write helpers are kept only for compatibility with older review-driven workflows. The unified `run` command no longer calls them.

`fix-acceptance` evaluates only the repaired harnesses that actually compiled successfully in `fix-loop`:

```bash
cargo run -p seraph-cli -- phase3 fix-acceptance \
  --fix-loop-index workspace/reports/fix_loop_001_index.json \
  --smoke-index workspace/reports/smoke_001_index.json \
  --output workspace/reports/fix_acceptance_001_index.json
```

The current acceptance status is intentionally simple:

- `accepted`: repaired harness compiled and smoke status is `ok`
- `needs_review`: smoke status is `review`, or smoke output is missing
- `bug`: smoke status is `bug`

This produces one round-level index for successful repaired harnesses only. It is suitable as an auto-generated review queue and can also be amended manually later if a reviewer wants to override an entry status or reason.


## Fix Acceptance Write Usage

Reviewers can amend one acceptance entry without hand-editing JSON:

```bash
cargo run -p seraph-cli -- phase3 fix-acceptance-write \
  --index workspace/reports/fix_acceptance_001_index.json \
  --harness workspace/fuzz/harness_001_01_fixed_01.rs \
  --status accepted \
  --reason manual_triage_ok \
  --coverage workspace/coverage.json \
  --context workspace/contexts/rag_target_001.md
```

Or apply a batch of reviewer decisions at once:

```bash
cargo run -p seraph-cli -- phase3 fix-acceptance-write-batch \
  --index workspace/reports/fix_acceptance_001_index.json \
  --decisions workspace/reports/fix_acceptance_001_decisions.json \
  --coverage workspace/coverage.json \
  --context workspace/contexts/rag_target_001.md
```

The decisions file may be either a top-level list or an object with a `decisions` list. Each decision contains:

- `harness`: exact harness path already present in the acceptance index
- `status`: `accepted`, `needs_review`, or `bug`
- `reason`: reviewer rationale
- optional `source`: reviewer/source label; defaults to `manual`

Current write-back scope stays intentionally narrow:

- updates one existing acceptance entry matched by exact harness path
- rewrites index-level `status` / `entry_count`
- preserves compile/smoke paths and runtime facts already present in the entry

By default these write commands only update the acceptance index itself. If `--coverage` and `--context` are provided, they also immediately re-ingest that acceptance index into `coverage.json` for the same target. This sync path stays intentionally narrow: it replays acceptance review state only, and expects earlier compile/fix stages to have already established the target's coverage record.


## Unified Run With Fix Loop

The unified CLI can now continue from generated harnesses into compile-check, fixer request generation, and batch fix-loop execution:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1 \
  --model-command 'python3 scripts/fake_model.py --input {input}' \
  --fix-loop \
  --fix-max-attempts 3 \
  --fix-model-command 'python3 scripts/fake_fixer.py --input {input} --attempt {attempt}' \
  --compile-command 'rustc --edition=2021 --crate-type bin {harness}'
```

When `--fix-loop` is enabled, `run` also enables:

- compile-check
- fixer-bundle

When `--smoke-command` is enabled, `run` also enables:

- compile-check

When `--smoke-command` is enabled and a runtime diagnosis command is available, `run` also writes:

- `workspace/reports/runtime_error_RRR_index.json`

By default:

- generated initial model response is written to `workspace/prompts/llm_response_RRR.md`
- fix requests and fix responses are both read from `workspace/fixes`
- run writes or updates `workspace/coverage.json`

Override the repair response location with `--fix-responses-dir <path>` if repair responses are stored elsewhere.

After compile-check or fix-loop completes, `run` updates `coverage.json` for the current target:

- compile success marks the target `validated`
- compile failure without fix-loop marks the target `attempted`
- failed fix-loop marks the target `attempted` and `exhausted`
- compile-successful harnesses that reach smoke remain `validated`
- runtime diagnosis with `asan_bug` adds the target API id to `found_bugs`
- non-ASan runtime issues are recorded in `runtime_error.json` and do not block `validated`
- if the same target is run again later, the latest Phase 3 result overwrites that target's old coverage state instead of accumulating stale `validated` / `bug` / harness entries

The current coverage update remains target-centric for the active round. It now records minimal runtime findings, and it also maintains a second, backward-compatible related-API coverage track:

- target coverage continues to use `total_api_ids`, `covered_api_ids`, `uncovered_api_ids`, and `coverage_rate`
- related static coverage uses `related_total_api_ids`, `related_covered_api_ids`, `related_uncovered_api_ids`, and `related_coverage_rate`
- this related track is intentionally heuristic: the active prompt contract now surfaces `Known Reachable Paths`, while coverage compatibility still accepts legacy `Required Setup APIs` during rollout. The tracker considers APIs listed in the same round's `Known Reachable Paths`, legacy `Required Setup APIs`, and `Related APIs`, then marks a related API as covered when a compile-successful harness source contains a static call-shaped use of that API

This keeps historical target coverage semantics stable while finally giving the system a way to measure neighborhood API combination coverage.

`coverage.json` now also stores minimal per-harness structure under `harnesses`:

- key `round:sub_index` for original generated harnesses
- key `round:sub_index:fixed:attempt` for repaired harness attempts
- optional `target_api_id` for explicit harness ownership during reruns
- compile-derived `status`
- `api_ids` now stores the target API plus statically detected related/setup APIs used by that harness
- optional `harness_path`, `compile_report_path`, `smoke_report_path`
- optional `runtime_status`, `runtime_classification`, `runtime_error_report_path`, `runtime_error_summary`
- optional `review_status`, `review_reason`, `review_report_path`

`coverage.json` also keeps `related_api_ids_by_target` so rerunning one target can refresh that target's related-context contribution instead of only growing a global union forever.

This is deliberately lightweight and backward-compatible. Target-level `found_bugs` / `needs_review` remain as string lists for orchestration and prioritization.

## LLM Codegen Contract

The LLM receives the prompt bundle and must generate 2-3 Rust harness variants by default. Each variant must:

- call the target API from the RAG context
- preserve exact `api_id` strings in `SERAPH_STEP_ENTER` / `SERAPH_STEP_OK` markers
- use public setup APIs from the related API section when possible
- keep unsafe blocks narrow
- use early return for recoverable `Result` and `Option` paths
- use the real crate import name from the RAG context, not `target_lib`

## Next Phase 3 Work

The next executable increments are:

1. enrich coverage harness records without reviving retired scenario/mapping artifacts
2. add provider-specific helper scripts/templates for real deployments
3. decide whether runtime markers should graduate from string lists to structured findings
4. decide whether compatibility-only review helpers should stay in-tree or move behind a legacy subcommand

## Unified Run Entry

Use the single unified run entry:

```bash
cargo run -p seraph-cli -- run \
  --knowledge workspace/knowledge.json \
  --workspace-dir workspace \
  --round 1
```

The Python `seraph_rag.cli` commands remain internal implementation entrypoints for development and debugging.
