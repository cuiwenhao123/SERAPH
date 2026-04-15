# Phase 2 Semantic Modeling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不引入 LLM 推理的前提下，把 Phase 1 的 `knowledge.json` 确定事实转换成 Phase 2 的 `models.json`，稳定产出 `FCG`、`SLM`、`API Contract Table`、`Risk Surface Map` 四类语义模型。

**Architecture:** 以 `crates/seraph-types/src/models.rs` 作为唯一的 Phase 2 schema 契约，在 `crates/s3-model` 内按 `fcg`、`slm`、`contracts`、`risk` 四个 builder 独立建模，再由顶层 pipeline 组装成 `Models`。所有字段都必须来自 Phase 1 已提取事实或其确定性重排，不引入 LLM 总结、不把 Phase 3 的上下文组装逻辑提前塞进 Phase 2。

**Tech Stack:** Rust 2021, `serde`, `serde_json`, `seraph-types`, crate-local unit tests, integration tests, JSON fixtures, `cargo test`

---

## File Structure

### Existing files to modify

- `crates/seraph-types/src/models.rs`
  Phase 2 顶层 schema；需要从占位结构升级成可落地的 `models.json` 契约。
- `crates/seraph-types/tests/schema_roundtrip.rs`
  为新的 Phase 2 schema 提供 roundtrip 回归测试。
- `crates/s3-model/Cargo.toml`
  补齐 `serde`、`serde_json`、`seraph-types` 等依赖。
- `crates/s3-model/src/lib.rs`
  顶层 pipeline 入口，负责组装四个 builder 并暴露读写接口。
- `crates/s3-model/README.md`
  记录 Phase 2 输入、输出、边界和运行方式。

### New files to create

- `crates/s3-model/src/io.rs`
  负责读取 `knowledge.json` 和写出 `models.json`。
- `crates/s3-model/src/fcg.rs`
  构建 `FunctionalCapabilityGraph`。
- `crates/s3-model/src/slm.rs`
  构建 `StateLifecycleModel`。
- `crates/s3-model/src/contracts.rs`
  构建 `ApiContract` 与 `GenericConstraints`。
- `crates/s3-model/src/risk.rs`
  构建 `RiskSurfaceMap`。
- `crates/s3-model/src/main.rs`
  提供 `s3-model --input ... --output ...` CLI。
- `crates/s3-model/tests/minimal_pipeline.rs`
  最小闭环测试：给一份内联 `Knowledge`，输出一份完整 `Models`。
- `crates/s3-model/tests/fixtures/minimal_knowledge.json`
  小型固定 fixture，覆盖一个构造 API、一个 trait-bound API、一个风险 API。
- `crates/s3-model/tests/fixtures/minimal_models.json`
  对应期望输出，用于 golden-style 断言。

### Explicit non-goals for this plan

- 不修改 `s3-context`、`s3-coverage`、`seraph-cli` 的消费逻辑。
- 不提前实现 Phase 3 skill orchestration。
- 不把真实 crate 源码 checkout 提交进仓库。

## Phase 2 Schema Decisions To Freeze First

- `Models` 仍保持 4 个顶层字段：`fcg`、`slm`、`api_contracts`、`risk_surface_map`。
- `FunctionalCapabilityGraph` 保留 `capabilities`、`capability_chains`、`capability_api_index`、`stage1_summary`。
- `StateLifecycleModel` 新增 `model_kind`，只允许 `full` 或 `simplified`；`stateless` 不单独落 entry，消费者通过缺席判断。
- `ApiContract` 新增 `side_effects`，补齐 v5 中明确列出的契约维度。
- `GenericConstraintParam` 从占位字符串升级为结构化字段：
  `name`、`direct_bounds`、`full_bound_chain`、`associated_type_constraints`、`is_unsafe_trait`、`strategy`、`bug_hunting_value`、`synthesis_guidance`。
- `RiskSurfaceMap` 继续只保留 `api_risks`、`type_synthesis_overview`、`rust_feature_risks` 三块，不把 Phase 3 的建议性文本混进 schema。
- 能稳定枚举的字段用 enum，不稳定开放集保持 `String`。

## Deterministic Heuristics To Implement

### FCG

- capability 节点按 `public_anchor_module_id` 聚类，不在第一版引入跨模块重新分簇。
- capability 名称优先取模块 `doc_sections.summary` 第一段；为空时回退到模块短名。
- `api_ids` 收录该 anchor module 下的全部 public API。
- `entry_api_ids` 只收录构造入口和无 receiver 的 free/assoc API。
- `connects_to` 通过“本能力 API 返回的本地类型，是否在另一能力拥有方法 surface”建立边。
- `capability_chains` 基于 `connects_to` 做去重路径枚举；若图没有有效边则为空。
- `stage1_summary` 用固定模板拼接，不调用模型生成自然语言。

### SLM

- `score >= 3` 才进入 full-scan；`score in [1, 2]` 进入 simplified；`0` 直接跳过。
- score 来源固定为 v5 中的五类信号：`Drop`、`&mut self` 方法数、stateful 文档词、被返回频率、状态相关 panic 文档。
- `simplified` 模型只允许 `Constructed -> InUse -> Dropped/Closed` 这类标准态。
- `full` 模型只在命中明确 method-name / doc-signal 时扩展状态名，如 `Configured`、`Active`、`Closed`、`Error`。
- 只有存在恢复 API 时才允许建 `Error` 状态。
- `forbidden_transitions` 只接受显式 `# Panics` / 文档短语 / 方法名强约束，不做自由推理。

### Contracts

- `preconditions`、`panic_conditions`、`error_conditions`、`safety` 全部直接来自 `DocSections` 的段落切分。
- `postconditions` 来自返回类型结构事实和显式文档句子，不做“应该如何使用”的扩写。
- `side_effects` 只记录可确定事实：
  `&mut self` 变更 receiver、`self` 消耗 receiver、extern ABI 调用、文档中出现 write/insert/remove/close 等词。
- `generic_constraints` 只消费 `generic_params`、`where_clauses`、`trait_registry`、`trait_impl_registry`。
- `full_bound_chain` 只做 supertrait 展开，不做语义归纳。
- `associated_type_constraints` 只记录可直接从 clause / trait definition 读到的 associated type 约束。

### Risk

- `api_risks` 由确定性评分表生成，原始证据只来自 Phase 1：
  `is_unsafe`、`contains_unsafe_block`、`extern_abi_apis`、`explicit_panic_sites`、`borrowed_return_apis`、`generic_constraints`、`doc_sections.safety`。
- `type_synthesis_overview` 只聚合 Phase 2 已经写出的 `strategy` 统计。
- `rust_feature_risks` 第一版只覆盖 5 类：
  `conditional_impl`、`extern_abi`、`borrowed_return`、`repr_packed`、`panic_in_drop`。

## Task 1: Freeze `seraph-types` Phase 2 Schema

**Files:**
- Modify: `crates/seraph-types/src/models.rs`
- Modify: `crates/seraph-types/tests/schema_roundtrip.rs`
- Test: `crates/seraph-types/tests/schema_roundtrip.rs`

- [ ] **Step 1: Write the failing schema roundtrip test**

```rust
#[test]
fn models_roundtrip_preserves_phase2_semantic_fields() {
    use seraph_types::{
        ApiContract, ApiId, ApiRisk, BugHuntingValue, CapabilityNode, CapId,
        ForbiddenTransition, FunctionalCapabilityGraph, GenericConstraintParam,
        GenericConstraints, Models, RiskLevel, RiskSurfaceMap, RustFeatureRisk,
        SlmModelKind, StateLifecycleModel, StateTransition, TypeId,
        TypeSynthesisOverview,
    };
    use std::collections::BTreeMap;

    let models = Models {
        fcg: FunctionalCapabilityGraph {
            capabilities: vec![CapabilityNode {
                cap_id: CapId::from("cap::demo::parse"),
                name: "parse".into(),
                description: "parse input".into(),
                api_ids: vec![ApiId::from("api::demo::parse")],
                entry_api_ids: vec![ApiId::from("api::demo::parse")],
                connects_to: vec![CapId::from("cap::demo::query")],
            }],
            capability_chains: vec![vec![
                CapId::from("cap::demo::parse"),
                CapId::from("cap::demo::query"),
            ]],
            capability_api_index: BTreeMap::from([(
                CapId::from("cap::demo::parse"),
                vec![ApiId::from("api::demo::parse")],
            )]),
            stage1_summary: "本库提供 1 项核心能力：parse（1 个 API）".into(),
        },
        slm: vec![StateLifecycleModel {
            type_id: TypeId::from("type::demo::Parser"),
            path: "demo::Parser".into(),
            model_kind: SlmModelKind::Full,
            states: vec!["Constructed".into(), "Active".into(), "Closed".into()],
            transitions: vec![StateTransition {
                from: "Constructed".into(),
                to: "Active".into(),
                via_api_id: ApiId::from("api::demo::Parser::start"),
                preconditions: vec![],
            }],
            fuzzable_states: vec!["Active".into()],
            forbidden_transitions: vec![ForbiddenTransition {
                from: "Closed".into(),
                via_api_id: ApiId::from("api::demo::Parser::start"),
                reason: "documented panic".into(),
            }],
        }],
        api_contracts: vec![ApiContract {
            api_id: ApiId::from("api::demo::parse"),
            path: "demo::parse".into(),
            preconditions: vec!["input must be valid".into()],
            postconditions: vec!["returns Ok(T) or Err(E)".into()],
            panic_conditions: vec![],
            error_conditions: vec!["invalid bytes".into()],
            safety: None,
            side_effects: vec!["consumes input bytes".into()],
            generic_constraints: Some(GenericConstraints {
                params: vec![GenericConstraintParam {
                    name: "R".into(),
                    direct_bounds: vec!["Read".into()],
                    full_bound_chain: vec!["Read".into()],
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
                api_id: ApiId::from("api::demo::parse"),
                risk_level: RiskLevel::High,
                reasons: vec!["extern input".into()],
                recommended_fuzz_strategy: "raw bytes".into(),
            }],
            type_synthesis_overview: TypeSynthesisOverview {
                generic_api_count: 1,
                strategy_distribution: BTreeMap::from([("C".into(), 1)]),
                one_liner: "1 个泛型 API 适合自定义类型合成".into(),
            },
            rust_feature_risks: vec![RustFeatureRisk {
                feature: "borrowed_return".into(),
                apis_affected: vec![ApiId::from("api::demo::parse")],
                risk: "borrow ties output to input".into(),
            }],
        },
    };

    let json = serde_json::to_string_pretty(&models).unwrap();
    let decoded: Models = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.slm[0].model_kind, SlmModelKind::Full);
    assert_eq!(
        decoded.api_contracts[0]
            .generic_constraints
            .as_ref()
            .unwrap()
            .params[0]
            .bug_hunting_value,
        BugHuntingValue::High
    );
    assert_eq!(decoded.api_contracts[0].side_effects, vec!["consumes input bytes"]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p seraph-types models_roundtrip_preserves_phase2_semantic_fields -- --nocapture`
Expected: FAIL with unknown fields / missing types such as `SlmModelKind`, `side_effects`, `full_bound_chain`, or `BugHuntingValue`.

- [ ] **Step 3: Upgrade `models.rs` from placeholder to locked schema**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateLifecycleModel {
    pub type_id: TypeId,
    pub path: String,
    pub model_kind: SlmModelKind,
    pub states: Vec<String>,
    pub transitions: Vec<StateTransition>,
    pub fuzzable_states: Vec<String>,
    pub forbidden_transitions: Vec<ForbiddenTransition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlmModelKind {
    Full,
    Simplified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiContract {
    pub api_id: ApiId,
    pub path: String,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub panic_conditions: Vec<String>,
    pub error_conditions: Vec<String>,
    pub safety: Option<String>,
    pub side_effects: Vec<String>,
    pub generic_constraints: Option<GenericConstraints>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericConstraintParam {
    pub name: String,
    pub direct_bounds: Vec<String>,
    pub full_bound_chain: Vec<String>,
    pub associated_type_constraints: Vec<AssociatedTypeConstraint>,
    pub is_unsafe_trait: bool,
    pub strategy: String,
    pub bug_hunting_value: BugHuntingValue,
    pub synthesis_guidance: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssociatedTypeConstraint {
    pub trait_id: Option<TraitId>,
    pub trait_path: String,
    pub associated_type_name: String,
    pub bounds: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BugHuntingValue {
    Low,
    Medium,
    High,
}
```

- [ ] **Step 4: Run schema tests**

Run: `cargo test -p seraph-types -- --nocapture`
Expected: PASS, with both旧有 schema roundtrip 和新的 Phase 2 roundtrip 全部通过。

- [ ] **Step 5: Commit**

```bash
git add crates/seraph-types/src/models.rs crates/seraph-types/tests/schema_roundtrip.rs
git commit -m "feat: freeze phase2 models schema"
```

## Task 2: Scaffold `s3-model` Pipeline And JSON I/O

**Files:**
- Modify: `crates/s3-model/Cargo.toml`
- Modify: `crates/s3-model/src/lib.rs`
- Create: `crates/s3-model/src/io.rs`
- Create: `crates/s3-model/src/main.rs`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Write the failing pipeline smoke test**

```rust
#[test]
fn build_models_from_knowledge_returns_all_phase2_sections() {
    let knowledge: seraph_types::Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    assert!(!models.fcg.capabilities.is_empty());
    assert!(!models.api_contracts.is_empty());
    assert!(!models.risk_surface_map.api_risks.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p s3-model build_models_from_knowledge_returns_all_phase2_sections -- --nocapture`
Expected: FAIL because `s3-model` does not yet expose `build_models_from_knowledge`.

- [ ] **Step 3: Create the top-level crate interfaces**

```rust
// crates/s3-model/src/lib.rs
#![forbid(unsafe_code)]

mod contracts;
mod fcg;
mod io;
mod risk;
mod slm;

use seraph_types::{Knowledge, Models};

pub fn build_models_from_knowledge(knowledge: &Knowledge) -> Result<Models, String> {
    let fcg = fcg::build_fcg(knowledge);
    let slm = slm::build_slm_models(knowledge);
    let api_contracts = contracts::build_api_contracts(knowledge);
    let risk_surface_map = risk::build_risk_surface_map(knowledge, &api_contracts);

    Ok(Models {
        fcg,
        slm,
        api_contracts,
        risk_surface_map,
    })
}

pub use io::{read_knowledge_json, write_models_json};
```

```rust
// crates/s3-model/src/io.rs
use seraph_types::{Knowledge, Models};
use std::fs;
use std::path::Path;

pub fn read_knowledge_json(path: impl AsRef<Path>) -> Result<Knowledge, String> {
    let raw = fs::read_to_string(path.as_ref()).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

pub fn write_models_json(path: impl AsRef<Path>, models: &Models) -> Result<(), String> {
    let json = serde_json::to_string_pretty(models).map_err(|err| err.to_string())?;
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path.as_ref(), json).map_err(|err| err.to_string())
}
```

```rust
// crates/s3-model/src/main.rs
#![forbid(unsafe_code)]

use s3_model::{build_models_from_knowledge, read_knowledge_json, write_models_json};
use std::env;
use std::process;

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input = args.next(),
            "--output" => output = args.next(),
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let input = input.ok_or_else(|| "missing required argument: --input <path>".to_owned())?;
    let output = output.ok_or_else(|| "missing required argument: --output <path>".to_owned())?;
    let knowledge = read_knowledge_json(input)?;
    let models = build_models_from_knowledge(&knowledge)?;
    write_models_json(output, &models)
}
```

- [ ] **Step 4: Run the smoke test**

Run: `cargo test -p s3-model -- --nocapture`
Expected: FAIL now only because builder modules still return nothing / are missing, not because crate wiring is absent.

- [ ] **Step 5: Commit**

```bash
git add crates/s3-model/Cargo.toml crates/s3-model/src/lib.rs crates/s3-model/src/io.rs crates/s3-model/src/main.rs crates/s3-model/tests/minimal_pipeline.rs
git commit -m "feat: scaffold phase2 modeling pipeline"
```

## Task 3: Implement FCG Builder

**Files:**
- Create: `crates/s3-model/src/fcg.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Create: `crates/s3-model/tests/fixtures/minimal_knowledge.json`
- Create: `crates/s3-model/tests/fixtures/minimal_models.json`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add the failing FCG-focused assertions**

```rust
#[test]
fn fcg_groups_apis_by_public_anchor_module_and_builds_stage1_summary() {
    let knowledge: seraph_types::Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    assert_eq!(models.fcg.capabilities.len(), 2);
    assert_eq!(
        models.fcg.capability_api_index["cap::demo::io"],
        vec![
            seraph_types::ApiId::from("api::demo::io::from_reader"),
            seraph_types::ApiId::from("api::demo::io::read_one"),
        ]
    );
    assert!(models.fcg.stage1_summary.contains("本库提供 2 项核心能力"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p s3-model fcg_groups_apis_by_public_anchor_module_and_builds_stage1_summary -- --nocapture`
Expected: FAIL because `fcg::build_fcg` is not implemented.

- [ ] **Step 3: Implement deterministic module-based capability clustering**

```rust
// crates/s3-model/src/fcg.rs
use seraph_types::{
    ApiInfo, ApiKind, CapId, FunctionalCapabilityGraph, Knowledge, ModuleId, TypeId,
    CapabilityNode,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_fcg(knowledge: &Knowledge) -> FunctionalCapabilityGraph {
    let mut by_module: BTreeMap<&ModuleId, Vec<&ApiInfo>> = BTreeMap::new();
    for api in &knowledge.apis {
        by_module
            .entry(&api.public_anchor_module_id)
            .or_default()
            .push(api);
    }

    let mut capabilities = Vec::new();
    let mut capability_api_index = BTreeMap::new();
    let mut type_to_module: BTreeMap<&TypeId, &ModuleId> = BTreeMap::new();
    for ty in &knowledge.types {
        type_to_module.insert(&ty.type_id, &ty.public_anchor_module_id);
    }

    for module in &knowledge.modules {
        let Some(apis) = by_module.get(&module.module_id) else {
            continue;
        };

        let cap_id = CapId::from(format!("cap::{}", module.canonical_path));
        let api_ids = apis.iter().map(|api| api.api_id.clone()).collect::<Vec<_>>();
        let entry_api_ids = apis
            .iter()
            .filter(|api| matches!(api.api_kind, ApiKind::FreeFunction | ApiKind::AssocFunction | ApiKind::Constructor))
            .map(|api| api.api_id.clone())
            .collect::<Vec<_>>();

        capability_api_index.insert(cap_id.clone(), api_ids.clone());
        capabilities.push(CapabilityNode {
            cap_id,
            name: first_non_empty(&module.doc_sections.summary, &module.name),
            description: module.doc_sections.summary.clone(),
            api_ids,
            entry_api_ids,
            connects_to: Vec::new(),
        });
    }

    attach_connects_to(&mut capabilities, &knowledge.apis, &type_to_module);
    let capability_chains = derive_capability_chains(&capabilities);
    let stage1_summary = build_stage1_summary(&capabilities);

    FunctionalCapabilityGraph {
        capabilities,
        capability_chains,
        capability_api_index,
        stage1_summary,
    }
}
```

- [ ] **Step 4: Run the FCG test**

Run: `cargo test -p s3-model fcg_groups_apis_by_public_anchor_module_and_builds_stage1_summary -- --nocapture`
Expected: PASS, and the broader `cargo test -p s3-model -- --nocapture` still only fails in not-yet-implemented SLM / Contract / Risk assertions.

- [ ] **Step 5: Commit**

```bash
git add crates/s3-model/src/fcg.rs crates/s3-model/tests/minimal_pipeline.rs crates/s3-model/tests/fixtures/minimal_knowledge.json crates/s3-model/tests/fixtures/minimal_models.json
git commit -m "feat: build phase2 capability graph"
```

## Task 4: Implement SLM Builder

**Files:**
- Create: `crates/s3-model/src/slm.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add the failing SLM assertions**

```rust
#[test]
fn slm_scores_stateful_types_and_emits_forbidden_transitions() {
    let knowledge: seraph_types::Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let parser_model = models
        .slm
        .iter()
        .find(|model| model.path == "demo::Parser")
        .unwrap();

    assert_eq!(parser_model.model_kind, seraph_types::SlmModelKind::Full);
    assert!(parser_model.states.contains(&"Closed".to_string()));
    assert_eq!(parser_model.forbidden_transitions.len(), 1);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p s3-model slm_scores_stateful_types_and_emits_forbidden_transitions -- --nocapture`
Expected: FAIL because `slm::build_slm_models` is not implemented.

- [ ] **Step 3: Implement score-driven lifecycle modeling**

```rust
// crates/s3-model/src/slm.rs
use seraph_types::{
    ApiInfo, ApiKind, Knowledge, RiskOwner, SlmModelKind, StateLifecycleModel,
    StateTransition, ForbiddenTransition, TypeId,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_slm_models(knowledge: &Knowledge) -> Vec<StateLifecycleModel> {
    let drop_types = knowledge
        .risk_facts
        .drop_impl_types
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    knowledge
        .types
        .iter()
        .filter_map(|ty| {
            let score = score_type(knowledge, &ty.type_id, &drop_types);
            if score == 0 {
                return None;
            }

            let model_kind = if score >= 3 {
                SlmModelKind::Full
            } else {
                SlmModelKind::Simplified
            };

            let (states, transitions, forbidden_transitions) =
                build_type_lifecycle(knowledge, &ty.type_id, model_kind);

            Some(StateLifecycleModel {
                type_id: ty.type_id.clone(),
                path: ty.canonical_path.clone(),
                model_kind,
                fuzzable_states: choose_fuzzable_states(&states),
                states,
                transitions,
                forbidden_transitions,
            })
        })
        .collect()
}

fn score_type(
    knowledge: &Knowledge,
    type_id: &TypeId,
    drop_types: &BTreeSet<TypeId>,
) -> u32 {
    let mut score = 0;
    if drop_types.contains(type_id) {
        score += 3;
    }
    score += count_mut_methods(knowledge, type_id).min(2);
    score += count_state_words(knowledge, type_id).min(2);
    score += count_returners(knowledge, type_id).min(1);
    score += count_stateful_panics(knowledge, type_id).min(1);
    score
}
```

- [ ] **Step 4: Run SLM tests**

Run: `cargo test -p s3-model slm_scores_stateful_types_and_emits_forbidden_transitions -- --nocapture`
Expected: PASS, with `demo::Parser` 得到 `full` 模型，`forbidden_transitions` 来自显式 panic 文档或 panic site。

- [ ] **Step 5: Commit**

```bash
git add crates/s3-model/src/slm.rs crates/s3-model/tests/minimal_pipeline.rs
git commit -m "feat: add deterministic lifecycle modeling"
```

## Task 5: Implement API Contract Builder

**Files:**
- Create: `crates/s3-model/src/contracts.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add the failing contract assertions**

```rust
#[test]
fn contract_builder_extracts_doc_sections_side_effects_and_generic_constraints() {
    let knowledge: seraph_types::Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();
    let from_reader = models
        .api_contracts
        .iter()
        .find(|contract| contract.path == "demo::io::from_reader")
        .unwrap();

    assert_eq!(from_reader.preconditions, vec!["reader must yield valid frames"]);
    assert_eq!(from_reader.side_effects, vec!["consumes input bytes"]);
    assert_eq!(
        from_reader
            .generic_constraints
            .as_ref()
            .unwrap()
            .params[0]
            .full_bound_chain,
        vec!["demo::io::Reader", "core::fmt::Debug"]
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p s3-model contract_builder_extracts_doc_sections_side_effects_and_generic_constraints -- --nocapture`
Expected: FAIL because `contracts::build_api_contracts` is not implemented.

- [ ] **Step 3: Implement direct-doc and trait-graph contract extraction**

```rust
// crates/s3-model/src/contracts.rs
use seraph_types::{
    ApiContract, ApiInfo, AssociatedTypeConstraint, BugHuntingValue, GenericConstraintParam,
    GenericConstraints, Knowledge, TraitId, TraitInfo, TraitOrigin,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_api_contracts(knowledge: &Knowledge) -> Vec<ApiContract> {
    knowledge
        .apis
        .iter()
        .map(|api| ApiContract {
            api_id: api.api_id.clone(),
            path: api.canonical_path.clone(),
            preconditions: split_doc_lines(&api.doc_sections.panics)
                .into_iter()
                .map(panic_line_to_precondition)
                .collect(),
            postconditions: derive_postconditions(api),
            panic_conditions: split_doc_lines(&api.doc_sections.panics),
            error_conditions: split_doc_lines(&api.doc_sections.errors),
            safety: non_empty(api.doc_sections.safety.clone()),
            side_effects: derive_side_effects(knowledge, api),
            generic_constraints: build_generic_constraints(knowledge, api),
        })
        .collect()
}

fn build_generic_constraints(
    knowledge: &Knowledge,
    api: &ApiInfo,
) -> Option<GenericConstraints> {
    if api.generic_params.is_empty() {
        return None;
    }

    let trait_index = knowledge
        .trait_registry
        .iter()
        .map(|item| (item.name.as_str(), item))
        .collect::<BTreeMap<_, _>>();

    Some(GenericConstraints {
        params: api
            .generic_params
            .iter()
            .map(|param| build_param_constraints(api, param, &trait_index))
            .collect(),
    })
}
```

- [ ] **Step 4: Run contract tests**

Run: `cargo test -p s3-model contract_builder_extracts_doc_sections_side_effects_and_generic_constraints -- --nocapture`
Expected: PASS, and the generated contract object carries raw doc facts plus expanded supertrait chain and associated type constraints.

- [ ] **Step 5: Commit**

```bash
git add crates/s3-model/src/contracts.rs crates/s3-model/tests/minimal_pipeline.rs
git commit -m "feat: add api contract extraction"
```

## Task 6: Implement Risk Surface Builder

**Files:**
- Create: `crates/s3-model/src/risk.rs`
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add the failing risk assertions**

```rust
#[test]
fn risk_builder_maps_phase1_facts_to_api_and_rust_feature_risks() {
    let knowledge: seraph_types::Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();

    let models = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    assert_eq!(models.risk_surface_map.api_risks[0].risk_level, seraph_types::RiskLevel::High);
    assert!(models
        .risk_surface_map
        .rust_feature_risks
        .iter()
        .any(|risk| risk.feature == "extern_abi"));
    assert_eq!(
        models.risk_surface_map.type_synthesis_overview.strategy_distribution["C"],
        1
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p s3-model risk_builder_maps_phase1_facts_to_api_and_rust_feature_risks -- --nocapture`
Expected: FAIL because `risk::build_risk_surface_map` is not implemented.

- [ ] **Step 3: Implement deterministic risk scoring**

```rust
// crates/s3-model/src/risk.rs
use seraph_types::{
    ApiContract, ApiId, ApiRisk, Knowledge, ReprKind, RiskLevel, RiskSurfaceMap,
    RustFeatureRisk, TypeSynthesisOverview,
};
use std::collections::BTreeMap;

pub fn build_risk_surface_map(
    knowledge: &Knowledge,
    api_contracts: &[ApiContract],
) -> RiskSurfaceMap {
    let api_risks = api_contracts
        .iter()
        .map(|contract| build_api_risk(knowledge, contract))
        .collect::<Vec<_>>();

    RiskSurfaceMap {
        api_risks,
        type_synthesis_overview: build_type_synthesis_overview(api_contracts),
        rust_feature_risks: build_rust_feature_risks(knowledge),
    }
}

fn build_api_risk(knowledge: &Knowledge, contract: &ApiContract) -> ApiRisk {
    let mut score = 0;
    let mut reasons = Vec::new();

    if is_extern_abi_api(knowledge, &contract.api_id) {
        score += 3;
        reasons.push("extern ABI boundary".into());
    }
    if api_has_internal_unsafe(knowledge, &contract.api_id) {
        score += 2;
        reasons.push("contains unsafe block".into());
    }
    if !contract.panic_conditions.is_empty() {
        score += 1;
        reasons.push("documented panic conditions".into());
    }
    if !contract.error_conditions.is_empty() {
        score += 1;
        reasons.push("recoverable error path".into());
    }

    let risk_level = match score {
        0 | 1 => RiskLevel::Low,
        2 | 3 => RiskLevel::Medium,
        _ => RiskLevel::High,
    };

    ApiRisk {
        api_id: contract.api_id.clone(),
        risk_level,
        recommended_fuzz_strategy: recommend_strategy(contract, &reasons),
        reasons,
    }
}
```

- [ ] **Step 4: Run risk tests**

Run: `cargo test -p s3-model risk_builder_maps_phase1_facts_to_api_and_rust_feature_risks -- --nocapture`
Expected: PASS, and `cargo test -p s3-model -- --nocapture` should now pass end to end for the fixture.

- [ ] **Step 5: Commit**

```bash
git add crates/s3-model/src/risk.rs crates/s3-model/tests/minimal_pipeline.rs
git commit -m "feat: add semantic risk surface modeling"
```

## Task 7: Finish End-to-End Output, Golden Fixture, And Documentation

**Files:**
- Modify: `crates/s3-model/tests/minimal_pipeline.rs`
- Modify: `crates/s3-model/README.md`
- Modify: `crates/s3-model/src/lib.rs`
- Test: `crates/s3-model/tests/minimal_pipeline.rs`

- [ ] **Step 1: Add golden-output comparison**

```rust
#[test]
fn models_output_matches_golden_fixture() {
    let knowledge: seraph_types::Knowledge =
        serde_json::from_str(include_str!("fixtures/minimal_knowledge.json")).unwrap();
    let expected: seraph_types::Models =
        serde_json::from_str(include_str!("fixtures/minimal_models.json")).unwrap();

    let actual = s3_model::build_models_from_knowledge(&knowledge).unwrap();

    assert_eq!(actual, expected);
}
```

- [ ] **Step 2: Run test to verify fixture drift**

Run: `cargo test -p s3-model models_output_matches_golden_fixture -- --nocapture`
Expected: If builders changed shape during earlier tasks, FAIL with a precise diff until `minimal_models.json` is updated to the now-frozen schema.

- [ ] **Step 3: Finalize README and public API docs**

````md
# s3-model

`s3-model` is the Phase 2 semantic modeling crate for SERAPH.

## Inputs

- `knowledge.json` generated by `s3-extract`

## Outputs

- `models.json`
  - `fcg`
  - `slm`
  - `api_contracts`
  - `risk_surface_map`

## Deterministic guarantees

- No LLM calls
- No inferred facts beyond rule-based reordering of Phase 1 evidence
- Stable join keys remain `api_id`, `type_id`, `trait_id`, `cap_id`

## CLI

```bash
cargo run -p s3-model -- --input ./workspace/knowledge.json --output ./workspace/models.json
```
````

- [ ] **Step 4: Run full verification**

Run: `cargo test -p seraph-types -- --nocapture`
Expected: PASS

Run: `cargo test -p s3-model -- --nocapture`
Expected: PASS

Run: `cargo run -p s3-model -- --input crates/s3-model/tests/fixtures/minimal_knowledge.json --output /tmp/seraph-phase2-models.json`
Expected: Command exits `0`, and `/tmp/seraph-phase2-models.json` contains all four top-level sections.

Run: `cargo run -p s3-extract -- --manifest-path /path/to/hashbrown/Cargo.toml --output /tmp/hashbrown-knowledge.json`
Expected: Command exits `0`.

Run: `cargo run -p s3-model -- --input /tmp/hashbrown-knowledge.json --output /tmp/hashbrown-models.json`
Expected: Command exits `0`, `fcg.capability_api_index` 非空，`api_contracts.len()` 与 public API 数量对齐，`risk_surface_map.rust_feature_risks` 至少反映 `conditional_impl` / `borrowed_return` / `repr_packed` 中实际存在的项。

Run: `cargo run -p s3-extract -- --manifest-path /path/to/moonfire-ffmpeg/Cargo.toml --output /tmp/moonfire-knowledge.json`
Expected: Command exits `0`.

Run: `cargo run -p s3-model -- --input /tmp/moonfire-knowledge.json --output /tmp/moonfire-models.json`
Expected: Command exits `0`, 并能看到 `IoContext` 相关 contract / trait-bound / risk 信号被正确落入 `models.json`。

- [ ] **Step 5: Commit**

```bash
git add crates/s3-model/src/lib.rs crates/s3-model/README.md crates/s3-model/tests/minimal_pipeline.rs crates/s3-model/tests/fixtures/minimal_knowledge.json crates/s3-model/tests/fixtures/minimal_models.json
git commit -m "feat: complete phase2 semantic modeling output"
```

## Self-Review

### Spec coverage

- `FCG`：已覆盖 capability 节点、chain、`capability_api_index`、`stage1_summary`。
- `SLM`：已覆盖 full/simplified 两级模型、核心类型评分、forbidden transition。
- `API Contract Table`：已覆盖 `preconditions`、`postconditions`、`panic_conditions`、`error_conditions`、`safety`、`side_effects`、`generic_constraints`。
- `Risk Surface Map`：已覆盖 `api_risks`、`type_synthesis_overview`、`rust_feature_risks`。
- 阶段边界：计划中没有把 Phase 3 的 context assembly、OpenHarness orchestration 或 codegen 实现塞进 Phase 2。

### Placeholder scan

- 无 `TODO`、`TBD`、`later` 之类占位描述。
- 所有任务都给出精确文件路径、测试命令、预期结果和 commit 粒度。

### Type consistency

- Phase 2 join key 统一使用 `api_id`、`type_id`、`trait_id`、`cap_id`。
- `models.json` 顶层结构和 v5 保持一致。
- `SlmModelKind`、`BugHuntingValue`、`GenericConstraints` 在 schema、builder、test 三处使用同一命名。
