# SERAPH Types Schema Skeleton Implementation Plan

> Status: Historical planning artifact. This file captures an early schema-building plan and may no longer match the active `knowledge.rs` contract.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first real `seraph-types` crate with stable ID wrappers and JSON schema skeletons for `knowledge.json`, `models.json`, `coverage.json`, and stage artifacts.

**Architecture:** The work starts by making the workspace minimally testable, because the current repository skeleton has member manifests without targets. The implementation then keeps all schema ownership inside `seraph-types`, using focused modules for IDs, shared structures, extraction artifacts, semantic models, coverage state, and stage artifacts.

**Tech Stack:** Rust, Cargo workspace, `serde`, `serde_json`, unit and integration tests

---

### Task 1: Make the Workspace Testable and Prove the Missing Feature

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/seraph-types/Cargo.toml`
- Create: `crates/seraph-types/tests/schema_roundtrip.rs`
- Create: `crates/seraph-types/src/lib.rs`
- Create: `crates/s3-extract/src/lib.rs`
- Create: `crates/s3-model/src/lib.rs`
- Create: `crates/s3-context/src/lib.rs`
- Create: `crates/s3-coverage/src/lib.rs`
- Create: `crates/seraph-cli/src/lib.rs`

- [ ] **Step 1: Add a failing integration test for the missing schema API**

Create `crates/seraph-types/tests/schema_roundtrip.rs` with tests that import these symbols before they exist:

```rust
use seraph_types::{ApiCoverageStatus, ApiId, Knowledge, ScenarioArtifact};

#[test]
fn api_status_uses_snake_case_json() {
    let json = serde_json::to_string(&ApiCoverageStatus::Validated).unwrap();
    assert_eq!(json, "\"validated\"");
}

#[test]
fn scenario_roundtrip_preserves_selected_capabilities() {
    let artifact = ScenarioArtifact {
        scenario_id: "scn_001".into(),
        round: 1,
        name: "parse and inspect".into(),
        description: "parse input and inspect fields".into(),
        scenario_type: seraph_types::ScenarioType::Functional,
        selected_capability_ids: vec!["cap_parse".into(), "cap_query".into()],
        fuzz_variation_points: vec!["input".into()],
        target_api_name_hints: vec!["from_reader".into()],
        semantic_constraints: vec!["must be valid usage".into()],
    };

    let json = serde_json::to_string(&artifact).unwrap();
    let decoded: ScenarioArtifact = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.selected_capability_ids.len(), 2);
    assert_eq!(decoded.selected_capability_ids[0].as_str(), "cap_parse");
}
```

- [ ] **Step 2: Run the test and confirm it fails**

Run: `cargo test -p seraph-types`
Expected: FAIL because `seraph-types` has no `src/lib.rs` target and the imported schema types do not exist.

- [ ] **Step 3: Add minimal workspace-target placeholders**

Create tiny `src/lib.rs` files for the non-target crates so the workspace can parse:

```rust
#![forbid(unsafe_code)]

//! Placeholder crate target for the repository scaffold stage.
```

- [ ] **Step 4: Add serde dependencies for schema work**

Update root `Cargo.toml`:

```toml
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Update `crates/seraph-types/Cargo.toml`:

```toml
[dependencies]
serde.workspace = true

[dev-dependencies]
serde_json.workspace = true
```

- [ ] **Step 5: Commit the scaffolding**

```bash
git add Cargo.toml crates/*/src/lib.rs crates/seraph-types/Cargo.toml crates/seraph-types/tests/schema_roundtrip.rs
git commit -m "test: add failing seraph-types schema tests"
```

### Task 2: Implement Stable IDs and Core Extraction/Model Types

**Files:**
- Modify: `crates/seraph-types/src/lib.rs`
- Create: `crates/seraph-types/src/ids.rs`
- Create: `crates/seraph-types/src/common.rs`
- Create: `crates/seraph-types/src/knowledge.rs`
- Create: `crates/seraph-types/src/models.rs`

- [ ] **Step 1: Declare module layout in `lib.rs`**

Create and re-export:

```rust
pub mod common;
pub mod coverage;
pub mod ids;
pub mod knowledge;
pub mod models;
pub mod stage_artifacts;

pub use common::*;
pub use coverage::*;
pub use ids::*;
pub use knowledge::*;
pub use models::*;
pub use stage_artifacts::*;
```

- [ ] **Step 2: Implement stable ID wrappers**

Create transparent wrappers in `ids.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApiId(pub String);
```

Repeat the same pattern for `CapId`, `TypeId`, `TraitId`, `ModuleId`, `ExampleId`, `ScenarioId`, `MappingId`, and `PlanId`, plus:

```rust
impl ApiId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

Also implement `From<&str>` and `From<String>` for each wrapper.

- [ ] **Step 3: Implement shared small structs and enums**

Create `common.rs` with:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeRef {
    pub file: String,
    pub start_line: u32,
    pub end_line: u32,
}
```

Also add `ScenarioType` with `functional`, `stateful`, and `resource_oriented`.

- [ ] **Step 4: Implement `knowledge.rs` skeleton**

Define `Knowledge` and nested structs for:

- `Level0Summary`
- `ModuleInfo`
- `TypeInfo`
- `ApiInfo`
- `TraitInfo`
- `ExampleInfo`
- `RawRiskSurface`

Use collection fields exactly in snake_case and keep them serializable with `serde`.

- [ ] **Step 5: Implement `models.rs` skeleton**

Define `Models` and nested structs for:

- `FunctionalCapabilityGraph`
- `CapabilityNode`
- `StateLifecycleModel`
- `StateTransition`
- `ForbiddenTransition`
- `ApiContract`
- `GenericConstraints`
- `GenericConstraintParam`
- `RiskSurfaceMap`
- `ApiRisk`
- `TypeSynthesisOverview`
- `RustFeatureRisk`

- [ ] **Step 6: Run the focused test target**

Run: `cargo test -p seraph-types --test schema_roundtrip`
Expected: still FAIL because coverage and stage artifact types are not implemented yet, but ID-based imports should now resolve further than before.

### Task 3: Implement Coverage and Stage Artifact Schemas

**Files:**
- Create: `crates/seraph-types/src/coverage.rs`
- Create: `crates/seraph-types/src/stage_artifacts.rs`
- Modify: `crates/seraph-types/tests/schema_roundtrip.rs`

- [ ] **Step 1: Implement `coverage.rs`**

Define:

- `ApiCoverageStatus` with `targeted`, `attempted`, `validated`
- `FailedAttempt`
- `HarnessRecord`
- `NextPriorityItem`
- `CoverageState`

Use:

```rust
pub struct CoverageState {
    pub total_api_ids: Vec<ApiId>,
    pub api_status: BTreeMap<ApiId, ApiCoverageStatus>,
    pub failed_attempts: BTreeMap<ApiId, FailedAttempt>,
    pub covered_api_ids: Vec<ApiId>,
    pub exhausted_api_ids: Vec<ApiId>,
    pub uncovered_api_ids: Vec<ApiId>,
    pub harnesses: BTreeMap<String, HarnessRecord>,
    pub found_bugs: Vec<String>,
    pub needs_review: Vec<String>,
    pub next_priority: Vec<NextPriorityItem>,
    pub coverage_rate: f64,
}
```

- [ ] **Step 2: Implement `stage_artifacts.rs`**

Define:

- `ScenarioArtifact`
- `ApiMappingArtifact`
- `ApiMappingEntry`
- `TypeNeed`
- `ApiPlanArtifact`
- `OrderedStep`
- `TypeSynthesisSpec`
- `CodegenConstraints`

Use `BTreeMap<String, String>` for lightweight fields such as `arg_sources`.

- [ ] **Step 3: Update tests to cover `Knowledge` and `CoverageState`**

Add a roundtrip test that checks map keys serialize back into `ApiId` values:

```rust
let mut api_status = BTreeMap::new();
api_status.insert(ApiId::from("fn_003"), ApiCoverageStatus::Validated);
```

- [ ] **Step 4: Run the package tests until green**

Run: `cargo test -p seraph-types`
Expected: PASS

- [ ] **Step 5: Commit the feature**

```bash
git add crates/seraph-types Cargo.toml crates/s3-*/src/lib.rs crates/seraph-cli/src/lib.rs
git commit -m "feat: add seraph-types schema skeleton"
```
