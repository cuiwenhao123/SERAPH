# Phase 2 V5 Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于当前已经可运行的 `s3-model`，把 Phase 2 从“模块中心 capability + 简化摘要”升级到 `semantic_harness_agent_design_v5.md` 当前定义的“类型中心 capability + 结构化 stage1 摘要 + 与 `Knowledge` 真实 schema 对齐”的版本。

**Architecture:** 保持 `crates/seraph-types/src/models.rs` 作为唯一 Phase 2 输出契约，继续使用 `crates/s3-model/src/fcg.rs`、`slm.rs`、`contracts.rs`、`risk.rs` 四个 builder 组装 `Models`。这次不是从零重写，而是在现有实现上逐步替换 FCG 表示、补齐 contracts/risk/slm 缺口，并同步 fixtures、README 与 golden tests。

**Tech Stack:** Rust 2021, `serde`, `serde_json`, `seraph-types`, crate-local unit tests, integration tests, golden JSON fixtures, `cargo test`

---

## Scope Guard

- 当前工作区已经存在未提交修改：
  - `crates/s3-model/src/contracts.rs`
  - `crates/s3-model/src/fcg.rs`
  - `crates/s3-model/src/risk.rs`
  - `crates/s3-model/src/slm.rs`
  - `crates/s3-model/tests/fixtures/minimal_models.json`
  - `crates/s3-model/tests/minimal_pipeline.rs`
  - `semantic_harness_agent_design_v5.md`
- 实施本计划时只能**在这些基础上继续演进**，不能回退用户已有修改。
- 不能把 `examples/target-crates/hashbrown/` 加入提交。
- 本计划**只覆盖 Phase 2**：`seraph-types::Models`、`s3-model`、tests、README；不实现 Phase 3 orchestration。

## File Structure

### Existing files to modify

- `crates/seraph-types/src/models.rs`
  需要把 FCG schema 从“模块桶”升级为“锚点 + role + 结构化 stage1 摘要”。
- `crates/seraph-types/tests/schema_roundtrip.rs`
  为新的 Phase 2 schema 提供 roundtrip 回归。
- `crates/s3-model/src/fcg.rs`
  从按 module 聚类的 capability builder 改成按 `type/module_entry + role` 建模。
- `crates/s3-model/src/contracts.rs`
  补齐 preconditions / side_effects / generic context / associated type constraints 的确定性抽取。
- `crates/s3-model/src/slm.rs`
  把现有 full/simplified SLM 再收紧到 v5 语义。
- `crates/s3-model/src/risk.rs`
  将 risk aggregation 对齐到 v5，补齐 `conditional_impl` 和 `panic_in_drop`。
- `crates/s3-model/tests/minimal_pipeline.rs`
  增加针对 type-centered FCG、contracts、SLM、risk 的增量断言。
- `crates/s3-model/tests/fixtures/minimal_models.json`
  刷新 golden fixture。
- `crates/s3-model/README.md`
  更新 Phase 2 输出字段说明，避免 README 仍停留在旧 FCG 心智。

### Files to leave alone

- `crates/s3-model/src/io.rs`
- `crates/s3-model/src/main.rs`
- `crates/s3-model/src/lib.rs`
  除非 schema 改动导致非常轻量的导出调整，否则不要扩大改动面。

## Plan Outcome Checklist

实现结束后，应同时满足：

1. `Models.fcg` 使用 v5 的 type-centered capability schema。
2. `stage1_summary` 变成结构化对象，不再是单个字符串。
3. `fcg.rs` 能稳定产出：
   - type anchor capability
   - module entry capability
   - role 分类
   - capability chain
4. `contracts.rs` 不再把 `preconditions` 留空，且能消费 `trait_impl_registry`。
5. `slm.rs` 能区分 full / simplified，并且只在有恢复路径时引入 `Error`。
6. `risk.rs` 至少覆盖：
   - `conditional_impl`
   - `extern_abi`
   - `borrowed_return`
   - `repr_packed`
   - `panic_in_drop`
7. `cargo test -p seraph-types -p s3-model -- --nocapture` 通过。

### Task 1: Freeze Phase 2 Schema Around Type-Centered FCG

**Files:**
- Modify: `crates/seraph-types/src/models.rs`
- Modify: `crates/seraph-types/tests/schema_roundtrip.rs`
- Test: `crates/seraph-types/tests/schema_roundtrip.rs`

- [ ] **Step 1: Write the failing schema roundtrip test**

```rust
#[test]
fn models_roundtrip_preserves_type_centered_fcg_fields() {
    use seraph_types::{
        ApiContract, ApiId, ApiRisk, BugHuntingValue, CapId, CapabilityAnchorKind,
        CapabilityNode, CapabilityRole, ForbiddenTransition, FunctionalCapabilityGraph,
        GenericConstraintParam, GenericConstraints, Models, RiskLevel, RiskSurfaceMap,
        RustFeatureRisk, SlmModelKind, Stage1CapabilityCard, Stage1Summary,
        StateLifecycleModel, StateTransition, TypeId, TypeSynthesisOverview,
    };
    use std::collections::BTreeMap;

    let models = Models {
        fcg: FunctionalCapabilityGraph {
            capabilities: vec![CapabilityNode {
                cap_id: CapId::from("cap::demo::query::Document::query"),
                anchor_kind: CapabilityAnchorKind::Type,
                anchor_module_id: "mod::demo::query".into(),
                anchor_type_id: Some("type::demo::query::Document".into()),
                role: CapabilityRole::Query,
                name: "Document 查询".into(),
                description: "围绕 Document 的只读查询".into(),
                api_ids: vec![ApiId::from("api::demo::query::Document::pointer")],
                entry_api_ids: vec![],
                connects_to: vec![],
            }],
            capability_chains: vec![],
            capability_api_index: BTreeMap::from([(
                CapId::from("cap::demo::query::Document::query"),
                vec![ApiId::from("api::demo::query::Document::pointer")],
            )]),
            stage1_summary: Stage1Summary {
                capability_cards: vec![Stage1CapabilityCard {
                    cap_id: CapId::from("cap::demo::query::Document::query"),
                    name: "Document 查询".into(),
                    anchor_path: "demo::query::Document".into(),
                    role: CapabilityRole::Query,
                    description: "围绕 Document 的只读查询".into(),
                }],
                recommended_chains: vec![],
                one_liner: "本库提供 1 项核心能力：Document 查询。".into(),
            },
        },
        slm: vec![StateLifecycleModel {
            type_id: TypeId::from("type::demo::query::Parser"),
            path: "demo::query::Parser".into(),
            model_kind: SlmModelKind::Full,
            states: vec!["Constructed".into(), "Active".into(), "Closed".into()],
            transitions: vec![StateTransition {
                from: "Constructed".into(),
                to: "Active".into(),
                via_api_id: ApiId::from("api::demo::query::Parser::start"),
                preconditions: vec![],
            }],
            fuzzable_states: vec!["Active".into()],
            forbidden_transitions: vec![ForbiddenTransition {
                from: "Closed".into(),
                via_api_id: ApiId::from("api::demo::query::Parser::start"),
                reason: "documented panic".into(),
            }],
        }],
        api_contracts: vec![ApiContract {
            api_id: ApiId::from("api::demo::io::from_reader"),
            path: "demo::io::from_reader".into(),
            preconditions: vec!["input must remain valid".into()],
            postconditions: vec!["returns Result<Document, Error>".into()],
            panic_conditions: vec![],
            error_conditions: vec!["malformed input".into()],
            safety: None,
            side_effects: vec!["consumes input bytes".into()],
            generic_constraints: Some(GenericConstraints {
                params: vec![GenericConstraintParam {
                    name: "R".into(),
                    direct_bounds: vec!["demo::io::Reader".into()],
                    full_bound_chain: vec!["demo::io::Reader".into()],
                    associated_type_constraints: vec![],
                    is_unsafe_trait: false,
                    strategy: "C".into(),
                    bug_hunting_value: BugHuntingValue::High,
                    synthesis_guidance: "custom reader with short-read".into(),
                }],
            }),
        }],
        risk_surface_map: RiskSurfaceMap {
            api_risks: vec![ApiRisk {
                api_id: ApiId::from("api::demo::io::from_reader"),
                risk_level: RiskLevel::High,
                reasons: vec!["external input surface".into()],
                recommended_fuzz_strategy: "raw bytes + custom reader".into(),
            }],
            type_synthesis_overview: TypeSynthesisOverview {
                generic_api_count: 1,
                strategy_distribution: BTreeMap::from([("C".into(), 1)]),
                one_liner: "1 个泛型 API，其中 1 个适合自定义类型合成".into(),
            },
            rust_feature_risks: vec![RustFeatureRisk {
                feature: "extern_abi".into(),
                apis_affected: vec![ApiId::from("api::demo::io::ffi_probe")],
                risk: "public API crosses extern ABI".into(),
            }],
        },
    };

    let json = serde_json::to_string_pretty(&models).unwrap();
    let decoded: Models = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.fcg.capabilities[0].role, CapabilityRole::Query);
    assert_eq!(
        decoded.fcg.stage1_summary.capability_cards[0].anchor_path,
        "demo::query::Document"
    );
    assert_eq!(decoded.slm[0].model_kind, SlmModelKind::Full);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p seraph-types models_roundtrip_preserves_type_centered_fcg_fields -- --nocapture`
Expected: FAIL because `CapabilityAnchorKind`, `CapabilityRole`, `Stage1Summary`, and the new `CapabilityNode` fields do not exist yet.

- [ ] **Step 3: Upgrade `models.rs` to the v5 FCG contract**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionalCapabilityGraph {
    pub capabilities: Vec<CapabilityNode>,
    pub capability_chains: Vec<Vec<CapId>>,
    pub capability_api_index: BTreeMap<CapId, Vec<ApiId>>,
    pub stage1_summary: Stage1Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityNode {
    pub cap_id: CapId,
    pub anchor_kind: CapabilityAnchorKind,
    pub anchor_module_id: ModuleId,
    pub anchor_type_id: Option<TypeId>,
    pub role: CapabilityRole,
    pub name: String,
    pub description: String,
    pub api_ids: Vec<ApiId>,
    pub entry_api_ids: Vec<ApiId>,
    pub connects_to: Vec<CapId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityAnchorKind {
    ModuleEntry,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityRole {
    Construction,
    Query,
    Mutation,
    Iteration,
    Conversion,
    Finalization,
    Ffi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage1Summary {
    pub capability_cards: Vec<Stage1CapabilityCard>,
    pub recommended_chains: Vec<Vec<CapId>>,
    pub one_liner: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage1CapabilityCard {
    pub cap_id: CapId,
    pub name: String,
    pub anchor_path: String,
    pub role: CapabilityRole,
    pub description: String,
}
```

- [ ] **Step 4: Run the schema tests**

Run: `cargo test -p seraph-types -- --nocapture`
Expected: PASS, including the new type-centered FCG roundtrip test.

- [ ] **Step 5: Commit the schema-only change**

```bash
git -C /home/cas/Desktop/SERAPH add \
  crates/seraph-types/src/models.rs \
  crates/seraph-types/tests/schema_roundtrip.rs
git -C /home/cas/Desktop/SERAPH commit -m "feat: lock phase2 type-centered fcg schema"
```

### Task 2: Rebuild FCG Around `type/module_entry + role`

**Files:**
- Modify: `crates/s3-model/src/fcg.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Modify: `crates/s3-model/tests/fixtures/minimal_models.json`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add a failing integration test for type-centered capability grouping**

```rust
#[test]
fn fcg_emits_type_centered_capabilities_and_structured_stage1_summary() {
    let knowledge = fixture_knowledge();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    let io_entry = models
        .fcg
        .capabilities
        .iter()
        .find(|cap| cap.cap_id == seraph_types::CapId::from("cap::demo::io::module_entry::construction"))
        .unwrap();
    assert_eq!(io_entry.anchor_kind, seraph_types::CapabilityAnchorKind::ModuleEntry);
    assert_eq!(io_entry.role, seraph_types::CapabilityRole::Construction);

    let document_query = models
        .fcg
        .capabilities
        .iter()
        .find(|cap| cap.cap_id == seraph_types::CapId::from("cap::demo::query::Document::query"))
        .unwrap();
    assert_eq!(document_query.anchor_kind, seraph_types::CapabilityAnchorKind::Type);
    assert_eq!(document_query.anchor_type_id, Some("type::demo::query::Document".into()));
    assert_eq!(document_query.role, seraph_types::CapabilityRole::Query);

    assert_eq!(
        models.fcg.stage1_summary.capability_cards[0].cap_id,
        io_entry.cap_id
    );
    assert!(
        models
            .fcg
            .stage1_summary
            .one_liner
            .contains("本库提供")
    );
}
```

- [ ] **Step 2: Run the focused FCG test and watch it fail**

Run: `cargo test -p s3-model fcg_emits_type_centered_capabilities_and_structured_stage1_summary -- --nocapture`
Expected: FAIL because the current builder still emits `cap::demo::io` / `cap::demo::query` module buckets and `stage1_summary` is still a string.

- [ ] **Step 3: Replace module-bucket capability construction with anchor + role grouping**

```rust
fn classify_role(api: &ApiInfo, knowledge: &Knowledge) -> CapabilityRole {
    if knowledge
        .risk_facts
        .extern_abi_apis
        .iter()
        .any(|fact| fact.api_id == api.api_id)
    {
        return CapabilityRole::Ffi;
    }
    if matches!(api.api_kind, ApiKind::Constructor)
        || matches!(api.api_kind, ApiKind::FreeFunction | ApiKind::AssocFunction)
            && returns_owner_type(api)
    {
        return CapabilityRole::Construction;
    }
    if receiver_is_mut(api.receiver.as_deref()) {
        return CapabilityRole::Mutation;
    }
    if looks_like_iteration(api) {
        return CapabilityRole::Iteration;
    }
    if looks_like_finalization(api.name.as_str()) {
        return CapabilityRole::Finalization;
    }
    if looks_like_conversion(api) {
        return CapabilityRole::Conversion;
    }
    CapabilityRole::Query
}

fn capability_id(anchor_path: &str, anchor_kind: CapabilityAnchorKind, role: CapabilityRole) -> CapId {
    match anchor_kind {
        CapabilityAnchorKind::ModuleEntry => {
            CapId::from(format!("cap::{anchor_path}::module_entry::{}", role_slug(role)))
        }
        CapabilityAnchorKind::Type => {
            CapId::from(format!("cap::{anchor_path}::{}", role_slug(role)))
        }
    }
}
```

- [ ] **Step 4: Build structured `Stage1Summary` instead of a single string**

```rust
fn build_stage1_summary(
    capabilities: &[CapabilityNode],
    capability_chains: &[Vec<CapId>],
) -> Stage1Summary {
    let capability_cards = capabilities
        .iter()
        .map(|cap| Stage1CapabilityCard {
            cap_id: cap.cap_id.clone(),
            name: cap.name.clone(),
            anchor_path: anchor_path_for_capability(cap),
            role: cap.role,
            description: cap.description.clone(),
        })
        .collect::<Vec<_>>();

    Stage1Summary {
        capability_cards,
        recommended_chains: capability_chains.to_vec(),
        one_liner: format!("本库提供 {} 项核心能力。", capabilities.len()),
    }
}
```

- [ ] **Step 5: Refresh the golden fixture**

```bash
cargo test -p s3-model fcg_emits_type_centered_capabilities_and_structured_stage1_summary -- --nocapture
cargo test -p s3-model models_json_roundtrip_through_s3_model_io -- --nocapture
```

Expected: both PASS after `crates/s3-model/tests/fixtures/minimal_models.json` is updated to the new FCG shape.

- [ ] **Step 6: Commit the FCG migration**

```bash
git -C /home/cas/Desktop/SERAPH add \
  crates/s3-model/src/fcg.rs \
  crates/s3-model/tests/minimal_pipeline.rs \
  crates/s3-model/tests/fixtures/minimal_models.json
git -C /home/cas/Desktop/SERAPH commit -m "feat: migrate phase2 fcg to type-centered capabilities"
```

### Task 3: Align `contracts.rs` With V5 Deterministic Rules

**Files:**
- Modify: `crates/s3-model/src/contracts.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Modify: `crates/s3-model/tests/fixtures/minimal_models.json`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add focused failing tests for preconditions and trait impl aware generic constraints**

```rust
#[test]
fn contracts_extract_preconditions_side_effects_and_owner_generic_context() {
    let knowledge = fixture_knowledge();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let start = models
        .api_contracts
        .iter()
        .find(|contract| contract.path == "demo::query::Parser::start")
        .unwrap();

    assert!(start.panic_conditions[0].contains("already closed"));
    assert!(start.preconditions.iter().any(|item| item.contains("already closed")));
    assert!(start.side_effects.iter().any(|item| item == "mutates receiver"));
}

#[test]
fn contracts_consume_trait_impl_registry_for_conditional_generic_surface() {
    let knowledge = fixture_knowledge();
    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    let from_reader = models
        .api_contracts
        .iter()
        .find(|contract| contract.path == "demo::io::from_reader")
        .unwrap();

    let param = &from_reader.generic_constraints.as_ref().unwrap().params[0];
    assert_eq!(param.name, "R");
    assert!(param.full_bound_chain.iter().any(|item| item.contains("Reader")));
    assert!(!param.associated_type_constraints.is_empty());
}
```

- [ ] **Step 2: Run the focused contracts tests**

Run: `cargo test -p s3-model contracts_ -- --nocapture`
Expected: FAIL because `derive_preconditions` still returns `Vec::new()` and side-effect/generic handling is not fully v5-aligned.

- [ ] **Step 3: Implement deterministic precondition and postcondition extraction**

```rust
fn derive_preconditions(api: &ApiInfo) -> Vec<String> {
    let mut items = Vec::new();

    for line in split_non_empty_lines(&api.doc_sections.panics) {
        if line.to_ascii_lowercase().contains("if ") {
            items.push(line);
        }
    }
    for line in split_non_empty_lines(&api.doc_sections.safety) {
        items.push(line);
    }

    dedup_vec(items)
}

fn derive_postconditions(api: &ApiInfo) -> Vec<String> {
    let mut items = Vec::new();
    if let Some(return_type) = &api.return_type {
        items.push(format!("returns {return_type}"));
    }
    if !api.doc_sections.errors.trim().is_empty() && api.return_type.is_some() {
        items.push("may return a documented error".to_owned());
    }
    dedup_vec(items)
}
```

- [ ] **Step 4: Keep `side_effects` and generic constraints strictly evidence-backed**

```rust
fn derive_side_effects(
    knowledge: &Knowledge,
    api: &ApiInfo,
    generic_constraints: Option<&GenericConstraints>,
) -> Vec<String> {
    let mut side_effects = Vec::new();

    match classify_receiver(api.receiver.as_deref()) {
        ReceiverKind::Mutates => side_effects.push("mutates receiver".to_owned()),
        ReceiverKind::Consumes => side_effects.push("consumes receiver".to_owned()),
        ReceiverKind::Other => {}
    }

    if knowledge
        .risk_facts
        .extern_abi_apis
        .iter()
        .any(|fact| fact.api_id == api.api_id)
    {
        side_effects.push("crosses extern ABI boundary".to_owned());
    }

    if generic_constraints
        .map(|constraints| constraints.params.iter().any(|param| param.strategy == "C"))
        .unwrap_or(false)
    {
        side_effects.push("consumes input bytes".to_owned());
    }

    dedup_vec(side_effects)
}
```

- [ ] **Step 5: Run the contracts-related tests and refresh the fixture**

Run: `cargo test -p s3-model contracts_ -- --nocapture`
Expected: PASS.

Run: `cargo test -p s3-model build_models_from_knowledge_returns_phase2_sections -- --nocapture`
Expected: PASS and `minimal_models.json` updated to match the new contract rows.

- [ ] **Step 6: Commit the contracts alignment**

```bash
git -C /home/cas/Desktop/SERAPH add \
  crates/s3-model/src/contracts.rs \
  crates/s3-model/tests/minimal_pipeline.rs \
  crates/s3-model/tests/fixtures/minimal_models.json
git -C /home/cas/Desktop/SERAPH commit -m "feat: align phase2 contracts with v5 rules"
```

### Task 4: Tighten `slm.rs` to the V5 Lifecycle Rules

**Files:**
- Modify: `crates/s3-model/src/slm.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Modify: `crates/s3-model/tests/fixtures/minimal_models.json`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add failing tests for `Configured` / `Error` gating**

```rust
#[test]
fn slm_only_emits_error_when_recovery_api_exists() {
    let mut knowledge = fixture_knowledge();
    let parser = knowledge
        .types
        .iter_mut()
        .find(|ty| ty.canonical_path == "demo::query::Parser")
        .unwrap();
    parser.docs = "Parser lifecycle with error state and reset support.".into();

    let reset = seraph_types::ApiInfo {
        api_id: "api::demo::query::Parser::reset".into(),
        name: "reset".into(),
        canonical_path: "demo::query::Parser::reset".into(),
        public_paths: vec!["demo::query::Parser::reset".into()],
        public_anchor_module_id: "mod::demo::query".into(),
        owner_type_id: Some("type::demo::query::Parser".into()),
        owner_trait_id: None,
        code_ref: seraph_types::CodeRef { file: "src/query.rs".into(), start_line: 50, end_line: 52 },
        docs: "Resets parser after an error.".into(),
        doc_sections: seraph_types::DocSections::default(),
        api_kind: seraph_types::ApiKind::InherentMethod,
        signature_text: "pub fn reset(&mut self)".into(),
        receiver: Some("&mut self".into()),
        generic_params: vec![],
        where_clauses: vec![],
        arg_types: vec![],
        return_type: Some("()".into()),
        return_shape: Some(seraph_types::ReturnShape {
            kind: seraph_types::ReturnShapeKind::Unit,
            inner_types: vec![],
        }),
        is_unsafe: false,
        is_async: false,
        is_const: false,
        has_body: true,
        contains_unsafe_block: false,
    };
    knowledge.apis.push(reset);

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let parser_model = models.slm.iter().find(|model| model.path == "demo::query::Parser").unwrap();

    assert!(parser_model.states.iter().any(|state| state == "Error"));
}
```

- [ ] **Step 2: Run the SLM-focused tests**

Run: `cargo test -p s3-model slm_ -- --nocapture`
Expected: FAIL because the current builder only emits `Constructed` / `InUse` / `Active` / `Closed`.

- [ ] **Step 3: Implement explicit state expansion with recovery-gated `Error`**

```rust
fn build_states(related_apis: &[&ApiInfo], model_kind: SlmModelKind) -> Vec<String> {
    let mut states = vec!["Constructed".to_owned()];

    if matches!(model_kind, SlmModelKind::Simplified) {
        if related_apis.iter().any(|api| api.receiver.is_some()) {
            states.push("InUse".to_owned());
        }
        if related_apis.iter().any(|api| is_close_like(api.name.as_str())) {
            states.push("Closed".to_owned());
        }
        return states;
    }

    if related_apis.iter().any(|api| is_configure_like(api.name.as_str())) {
        states.push("Configured".to_owned());
    }
    if related_apis.iter().any(|api| !is_constructor_like(api) && is_activate_like(api.name.as_str())) {
        states.push("Active".to_owned());
    }
    if related_apis.iter().any(|api| is_recovery_like(api.name.as_str())) {
        states.push("Error".to_owned());
    }
    if related_apis.iter().any(|api| is_close_like(api.name.as_str())) {
        states.push("Closed".to_owned());
    }

    dedup_vec(states)
}
```

- [ ] **Step 4: Keep forbidden transitions evidence-backed**

```rust
fn build_forbidden_transitions(related_apis: &[&ApiInfo]) -> Vec<ForbiddenTransition> {
    related_apis
        .iter()
        .filter_map(|api| {
            let reason = api.doc_sections.panics.trim();
            if reason.is_empty() {
                return None;
            }

            Some(ForbiddenTransition {
                from: infer_forbidden_from_state(reason),
                via_api_id: api.api_id.clone(),
                reason: reason.to_owned(),
            })
        })
        .collect()
}
```

- [ ] **Step 5: Run SLM and pipeline tests**

Run: `cargo test -p s3-model slm_ -- --nocapture`
Expected: PASS.

Run: `cargo test -p s3-model build_models_from_knowledge_returns_phase2_sections -- --nocapture`
Expected: PASS and `minimal_models.json` updated if state labels changed.

- [ ] **Step 6: Commit the SLM alignment**

```bash
git -C /home/cas/Desktop/SERAPH add \
  crates/s3-model/src/slm.rs \
  crates/s3-model/tests/minimal_pipeline.rs \
  crates/s3-model/tests/fixtures/minimal_models.json
git -C /home/cas/Desktop/SERAPH commit -m "feat: align phase2 slm with v5 lifecycle rules"
```

### Task 5: Expand `risk.rs` to the Full V5 Rust Feature Risk Set

**Files:**
- Modify: `crates/s3-model/src/risk.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Modify: `crates/s3-model/tests/fixtures/minimal_models.json`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add failing tests for `conditional_impl` and `panic_in_drop`**

```rust
#[test]
fn risk_surface_reports_conditional_impl_and_panic_in_drop() {
    let knowledge = fixture_knowledge();
    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    assert!(models.risk_surface_map.rust_feature_risks.iter().any(|risk| risk.feature == "conditional_impl"));
    assert!(models.risk_surface_map.rust_feature_risks.iter().any(|risk| risk.feature == "panic_in_drop"));
}
```

- [ ] **Step 2: Run the focused risk tests**

Run: `cargo test -p s3-model risk_surface_reports_conditional_impl_and_panic_in_drop -- --nocapture`
Expected: FAIL because `risk.rs` currently only emits `extern_abi`, `borrowed_return`, and `repr_packed`.

- [ ] **Step 3: Derive `conditional_impl` from `trait_impl_registry.cfg_attrs`**

```rust
let conditional_impl_apis = knowledge
    .trait_impl_registry
    .iter()
    .filter(|impl_info| !impl_info.cfg_attrs.is_empty())
    .flat_map(|impl_info| {
        knowledge
            .apis
            .iter()
            .filter(move |api| api.owner_type_id.as_ref() == Some(&impl_info.target_type_id))
            .map(|api| api.api_id.clone())
    })
    .collect::<Vec<_>>();

if !conditional_impl_apis.is_empty() {
    risks.push(RustFeatureRisk {
        feature: "conditional_impl".to_owned(),
        apis_affected: conditional_impl_apis,
        risk: "public surface depends on cfg-gated impl availability".to_owned(),
    });
}
```

- [ ] **Step 4: Derive `panic_in_drop` from explicit panic sites owned by Drop impls**

```rust
let drop_impl_ids = knowledge
    .trait_impl_registry
    .iter()
    .filter(|impl_info| impl_info.trait_canonical_path == "core::ops::drop::Drop")
    .map(|impl_info| impl_info.trait_impl_id.clone())
    .collect::<BTreeSet<_>>();

let panic_in_drop_apis = knowledge
    .apis
    .iter()
    .filter_map(|api| {
        api.owner_type_id.as_ref().filter(|owner_type| {
            knowledge
                .trait_impl_registry
                .iter()
                .any(|impl_info| {
                    &impl_info.target_type_id == *owner_type
                        && drop_impl_ids.contains(&impl_info.trait_impl_id)
                        && knowledge.risk_facts.explicit_panic_sites.iter().any(|fact| {
                            matches!(&fact.owner, seraph_types::RiskOwner::TraitImpl(id) if id == &impl_info.trait_impl_id)
                        })
                })
        })?;
        Some(api.api_id.clone())
    })
    .collect::<Vec<_>>();
```

- [ ] **Step 5: Run the risk tests and refresh the fixture**

Run: `cargo test -p s3-model risk_ -- --nocapture`
Expected: PASS.

Run: `cargo test -p s3-model -- --nocapture`
Expected: PASS with the refreshed `minimal_models.json`.

- [ ] **Step 6: Commit the risk alignment**

```bash
git -C /home/cas/Desktop/SERAPH add \
  crates/s3-model/src/risk.rs \
  crates/s3-model/tests/minimal_pipeline.rs \
  crates/s3-model/tests/fixtures/minimal_models.json
git -C /home/cas/Desktop/SERAPH commit -m "feat: expand phase2 risk surface to v5 coverage"
```

### Task 6: Refresh README and Run Full Verification

**Files:**
- Modify: `crates/s3-model/README.md`
- Modify: `crates/s3-model/tests/fixtures/minimal_models.json`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Update the README to describe the new FCG schema**

```md
## Outputs

- `models.json`
- Top-level sections:
  - `fcg`
    - `capabilities`
    - `capability_chains`
    - `capability_api_index`
    - `stage1_summary`
  - `slm`
  - `api_contracts`
  - `risk_surface_map`

## FCG model

- capability anchor is `type` or `module_entry`
- capability id is stable and role-qualified
- `stage1_summary` is structured data, not a plain string
```

- [ ] **Step 2: Regenerate the golden fixture through the real CLI**

Run: `cargo run -p s3-model -- --input crates/s3-model/tests/fixtures/minimal_knowledge.json --output /tmp/seraph-minimal-models.json`
Expected: exit code `0` and `/tmp/seraph-minimal-models.json` created.

- [ ] **Step 3: Diff the regenerated output against the repo fixture**

Run: `diff -u crates/s3-model/tests/fixtures/minimal_models.json /tmp/seraph-minimal-models.json`
Expected: no diff after the fixture is refreshed.

- [ ] **Step 4: Run the full verification suite**

Run: `cargo test -p seraph-types -p s3-model -- --nocapture`
Expected: PASS with `0 failed`.

- [ ] **Step 5: Commit the final Phase 2 v5 alignment batch**

```bash
git -C /home/cas/Desktop/SERAPH add \
  crates/s3-model/README.md \
  crates/s3-model/tests/fixtures/minimal_models.json
git -C /home/cas/Desktop/SERAPH commit -m "docs: refresh phase2 readme and fixtures for v5 alignment"
```

## Self-Review

### Spec coverage

- v5 的 type-centered FCG: Task 1 + Task 2
- structured `stage1_summary`: Task 1 + Task 2
- Stage 2 需要的 contracts/generic constraints: Task 3
- v5 SLM rules: Task 4
- v5 risk feature coverage: Task 5
- golden fixture / README / CLI verification: Task 6

### Placeholder scan

- 没有 `TODO` / `TBD`
- 每个任务都给了明确文件、测试命令、预期失败/成功结果
- commit 命令都使用显式文件列表，避免误提交 `hashbrown/` 或其他脏文件

### Type consistency

- FCG 新字段统一使用：
  - `anchor_kind`
  - `anchor_module_id`
  - `anchor_type_id`
  - `role`
  - `Stage1Summary`
- 计划里始终使用 `CapabilityRole::{Construction, Query, Mutation, Iteration, Conversion, Finalization, Ffi}`
- 计划里始终把 `stage1_summary` 当结构化对象，而不是字符串

