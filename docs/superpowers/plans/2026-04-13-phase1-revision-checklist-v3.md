# Phase 1 修订清单 v3

> Status: Historical planning artifact. This checklist is useful as an audit record of the revision process, but the live Phase 1 contract is the active v3 schema spec plus current code and tests.

> 基于 `hashbrown` 真实执行与源码对照的修订清单。

## Scope

- 被测 crate:
  - `/home/cas/Desktop/SERAPH/examples/target-crates/hashbrown`
- 实际执行目录:
  - `/tmp/hashbrown-seraph-eval`
- 抽取输出:
  - `/tmp/seraph_hashbrown_knowledge.json`

本清单不讨论 `s3-model`，只覆盖 Phase 1 的 schema 契约、字段语义、最小抽取闭环与实现优先级。

## 实测命令

直接对仓库内嵌路径执行会失败：

```bash
cargo run -p s3-extract -- \
  --manifest-path /home/cas/Desktop/SERAPH/examples/target-crates/hashbrown/Cargo.toml \
  --output /tmp/seraph_hashbrown_knowledge.json
```

失败原因是 Cargo workspace 归属冲突。当前可工作的验证路径是：

```bash
rm -rf /tmp/hashbrown-seraph-eval
cp -a /home/cas/Desktop/SERAPH/examples/target-crates/hashbrown /tmp/hashbrown-seraph-eval

cargo run -p s3-extract -- \
  --manifest-path /tmp/hashbrown-seraph-eval/Cargo.toml \
  --output /tmp/seraph_hashbrown_knowledge.json
```

## 已被实测证实正确的字段

### 1. CrateMeta 基本可信

以下字段与 `/tmp/hashbrown-seraph-eval/Cargo.toml` 一致：

- `package_name = "hashbrown"`
- `version = "0.17.0"`
- `edition = "2024"`
- `rust_version = "1.85.0"`
- `repository = "https://github.com/rust-lang/hashbrown"`
- `cargo_description = "A Rust port of Google's SwissTable hash map"`
- `default_features = ["default-hasher", "inline-more", "allocator-api2", "equivalent", "raw-entry"]`

`root_docs` 与 `/tmp/hashbrown-seraph-eval/src/lib.rs` 顶部 crate doc 一致。

### 2. code_ref 起始行高度可信

全量对照结果：

- modules: `4 / 4` 起始行能对上源码声明
- types: `52 / 52` 起始行能对上源码声明
- apis: `231 / 231` 起始行能对上源码声明

代表样本：

- `hashbrown::hash_map` 对应 `src/lib.rs:72`
- `hashbrown::map::HashMap` 对应 `src/map.rs:182`
- `hashbrown::map::HashMap::new` 对应 `src/map.rs:272`
- `hashbrown::map::Entry::insert` 对应 `src/map.rs:3460`

### 3. docs 基本可信

实测统计：

- module docs 精确一致: `4 / 4`
- type docs 精确一致: `32 / 52`
- type docs 归一化后完全一致: `52 / 52`
- api docs 精确一致: `121 / 231`
- api docs 归一化后完全一致: `231 / 231`

这里的“不精确一致”主要来自 rustdoc 对代码块缩进的标准化，不是语义漂移。

### 4. 结构关系基本正确

- `owner_type_id`: `231 / 231` API 能回指到正确 type
- `receiver`: `211 / 211` 和 `self / &self / &mut self` 语义一致
- `is_unsafe`: `14 / 14` 能和源码里的 `unsafe fn` 对上
- `is_const`: `6 / 6` 能和源码里的 `const fn` 对上
- `public_paths`: 对 `HashMap` / `HashSet` / `HashTable` 的 re-export 恢复正确

## 已被实测证实存在问题的字段或阶段

### 1. `module_id` 字段名和实际内容不对应

`hashbrown` 上的真实情况是：

- `canonical_path = "hashbrown::map::HashMap"`
- `module_id = "mod::hashbrown::hash_map"`

也就是说，当前 `module_id` 指向的是“公开暴露锚点模块”，不是“canonical 定义父模块”。

这个现象不是个例：

- types 中 `51 / 52` 存在此类偏差
- apis 中 `231 / 231` 存在此类偏差

当前问题：

- 字段名像“定义归属”
- 实际值像“公共展示锚点”

建议：

- 将 `TypeInfo.module_id` / `ApiInfo.module_id` 重命名为 `public_anchor_module_id`
- schema 明确该字段指向“最主要 public path 所属模块”
- 不再把它描述为 canonical 定义父模块

不建议在当前阶段把所有私有定义模块都纳入 `modules`，否则会把 Phase 1 的 public surface 模块表扩展成另一套私有模块图，复杂度会明显上升。

### 2. `where_clauses` 字段内容不符合 schema 预期

当前 `where_clauses` 不是 Rust 文本，而是 rustdoc 内部结构序列化后的 JSON 字符串。

实测统计：

- type `where_clauses` 非空共 `9` 条，其中 `9 / 9` 是 JSON 字符串
- api `where_clauses` 非空共 `113` 条，其中 `113 / 113` 是 JSON 字符串

代表反例：

- 源码 `src/map.rs:2685`:

```rust
pub enum Entry<'a, K, V, S, A = Global>
where
    A: Allocator,
```

- 当前抽取结果：

```text
{"bound_predicate":{"bounds":[{"trait_bound":{"generic_params":[],"modifier":"none","trait":{"args":{"angle_bracketed":{"args":[],"constraints":[]}},"id":123,"path":"Allocator"}}}],"generic_params":[],"type":{"generic":"A"}}}
```

建议：

- `where_clauses` 必须改成 Rust 风格可读文本
- 最低目标：
  - `A: Allocator`
  - `Q: Hash + Equivalent<K> + ?Sized`
  - `S: BuildHasher`
- 如果确实需要 lossless IR，应只保留在 extractor 内部，不进入 `knowledge.rs`

### 3. `trait_registry` 还没有达到 schema 定义

当前 registry 只有 1 条：

- `equivalent::Equivalent`

但在 `hashbrown` 的 public bounds 里，实测已经出现：

- `Hash`
- `BuildHasher`
- `Equivalent`
- `Sized`
- `FnOnce`
- `FnMut`
- `Fn`
- `ToOwned`
- `Borrow`

代表样本：

- `HashMap::contains_key` 的源码在 `src/map.rs:1382`：

```rust
pub fn contains_key<Q>(&self, k: &Q) -> bool
where
    Q: Hash + Equivalent<K> + ?Sized,
```

这说明当前 `trait_registry` 只实现了“re-export trait 捕获”，还没有实现：

- public bound trait 捕获
- direct supertrait 捕获
- `used_by_api_ids`
- `used_by_type_ids`

建议：

- 将 `trait_registry` 的最小闭环定义收紧为：
  - local public trait
  - public re-export trait
  - 出现在 public type / api generic bounds 与 where clauses 中的 trait
  - 上述 trait 的 direct supertrait
- 第一版先不做 impl 图，只先把 public-bound graph 做对

### 4. `examples` 阶段尚未接上，但源数据明确存在

当前输出：

- `examples = []`

但 `hashbrown` 源码里 doc code fence 很多，实测统计：

- Rust 源文件中的 code fence 总数约 `594`

因此这里不是“源里没有例子”，而是“examples 抽取尚未实现”。

建议：

- 第一版 examples 不要做全项目自由扫描
- 先只做 public item doc comments 中的 fenced code blocks
- anchor 直接绑定到 module/type/api/trait
- `snippet` 存证据文本
- `involved_api_ids` 先做保守关联：
  - anchor 自身 API
  - 或 code block 中显式出现的同 crate public API 名

### 5. `risk_facts` 当前为空，但其中一部分字段本身设计上就有冗余

当前输出：

- `unsafe_api_ids = []`
- `documented_panic_api_ids = []`
- `documented_safety_api_ids = []`
- `extern_abi_apis = []`
- `repr_types = []`
- `drop_impl_types = []`

但 `hashbrown` 源码里已经能证明部分数据可见：

- `unsafe fn` 实测有 `14` 个
- 含 `# Safety` 的 public API docs 至少 `11` 个
- 含 `# Panics` 的 public API docs 至少 `9` 个

因此这里应拆成两类问题：

第一类：实现未接线

- `extern_abi_apis`
- `repr_types`
- `drop_impl_types`

第二类：schema 本身存在冗余

- `unsafe_api_ids` 与 `ApiInfo.is_unsafe` 重复
- `documented_panic_api_ids` 与 `ApiInfo.docs` 中的 `# Panics` 可推导
- `documented_safety_api_ids` 与 `ApiInfo.docs` 中的 `# Safety` 可推导

建议：

- 从 `RiskFacts` 中删除：
  - `unsafe_api_ids`
  - `documented_panic_api_ids`
  - `documented_safety_api_ids`
- 保留：
  - `extern_abi_apis`
  - `repr_types`
  - `drop_impl_types`

这样更符合“Phase 1 不缓存可由现有字段直接推导的索引”。

### 6. 嵌套 workspace manifest 目前不能直接跑

当前对以下路径直接执行会失败：

- `/home/cas/Desktop/SERAPH/examples/target-crates/hashbrown/Cargo.toml`

失败原因：

- 该 crate 位于 SERAPH workspace 目录树下
- 但不是 workspace member
- `cargo metadata` 会直接退出

建议：

- 在 extractor 中检测 Cargo stderr 中的 workspace mismatch 文案
- 命中后走 temp copy fallback
- 在临时目录重新执行 `cargo metadata` 和 `cargo rustdoc`

这不是最优雅的方案，但对 Phase 1 的“外部 crate 实测”最稳。

## 推荐修订顺序

### P0. 先修 schema 契约，再扩 extractor

顺序建议：

1. 明确 `module_id` 语义，推荐重命名为 `public_anchor_module_id`
2. 将 `where_clauses` 契约改成 Rust 文本，不再接受 JSON 字符串
3. 清理 `RiskFacts` 中的派生冗余字段

这三项必须先定，否则后续 extractor 会继续围绕含糊字段堆逻辑。

### P1. 用 `hashbrown` 写现实回归测试

最低应固化以下断言：

- `crate_meta` 与 `Cargo.toml` 对齐
- `root_docs` 与 crate doc 对齐
- `HashMap` / `HashMap::new` / `Entry::insert` 的 `code_ref` 对齐
- `where_clauses` 为 Rust 文本而不是 JSON
- `trait_registry` 至少包含：
  - `equivalent::Equivalent`
  - `core::hash::Hash`
  - `core::hash::BuildHasher` 或其实际 canonical path

### P2. 补齐 `trait_registry` 的最小闭环

落地顺序：

1. 从 public type bounds 抽 trait
2. 从 public api generic bounds / where clauses 抽 trait
3. 填 `used_by_api_ids` / `used_by_type_ids`
4. 补 `direct_supertrait_ids`
5. 最后再处理 re-export 和 local trait 合并去重

### P3. 落 `examples`

第一版只接：

- public item doc comments
- fenced code blocks
- anchor 与 snippet

第二版再考虑：

- `involved_api_ids` 的更精确解析
- doc block 精确行号

### P4. 落 `RiskFacts`

第一版只接：

- `extern_abi_apis`
- `repr_types`
- `drop_impl_types`

不要在这一版重新引入可推导索引。

### P5. 修 `workspace` 执行兼容

这是工程可用性修补，不是 schema 问题，但应该尽早做，否则仓库内嵌外部 crate 的验证流程会反复卡住。

## 建议的下一次代码修改范围

如果下一轮开始实际修改，建议只动以下文件：

- `crates/seraph-types/src/knowledge.rs`
- `crates/seraph-types/tests/schema_roundtrip.rs`
- `docs/superpowers/specs/2026-04-15-knowledge-rs-v3-schema-spec.md`
- `crates/s3-extract/src/lib.rs`
- `crates/s3-extract/tests/minimal_extract.rs`

建议不要在同一轮同时启动：

- source-scan 全量实现
- examples 全量实现
- risk_facts 全量实现
- workspace fallback

更稳妥的切分是：

1. schema 收口
2. trait / where-clause 改正
3. 风险事实
4. examples
5. workspace fallback
