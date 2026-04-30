# SERAPH Selector-Safe Seed Unification Design

## Background

SERAPH's merged-harness fuzzing currently has two different initial-corpus behaviors:

- the normal `scripts/bootstrap-fuzz-target.sh` path creates a single fallback `seed.bin` when the corpus directory is empty
- some experiments prepared `selector_XXXX.bin` files ahead of launch so the merged harness can immediately reach each merged small case

This split is not intentional product behavior. It is an artifact of the current implementation boundary:

- `rag/seraph_rag/merge_harnesses.py` knows exactly which small cases were selected into the merged harness
- `scripts/bootstrap-fuzz-target.sh` only sees a merged Cargo target and an optional corpus directory

As a result, the repository has no mainline, tool-owned step that always converts merged-case knowledge into selector-safe initial seeds.

## Goal

Unify the repository main flow so that every merged harness produced by SERAPH automatically gets selector-safe initial seeds by default.

Concretely:

- every merged harness should get one deterministic selector seed per selected case
- the standard `phase3 merge-harnesses -> phase3 afl-bootstrap` flow should consume those seeds automatically
- the old single fallback `seed.bin` behavior should remain only as a manual or legacy fallback, not the standard merged-harness path

## Non-Goals

- Do not add payload-rich per-case initial corpora in this change.
- Do not change merged harness dispatch semantics.
- Do not remove `--corpus-dir` override support from `bootstrap-fuzz-target.sh`.
- Do not make selector width dynamic in this change.
- Do not auto-replay or mine crash artifacts into the initial corpus.

## Decision

Use merge-time corpus generation as the single mainline source of selector-safe seeds.

Why this approach:

- `merge_harnesses` already owns the authoritative `selected_cases` list
- it can generate seeds without reverse-engineering the merged binary later
- it keeps bootstrap simple: bootstrap consumes merge artifacts instead of inferring them
- it makes case count, selected cases, and generated seeds auditable from one report

Rejected alternative:

- generating selector seeds inside `bootstrap-fuzz-target.sh`
  This would duplicate merge knowledge in the launch script and require bootstrap to infer case layout from a report or source layout it does not currently own.

## Seed Semantics

### Selector width

Selector-safe seeds will use the existing merged-harness selector contract:

- the first 2 bytes are the selector
- the remaining bytes are the payload

This matches the current merged harness dispatcher.

### Seed contents

For a merged harness with `N` selected cases, SERAPH generates exactly `N` safe seeds:

- `selector_0000.bin`
- `selector_0001.bin`
- ...
- `selector_<N-1>.bin`

Each file contains exactly 2 bytes:

- seed `i` stores the big-endian two-byte representation of `i`
- no payload bytes are appended

This means:

- each selected case is reachable from at least one initial seed
- the payload forwarded to `run_case(input)` is initially empty `&[]`
- this change solves case reachability and launch stability, not payload diversity

### Seed count limit

This design keeps the current 2-byte selector format.

If `selected_case_count > 65536`, merge generation must fail with a clear error instead of silently truncating or wrapping seed identities.

This is acceptable for the current SERAPH workflow and dataset sizes.

## Mainline Artifact Layout

Selector-safe seeds become part of the standard merged-harness output.

For merged target `merged_<crate>`, `write_merged_harnesses()` should create:

```text
workspace/
  afl/
    merged_<crate>/
      corpus/
        selector_0000.bin
        selector_0001.bin
        ...
      corpus_meta/
        summary.json
```

This intentionally matches the default AFL campaign root that `bootstrap-fuzz-target.sh` already uses:

- `workspace/afl/<target_name>/corpus`
- `workspace/afl/<target_name>/findings`

That alignment lets bootstrap consume the generated corpus without any extra caller wiring.

## Merge Report Schema Changes

`merge_<crate>.json` should be extended from the current `seraph.phase3.merge_harnesses.v1` shape to a new `v2` shape.

New required fields:

- `default_corpus_dir`
- `default_corpus_meta_dir`
- `default_seed_files`
- `selected_case_count`
- `safe_seed_count`

Example shape:

```json
{
  "version": "seraph.phase3.merge_harnesses.v2",
  "target_name": "merged_fixture",
  "manifest_path": "...",
  "selected_cases": ["..."],
  "selected_case_count": 3,
  "default_corpus_dir": ".../workspace/afl/merged_fixture/corpus",
  "default_corpus_meta_dir": ".../workspace/afl/merged_fixture/corpus_meta",
  "safe_seed_count": 3,
  "default_seed_files": [
    ".../selector_0000.bin",
    ".../selector_0001.bin",
    ".../selector_0002.bin"
  ]
}
```

## Corpus Metadata Summary

`write_merged_harnesses()` should also emit:

- `workspace/afl/<target_name>/corpus_meta/summary.json`

This file should standardize the ad hoc experimental summary shape already used in some batch runs.

Required fields:

- `version`
- `target_name`
- `manifest_path`
- `merge_report`
- `corpus_dir`
- `meta_dir`
- `selected_case_count`
- `safe_seed_count`
- `crash_seed_count`
- `seed_files`

For this change:

- `crash_seed_count` is always `0`

This preserves a future-compatible place to record crash-derived seeds later without redesigning the schema.

## Bootstrap Behavior

`scripts/bootstrap-fuzz-target.sh` should change its default corpus resolution order.

### Default behavior

If the caller does not pass `--corpus-dir`:

1. read `default_corpus_dir` from the merge report
2. use that directory as the AFL input corpus

### Manual override behavior

If the caller passes `--corpus-dir`:

- use the explicit override exactly as requested
- do not silently switch back to the merge-generated corpus

### Empty-directory fallback

If the final resolved corpus directory is empty at bootstrap time:

- keep the existing fallback behavior
- create a single 64-byte zero `seed.bin`

This means:

- standard merged-harness runs use selector-safe seeds by default
- manual custom corpus usage still works
- the script remains usable for non-standard or legacy scenarios

## Backward Compatibility

`bootstrap-fuzz-target.sh` should remain compatible with older merge reports that do not have `default_corpus_dir`.

If the field is missing:

- fall back to the current behavior
- use `workspace/afl/<target_name>/corpus`
- if empty, create `seed.bin`

This allows older artifacts and tests to keep working while new merged reports adopt selector-safe defaults.

## Failure Semantics

### Merge-time failures

`write_merged_harnesses()` must fail if:

- there are no selected cases
- selected case count exceeds `65536`
- a seed file cannot be written

These should be hard failures because they invalidate the merged-harness fuzz target.

### Bootstrap-time failures

`bootstrap-fuzz-target.sh` should fail if:

- the merge report is missing required core fields such as `manifest_path` or `target_name`
- a merge report explicitly points to a malformed path that cannot be created or used

Bootstrap should not attempt to regenerate selector-safe seeds itself.

## Testing Strategy

### Python tests

Add or extend tests around `write_merged_harnesses()` to verify:

- selector-safe seeds are generated for every selected case
- generated files contain the expected 2-byte selector encoding
- `merge_<crate>.json` includes the new corpus fields
- `corpus_meta/summary.json` is emitted with the expected counts

### Bootstrap tests

Extend bootstrap dry-run tests to verify:

- when merge report contains `default_corpus_dir`, dry-run prints that selector-safe corpus path by default
- explicit `--corpus-dir` still overrides the merge-provided default

### Regression expectation

After this change, standard merged-harness bootstrap should no longer depend on an initially empty corpus directory to start.

## Implementation Outline

1. Extend `rag/seraph_rag/merge_harnesses.py`
   - generate selector-safe seeds
   - emit merge-report corpus fields
   - emit `corpus_meta/summary.json`

2. Extend `rag/tests/test_merge_harnesses.py`
   - add failing tests first for seed generation and report fields

3. Extend `scripts/bootstrap-fuzz-target.sh`
   - read `default_corpus_dir` from merge report when no override is passed
   - preserve empty-directory fallback

4. Extend `crates/seraph-cli/tests/bootstrap_fuzz_target.rs`
   - assert dry-run uses merge-generated default corpus
   - assert explicit override still wins

## Expected User-Visible Outcome

After this change, the normal SERAPH merged-harness path becomes:

1. merge selected small harnesses
2. auto-generate one selector-safe seed per selected case
3. bootstrap AFL++ using that generated corpus automatically

So the repository main flow will no longer have two competing default initialization behaviors for merged-harness fuzzing.
