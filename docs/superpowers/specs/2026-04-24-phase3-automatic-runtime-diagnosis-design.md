# SERAPH Phase 3 Automatic Compile-Fix + Runtime Diagnosis Design

## Background

The current executable Phase 3 path already supports:

- RAG context retrieval
- prompt bundle generation
- harness generation
- compile-check
- compile-fix loop
- smoke-run
- coverage update

But its active flow still carries a manual-review/acceptance branch. That no longer matches the intended operating model.

The new Phase 3 goal is a fully automatic loop:

1. generate harnesses for an unsafe target
2. repair compile failures with LLM assistance
3. run smoke after successful compile
4. diagnose runtime failures with LLM assistance
5. record runtime findings into a separate runtime knowledge artifact
6. still count compile-successful harnesses as covered once they reach runtime execution

This document defines the active Phase 3 design that replaces the acceptance/review-driven path.

## Goals

- Remove manual review from the active Phase 3 path.
- Keep compile repair automated, bounded, and deterministic.
- Keep runtime diagnosis automated, but do not auto-repair runtime issues yet.
- Record runtime findings in a dedicated artifact: `runtime_error.json`.
- Treat `compile success + entered smoke-run` as sufficient for coverage.
- Keep the design simple and backward-compatible where possible.

## Non-Goals

- Do not auto-fix runtime failures in this iteration.
- Do not redesign Phase 1 extraction or Phase 2 retrieval architecture.
- Do not replace `coverage.json` with a new top-level multi-API coverage model yet.
- Do not keep acceptance/review as part of the active orchestration path.

## Active Phase 3 Flow

The active Phase 3 loop becomes:

1. `phase2 retrieve`
2. `phase3 harness-prompt`
3. `phase3 model-response`
4. `phase3 harness-write`
5. `phase3 compile-check`
6. if compile fails, generate compile-fix request
7. `phase3 fix-loop` with at most 2 repair attempts
8. once any harness variant compiles successfully, run `phase3 smoke-run`
9. if smoke fails, run `phase3 runtime-diagnose`
10. write/update `coverage.json`

There is no active manual review gate in this flow.

## Compile Repair Semantics

Compile repair stays bounded and deterministic:

- Original generated harness is checked first.
- If compile fails, the system creates a compile-fix request containing:
  - failing harness source
  - rustc stderr/stdout
  - target API id
  - relevant context material
- LLM repairs are attempted in order.
- Maximum compile repair attempts: `2`.
- If all repair attempts fail:
  - the harness is dropped
  - the failure is recorded
  - the harness does not enter smoke-run
  - the harness does not contribute validated coverage

`fix-loop` remains responsible only for making the harness compile.

## Runtime Diagnosis Semantics

Smoke-run remains required after compile success because many Rust issues appear only at runtime:

- panics
- invalid fuzzed inputs / violated preconditions
- sanitizer failures
- other runtime execution failures

When smoke fails:

1. the harness is not auto-repaired
2. a runtime diagnosis request is built from:
   - target API id
   - harness source
   - compile success fact
   - smoke stdout
   - smoke stderr
   - smoke exit code
3. LLM summarizes what kind of runtime issue occurred
4. the summary is written into `runtime_error.json`

The diagnosis step is classification and recording only. It does not produce a new harness revision.

## Runtime Error Knowledge Artifact

Runtime findings are recorded separately from `knowledge.json`.

### Artifact Locations

- round-level index:
  - `workspace/reports/runtime_error_RRR_index.json`
- per-harness detail:
  - `workspace/reports/runtime_error_RRR_SS.json`
  - or repaired-harness detail:
  - `workspace/reports/runtime_error_RRR_SS_fixed_AA.json`

This artifact is a Phase 3 runtime knowledge product, not a Phase 1 extraction product.

### Runtime Error Record Fields

Each runtime error record should include at least:

- `version`
- `round`
- `variant`
- `attempt`
- `target_api_id`
- `harness`
- `compile_report`
- `smoke_report`
- `status`
- `classification`
- `summary`
- `evidence`
- `bug`

Field intent:

- `status`
  - `runtime_error`
  - `bug`
- `classification`
  - `asan_bug`
  - `panic_or_crash`
  - `invalid_input_or_precondition`
  - `other_runtime_issue`
- `summary`
  - short LLM diagnosis
- `evidence`
  - compact raw evidence derived from smoke report
- `bug`
  - `true` only for sanitizer/ASan-class failures

### LLM Runtime Diagnosis Contract

The runtime diagnosis model should be asked for structured JSON output. The minimal contract is:

```json
{
  "classification": "panic_or_crash",
  "summary": "The harness reached the target API and then panicked because the runtime path violated a library precondition.",
  "is_bug": false
}
```

The orchestrator remains authoritative for file writes and final artifact structure.

## Coverage Semantics

Coverage semantics change in one important way:

> If a harness compiles successfully and is executed by smoke-run, it counts as covered even if smoke reports a runtime problem.

This is intentional. Compile-successful runtime-reaching harnesses demonstrate meaningful target reachability and are valuable even when they expose runtime faults.

### Target-Level State

Target-level state remains:

- `targeted`
- `attempted`
- `validated`

Recommended transition rules:

- no compile-successful harness yet:
  - target remains `attempted`
- all compile-fix attempts exhausted:
  - target remains `attempted`
  - target may enter `exhausted`
- at least one harness compiles and enters smoke-run:
  - target becomes `validated`

### Runtime Finding Interaction

- smoke failure does not revoke validation
- runtime diagnosis with `asan_bug` additionally records the target in `found_bugs`
- non-ASan runtime issues do not become bugs
- manual-review style `needs_review` is no longer part of the active Phase 3 semantics

## Harness-Level Coverage Enrichment

`coverage.harnesses` remains the lightweight place to store per-harness facts.

Existing harness records should be extended or repurposed to carry:

- compile status
- smoke status
- runtime diagnosis classification
- runtime diagnosis summary
- runtime diagnosis report path
- `api_ids`

`api_ids` should continue to contain:

- the target API id
- nearby graph-related APIs
- any additional related helper APIs the harness is known to exercise

This keeps the target-centric top-level coverage model stable while still preserving richer multi-API execution context for later evolution.

## CLI and Orchestration Changes

The only public CLI remains `seraph-cli`.

### Active Public Phase 3 Commands

- keep:
  - `phase3 harness-prompt`
  - `phase3 model-response`
  - `phase3 harness-write`
  - `phase3 compile-check`
  - `phase3 fixer-bundle`
  - `phase3 fix-loop`
  - `phase3 smoke-run`
- add:
  - `phase3 runtime-diagnose`

### Commands Removed From Active Path

These commands become compatibility-only, not active-flow commands:

- `phase3 fix-acceptance`
- `phase3 fix-acceptance-write`
- `phase3 fix-acceptance-write-batch`

They should first be removed from the unified `run` path and active docs. Physical deletion can happen later once no dependency remains.

### Unified `run` Behavior

`seraph-cli run` should orchestrate:

1. Phase 1/2 context preparation
2. harness generation
3. compile-check
4. bounded compile repair
5. smoke-run
6. runtime diagnosis for smoke failures
7. coverage update

### Recommended `run` Parameters

- `--model-command`
  - initial harness generation
- `--compile-command`
  - compile command
- `--fix-model-command`
  - compile repair generation
- `--fix-max-attempts`
  - default should be `2`
- `--smoke-command`
  - runtime execution
- `--runtime-model-command`
  - runtime diagnosis generation

Fallback order for runtime diagnosis command:

1. `--runtime-model-command`
2. `--fix-model-command`
3. `--model-command`

This keeps minimal deployment simple.

## Coverage Input Changes

`coverage.json` should stop using acceptance artifacts as an active input.

Active coverage ingestion inputs become:

- `compile_index`
- `fix_loop_index`
- `smoke_index`
- `runtime_error_index`

Acceptance-derived review state becomes compatibility-only.

## Runtime Issue Classification Rules

Runtime diagnosis results should be interpreted as:

- `asan_bug`
  - record in `runtime_error.json`
  - add target to `found_bugs`
  - keep target `validated`
- `panic_or_crash`
  - record in `runtime_error.json`
  - do not auto-repair
  - keep target `validated`
- `invalid_input_or_precondition`
  - record in `runtime_error.json`
  - do not auto-repair
  - keep target `validated`
- `other_runtime_issue`
  - record in `runtime_error.json`
  - do not auto-repair
  - keep target `validated`

## Recommended Implementation Order

1. add `runtime-diagnose` command and JSON artifact generation
2. update unified `run` to use:
   - compile-check
   - fix-loop with max 2 attempts
   - smoke-run
   - runtime-diagnose
3. update `s3-coverage` to ingest runtime diagnosis instead of acceptance
4. downgrade acceptance commands from active docs and active orchestration
5. add end-to-end tests

## Verification Requirements

The redesign is not complete until these paths are tested:

1. compile fails once, repair succeeds, smoke succeeds
2. compile succeeds, smoke fails with panic/precondition-like runtime issue, diagnosis recorded, target still validated
3. compile succeeds, smoke fails with ASan-like runtime issue, diagnosis recorded, target validated, bug recorded
4. compile fails and both repair attempts fail, harness dropped, target remains attempted/exhausted

## Migration Notes

- keep acceptance-related code temporarily for compatibility
- remove acceptance from active architecture documents
- do not mutate Phase 1 `knowledge.json`
- write runtime findings only to `runtime_error.json`

## Final Design Summary

The active Phase 3 design becomes:

- automatic compile repair with at most 2 attempts
- mandatory smoke-run after compile success
- automatic runtime diagnosis for all smoke failures
- no automatic runtime repair in this iteration
- no manual review in the active loop
- dedicated runtime knowledge artifact: `runtime_error.json`
- coverage counted once a harness compiles and reaches smoke execution

This keeps the system aligned with the desired operating model: a simple automatic loop for generation, repair, runtime diagnosis, and coverage progression around unsafe targets.
