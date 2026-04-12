# S3 Extract Minimal Knowledge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `s3-extract` produce a valid minimal `knowledge.json` artifact and expose a tiny CLI that writes it to disk.

**Architecture:** Keep the first implementation intentionally narrow: a library function builds a deterministic placeholder `Knowledge` value from a crate name, and a thin CLI writes that structure as pretty JSON. This establishes the Phase 1 artifact contract without prematurely implementing rustdoc JSON or AST extraction.

**Tech Stack:** Rust, Cargo workspace, `serde_json`, `seraph-types`, integration tests

---

### Task 1: Add failing tests for the missing extraction surface

**Files:**
- Modify: `crates/s3-extract/Cargo.toml`
- Create: `crates/s3-extract/tests/minimal_extract.rs`

- [ ] **Step 1: Write tests against the intended API**

Create `crates/s3-extract/tests/minimal_extract.rs`:

```rust
use seraph_types::Knowledge;

#[test]
fn build_minimal_knowledge_normalizes_import_name() {
    let knowledge = s3_extract::build_minimal_knowledge("serde-json-wrapper");

    assert_eq!(knowledge.crate_name, "serde-json-wrapper");
    assert_eq!(knowledge.crate_import_name, "serde_json_wrapper");
    assert_eq!(knowledge.public_api_count, 0);
}

#[test]
fn cli_writes_valid_knowledge_json() {
    let output = std::env::temp_dir().join("seraph_s3_extract_test_knowledge.json");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_s3-extract"))
        .args(["--crate", "demo-crate", "--output", output.to_str().unwrap()])
        .status()
        .unwrap();

    assert!(status.success());

    let raw = std::fs::read_to_string(&output).unwrap();
    let knowledge: Knowledge = serde_json::from_str(&raw).unwrap();

    assert_eq!(knowledge.crate_name, "demo-crate");
    assert_eq!(knowledge.crate_import_name, "demo_crate");
}
```

- [ ] **Step 2: Run the package tests to confirm failure**

Run: `cargo test -p s3-extract`
Expected: FAIL because `build_minimal_knowledge` and the `s3-extract` binary do not exist yet.

### Task 2: Implement a minimal library and CLI

**Files:**
- Modify: `crates/s3-extract/Cargo.toml`
- Modify: `crates/s3-extract/src/lib.rs`
- Create: `crates/s3-extract/src/main.rs`

- [ ] **Step 1: Add dependencies**

Update `crates/s3-extract/Cargo.toml` with:

```toml
[dependencies]
serde_json.workspace = true
seraph-types = { path = "../seraph-types" }
```

- [ ] **Step 2: Implement the minimal library function**

Replace the placeholder `src/lib.rs` with a tiny extraction surface that exports:

```rust
pub fn build_minimal_knowledge(crate_name: &str) -> Knowledge
```

It should:

- preserve `crate_name`
- normalize `crate_import_name` by replacing `-` with `_`
- fill empty vectors for raw extraction collections
- provide a clear placeholder summary indicating Phase 1 extraction is not fully implemented yet

- [ ] **Step 3: Add a tiny CLI**

Create `src/main.rs` that accepts:

- `--crate <name>`
- `--output <path>`

and writes pretty JSON to the requested path.

- [ ] **Step 4: Run tests until green**

Run: `cargo test -p s3-extract`
Expected: PASS

### Task 3: Verify downstream compatibility and record the result

**Files:**
- Modify: `crates/s3-extract/README.md`

- [ ] **Step 1: Document the current extraction scope**

Update the README to state that the current implementation is a minimal artifact bootstrap, not the final rustdoc/AST extractor.

- [ ] **Step 2: Re-run focused verification**

Run: `cargo test -p s3-extract && cargo test -p seraph-types`
Expected: both commands PASS

- [ ] **Step 3: Commit the foundation**

```bash
git add crates/s3-extract crates/seraph-types Cargo.toml Cargo.lock docs/superpowers/plans/2026-04-12-s3-extract-minimal-knowledge.md docs/superpowers/plans/2026-04-12-seraph-types-schema.md
git commit -m "feat: add minimal s3-extract knowledge generator"
```
