# s3-coverage

Coverage control surface for SERAPH runtime state.

Current responsibilities:

- initialize `coverage.json`
- update `targeted` / `attempted` / `validated` states from Phase 3 artifacts
- track compile failure counters and last failure reasons
- mark exhausted targets after failed fix loops
- write minimal structured `harnesses` records for compile, fix-loop, smoke, and runtime-diagnosis artifacts
- keep backward-compatible target-level `found_bugs` / `needs_review` lists
- recompute covered / uncovered / next-priority views
- overwrite a target's old coverage state with the latest rerun result instead of keeping stale target-local status and harness records

Current `coverage.harnesses` key convention:

- original generated harness: `round:sub_index`
- repaired harness attempt: `round:sub_index:fixed:attempt`

Current `HarnessRecord` scope is intentionally small:

- optional `target_api_id`
- compile-derived `status`
- `harness_path`
- `compile_report_path`
- `smoke_report_path`
- `runtime_status`
- `runtime_classification`
- `runtime_error_report_path`
- `runtime_error_summary`
- `review_status`
- `review_reason`
- `review_report_path`

Legacy `scenario_id` / `mapping_id` / `plan_id` fields remain optional for backward compatibility only.

`CoverageState.related_api_ids_by_target` keeps each target's current related-context contribution so related coverage can refresh cleanly on rerun.

Active post-smoke ingestion now prefers `runtime_error_RRR_index.json`. Older `fix_acceptance_RRR_index.json` inputs are still accepted for compatibility.

Acceptance-derived review fields follow the compatibility-only lightweight contract:

- `review_status`: `accepted`, `needs_review`, or `bug`
- `review_reason`: current rollup or manual override reason
- `review_report_path`: source `fix_acceptance_RRR_index.json`
