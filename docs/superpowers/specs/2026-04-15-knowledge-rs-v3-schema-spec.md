# Knowledge.rs V3 Schema Spec

> Status: Active schema contract. When historical notes conflict with this file, prefer this spec together with the current implementations in `crates/seraph-types/src/knowledge.rs`, `crates/s3-extract/src/lib.rs`, and their regression tests.

## Goal

将 `seraph-types::knowledge` 收敛为更完整的 Phase 1 事实层契约。

这版 schema 只保存：

- 可直接从源码、rustdoc JSON、Cargo metadata 或源码 span 扫描获得的原始事实
- 由确定性语法规则得到的结构化结果

这版 schema 不保存：

- 语义分类
- 能力标签
- 风险评级
- 行为解释
- 推荐用法

## Design Rules

- Phase 1 只面向单个本地 library crate。
- Phase 1 只保存可回证事实，不保存 inferred semantics。
- 所有 `docs: String` / `root_docs: String` 字段在无文档时统一写空字符串，不使用 `Option<String>`。
- 所有公开项统一使用：
  - `canonical_path` 表示定义位点
  - `public_paths` 表示对外可访问路径，可为空或为多个
- 能由关系推导出的索引不在 schema 中重复缓存。
- `trait_registry` 是 public-API-relevant trait 节点表，不是“本 crate 的 pub trait 列表”。
- `trait_impl_registry` 是 public trait impl surface 的基础表，不做高层语义分桶。
- 所有“风险面”字段只保存原始证据，不保存风险解释。

## Audit-Driven Constraints

本版 schema 经过真实 crate 审计验证，样本包括：

- `hashbrown`
- `moonfire-ffmpeg`
- `semver`
- `sqlx-core`
- `sqlx-postgres`

审计结论固化为以下硬约束：

- `fields` 如果只依赖 rustdoc JSON，则不能定义为“完整字段列表”。
  对 public nominal type，rustdoc JSON 可能裁剪 private field，仅保留：
  - 可见字段
  - `has_stripped_fields` / tuple slot 为 `null` 这类“存在隐藏字段”的事实
- `borrowed_return_apis` 不能只依赖显式 lifetime 名匹配。
  必须同时覆盖 Rust lifetime elision，尤其是：
  - `&self -> &T`
  - `&mut self -> &mut T`
  - 单个 borrowed 参数流入 borrowed 返回值
- `cfg_attrs` 在 Phase 1 中只保存原始 attr 文本，不做 feature 语义解释。
- `explicit_panic_sites` 允许来自源码 span 扫描，但只记录显式 panic-like 证据。

## Top-Level Shape

```rust
pub struct Knowledge {
    pub crate_meta: CrateMeta,
    pub modules: Vec<ModuleInfo>,
    pub types: Vec<TypeInfo>,
    pub apis: Vec<ApiInfo>,
    #[serde(default)]
    pub symbols: Vec<SymbolInfo>,
    pub trait_registry: Vec<TraitInfo>,
    #[serde(default)]
    pub trait_impl_registry: Vec<TraitImplInfo>,
    #[serde(default)]
    pub examples: Vec<ExampleInfo>,
    pub risk_facts: RiskFacts,
}
```

## Shared Doc Structure

```rust
pub struct DocSections {
    pub summary: String,
    pub panics: String,
    pub errors: String,
    pub safety: String,
    pub examples: String,
}
```

约束：

- `summary` 表示第一段概述。
- 其余字段表示按 markdown heading 切出的原文片段。
- 没有对应章节时统一为空字符串。
- `docs` 原文与 `doc_sections` 必须同时保留。

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
    pub root_doc_sections: DocSections,
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
    pub doc_sections: DocSections,
}
```

说明：

- 不缓存 `type_ids`、`api_ids`、`trait_ids`。
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
    pub doc_sections: DocSections,
    pub kind: TypeKind,
    pub generic_params: Vec<String>,
    pub where_clauses: Vec<String>,
    pub is_non_exhaustive: bool,
    #[serde(default)]
    pub fields: Vec<TypeFieldInfo>,
    #[serde(default)]
    pub variants: Vec<EnumVariantInfo>,
    #[serde(default)]
    pub has_hidden_fields: bool,
    #[serde(default)]
    pub has_hidden_variants: bool,
}

pub enum TypeKind {
    Struct,
    Enum,
    Union,
    TypeAlias,
    Opaque,
}
```

```rust
pub struct TypeFieldInfo {
    pub position: u32,
    pub name: Option<String>,
    pub type_text: String,
    pub visibility_text: String,
    pub source: Option<CodeRef>,
}
```

```rust
pub struct EnumVariantInfo {
    pub name: String,
    pub kind: VariantKind,
    pub is_non_exhaustive: bool,
    pub discriminant_text: Option<String>,
    #[serde(default)]
    pub fields: Vec<TypeFieldInfo>,
    pub source: Option<CodeRef>,
}

pub enum VariantKind {
    Unit,
    Tuple,
    Struct,
}
```

说明：

- `public_paths` 非空即可视为该类型被公开暴露，不再额外保存 `is_pub`。
- `public_anchor_module_id` 表示该类型最主要的 public 暴露模块锚点。
- 它不承诺等于 canonical 定义父模块；对 facade/re-export 场景允许不同。
- `fields` 表示 rustdoc / extractor 可恢复的字段列表。
- 如果只依赖 rustdoc JSON，private field 可能被裁剪，因此必须允许：
  - `fields` 不完整
  - `has_hidden_fields = true`
- `variants` 对 enum 保存变体表面事实。
- `has_hidden_variants` 用于记录 rustdoc 只暴露部分变体的情况。
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
    pub owner_trait_id: Option<TraitId>,
    pub code_ref: CodeRef,
    pub docs: String,
    pub doc_sections: DocSections,
    pub api_kind: ApiKind,
    pub signature_text: String,
    pub receiver: Option<String>,
    pub generic_params: Vec<String>,
    pub where_clauses: Vec<String>,
    pub arg_types: Vec<String>,
    pub return_type: Option<String>,
    pub return_shape: Option<ReturnShape>,
    pub is_unsafe: bool,
    pub is_async: bool,
    pub is_const: bool,
    pub has_body: bool,
    pub contains_unsafe_block: bool,
}

pub enum ApiKind {
    FreeFunction,
    AssocFunction,
    InherentMethod,
    TraitMethod,
    Constructor,
}
```

```rust
pub struct ReturnShape {
    pub kind: ReturnShapeKind,
    pub inner_types: Vec<String>,
}

pub enum ReturnShapeKind {
    Unit,
    Never,
    Primitive,
    Nominal,
    Tuple,
    Array,
    Slice,
    Ref,
    RefMut,
    RawPtr,
    Result,
    Option,
    ImplTrait,
    DynTrait,
    Other,
}
```

说明：

- `has_body` 表示该项是否带实现体。
- 对 trait method，`has_body = true` 表示存在默认实现。
- `signature_text` 只作为展示文本，结构化信息以其余字段为准。
- `public_anchor_module_id` 表示该 API 最主要的 public 暴露模块锚点，不承诺等于 canonical 定义父模块。
- `where_clauses` 必须保存 Rust 风格可读文本，不接受 rustdoc 内部 JSON 序列化字符串。
- `return_shape` 是确定性结构化归纳，不是语义推理。
- `contains_unsafe_block` 表示源码 span 中存在显式 `unsafe { ... }`。

## SymbolInfo

```rust
pub struct SymbolInfo {
    pub symbol_id: SymbolId,
    pub name: String,
    pub canonical_path: String,
    pub public_paths: Vec<String>,
    pub public_anchor_module_id: ModuleId,
    pub owner_type_id: Option<TypeId>,
    pub code_ref: CodeRef,
    pub docs: String,
    pub doc_sections: DocSections,
    pub symbol_kind: SymbolKind,
    pub signature_text: Option<String>,
    pub type_text: Option<String>,
    pub value_text: Option<String>,
}

pub enum SymbolKind {
    Macro,
    Constant,
    AssociatedConstant,
}
```

说明：

- `owner_type_id` 仅对 inherent associated const 有值。
- `Constant` 表示顶层 `pub const`。
- `AssociatedConstant` 表示 public inherent associated const，例如 `Type::NAME`。

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
    pub doc_sections: DocSections,
    pub origin: TraitOrigin,
    pub exposure_kinds: Vec<TraitExposureKind>,
    pub is_unsafe: bool,
    pub direct_supertrait_ids: Vec<TraitId>,
    pub required_methods: Vec<String>,
    pub provided_methods: Vec<String>,
    #[serde(default)]
    pub associated_type_defs: Vec<TraitAssociatedTypeDef>,
    #[serde(default)]
    pub associated_const_defs: Vec<TraitAssociatedConstDef>,
    #[serde(default)]
    pub used_by_api_ids: Vec<ApiId>,
    #[serde(default)]
    pub used_by_trait_ids: Vec<TraitId>,
    #[serde(default)]
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
    SupertraitDependency,
}
```

```rust
pub struct TraitAssociatedTypeDef {
    pub name: String,
    pub generic_params: Vec<String>,
    pub where_clauses: Vec<String>,
    pub bounds: Vec<String>,
    pub default_type: Option<String>,
    pub source: Option<CodeRef>,
}
```

```rust
pub struct TraitAssociatedConstDef {
    pub name: String,
    pub type_text: String,
    pub default_value_text: Option<String>,
    pub source: Option<CodeRef>,
}
```

关键定义：

- `trait_registry` 包含所有对 public API 有意义的 trait 节点。
- 不只包含 `pub trait`。
- 也包含：
  - re-export trait
  - 出现在 public 签名、bound、where clause、associated type、associated const 中的外部 trait
  - 出现在 public 签名中的本 crate 私有 trait 约束
  - public-API-relevant trait 的直接 supertrait
- `direct_supertrait_ids` 只保存直接关系，不保存传递闭包。
- `used_by_api_ids` / `used_by_trait_ids` / `used_by_type_ids` 是显式抽取出的曝光边，不视为纯缓存。

## TraitImplInfo

```rust
pub struct TraitImplInfo {
    pub trait_impl_id: TraitImplId,
    pub target_type_id: TypeId,
    pub trait_ref_text: String,
    pub for_type_text: String,
    pub trait_id: TraitId,
    pub trait_name: String,
    pub trait_canonical_path: String,
    pub trait_origin: TraitOrigin,
    pub source: CodeRef,
    #[serde(default)]
    pub associated_type_bindings: Vec<TraitAssociatedTypeBinding>,
    #[serde(default)]
    pub associated_const_bindings: Vec<TraitAssociatedConstBinding>,
    pub where_clauses: Vec<String>,
    #[serde(default)]
    pub cfg_attrs: Vec<String>,
    pub is_unsafe: bool,
}
```

```rust
pub struct TraitAssociatedTypeBinding {
    pub name: String,
    pub generic_params: Vec<String>,
    pub where_clauses: Vec<String>,
    pub bounds: Vec<String>,
    pub assigned_type: Option<String>,
    pub source: Option<CodeRef>,
}
```

```rust
pub struct TraitAssociatedConstBinding {
    pub name: String,
    pub value_text: Option<String>,
    pub source: Option<CodeRef>,
}
```

说明：

- `trait_impl_registry` 是 public trait impl surface 的基础表。
- Phase 1 不保留 `surface_bucket` 这类语义分桶字段。
- `cfg_attrs` 只保存原始 attr 文本，不做 feature 图解释。
- `is_unsafe` 表示 impl header 是否为 `unsafe impl`。

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
    Symbol(SymbolId),
    Trait(TraitId),
}
```

约束：

- `anchor` 是主归属点。
- `involved_api_ids` 表示关联范围。
- `snippet` 仅作为证据文本，不参与唯一性判断，不作为主键。
- Phase 1 不保存 capability tags。

## RiskFacts

```rust
pub struct RiskFacts {
    #[serde(default)]
    pub extern_abi_apis: Vec<ExternAbiApiFact>,
    #[serde(default)]
    pub repr_types: Vec<TypeLayoutFact>,
    #[serde(default)]
    pub drop_impl_types: Vec<TypeId>,
    #[serde(default)]
    pub explicit_panic_sites: Vec<ExplicitPanicSiteFact>,
    #[serde(default)]
    pub borrowed_return_apis: Vec<BorrowedReturnFact>,
}
```

```rust
pub struct ExternAbiApiFact {
    pub api_id: ApiId,
    pub abi: String,
    pub source: Option<CodeRef>,
}
```

```rust
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

```rust
pub struct ExplicitPanicSiteFact {
    pub owner: RiskOwner,
    pub panic_kind: String,
    pub source: CodeRef,
}

pub enum RiskOwner {
    Api(ApiId),
    TraitImpl(TraitImplId),
}
```

```rust
pub struct BorrowedReturnFact {
    pub api_id: ApiId,
    pub return_type_text: String,
    pub from_self: bool,
    pub from_arg_positions: Vec<u32>,
    pub lifetime_names: Vec<String>,
    pub source: Option<CodeRef>,
}
```

说明：

- `extern_abi_apis` 表示 public API 使用了非 Rust ABI。
- 它不表示 crate 内部调用了 C / C++ / system library。
- `explicit_panic_sites` 只记录显式 panic-like 证据，例如：
  - `panic!`
  - `assert!`
  - `assert_eq!`
  - `assert_ne!`
  - `debug_assert!`
  - `debug_assert_eq!`
  - `debug_assert_ne!`
  - `unreachable!`
  - `todo!`
  - `unimplemented!`
- `borrowed_return_apis` 只记录签名层面的借用来源事实，不表示风险评级。
- `borrowed_return_apis` 必须同时覆盖：
  - 显式 lifetime 名匹配
  - Rust lifetime elision

## Not In Phase 1

以下字段或概念明确不进入本版 schema：

- builder 标签
- example capability tags
- panic risk 评级
- lifetime risk 评级
- view / handle / cursor / builder 这类语义分类
- trait impl surface bucket
- 推荐用法与模式识别

## Recommended Extraction Order

- 先补 `DocSections`
- 再补 `extern_abi_apis`
- 再补 trait / trait impl 的 associated const
- 再补 `cfg_attrs`
- 再补 `return_shape`
- 再补 `borrowed_return_apis`
- 再补 `explicit_panic_sites`
- 最后补 `TypeInfo.fields / variants / hidden` 相关字段

## Final Boundary

一句话定义：

Phase 1 只产出“可定位、可回证、可复现”的源码事实与确定性结构化结果；Phase 2 才产出解释性、分类性、风险性、策略性的语义结论。
