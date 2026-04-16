# Phase 2 Post-Audit Refinement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 Phase 2 在真实 crate 审查中暴露出的 5 类缺口收敛掉，重点修复 schema 边界、type-level risk 保留、lifetime 参数拆分、SLM 弱 full 降级、FCG/summary 输出过松的问题。

**Architecture:** 继续以 `crates/seraph-types/src/models.rs` 作为唯一 Phase 2 输出契约，保留 `crates/s3-model/src/contracts.rs`、`fcg.rs`、`slm.rs`、`risk.rs` 四个 deterministic builder。实现策略是先冻结 schema，再用测试驱动逐个收紧 builder 规则，最后刷新 golden fixture、README 和真实 crate 审查输出。

**Tech Stack:** Rust 2021, `serde`, `serde_json`, `cargo test`, crate-local unit tests, integration tests, golden JSON fixtures, real-crate CLI validation

---

## Scope Guard

- 这份计划只覆盖 Phase 2：
  - `crates/seraph-types/src/models.rs`
  - `crates/seraph-types/tests/schema_roundtrip.rs`
  - `crates/s3-model/src/contracts.rs`
  - `crates/s3-model/src/fcg.rs`
  - `crates/s3-model/src/slm.rs`
  - `crates/s3-model/src/risk.rs`
  - `crates/s3-model/tests/minimal_pipeline.rs`
  - `crates/s3-model/tests/fixtures/minimal_models.json`
  - `crates/s3-model/README.md`
- 不改 Phase 1 抽取器。
- 不改 Phase 3 orchestration。
- 不提交 `examples/target-crates/hashbrown/`。
- 实施时必须在独立 worktree 内进行，不直接在 `main` 上改代码。

## File Structure

### Existing files to modify

- `crates/seraph-types/src/models.rs`
  - 新增 `GenericConstraints.lifetime_params`
  - 新增 `RustFeatureRisk.types_affected`
- `crates/seraph-types/tests/schema_roundtrip.rs`
  - 增加 roundtrip 断言，确保新字段 JSON 读写稳定
- `crates/s3-model/src/risk.rs`
  - 把 `repr_packed`、`panic_in_drop`、`conditional_impl` 改成 type-level risk
  - 保持 `extern_abi`、`borrowed_return` 仍然是 API-level risk
- `crates/s3-model/src/contracts.rs`
  - 把 lifetime 参数从 `params` 拆到 `lifetime_params`
- `crates/s3-model/src/slm.rs`
  - 收紧 `full` 的判定门槛
- `crates/s3-model/src/fcg.rs`
  - 收紧跨锚点边
  - 压缩 `stage1_summary.one_liner`
- `crates/s3-model/tests/minimal_pipeline.rs`
  - 增加 risk/contracts/slm/fcg 回归测试
- `crates/s3-model/tests/fixtures/minimal_models.json`
  - 刷新 golden fixture
- `crates/s3-model/README.md`
  - 同步新字段和新语义

### Files to leave alone

- `crates/s3-model/src/io.rs`
- `crates/s3-model/src/lib.rs`
- `crates/s3-model/src/main.rs`
- `crates/s3-extract/**`

## Plan Outcome Checklist

实现结束后，应同时满足：

1. `GenericConstraints` 的生命周期参数不再混入 `params`。
2. `RustFeatureRisk` 能同时表达 `apis_affected` 和 `types_affected`。
3. `panic_in_drop` 即使没有直接 public API，也不会从 Phase 2 输出中消失。
4. `conditional_impl`、`repr_packed` 不再被扩散成“该类型全部 API 都受影响”。
5. `SLM full` 不再出现在只有 `Constructed` 的弱模型上。
6. `FCG recommended_chains` 比当前输出更保守，避免 query-to-query 的机械跨锚点跳转。
7. `stage1_summary.one_liner` 在 `hashbrown` 这类大 crate 上显著变短。
8. `cargo test -p seraph-types -p s3-model -- --nocapture` 通过。
9. `s3-extract -> s3-model` 在 `s3-audit-fixture`、`hashbrown`、`moonfire-ffmpeg`、`semver` 上通过验证脚本。

### Task 0: Create the Dedicated Worktree and Freeze the Baseline

**Files:**
- Create: `/home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement`
- Test: baseline repo state only

- [ ] **Step 1: Create the feature worktree from `main`**

```bash
git -C /home/cas/Desktop/SERAPH worktree add \
  -b feat/phase2-post-audit-refinement \
  /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement \
  main
```

- [ ] **Step 2: Verify the worktree is clean and on the right branch**

Run:

```bash
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement status --short
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement branch --show-current
```

Expected:

- first command prints nothing
- second command prints `feat/phase2-post-audit-refinement`

- [ ] **Step 3: Run the current Phase 2 test baseline before edits**

Run:

```bash
cargo test -p seraph-types -p s3-model -- --nocapture
```

Workdir:

```bash
/home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement
```

Expected: PASS. This is the baseline that the refinement work must preserve.

### Task 1: Freeze the Phase 2 Schema Around the New Boundaries

**Files:**
- Modify: `crates/seraph-types/src/models.rs`
- Modify: `crates/seraph-types/tests/schema_roundtrip.rs`
- Test: `crates/seraph-types/tests/schema_roundtrip.rs`

- [ ] **Step 1: Extend the roundtrip test to cover `lifetime_params` and `types_affected`**

Append these fields to the existing `models_roundtrip_preserves_phase2_semantic_fields` fixture in `crates/seraph-types/tests/schema_roundtrip.rs`:

```rust
generic_constraints: Some(GenericConstraints {
    params: vec![GenericConstraintParam {
        name: "R".into(),
        direct_bounds: vec!["Read".into()],
        full_bound_chain: vec!["Read".into()],
        associated_type_constraints: vec![AssociatedTypeConstraint {
            trait_id: Some(TraitId::from("trait::demo::Reader")),
            trait_path: "demo::Reader".into(),
            associated_type_name: "Item".into(),
            bounds: vec!["Debug".into()],
        }],
        is_unsafe_trait: false,
        strategy: "C".into(),
        bug_hunting_value: BugHuntingValue::High,
        synthesis_guidance: "custom reader with short-read".into(),
    }],
    lifetime_params: vec!["'a".into()],
}),
```

and:

```rust
rust_feature_risks: vec![RustFeatureRisk {
    feature: "borrowed_return".into(),
    apis_affected: vec![ApiId::from("api::demo::parse")],
    types_affected: vec![TypeId::from("type::demo::Parser")],
    risk: "borrow ties output to input".into(),
}],
```

Add the assertions:

```rust
assert_eq!(
    decoded.api_contracts[0]
        .generic_constraints
        .as_ref()
        .unwrap()
        .lifetime_params,
    vec!["'a".to_string()]
);
assert_eq!(
    decoded.risk_surface_map.rust_feature_risks[0].types_affected,
    vec![TypeId::from("type::demo::Parser")]
);
```

- [ ] **Step 2: Run the targeted schema test and confirm it fails**

Run:

```bash
cargo test -p seraph-types models_roundtrip_preserves_phase2_semantic_fields -- --nocapture
```

Workdir:

```bash
/home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement
```

Expected: FAIL with compile errors because `GenericConstraints` has no `lifetime_params` field and `RustFeatureRisk` has no `types_affected` field yet.

- [ ] **Step 3: Add the schema fields in `models.rs`**

Change `crates/seraph-types/src/models.rs` to:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericConstraints {
    pub params: Vec<GenericConstraintParam>,
    #[serde(default)]
    pub lifetime_params: Vec<String>,
}
```

and:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustFeatureRisk {
    pub feature: String,
    #[serde(default)]
    pub apis_affected: Vec<ApiId>,
    #[serde(default)]
    pub types_affected: Vec<TypeId>,
    pub risk: String,
}
```

- [ ] **Step 4: Run the full `seraph-types` test crate**

Run:

```bash
cargo test -p seraph-types -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit the schema boundary change**

```bash
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement add \
  crates/seraph-types/src/models.rs \
  crates/seraph-types/tests/schema_roundtrip.rs
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement commit -m \
  "feat: add phase2 lifetime and type risk schema fields"
```

### Task 2: Preserve Type-Level Risks in `risk.rs`

**Files:**
- Modify: `crates/s3-model/src/risk.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add a failing regression test for type-level risks**

Add this test to `crates/s3-model/tests/minimal_pipeline.rs`:

```rust
#[test]
fn risk_surface_keeps_type_level_risks_without_api_expansion() {
    use seraph_types::{
        CodeRef, DocSections, ExplicitPanicSiteFact, TraitImplInfo, TraitOrigin, TypeInfo,
        TypeKind,
    };

    let mut knowledge = fixture_knowledge();

    knowledge.types.push(TypeInfo {
        type_id: seraph_types::TypeId::from("type::demo::drop::Bomb"),
        name: "Bomb".into(),
        canonical_path: "demo::drop::Bomb".into(),
        public_paths: vec!["demo::drop::Bomb".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::drop"),
        code_ref: CodeRef {
            file: "src/drop.rs".into(),
            start_line: 1,
            end_line: 8,
        },
        docs: "Panicking drop bomb.".into(),
        doc_sections: DocSections::default(),
        kind: TypeKind::Struct,
        generic_params: vec![],
        where_clauses: vec![],
        is_non_exhaustive: false,
        fields: vec![],
        variants: vec![],
        has_hidden_fields: false,
        has_hidden_variants: false,
    });

    knowledge.trait_impl_registry.push(TraitImplInfo {
        trait_impl_id: seraph_types::TraitImplId::from(
            "trait_impl::core::ops::drop::Drop::for::demo::drop::Bomb",
        ),
        target_type_id: seraph_types::TypeId::from("type::demo::drop::Bomb"),
        trait_ref_text: "core::ops::drop::Drop".into(),
        for_type_text: "demo::drop::Bomb".into(),
        trait_id: seraph_types::TraitId::from("trait::core::ops::drop::Drop"),
        trait_name: "Drop".into(),
        trait_canonical_path: "core::ops::drop::Drop".into(),
        trait_origin: TraitOrigin::External,
        source: CodeRef {
            file: "src/drop.rs".into(),
            start_line: 10,
            end_line: 14,
        },
        associated_type_bindings: vec![],
        associated_const_bindings: vec![],
        where_clauses: vec![],
        cfg_attrs: vec![],
        is_unsafe: false,
    });

    knowledge.trait_impl_registry.push(TraitImplInfo {
        trait_impl_id: seraph_types::TraitImplId::from(
            "trait_impl::demo::Feature::for::demo::query::Parser",
        ),
        target_type_id: seraph_types::TypeId::from("type::demo::query::Parser"),
        trait_ref_text: "demo::Feature".into(),
        for_type_text: "demo::query::Parser".into(),
        trait_id: seraph_types::TraitId::from("trait::demo::Feature"),
        trait_name: "Feature".into(),
        trait_canonical_path: "demo::Feature".into(),
        trait_origin: TraitOrigin::Local,
        source: CodeRef {
            file: "src/query.rs".into(),
            start_line: 40,
            end_line: 40,
        },
        associated_type_bindings: vec![],
        associated_const_bindings: vec![],
        where_clauses: vec![],
        cfg_attrs: vec!["#[cfg(feature = \"nightly\")]".into()],
        is_unsafe: false,
    });

    knowledge
        .risk_facts
        .drop_impl_types
        .push(seraph_types::TypeId::from("type::demo::drop::Bomb"));
    knowledge
        .risk_facts
        .explicit_panic_sites
        .push(ExplicitPanicSiteFact {
            owner: seraph_types::RiskOwner::TraitImpl(seraph_types::TraitImplId::from(
                "trait_impl::core::ops::drop::Drop::for::demo::drop::Bomb",
            )),
            panic_kind: "panic!".into(),
            source: CodeRef {
                file: "src/drop.rs".into(),
                start_line: 12,
                end_line: 12,
            },
        });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    let panic_in_drop = models
        .risk_surface_map
        .rust_feature_risks
        .iter()
        .find(|risk| risk.feature == "panic_in_drop")
        .unwrap();
    assert!(panic_in_drop.apis_affected.is_empty());
    assert_eq!(
        panic_in_drop.types_affected,
        vec![seraph_types::TypeId::from("type::demo::drop::Bomb")]
    );

    let conditional_impl = models
        .risk_surface_map
        .rust_feature_risks
        .iter()
        .find(|risk| risk.feature == "conditional_impl")
        .unwrap();
    assert!(conditional_impl.apis_affected.is_empty());
    assert_eq!(
        conditional_impl.types_affected,
        vec![seraph_types::TypeId::from("type::demo::query::Parser")]
    );
}
```

- [ ] **Step 2: Run the targeted regression test and confirm it fails**

Run:

```bash
cargo test -p s3-model risk_surface_keeps_type_level_risks_without_api_expansion -- --nocapture
```

Expected: FAIL because current `risk.rs` still expands `conditional_impl` and `panic_in_drop` into `apis_affected`.

- [ ] **Step 3: Rework `build_rust_feature_risks` to keep type-level facts on `types_affected`**

Replace the feature construction in `crates/s3-model/src/risk.rs` with this shape:

```rust
if !extern_abi_apis.is_empty() {
    risks.push(RustFeatureRisk {
        feature: "extern_abi".to_owned(),
        apis_affected: extern_abi_apis,
        types_affected: vec![],
        risk: "public API crosses a non-Rust ABI boundary".to_owned(),
    });
}

if !borrowed_return_apis.is_empty() {
    risks.push(RustFeatureRisk {
        feature: "borrowed_return".to_owned(),
        apis_affected: borrowed_return_apis,
        types_affected: vec![],
        risk: "returned value borrows from self or input arguments".to_owned(),
    });
}

let packed_type_ids = knowledge
    .risk_facts
    .repr_types
    .iter()
    .filter(|fact| fact.repr_kinds.iter().any(|kind| matches!(kind, ReprKind::Packed)))
    .map(|fact| fact.type_id.clone())
    .collect::<BTreeSet<TypeId>>()
    .into_iter()
    .collect::<Vec<_>>();
if !packed_type_ids.is_empty() {
    risks.push(RustFeatureRisk {
        feature: "repr_packed".to_owned(),
        apis_affected: vec![],
        types_affected: packed_type_ids,
        risk: "packed repr can make references invalid or surprising".to_owned(),
    });
}

let conditional_impl_type_ids = knowledge
    .trait_impl_registry
    .iter()
    .filter(|impl_info| !impl_info.cfg_attrs.is_empty())
    .map(|impl_info| impl_info.target_type_id.clone())
    .collect::<BTreeSet<TypeId>>()
    .into_iter()
    .collect::<Vec<_>>();
if !conditional_impl_type_ids.is_empty() {
    risks.push(RustFeatureRisk {
        feature: "conditional_impl".to_owned(),
        apis_affected: vec![],
        types_affected: conditional_impl_type_ids,
        risk: "public surface depends on cfg-gated impl availability".to_owned(),
    });
}

let panic_in_drop_type_ids = knowledge
    .trait_impl_registry
    .iter()
    .filter(|impl_info| impl_info.trait_canonical_path == "core::ops::drop::Drop")
    .filter(|impl_info| {
        knowledge.risk_facts.explicit_panic_sites.iter().any(|fact| {
            matches!(&fact.owner, RiskOwner::TraitImpl(id) if id == &impl_info.trait_impl_id)
        })
    })
    .map(|impl_info| impl_info.target_type_id.clone())
    .collect::<BTreeSet<TypeId>>()
    .into_iter()
    .collect::<Vec<_>>();
if !panic_in_drop_type_ids.is_empty() {
    risks.push(RustFeatureRisk {
        feature: "panic_in_drop".to_owned(),
        apis_affected: vec![],
        types_affected: panic_in_drop_type_ids,
        risk: "Drop impl can panic during destruction".to_owned(),
    });
}
```

- [ ] **Step 4: Run the targeted and broader risk tests**

Run:

```bash
cargo test -p s3-model risk_surface_keeps_type_level_risks_without_api_expansion -- --nocapture
cargo test -p s3-model risk_builder_maps_phase1_facts_to_api_and_rust_feature_risks -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit the risk boundary fix**

```bash
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement add \
  crates/s3-model/src/risk.rs \
  crates/s3-model/tests/minimal_pipeline.rs
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement commit -m \
  "feat: preserve type-level phase2 feature risks"
```

### Task 3: Split Lifetime Parameters Out of Contract Synthesis Params

**Files:**
- Modify: `crates/s3-model/src/contracts.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add a failing lifetime-separation regression test**

Add this test to `crates/s3-model/tests/minimal_pipeline.rs`:

```rust
#[test]
fn contract_builder_separates_lifetime_params_from_synthesis_params() {
    let mut knowledge = fixture_knowledge();
    let from_reader = knowledge
        .apis
        .iter_mut()
        .find(|api| api.canonical_path == "demo::io::from_reader")
        .unwrap();
    from_reader.generic_params = vec!["'a".into(), "R".into()];
    from_reader.where_clauses = vec!["R: demo::io::Reader".into()];

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let constraints = models
        .api_contracts
        .iter()
        .find(|contract| contract.path == "demo::io::from_reader")
        .unwrap()
        .generic_constraints
        .as_ref()
        .unwrap();

    assert_eq!(constraints.lifetime_params, vec!["'a".to_string()]);
    assert_eq!(
        constraints
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>(),
        vec!["R".to_string()]
    );
}
```

- [ ] **Step 2: Run the targeted contract test and confirm it fails**

Run:

```bash
cargo test -p s3-model contract_builder_separates_lifetime_params_from_synthesis_params -- --nocapture
```

Expected: FAIL because current `build_generic_constraints` pushes `'a` into `params`.

- [ ] **Step 3: Split generic params into lifetimes and synthesis-relevant params**

Update `crates/s3-model/src/contracts.rs` as follows:

```rust
fn build_generic_constraints(
    knowledge: &Knowledge,
    api: &ApiInfo,
    trait_index: &TraitIndex<'_>,
    type_index: &TypeIndex<'_>,
) -> Option<GenericConstraints> {
    let (generic_params, where_clauses) = collect_generic_context(knowledge, api, type_index);
    if generic_params.is_empty() {
        return None;
    }

    let (lifetime_params, type_params): (Vec<_>, Vec<_>) = generic_params
        .into_iter()
        .partition(|param| param.starts_with('\''));

    if type_params.is_empty() && lifetime_params.is_empty() {
        return None;
    }

    Some(GenericConstraints {
        params: type_params
            .iter()
            .map(|param| build_generic_param(param.as_str(), &where_clauses, trait_index))
            .collect(),
        lifetime_params,
    })
}
```

Keep `build_generic_param` unchanged except that it should now only receive non-lifetime params.

- [ ] **Step 4: Run the contract regression set**

Run:

```bash
cargo test -p s3-model contract_builder_separates_lifetime_params_from_synthesis_params -- --nocapture
cargo test -p s3-model contract_builder_includes_owner_type_generic_constraints_for_assoc_functions -- --nocapture
cargo test -p s3-model contract_builder_normalizes_receiver_side_effects_and_keeps_panics_separate -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit the contract generic split**

```bash
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement add \
  crates/s3-model/src/contracts.rs \
  crates/s3-model/tests/minimal_pipeline.rs
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement commit -m \
  "feat: separate lifetime params in phase2 contracts"
```

### Task 4: Tighten SLM and FCG Output Semantics

**Files:**
- Modify: `crates/s3-model/src/slm.rs`
- Modify: `crates/s3-model/src/fcg.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add a failing SLM regression for weak full models**

Add this test to `crates/s3-model/tests/minimal_pipeline.rs`:

```rust
#[test]
fn slm_downgrades_drop_only_types_without_runtime_states() {
    use seraph_types::{CodeRef, DocSections, TypeInfo, TypeKind};

    let mut knowledge = fixture_knowledge();

    knowledge.types.push(TypeInfo {
        type_id: seraph_types::TypeId::from("type::demo::drop::DropBomb"),
        name: "DropBomb".into(),
        canonical_path: "demo::drop::DropBomb".into(),
        public_paths: vec!["demo::drop::DropBomb".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::drop"),
        code_ref: CodeRef {
            file: "src/drop.rs".into(),
            start_line: 1,
            end_line: 8,
        },
        docs: "Stateful drop bomb.".into(),
        doc_sections: DocSections::default(),
        kind: TypeKind::Struct,
        generic_params: vec![],
        where_clauses: vec![],
        is_non_exhaustive: false,
        fields: vec![],
        variants: vec![],
        has_hidden_fields: false,
        has_hidden_variants: false,
    });
    knowledge.apis.push(seraph_types::ApiInfo {
        api_id: seraph_types::ApiId::from("api::demo::drop::DropBomb::new"),
        name: "new".into(),
        canonical_path: "demo::drop::DropBomb::new".into(),
        public_paths: vec!["demo::drop::DropBomb::new".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::drop"),
        owner_type_id: Some(seraph_types::TypeId::from("type::demo::drop::DropBomb")),
        owner_trait_id: None,
        code_ref: CodeRef {
            file: "src/drop.rs".into(),
            start_line: 10,
            end_line: 10,
        },
        docs: "Creates a drop bomb.".into(),
        doc_sections: DocSections::default(),
        api_kind: seraph_types::ApiKind::Constructor,
        signature_text: "pub fn new() -> DropBomb".into(),
        receiver: None,
        generic_params: vec![],
        where_clauses: vec![],
        arg_types: vec![],
        return_type: Some("DropBomb".into()),
        return_shape: Some(seraph_types::ReturnShape {
            kind: seraph_types::ReturnShapeKind::Nominal,
            inner_types: vec!["DropBomb".into()],
        }),
        is_unsafe: false,
        is_async: false,
        is_const: false,
        has_body: true,
        contains_unsafe_block: false,
    });
    knowledge
        .risk_facts
        .drop_impl_types
        .push(seraph_types::TypeId::from("type::demo::drop::DropBomb"));

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let model = models
        .slm
        .iter()
        .find(|model| model.path == "demo::drop::DropBomb")
        .unwrap();

    assert_eq!(model.model_kind, seraph_types::SlmModelKind::Simplified);
    assert_eq!(model.states, vec!["Constructed".to_string()]);
}
```

- [ ] **Step 2: Add a failing FCG regression for weak cross-anchor query chains and long one-liners**

Add this test to `crates/s3-model/tests/minimal_pipeline.rs`:

```rust
#[test]
fn fcg_avoids_query_to_query_cross_anchor_edges_and_compresses_one_liner() {
    use seraph_types::{ApiInfo, ApiKind, CodeRef, DocSections, ReturnShape, ReturnShapeKind};

    let mut knowledge = fixture_knowledge();

    knowledge.types.push(seraph_types::TypeInfo {
        type_id: seraph_types::TypeId::from("type::demo::status::Status"),
        name: "Status".into(),
        canonical_path: "demo::status::Status".into(),
        public_paths: vec!["demo::status::Status".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::status"),
        code_ref: CodeRef {
            file: "src/status.rs".into(),
            start_line: 1,
            end_line: 8,
        },
        docs: "Simple status value.".into(),
        doc_sections: DocSections::default(),
        kind: seraph_types::TypeKind::Struct,
        generic_params: vec![],
        where_clauses: vec![],
        is_non_exhaustive: false,
        fields: vec![],
        variants: vec![],
        has_hidden_fields: false,
        has_hidden_variants: false,
    });

    knowledge.apis.push(ApiInfo {
        api_id: seraph_types::ApiId::from("api::demo::status::Status::code"),
        name: "code".into(),
        canonical_path: "demo::status::Status::code".into(),
        public_paths: vec!["demo::status::Status::code".into()],
        public_anchor_module_id: seraph_types::ModuleId::from("mod::demo::status"),
        owner_type_id: Some(seraph_types::TypeId::from("type::demo::status::Status")),
        owner_trait_id: None,
        code_ref: CodeRef {
            file: "src/status.rs".into(),
            start_line: 10,
            end_line: 10,
        },
        docs: "Returns the numeric code.".into(),
        doc_sections: DocSections::default(),
        api_kind: ApiKind::InherentMethod,
        signature_text: "pub fn code(&self) -> u32".into(),
        receiver: Some("&self".into()),
        generic_params: vec![],
        where_clauses: vec![],
        arg_types: vec![],
        return_type: Some("u32".into()),
        return_shape: Some(ReturnShape {
            kind: ReturnShapeKind::Primitive,
            inner_types: vec![],
        }),
        is_unsafe: false,
        is_async: false,
        is_const: false,
        has_body: true,
        contains_unsafe_block: false,
    });

    let pointer = knowledge
        .apis
        .iter_mut()
        .find(|api| api.canonical_path == "demo::query::Document::pointer")
        .unwrap();
    pointer.return_type = Some("Option<demo::status::Status>".into());
    pointer.return_shape = Some(ReturnShape {
        kind: ReturnShapeKind::Option,
        inner_types: vec!["demo::status::Status".into()],
    });

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let document_query = models
        .fcg
        .capabilities
        .iter()
        .find(|cap| cap.cap_id == seraph_types::CapId::from("cap::demo::query::Document::query"))
        .unwrap();

    assert!(document_query.connects_to.is_empty());
    assert!(models.fcg.stage1_summary.one_liner.len() <= 80);
}
```

- [ ] **Step 3: Run the new regressions and confirm they fail**

Run:

```bash
cargo test -p s3-model slm_downgrades_drop_only_types_without_runtime_states -- --nocapture
cargo test -p s3-model fcg_avoids_query_to_query_cross_anchor_edges_and_compresses_one_liner -- --nocapture
```

Expected:

- first test fails because current `score_type` can still emit `full` for drop-only types
- second test fails because current `fcg.rs` still creates loose cross-anchor edges and lists every capability in `one_liner`

- [ ] **Step 4: Tighten `slm.rs` and `fcg.rs`**

In `crates/s3-model/src/slm.rs`, classify `full` only when it has real planning value:

```rust
let stateful_labels = states
    .iter()
    .filter(|state| !matches!(state.as_str(), "Constructed" | "InUse"))
    .count();
let has_non_constructor_transition = transitions.iter().any(|transition| {
    !(transition.from == "Uninitialized" && transition.to == "Constructed")
        && !(transition.from == "Constructed" && transition.to == "Constructed")
});
let qualifies_for_full =
    stateful_labels > 0 && (has_non_constructor_transition || !forbidden_transitions.is_empty());
let model_kind = if qualifies_for_full {
    SlmModelKind::Full
} else {
    SlmModelKind::Simplified
};
```

In `crates/s3-model/src/fcg.rs`, only create cross-anchor edges when the source capability is a strong handoff point and the target is not a weak query-to-query hop:

```rust
fn role_allows_cross_anchor(role: CapabilityRole) -> bool {
    matches!(role, CapabilityRole::Construction | CapabilityRole::Conversion)
}
```

Use it in the edge loop:

```rust
if !role_allows_cross_anchor(capability.role) {
    continue;
}
```

and compress the one-liner by selecting at most 3 featured capability names:

```rust
fn summarize_capabilities(capabilities: &[CapabilityDraft], recommended: &[Vec<CapId>]) -> String {
    if capabilities.is_empty() {
        return "本库当前没有可建模 capability。".to_owned();
    }

    let featured = select_featured_capabilities(capabilities, recommended, 3);
    let featured_names = featured
        .iter()
        .map(|cap| cap.name.as_str())
        .collect::<Vec<_>>()
        .join("、");

    if capabilities.len() <= featured.len() {
        format!("本库提供 {} 项核心能力，重点围绕 {}。", capabilities.len(), featured_names)
    } else {
        format!(
            "本库提供 {} 项核心能力，重点围绕 {}，另有其他能力。",
            capabilities.len(),
            featured_names
        )
    }
}
```

- [ ] **Step 5: Run the focused SLM/FCG suite**

Run:

```bash
cargo test -p s3-model slm_ -- --nocapture
cargo test -p s3-model fcg_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit the semantic tightening**

```bash
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement add \
  crates/s3-model/src/slm.rs \
  crates/s3-model/src/fcg.rs \
  crates/s3-model/tests/minimal_pipeline.rs
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement commit -m \
  "feat: tighten phase2 slm and capability graph semantics"
```

### Task 5: Refresh Golden Outputs, README, and Real-Crate Validation

**Files:**
- Modify: `crates/s3-model/tests/fixtures/minimal_models.json`
- Modify: `crates/s3-model/README.md`
- Test: integration tests plus real-crate CLI outputs

- [ ] **Step 1: Run the golden test and confirm it fails before refreshing the fixture**

Run:

```bash
cargo test -p s3-model models_output_matches_golden_fixture -- --nocapture
```

Expected: FAIL because the schema and builders now emit `lifetime_params`, `types_affected`, tighter `slm`, and a shorter `one_liner`.

- [ ] **Step 2: Regenerate `minimal_models.json` from the current builder output**

Run:

```bash
cargo run -p s3-model -- \
  --input crates/s3-model/tests/fixtures/minimal_knowledge.json \
  --output /tmp/seraph-minimal-models.json
cp /tmp/seraph-minimal-models.json \
  crates/s3-model/tests/fixtures/minimal_models.json
```

- [ ] **Step 3: Update `README.md` so it matches the new semantics**

Replace the `Phase 2 model shape` bullets in `crates/s3-model/README.md` with:

```md
- `fcg`
  - type-centered capability graph keyed by `cap_id`
  - cross-anchor edges are conservative and biased toward real handoff flows
  - `stage1_summary.one_liner` is a short digest, while `capability_cards` remains the detailed view
- `slm`
  - emits `full` only when a lifecycle model contains useful nontrivial states or forbidden transitions
  - downgrades weak constructor-only models to `simplified`
- `api_contracts`
  - keeps `preconditions`, `panic_conditions`, `error_conditions`, `side_effects`, and `generic_constraints`
  - `generic_constraints.params` contains synthesis-relevant type params only
  - `generic_constraints.lifetime_params` keeps explicit borrow facts separate
- `risk_surface_map`
  - `api_risks` stays API-centric
  - `rust_feature_risks` can now report either `apis_affected` or `types_affected`
```

- [ ] **Step 4: Run the full automated test suite for Phase 2**

Run:

```bash
cargo test -p seraph-types -p s3-model -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run real-crate validation for `s3-audit-fixture`, `hashbrown`, `moonfire-ffmpeg`, and `semver`**

Run:

```bash
mkdir -p /tmp/seraph-phase2-post-audit/{audit_fixture,hashbrown,moonfire_ffmpeg,semver}

cargo run -p s3-extract -- \
  --manifest-path examples/target-crates/s3-audit-fixture/Cargo.toml \
  --output /tmp/seraph-phase2-post-audit/audit_fixture/knowledge.json
cargo run -p s3-model -- \
  --input /tmp/seraph-phase2-post-audit/audit_fixture/knowledge.json \
  --output /tmp/seraph-phase2-post-audit/audit_fixture/models.json

cargo run -p s3-extract -- \
  --manifest-path examples/target-crates/hashbrown/Cargo.toml \
  --output /tmp/seraph-phase2-post-audit/hashbrown/knowledge.json
cargo run -p s3-model -- \
  --input /tmp/seraph-phase2-post-audit/hashbrown/knowledge.json \
  --output /tmp/seraph-phase2-post-audit/hashbrown/models.json

cargo run -p s3-extract -- \
  --manifest-path /tmp/seraph-real-crates/moonfire-ffmpeg/Cargo.toml \
  --output /tmp/seraph-phase2-post-audit/moonfire_ffmpeg/knowledge.json
cargo run -p s3-model -- \
  --input /tmp/seraph-phase2-post-audit/moonfire_ffmpeg/knowledge.json \
  --output /tmp/seraph-phase2-post-audit/moonfire_ffmpeg/models.json

cargo run -p s3-extract -- \
  --manifest-path /tmp/seraph-real-crates/semver/Cargo.toml \
  --output /tmp/seraph-phase2-post-audit/semver/knowledge.json
cargo run -p s3-model -- \
  --input /tmp/seraph-phase2-post-audit/semver/knowledge.json \
  --output /tmp/seraph-phase2-post-audit/semver/models.json
```

Then run this validation script:

```bash
python3 - <<'PY'
import json
from pathlib import Path

root = Path("/tmp/seraph-phase2-post-audit")

audit = json.loads((root / "audit_fixture" / "models.json").read_text())
risks = {r["feature"]: r for r in audit["risk_surface_map"]["rust_feature_risks"]}
assert "panic_in_drop" in risks, risks
assert risks["panic_in_drop"]["apis_affected"] == [], risks["panic_in_drop"]
assert len(risks["panic_in_drop"]["types_affected"]) == 1, risks["panic_in_drop"]
assert risks["conditional_impl"]["apis_affected"] == [], risks["conditional_impl"]
assert len(risks["conditional_impl"]["types_affected"]) >= 1, risks["conditional_impl"]

hashbrown = json.loads((root / "hashbrown" / "models.json").read_text())
one_liner = hashbrown["fcg"]["stage1_summary"]["one_liner"]
assert len(one_liner) <= 120, one_liner
cap_roles = {cap["cap_id"]: cap["role"] for cap in hashbrown["fcg"]["capabilities"]}
for chain in hashbrown["fcg"]["recommended_chains"]:
    assert chain, chain
    assert cap_roles[chain[0]] == "construction", chain

moonfire = json.loads((root / "moonfire_ffmpeg" / "models.json").read_text())
for model in moonfire["slm"]:
    if model["model_kind"] != "full":
        continue
    nontrivial_states = [s for s in model["states"] if s not in ("Constructed", "InUse")]
    has_non_constructor_transition = any(
        not (t["from"] == "Uninitialized" and t["to"] == "Constructed")
        and not (t["from"] == "Constructed" and t["to"] == "Constructed")
        for t in model["transitions"]
    )
    assert nontrivial_states, model
    assert has_non_constructor_transition or model["forbidden_transitions"], model

semver = json.loads((root / "semver" / "models.json").read_text())
assert semver["slm"] == [], semver["slm"]

print("phase2 real-crate validation passed")
PY
```

Expected: script prints `phase2 real-crate validation passed`.

- [ ] **Step 6: Commit the fixtures, docs, and validation-backed result**

```bash
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement add \
  crates/s3-model/tests/fixtures/minimal_models.json \
  crates/s3-model/README.md \
  crates/s3-model/tests/minimal_pipeline.rs
git -C /home/cas/Desktop/SERAPH/.worktrees/phase2-post-audit-refinement commit -m \
  "test: refresh phase2 fixtures and validation coverage"
```

## Self-Review Checklist

- Spec coverage:
  - `lifetime_params`: Task 1 + Task 3
  - `types_affected`: Task 1 + Task 2
  - `panic_in_drop` / `conditional_impl` 不再丢失：Task 2 + Task 5
  - `SLM full` 收紧：Task 4 + Task 5
  - `FCG` 跨锚点链收紧：Task 4 + Task 5
  - `one_liner` 压缩：Task 4 + Task 5
- Placeholder scan:
  - 没有占位词
  - 每个改动步骤都给了精确代码块或命令
- Type consistency:
  - 新字段名始终使用 `lifetime_params`
  - 新字段名始终使用 `types_affected`
  - `RustFeatureRisk` 仍保留 `apis_affected`
