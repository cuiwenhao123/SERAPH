# S3 Extract Rustdoc JSON Implementation Plan

> Status: Historical planning artifact. This file records an earlier extraction plan and should not override the current schema spec, code, or regression tests.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 `s3-extract` 在 `cargo metadata` 之后继续调用 `rustdoc JSON`，填充 `root_docs`、`modules`、`types`、`apis`、`trait_registry`。

**Architecture:** 继续保持 `s3-extract` 的分层输入面。`cargo metadata` 只负责 `crate_meta` 的包级信息，`rustdoc JSON` 只负责结构化文档项和公开暴露关系。源码轻量扫描继续留到下一步，不在这轮实现。

**Tech Stack:** Rust std `Command`、`cargo +nightly-2025-01-26 rustdoc`、`serde`/`serde_json`

---

### Task 1: Add Rustdoc-Focused Tests

**Files:**
- Modify: `crates/s3-extract/tests/minimal_extract.rs`

- [ ] **Step 1: Add assertions for crate root docs**

Assert that extracting `hashbrown` fills:

```rust
assert!(knowledge.crate_meta.root_docs.contains("SwissTable"));
```

- [ ] **Step 2: Add assertions for modules, types, APIs, and traits**

Assert at least:

```rust
assert!(knowledge.modules.iter().any(|m| m.name == "hash_map"));
assert!(knowledge.types.iter().any(|t| t.name == "HashMap"));
assert!(knowledge.apis.iter().any(|a| a.name == "new"));
assert!(knowledge.trait_registry.iter().any(|t| t.name == "Equivalent"));
```

- [ ] **Step 3: Add assertions for public path enrichment**

Assert that the extracted `HashMap` type includes:

```rust
"hashbrown::HashMap"
```

in `public_paths`.

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p s3-extract`

Expected: FAIL because rustdoc extraction is not implemented yet.

### Task 2: Implement Rustdoc JSON Loading

**Files:**
- Modify: `crates/s3-extract/src/lib.rs`

- [ ] **Step 1: Add rustdoc JSON command execution**

Run:

```bash
cargo +nightly-2025-01-26 rustdoc --lib --manifest-path <path> -- -Z unstable-options --output-format json
```

from Rust code and read:

```text
<crate-dir>/target/doc/<lib-target-name>.json
```

- [ ] **Step 2: Add narrow rustdoc JSON structs**

Define typed wrappers for:
- root id
- item index
- paths map
- item span

Keep `inner` as `serde_json::Value` to avoid over-modeling.

- [ ] **Step 3: Add rustdoc-specific error variants**

Cover:
- rustdoc launch failure
- rustdoc non-zero exit
- missing JSON file
- JSON parse failure

### Task 3: Extract Raw Knowledge Items

**Files:**
- Modify: `crates/s3-extract/src/lib.rs`

- [ ] **Step 1: Fill `crate_meta.root_docs` from the root item**

- [ ] **Step 2: Extract modules**

Build `ModuleInfo` for local modules, with:
- canonical path from `paths`
- public paths from recursive public module/use traversal

- [ ] **Step 3: Extract types**

Build `TypeInfo` for public/re-exported local types, with:
- canonical path from `paths`
- public paths from traversal
- module ownership from canonical parent module

- [ ] **Step 4: Extract methods and functions**

Use:
- module traversal for direct free functions
- type impl traversal for inherent methods / assoc functions / constructors

- [ ] **Step 5: Extract traits**

At minimum support:
- local public trait definitions
- public re-exported traits from `use`

### Task 4: Verify on Hashbrown

**Files:**
- No code changes

- [ ] **Step 1: Run focused test suite**

Run: `cargo test -p s3-extract`

Expected: PASS

- [ ] **Step 2: Run real extraction**

Run:

```bash
cargo run -p s3-extract -- \
  --manifest-path /tmp/hashbrown-eval/Cargo.toml \
  --output /tmp/hashbrown_rustdoc_knowledge.json
```

Expected:
- root docs non-empty
- modules non-empty
- types non-empty
- apis non-empty
- trait registry non-empty

- [ ] **Step 3: Re-run workspace verification**

Run: `cargo test --workspace`

Expected: PASS
