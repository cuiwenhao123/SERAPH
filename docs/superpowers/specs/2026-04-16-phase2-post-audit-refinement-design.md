# Phase 2 Post-Audit Refinement Design

## Context

在真实跑通 `s3-extract -> s3-model` 并审查 `hashbrown`、`moonfire-ffmpeg`、`semver`、`s3-audit-fixture` 后，当前 Phase 2 已经可用，但还没有完全达到 `semantic_harness_agent_design_v5.md` 想要的语义质量。

本设计只处理这次审查里已经确认的 Phase 2 缺口，不扩展到 Phase 3，也不重开新的建模体系。

## Goals

本轮修订的目标只有三类：

1. 让 Phase 2 的输出更贴近真实可用的场景规划信息，而不是“结构上存在、语义上偏弱”的中间物。
2. 让 risk / contracts 的字段边界更清楚，避免把类型级事实硬塞成 API 级风险。
3. 在保持 Phase 1 “只提取事实”的前提下，让 Phase 2 的建模仍然可追溯、可解释、可验证。

## Non-Goals

- 不改 Phase 1 的抽取边界。
- 不在 Phase 2 做生成式推断或“猜测式补全”。
- 不引入新的 orchestrator、planner 或 Stage 3 行为。
- 不把所有风险都强行映射成 API 级结论。

## Chosen Direction

采用“最小 schema 调整 + builder 规则收紧”的方案：

- schema 只改真正会影响语义边界的部分；
- `SLM`、`FCG`、`stage1_summary` 主要通过 builder 规则收紧；
- `risk_surface_map` 和 `generic_constraints` 做必要的结构修订；
- 不新增新的 Phase 2 大对象，不把模型拆得更碎。

这样可以修掉这次审查里最关键的误差，同时避免 Phase 2 结构继续膨胀。

## Design

### 1. Generic Constraints Boundary

`GenericConstraints` 需要把生命周期参数从“可合成的泛型参数”中拆出去。

目标结构：

- `params: Vec<GenericConstraintParam>`
- `lifetime_params: Vec<String>`

语义约束：

- `params` 只包含真正参与 trait bound 分析和类型合成的参数。
- `lifetime_params` 只记录显式生命周期名称，例如 `'a`、`'ctx`。
- `strategy`、`bug_hunting_value`、`synthesis_guidance` 只对 `params` 生效。
- associated type 约束只挂到对应的类型参数上，不挂到生命周期上。

这样可以避免把借用事实误当成“需要构造的类型输入”，也能让后续 synthesis 逻辑不再被 `'a` 这类参数污染。

### 2. Rust Feature Risk Boundary

`RustFeatureRisk` 需要明确区分 API 级风险和类型级风险。

目标结构：

- `feature: String`
- `apis_affected: Vec<ApiId>`
- `types_affected: Vec<TypeId>`
- `risk: String`

语义约束：

- `apis_affected` 只记录直接位于 public API surface 上的风险。
- `types_affected` 只记录类型定义或 impl 语义上的风险。
- 不再为了统一格式，把类型级风险扩散到该类型的所有 public API。

这可以修复 `panic_in_drop` 这类风险在当前模型里“有事实、但最终消失”的问题，也能避免 `repr_packed`、`conditional_impl` 这类风险被错误地 API 化。

### 3. Rust Feature Emission Rules

本轮固定以下发射规则：

- `extern_abi`
  - 事实来源：Phase 1 明确提取到 public `extern "..."`
  - 输出：只填 `apis_affected`
- `borrowed_return`
  - 事实来源：Phase 1 明确提取到返回值借用了 `self` 或输入
  - 输出：只填 `apis_affected`
- `repr_packed`
  - 事实来源：Phase 1 明确提取到 public type 的 `repr(packed)`
  - 输出：只填 `types_affected`
- `panic_in_drop`
  - 事实来源：某个 `Drop` impl 内存在明确 panic 证据
  - 输出：只填 `types_affected`
- `conditional_impl`
  - 事实来源：public-relevant trait impl 带 `cfg(...)`
  - 输出：只填 `types_affected`

这里的核心原则是：风险归属跟着事实载体走，而不是跟着“最方便 downstream 使用的形状”走。

### 4. API Risk vs Feature Risk

`ApiRisk` 与 `RustFeatureRisk` 的职责要显式分离。

`ApiRisk`：

- 服务于 Stage 1 / 1.5 / 2 的 API 选择与优先级排序；
- 只描述某个 API 本身为什么值得测；
- 可以继续使用 `is_unsafe`、`contains_unsafe_block`、`extern ABI boundary`、`borrowed return` 等事实进行打分。

`RustFeatureRisk`：

- 服务于 crate 级或 type-cluster 级风险提示；
- 不要求每条风险都能落到单个 API；
- 允许只在 `types_affected` 上有内容。

这意味着：

- `contains_unsafe_block` 仍然留在 `ApiRisk.reasons`；
- 不单独升格成 `RustFeatureRisk`；
- `panic_in_drop` 这类无法自然落到某个普通 API 的风险不再丢失。

### 5. SLM Emission Tightening

`SLM` 的问题不是缺少条目，而是部分 `full` 模型提供的增量语义太弱。

新的发射规则：

- 只有在下游规划器能明显受益时才发 `full`；
- `full` 至少需要满足：
  - 存在一个非平凡状态：`Configured`、`Active`、`Error`、`Closed`
  - 并且存在至少一个非构造型转移，或至少一个有实际约束价值的 `forbidden_transition`
- 如果模型只有 `Constructed`，或者只有机械性的自环转移，则降级为 `simplified`
- 如果一个类型既无生命周期证据，也无状态文档、风险证据、显式转移线索，则不发 `SLM`

`full` 的含义从“像是有状态”收紧为“足以改变 harness 计划”。

### 6. FCG Edge Tightening

当前 `FCG` 的问题是部分跨锚点连接太松。

新的边生成规则：

- 同锚点连接优先：
  - `construction -> query`
  - `construction -> mutation`
  - `construction -> iteration`
  - `construction -> finalization`
- 跨锚点连接只在以下条件都满足时建立：
  - 源 capability 的某个 public API 的主要返回类型能解析到明确的 public type
  - 目标 type 上确实存在可继续探索的 capability
- 不因为以下情况建立跨锚点边：
  - 只是容器内部顺带提到的类型
  - 未解析清楚的 associated type / 泛型占位
  - 纯 query 到纯 query 的机械跳转

`connects_to` 可以保留结构上可能继续的边，但 `recommended_chains` 必须更保守，只保留像真实使用路径的链。

### 7. Stage 1 Summary Compression

`stage1_summary.capability_cards` 继续保留完整信息，主要收紧 `one_liner`。

新的摘要规则：

- `one_liner` 只做稳定短摘要；
- 最多点出 2 到 3 个代表性 capability；
- 优先级按以下顺序决定：
  - 有 `entry_api_ids`
  - 位于推荐链核心位置
  - 具有更高语义风险权重，如 `ffi`、`finalization`
  - `api_ids` 更多
- 其余 capability 用聚合描述，不逐个展开

目标是让 `one_liner` 适合作为 Stage 1 的快速引导，而不是重复 `capability_cards` 的全文。

## Data Compatibility

本轮只做两项 schema 变更：

1. `GenericConstraints` 新增 `lifetime_params`
2. `RustFeatureRisk` 新增 `types_affected`

其余收敛全部通过 builder 规则完成，不新增新的顶层模型对象。

因此迁移成本可控：

- `serde` fixture 需要更新；
- Phase 2 golden JSON 需要重刷；
- 依赖旧 `RustFeatureRisk` 结构的测试断言需要调整；
- Phase 3 暂时不需要感知新字段也不会被阻塞。

## Validation Plan

修订后至少重新验证以下四类 crate：

1. `examples/target-crates/s3-audit-fixture`
   - 验证 `panic_in_drop` 不再丢失
   - 验证 `conditional_impl` 留在类型级风险
2. `examples/target-crates/hashbrown`
   - 验证 `stage1_summary.one_liner` 显著变短
   - 验证 capability 链不再机械扩张
3. `examples/target-crates/moonfire-ffmpeg`
   - 验证 `SLM` 不再输出弱 `full`
   - 验证 `ApiRisk` 仍能保留 `contains_unsafe_block`
4. `examples/target-crates/semver`
   - 验证 value-type crate 不会被过度建模

验证口径：

- 与 Phase 1 事实能逐项对回源码或 rustdoc；
- 与 Phase 2 的设计意图一致；
- 不因压缩摘要或收紧边而损失关键公共语义。

## Rollout

实现顺序固定为：

1. 调整 `seraph-types` schema
2. 更新 roundtrip test
3. 调整 `risk.rs`
4. 调整 `contracts.rs`
5. 调整 `slm.rs`
6. 调整 `fcg.rs`
7. 更新 Phase 2 fixtures 与 integration tests
8. 用真实 crate 重新跑审查验证

这个顺序的原因是：先冻结 schema 边界，再修改 builder，最后刷新 golden 和真实输出。

## Open Decisions Resolved

以下问题在本设计中已经定稿：

- 生命周期参数不再参与 synthesis strategy 分桶。
- `panic_in_drop` 不再通过 API 扩散表达。
- `conditional_impl` 也按类型级风险建模。
- `one_liner` 只保留短摘要，不承担完整概览职责。
- `SLM full` 的门槛收紧，但不新增第三种 model kind。

## Acceptance Criteria

本设计完成后，Phase 2 应满足以下条件：

1. `panic_in_drop` 即使没有直接关联 public API，也不会从最终模型中消失。
2. 生命周期参数不会再出现在可合成类型参数列表中。
3. `SLM full` 只出现在对后续 harness 规划确实有帮助的类型上。
4. `FCG recommended_chains` 比当前版本更像真实使用路径，而不是结构可达路径。
5. `stage1_summary.one_liner` 在大 crate 上仍然短小、稳定、可读。

