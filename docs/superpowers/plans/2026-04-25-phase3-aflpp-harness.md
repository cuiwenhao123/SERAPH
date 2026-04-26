# Phase 3 AFL++ Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the active Phase 3 harness generation path emit AFL++-friendly Rust binary harnesses instead of libFuzzer-style sources, while keeping the existing compile-check → fix-loop → smoke-run flow intact.

**Architecture:** Keep one public orchestration path through `seraph-cli run`. Add a small prompt-style selector that flows from Rust CLI to the Python prompt bundle writer, then tighten the prompt rules so generated harnesses use a normal `fn main()` and std-based stdin/`@@` file input. Validate with focused Python and Rust tests first, then update docs/examples.

**Tech Stack:** Rust (`seraph-cli`), Python (`rag/seraph_rag`), pytest, Rust integration tests, existing Phase 3 command templates.

---

## File Structure

- Modify: `rag/seraph_rag/harness_prompt.py` — add prompt profile/style support and AFL++-friendly prompt text.
- Modify: `rag/seraph_rag/cli.py` — accept prompt style options on `harness-prompt`.
- Modify: `rag/tests/test_harness_prompt.py` — unit tests for new prompt semantics.
- Modify: `rag/tests/test_cli.py` — CLI tests for prompt style plumbing.
- Modify: `crates/seraph-cli/src/lib.rs` — pass prompt style through command plans.
- Modify: `crates/seraph-cli/src/main.rs` — expose one unified `run` flag for AFL++ prompt generation.
- Modify: `crates/seraph-cli/tests/run_dry_run.rs` — dry-run assertions for the new option.
- Modify: `README.md` — document the active AFL++-friendly Phase 3 path.
- Modify: `docs/architecture/phase3-harness-generation.md` — record the new harness shape and constraints.

---

### Task 1: Define Failing Prompt Tests

**Files:**
- Modify: `rag/tests/test_harness_prompt.py`
- Modify: `rag/tests/test_cli.py`
- Modify: `crates/seraph-cli/tests/run_dry_run.rs`

- [ ] Add Python unit tests asserting the prompt bundle can request an `aflpp` style and that the system prompt forbids `libfuzzer_sys` while requiring `fn main()` plus stdin/argument-based input.
- [ ] Add a Python CLI test asserting `seraph_rag.cli harness-prompt --style aflpp` writes `"style": "aflpp"` into the prompt bundle.
- [ ] Add a Rust dry-run test asserting `seraph-cli run --phase3-style aflpp` forwards `--style aflpp` to `python3 -m seraph_rag.cli harness-prompt`.

### Task 2: Implement Minimal Prompt-Style Plumbing

**Files:**
- Modify: `rag/seraph_rag/harness_prompt.py`
- Modify: `rag/seraph_rag/cli.py`
- Modify: `crates/seraph-cli/src/lib.rs`
- Modify: `crates/seraph-cli/src/main.rs`

- [ ] Extend the prompt bundle builder/writer to accept a style string, defaulting to the current active style.
- [ ] Update the Python CLI and Rust command-plan helpers to pass the style through without changing unrelated Phase 3 behavior.
- [ ] Keep old command invocations valid by making the new option optional.

### Task 3: Switch Active Prompt Rules to AFL++-Friendly Binaries

**Files:**
- Modify: `rag/seraph_rag/harness_prompt.py`

- [ ] Replace the stale `libFuzzer-compatible` wording with AFL++-friendly binary rules.
- [ ] Require a normal `fn main()` harness that uses only stdlib input handling and explicitly forbids `libfuzzer_sys`.
- [ ] Preserve SERAPH markers, target call requirements, and compile-fixable fallback behavior.

### Task 4: Refresh Docs

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture/phase3-harness-generation.md`

- [ ] Update the documented Phase 3 flow to describe AFL++-friendly harness generation.
- [ ] Add one concrete `seraph-cli run` example using the unified style flag.

### Task 5: Verify

**Files:**
- Test: `rag/tests/test_harness_prompt.py`
- Test: `rag/tests/test_cli.py`
- Test: `crates/seraph-cli/tests/run_dry_run.rs`

- [ ] Run focused pytest coverage for the prompt/CLI changes.
- [ ] Run focused Rust integration tests for `seraph-cli` dry-run behavior.
- [ ] Only then report the implementation state and any remaining follow-up work.
