# Knowledge.rs V2 Schema Spec

> Status: Historical snapshot. This document is kept for audit trail only and is superseded by `2026-04-15-knowledge-rs-v3-schema-spec.md`, current code, and current regression tests.

## Goal

将 `seraph-types::knowledge` 收敛为 Phase 1 的原始事实层契约。

这版 schema 只保存可重复抽取、可回溯到证据源的事实，不保存摘要、推断、能力分组、生命周期判断或风险分级。

## Design Rules

- Phase 1 只面向单个本地 library crate。
- Phase 1 只保存 raw facts，不保存 inferred semantics。
- 所有 `docs: String` / `root_docs: String` 字段在无文档时统一写空字符串，不使用 `Option<String>`。
- 所有公开项统一使用：
  - `canonical_path` 表示定义位点
  - `public_paths` 表示对外可访问路径，可为空或为多个
- 能由关系推导出的索引不在 schema 中重复缓存。
- `trait_registry` 是 public-API-relevant trait 节点表，不是“本 crate 的 pub trait 列表”。

## Top-Level Shape

```rust
pub struct Knowledge {
    pub crate_meta: CrateMeta,
    pub modules: Vec<ModuleInfo>,
    pub types: Vec<TypeInfo>,
    pub apis: Vec<ApiInfo>,
    pub trait_registry: Vec<TraitInfo>,
    pub examples: Vec<ExampleInfo>,
    pub risk_facts: RiskFacts,
}
```

## CrateMeta

```rust
pub struct CrateMeta {
    pub package_name: String,
    pub lib_target_name: String,
    pub crate_import_name: String,
    pub version: String,
    pub edition: String,
    pub rust_version: Option<String>,
    pub repository: Option<String>,
    pub manifest_path: String,
    pub lib_rs_path: String,
    pub default_features: Vec<String>,
    pub cargo_description: Option<String>,
    pub root_docs: String,
}
```

约束：

- `crate_import_name` 优先等于 lib target 名称，不由 package 名机械推导。
- `cargo_description` 与 `root_docs` 必须分离。
- `root_docs` 无内容时写空字符串。

## ModuleInfo

```rust
pub struct ModuleInfo {
    pub module_id: ModuleId,
    pub name: String,
    pub canonical_path: String,
    pub public_paths: Vec<String>,
    pub parent_module_id: Option<ModuleId>,
    pub code_ref: CodeRef,
    pub docs: String,
}
```

说明：

- 不再缓存 `type_ids`、`api_ids`、`trait_ids`。
- 类型、API、trait 到模块的关系统一通过各自的 public anchor 字段表达，避免双向冗余。

## TypeInfo

```rust
pub struct TypeInfo {
    pub type_id: TypeId,
    pub name: String,
    pub canonical_path: String,
    pub public_paths: Vec<String>,
    pub public_anchor_module_id: ModuleId,
    pub code_ref: CodeRef,
    pub docs: String,
    pub kind: TypeKind,
    pub generic_params: Vec<String>,
    pub where_clauses: Vec<String>,
}

pub enum TypeKind {
    Struct,
    Enum,
    Union,
    TypeAlias,
    Opaque,
}
```

说明：

- 删除 `lifecycle_hint`。
- 删除 `constructors`、`method_ids`、`trait_impls` 等可由后续关系推导出的缓存字段。
- `public_paths` 非空即可视为该类型被公开暴露，不再额外保存 `is_pub`。
- `public_anchor_module_id` 表示该类型最主要的 public 暴露模块锚点。
- 它不承诺等于 canonical 定义父模块；对 facade/re-export 场景允许不同。
- `where_clauses` 必须保存 Rust 风格可读文本，不接受 rustdoc 内部 JSON 序列化字符串。

## ApiInfo

```rust
pub struct ApiInfo {
    pub api_id: ApiId,
    pub name: String,
    pub canonical_path: String,
    pub public_paths: Vec<String>,
    pub public_anchor_module_id: ModuleId,
    pub owner_type_id: Option<TypeId>,
    pub code_ref: CodeRef,
    pub docs: String,
    pub api_kind: ApiKind,
    pub signature_text: String,
    pub receiver: Option<String>,
    pub generic_params: Vec<String>,
    pub where_clauses: Vec<String>,
    pub arg_types: Vec<String>,
    pub return_type: Option<String>,
    pub is_unsafe: bool,
    pub is_async: bool,
    pub is_const: bool,
    pub has_body: bool,
}

pub enum ApiKind {
    FreeFunction,
    AssocFunction,
    InherentMethod,
    TraitMethod,
    Constructor,
}
```

说明：

- `has_body` 表示该项是否带实现体。
- 对 trait method，`has_body = true` 表示存在默认实现。
- `signature_text` 只作为展示文本，结构化信息以其余字段为准。
- `public_anchor_module_id` 表示该 API 最主要的 public 暴露模块锚点，不承诺等于 canonical 定义父模块。
- `where_clauses` 必须保存 Rust 风格可读文本，不接受 rustdoc 内部 JSON 序列化字符串。
- API 到示例的关联不再保存 `related_example_ids` 反向索引，统一通过 `ExampleInfo.involved_api_ids` 表达，避免双向冗余。

## TraitInfo

```rust
pub struct TraitInfo {
    pub trait_id: TraitId,
    pub name: String,
    pub canonical_path: String,
    pub public_paths: Vec<String>,
    pub public_anchor_module_id: Option<ModuleId>,
    pub code_ref: Option<CodeRef>,
    pub docs: String,
    pub origin: TraitOrigin,
    pub exposure_kinds: Vec<TraitExposureKind>,
    pub is_unsafe: bool,
    pub direct_supertrait_ids: Vec<TraitId>,
    pub required_methods: Vec<String>,
    pub provided_methods: Vec<String>,
    pub associated_types: Vec<String>,
    pub used_by_api_ids: Vec<ApiId>,
    pub used_by_type_ids: Vec<TypeId>,
}

pub enum TraitOrigin {
    Local,
    Reexported,
    External,
}

pub enum TraitExposureKind {
    Defined,
    Reexported,
    Bound,
    Impl,
    SupertraitDependency,
}
```

关键定义：

- `trait_registry` 包含所有对 public API 有意义的 trait 节点。
- 不只包含 `pub trait`。
- `public_anchor_module_id` 表示该 trait 在当前 crate 中的主要 public 暴露锚点；外部依赖 trait 可以为空。
- `TraitOrigin` 的语义是“相对当前 crate 的出现方式”：
  - `Local`: trait 定义在本 crate 中
  - `Reexported`: trait 不定义在本 crate 中，但被本 crate 以 public path 暴露
  - `External`: trait 不定义在本 crate 中，也没有被本 crate 直接 re-export，只是作为 public API 依赖出现
- 也包含：
  - re-export trait
  - 出现在 public 签名、bound、where clause、associated type 中的外部 trait
  - 出现在 public 签名中的本 crate 私有 trait 约束
  - public-API-relevant trait 的直接 supertrait
- `direct_supertrait_ids` 只保存直接关系，不保存传递闭包。
- `used_by_api_ids` / `used_by_type_ids` 是显式抽取出的曝光边，不视为纯缓存。
  因为 Phase 1 中 `where_clauses`、`generic_params` 仍以文本形式保存，没有单独的规范化 trait 引用表。

## ExampleInfo

```rust
pub struct ExampleInfo {
    pub example_id: ExampleId,
    pub anchor: ExampleAnchor,
    pub involved_api_ids: Vec<ApiId>,
    pub code_ref: CodeRef,
    pub snippet: Option<String>,
}

pub enum ExampleAnchor {
    Module(ModuleId),
    Type(TypeId),
    Api(ApiId),
    Trait(TraitId),
}
```

约束：

- `anchor` 是主归属点。
- `involved_api_ids` 表示关联范围。
- `snippet` 仅作为证据文本，不参与唯一性判断，不作为主键。

## RiskFacts

```rust
pub struct RiskFacts {
    pub ffi_apis: Vec<FfiApiFact>,
    pub repr_types: Vec<TypeLayoutFact>,
    pub drop_impl_types: Vec<TypeId>,
}

pub struct FfiApiFact {
    pub api_id: ApiId,
    pub abi: String,
    pub source: Option<CodeRef>,
}

pub struct TypeLayoutFact {
    pub type_id: TypeId,
    pub repr_kinds: Vec<ReprKind>,
    pub source: Option<CodeRef>,
}

pub enum ReprKind {
    C,
    Transparent,
    Packed,
    Align(u32),
    Other(String),
}
```

说明：

- `ffi_apis` 明确表示 FFI / extern ABI 事实，不再使用含糊的 `extern_api_ids`。
- `unsafe` / `# Panics` / `# Safety` 不再单独缓存为派生索引。
  - `unsafe` 由 `ApiInfo.is_unsafe` 表达
  - 文档中的 panic/safety 事实由 `ApiInfo.docs` 保留原文，后续需要时再从文本推导
- 删除 `panic_points`、`ffi_boundaries`、`panic_in_drop_types` 这类难以稳定抽取的自由文本字段。

## Data Source Contract

- `cargo metadata`
  - `CrateMeta`
- `rustdoc JSON`
  - `modules`
  - `types`
  - `apis`
  - `trait_registry`
  - docs、span、signature、supertrait
- 源码轻量扫描
  - `examples`
  - `risk_facts`

## Intentional Deletions From V1

- `crate_name`
- `crate_doc`
- `public_api_count`
- `Level0Summary`
- `ModuleTypeRef`
- `TypeInfo.lifecycle_hint`
- `ModuleInfo.api_names`
- `ExampleInfo.source_api_id`
- `RawRiskSurface.panic_points`
- `RawRiskSurface.ffi_boundaries`
- `RawRiskSurface.panic_in_drop_types`

## ID Strategy

- `module_id = "mod::<canonical_path>"`
- `type_id = "type::<canonical_path>"`
- `api_id = "api::<canonical_path>"`
- `trait_id = "trait::<canonical_path>"`
- `example_id = "ex::<file>::<start_line>"`

ID 生成必须稳定，不能依赖遍历顺序。
