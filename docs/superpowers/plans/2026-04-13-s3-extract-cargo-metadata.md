# S3 Extract Cargo Metadata Implementation Plan

> Status: Historical planning artifact. This file records one implementation slice and may describe intermediate behavior that has since been refined.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 `s3-extract` 以本地 crate 的 `Cargo.toml` 为输入，调用 `cargo metadata` 真实填充 `Knowledge.crate_meta`。

**Architecture:** 保持 `s3-extract` 的实现很窄，只接入 `cargo metadata` 作为第一个真实数据源。CLI 从“只接收 crate 名”推进到“接收 `--manifest-path` 并可选 `--crate` 作为回退显示名”，其余 schema 字段继续输出空骨架，避免和后续 `rustdoc JSON` 职责重叠。

**Tech Stack:** Rust std `Command`、`cargo metadata`、`serde`/`serde_json`

---

### Task 1: Add Metadata-Focused Tests

**Files:**
- Modify: `crates/s3-extract/tests/minimal_extract.rs`

- [ ] **Step 1: Write a failing library test for manifest metadata extraction**

Add a test that calls a new extraction entrypoint with `/tmp/hashbrown-eval/Cargo.toml` and asserts:

```rust
assert_eq!(knowledge.crate_meta.package_name, "hashbrown");
assert_eq!(knowledge.crate_meta.lib_target_name, "hashbrown");
assert_eq!(knowledge.crate_meta.crate_import_name, "hashbrown");
assert_eq!(knowledge.crate_meta.version, "0.17.0");
assert_eq!(knowledge.crate_meta.edition, "2024");
assert_eq!(knowledge.crate_meta.rust_version.as_deref(), Some("1.85.0"));
assert_eq!(
    knowledge.crate_meta.repository.as_deref(),
    Some("https://github.com/rust-lang/hashbrown")
);
assert!(knowledge
    .crate_meta
    .default_features
    .contains(&"raw-entry".to_owned()));
```

- [ ] **Step 2: Write a failing CLI test for `--manifest-path`**

Update the CLI integration test to run:

```bash
s3-extract --manifest-path /tmp/hashbrown-eval/Cargo.toml --output <tempfile>
```

and assert the decoded JSON has non-empty `manifest_path`, `lib_rs_path`, and `package_name == "hashbrown"`.

- [ ] **Step 3: Run tests to verify they fail for the right reason**

Run: `cargo test -p s3-extract`

Expected: FAIL because the new entrypoint and CLI support are not implemented yet.

### Task 2: Implement Cargo Metadata Extraction

**Files:**
- Modify: `crates/s3-extract/Cargo.toml`
- Modify: `crates/s3-extract/src/lib.rs`

- [ ] **Step 1: Add `serde` dependency**

Add:

```toml
serde.workspace = true
```

to `crates/s3-extract/Cargo.toml`.

- [ ] **Step 2: Add minimal metadata response structs**

In `crates/s3-extract/src/lib.rs`, define only the typed fields needed from `cargo metadata`:

```rust
#[derive(Debug, Deserialize)]
struct MetadataResponse {
    packages: Vec<MetadataPackage>,
}
```

plus package/target structs for name, version, edition, rust version, repository, description, features, and targets.

- [ ] **Step 3: Add a real extraction entrypoint**

Implement:

```rust
pub fn extract_knowledge_from_manifest(manifest_path: impl AsRef<Path>) -> Result<Knowledge, ExtractError>
```

Behavior:
- run `cargo metadata --format-version 1 --no-deps --manifest-path <path>`
- parse stdout JSON
- pick the package entry
- find the `lib` target
- fill `Knowledge.crate_meta`
- keep `modules/types/apis/trait_registry/examples/risk_facts` empty

- [ ] **Step 4: Add a small error type**

Add a narrow `ExtractError` enum covering:
- command launch failure
- non-zero `cargo metadata`
- invalid UTF-8 or JSON
- missing package
- missing lib target

- [ ] **Step 5: Preserve the old placeholder helper as a fallback only**

Keep `build_minimal_knowledge(crate_name: &str)` for tests/bootstrap, but do not use it for real crate extraction when `--manifest-path` is provided.

### Task 3: Update CLI

**Files:**
- Modify: `crates/s3-extract/src/main.rs`

- [ ] **Step 1: Extend argument parsing**

Support:

```text
--manifest-path <path>
--output <path>
--crate <name>   # optional fallback only
```

- [ ] **Step 2: Prefer real extraction when manifest path is present**

Behavior:
- if `--manifest-path` is given, call `extract_knowledge_from_manifest`
- else if `--crate` is given, keep the placeholder path
- else return a clear argument error

- [ ] **Step 3: Keep error messages user-facing**

Map extraction errors into concise CLI strings like:

```text
failed to extract knowledge from manifest: ...
```

### Task 4: Verify on Hashbrown

**Files:**
- No code changes

- [ ] **Step 1: Run focused test suite**

Run: `cargo test -p s3-extract`

Expected: PASS

- [ ] **Step 2: Run the CLI against hashbrown**

Run:

```bash
cargo run -p s3-extract -- \
  --manifest-path /tmp/hashbrown-eval/Cargo.toml \
  --output /tmp/hashbrown_metadata_knowledge.json
```

Expected:
- command exits 0
- output JSON has `package_name = "hashbrown"`
- output JSON has `version = "0.17.0"`
- output JSON has `edition = "2024"`
- output JSON has non-empty `manifest_path`
- output JSON has non-empty `lib_rs_path`

- [ ] **Step 3: Re-run workspace verification**

Run: `cargo test --workspace`

Expected: PASS
