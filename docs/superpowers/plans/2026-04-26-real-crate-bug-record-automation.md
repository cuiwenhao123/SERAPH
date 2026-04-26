# Real Crate Bug Record Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a lightweight, deterministic helper that turns a real-crate Phase 3 workspace into a reusable bug-record Markdown snippet for `docs/architecture/rust-feature-bug-record.md`.

**Architecture:** Keep the logic in a small Python module under `rag/seraph_rag/` so it is testable with `pytest`, then expose it through a thin `scripts/` wrapper. The helper reads existing SERAPH artifacts (`context`, `compile/smoke/runtime indexes`, `coverage.json`), extracts structured facts, and renders a Markdown template with optional human-supplied root-cause/fix text.

**Tech Stack:** Python 3.8+, stdlib (`argparse`, `json`, `re`, `pathlib`), existing `pytest` test suite.

---

### Task 1: Add failing tests for workspace parsing and markdown rendering

**Files:**
- Create: `rag/tests/test_bug_record.py`
- Reference: `rag/tests/test_runtime_diagnose.py`

- [ ] **Step 1: Write a failing unit test for a successful workspace**

Create a temp workspace with:

- `contexts/rag_target_001.md`
- `reports/compile_001_index.json`
- `reports/smoke_001_index.json`
- `reports/runtime_error_001_index.json`
- `coverage.json`

Assert the renderer includes:

- crate / phase line
- target API
- compile / smoke / runtime statuses
- related coverage summary
- evidence paths

- [ ] **Step 2: Run the test and confirm it fails**

Run: `pytest -q rag/tests/test_bug_record.py::test_render_bug_record_entry_from_success_workspace`

Expected: `ImportError` or missing-symbol failure because the module does not exist yet.

- [ ] **Step 3: Write a failing unit test for compile-error extraction**

Create a temp workspace whose compile report `stderr` contains errors such as:

- `error[E0502]`
- `error[E0133]`

Assert the renderer surfaces these error codes in the Markdown snippet.

- [ ] **Step 4: Run the test and confirm it fails**

Run: `pytest -q rag/tests/test_bug_record.py::test_render_bug_record_entry_collects_compile_error_codes`

Expected: failure because compile error parsing is not implemented yet.

### Task 2: Implement the reusable renderer module

**Files:**
- Create: `rag/seraph_rag/bug_record.py`
- Test: `rag/tests/test_bug_record.py`

- [ ] **Step 1: Add minimal parsing helpers**

Implement helpers for:

- loading JSON if present
- extracting target API ID from `rag_target_RRR.md`
- discovering round number from workspace files
- collecting compile error codes from compile reports

- [ ] **Step 2: Add Markdown rendering**

Implement a function that accepts:

- `workspace_dir`
- `crate_name`
- optional `phase`
- optional `round_no`
- optional qualitative text lists (`symptom`, `rust_feature`, `root_cause`, `tool_fix`, `capability_gain`)

Return one Markdown section that includes structured facts plus placeholders when optional text is omitted.

- [ ] **Step 3: Re-run the focused test file**

Run: `pytest -q rag/tests/test_bug_record.py`

Expected: all tests pass.

### Task 3: Add a thin script entrypoint

**Files:**
- Create: `scripts/real_crate_bug_record.py`
- Modify: `scripts/README.md`

- [ ] **Step 1: Add CLI wrapper**

Expose arguments for:

- `--workspace-dir`
- `--crate`
- `--phase`
- `--round`
- `--title`
- repeatable qualitative flags such as `--symptom`, `--rust-feature`, `--root-cause`, `--tool-fix`, `--capability-gain`
- `--output`
- `--append-doc`

- [ ] **Step 2: Document the script**

Add a short section to `scripts/README.md` with:

- what artifacts the script reads
- stdout usage
- `--append-doc` usage

- [ ] **Step 3: Run one script-level smoke check**

Run: `python3 scripts/real_crate_bug_record.py --workspace-dir <workspace> --crate bytes`

Expected: Markdown is printed to stdout.

### Task 4: Verify against a real `bytes` workspace and sync docs

**Files:**
- Modify: `docs/architecture/current-phase1-phase2-flow.md` only if needed for cross-reference
- Reference: `docs/architecture/rust-feature-bug-record.md`

- [ ] **Step 1: Run the helper on a real verified workspace**

Use:

- `/tmp/seraph-bytes-bugfix-verify-rerun-20260426/uninit_new`

Confirm the output matches the current manual bug record style.

- [ ] **Step 2: Optionally append to a temp markdown file**

Run with `--append-doc /tmp/seraph-bug-record-preview.md` and verify the section is appended cleanly.

- [ ] **Step 3: Run final verification commands**

Run:

- `pytest -q rag/tests/test_bug_record.py`
- `python3 scripts/real_crate_bug_record.py --workspace-dir /tmp/seraph-bytes-bugfix-verify-rerun-20260426/uninit_new --crate bytes --phase 'Phase 3 real rerun'`

Expected: tests pass and script prints a complete Markdown section.
